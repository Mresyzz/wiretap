use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Instant;

use axum::extract::{Request, State};
use axum::http::{HeaderMap, HeaderName, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use http_body_util::BodyExt;
use hudsucker::certificate_authority::RcgenAuthority;
use hudsucker::hyper::body::Bytes;
use hudsucker::hyper::{Request as HyperRequest, Response as HyperResponse, StatusCode as HyperStatusCode, Uri};
use hudsucker::rcgen::{BasicConstraints, CertificateParams, IsCa, Issuer, KeyPair};
use hudsucker::rustls::crypto::aws_lc_rs;
use hudsucker::tokio_tungstenite::tungstenite::Message;
use hudsucker::{Body, HttpContext, HttpHandler, Proxy, RequestOrResponse, WebSocketContext, WebSocketHandler};
use reqwest::Method;

use crate::model::{KeyValue, TrafficEntry};
use crate::store::TrafficStore;

#[derive(Clone)]
pub struct WireTapProxyHandler {
    pub target_base_url: Arc<tokio::sync::RwLock<Option<String>>>,
    pub store: TrafficStore,
    pending: Option<PendingRequest>,
}

#[derive(Clone)]
struct PendingRequest {
    start_time: Instant,
    method: String,
    path: String,
    query: Option<String>,
    full_url: String,
    request_headers: Vec<KeyValue>,
    request_body: Option<String>,
}

impl WireTapProxyHandler {
    pub fn new(store: TrafficStore, target_base_url: Option<String>) -> Self {
        Self {
            target_base_url: Arc::new(tokio::sync::RwLock::new(target_base_url)),
            store,
            pending: None,
        }
    }

    pub fn new_shared(store: TrafficStore, target_base_url: Arc<tokio::sync::RwLock<Option<String>>>) -> Self {
        Self {
            target_base_url,
            store,
            pending: None,
        }
    }

    pub async fn process_request(
        &mut self,
        mut req: HyperRequest<Body>,
    ) -> RequestOrResponse {
        let start_time = Instant::now();
        let method = req.method().to_string();
        let original_uri = req.uri().clone();
        let path = original_uri.path().to_string();
        let query = original_uri.query().map(|q| q.to_string());

        let target_opt = self.target_base_url.read().await.clone();

        let full_url = match &target_opt {
            Some(target) => {
                let target = target.trim_end_matches('/');
                match &query {
                    Some(q) => format!("{}{}{}?{}", target, if path.starts_with('/') { "" } else { "/" }, path, q),
                    None => format!("{}{}{}", target, if path.starts_with('/') { "" } else { "/" }, path),
                }
            }
            None => original_uri.to_string(),
        };

        if let Some(target) = &target_opt {
            let target = target.trim_end_matches('/');
            let target_uri_str = match &query {
                Some(q) => format!("{}{}{}?{}", target, if path.starts_with('/') { "" } else { "/" }, path, q),
                None => format!("{}{}{}", target, if path.starts_with('/') { "" } else { "/" }, path),
            };
            if let Ok(new_uri) = target_uri_str.parse::<Uri>() {
                *req.uri_mut() = new_uri;
            }
        }

        let mut req_headers_list = Vec::new();
        for (k, v) in req.headers() {
            req_headers_list.push(KeyValue {
                key: k.as_str().to_string(),
                value: v.to_str().unwrap_or("").to_string(),
            });
        }

        let (parts, body) = req.into_parts();
        let body_bytes = match body.collect().await {
            Ok(collected) => collected.to_bytes(),
            Err(_) => Bytes::new(),
        };

        let req_body_str = if body_bytes.is_empty() {
            None
        } else {
            String::from_utf8(body_bytes.to_vec()).ok()
        };

        self.pending = Some(PendingRequest {
            start_time,
            method,
            path,
            query,
            full_url,
            request_headers: req_headers_list,
            request_body: req_body_str,
        });

        let new_req = HyperRequest::from_parts(parts, Body::from(body_bytes));
        new_req.into()
    }

    pub async fn process_response(
        &mut self,
        res: HyperResponse<Body>,
    ) -> HyperResponse<Body> {
        let (parts, body) = res.into_parts();
        let status_u16 = parts.status.as_u16();
        let status_text = parts.status.canonical_reason().unwrap_or("Unknown").to_string();

        let mut resp_headers_list = Vec::new();
        for (k, v) in &parts.headers {
            resp_headers_list.push(KeyValue {
                key: k.as_str().to_string(),
                value: v.to_str().unwrap_or("").to_string(),
            });
        }

        let body_bytes = match body.collect().await {
            Ok(collected) => collected.to_bytes(),
            Err(_) => Bytes::new(),
        };

        let resp_body_str = if body_bytes.is_empty() {
            None
        } else {
            String::from_utf8(body_bytes.to_vec()).ok()
        };

        if let Some(pending) = self.pending.take() {
            let duration_ms = pending.start_time.elapsed().as_millis() as u64;
            let entry = TrafficEntry::new(
                pending.method,
                pending.path,
                pending.query,
                pending.full_url,
                pending.request_headers,
                pending.request_body,
                status_u16,
                status_text,
                resp_headers_list,
                resp_body_str,
                duration_ms,
                false,
            );
            self.store.add(entry).await;
        }

        HyperResponse::from_parts(parts, Body::from(body_bytes))
    }
}

impl HttpHandler for WireTapProxyHandler {
    async fn handle_request(
        &mut self,
        _ctx: &HttpContext,
        req: HyperRequest<Body>,
    ) -> RequestOrResponse {
        self.process_request(req).await
    }

    async fn handle_response(
        &mut self,
        _ctx: &HttpContext,
        res: HyperResponse<Body>,
    ) -> HyperResponse<Body> {
        self.process_response(res).await
    }

    async fn handle_error(
        &mut self,
        _ctx: &HttpContext,
        err: hudsucker::hyper_util::client::legacy::Error,
    ) -> HyperResponse<Body> {
        let error_msg = format!("WireTap Proxy Error: Upstream unreachable: {}", err);
        if let Some(pending) = self.pending.take() {
            let duration_ms = pending.start_time.elapsed().as_millis() as u64;
            let entry = TrafficEntry::new(
                pending.method,
                pending.path,
                pending.query,
                pending.full_url,
                pending.request_headers,
                pending.request_body,
                502,
                "Bad Gateway (Upstream Unreachable)".to_string(),
                vec![KeyValue {
                    key: "Content-Type".to_string(),
                    value: "text/plain".to_string(),
                }],
                Some(error_msg.clone()),
                duration_ms,
                false,
            );
            self.store.add(entry).await;
        }

        HyperResponse::builder()
            .status(HyperStatusCode::BAD_GATEWAY)
            .header("Content-Type", "text/plain")
            .body(Body::from(error_msg))
            .unwrap_or_else(|_| HyperResponse::new(Body::empty()))
    }
}

impl WebSocketHandler for WireTapProxyHandler {
    async fn handle_message(
        &mut self,
        _ctx: &WebSocketContext,
        msg: Message,
    ) -> Option<Message> {
        Some(msg)
    }
}

pub fn build_ca() -> RcgenAuthority {
    let key_pair = KeyPair::generate().expect("generate ca key");
    let mut params = CertificateParams::default();
    params.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
    params.distinguished_name.push(hudsucker::rcgen::DnType::CommonName, "WireTap CA");
    let cert = params.self_signed(&key_pair).expect("generate self-signed ca cert");
    let issuer = Issuer::from_ca_cert_pem(&cert.pem(), key_pair).expect("create issuer");
    RcgenAuthority::new(issuer, 1000, aws_lc_rs::default_provider())
}

pub async fn start_hudsucker_proxy(
    listen_port: u16,
    target_base_url: Arc<tokio::sync::RwLock<Option<String>>>,
    store: TrafficStore,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let ca = build_ca();
    let addr = SocketAddr::from(([0, 0, 0, 0], listen_port));
    let handler = WireTapProxyHandler::new_shared(store, target_base_url);

    let proxy = Proxy::builder()
        .with_addr(addr)
        .with_ca(ca)
        .with_rustls_connector(aws_lc_rs::default_provider())
        .with_http_handler(handler.clone())
        .with_websocket_handler(handler)
        .build()?;

    proxy.start().await?;
    Ok(())
}

#[derive(Clone)]
pub struct ProxyConfig {
    pub target_base_url: String,
    pub store: TrafficStore,
}

pub async fn proxy_handler(
    State(config): State<Arc<ProxyConfig>>,
    req: Request,
) -> Response {
    let method = req.method().clone();
    let uri = req.uri().clone();
    let path = uri.path().to_string();
    let query = uri.query().map(|q| q.to_string());

    // 格式化 upstream URL
    let target_base = config.target_base_url.trim_end_matches('/');
    let target_url = match &query {
        Some(q) => format!("{}{}{}?{}", target_base, if path.starts_with('/') { "" } else { "/" }, path, q),
        None => format!("{}{}{}", target_base, if path.starts_with('/') { "" } else { "/" }, path),
    };

    // 收集请求头
    let mut req_headers_list = Vec::new();
    let mut client_headers = reqwest::header::HeaderMap::new();

    for (k, v) in req.headers() {
        let key_str = k.as_str().to_string();
        let val_str = v.to_str().unwrap_or("").to_string();
        req_headers_list.push(KeyValue {
            key: key_str.clone(),
            value: val_str.clone(),
        });

        // 转发头过滤
        let key_lower = key_str.to_lowercase();
        if key_lower != "host" && key_lower != "content-length" {
            if let (Ok(h_name), Ok(h_val)) = (
                HeaderName::from_bytes(key_str.as_bytes()),
                HeaderValue::from_str(&val_str),
            ) {
                client_headers.insert(h_name, h_val);
            }
        }
    }

    // 收集请求体
    let body_bytes = match axum::body::to_bytes(req.into_body(), 10 * 1024 * 1024).await {
        Ok(b) => b,
        Err(e) => {
            return (StatusCode::BAD_REQUEST, format!("Failed to read request body: {}", e)).into_response();
        }
    };

    let req_body_str = if body_bytes.is_empty() {
        None
    } else {
        String::from_utf8(body_bytes.to_vec()).ok()
    };

    let client = reqwest::Client::builder()
        .build()
        .unwrap_or_else(|_| reqwest::Client::new());

    let reqwest_method = Method::from_bytes(method.as_str().as_bytes())
        .unwrap_or(Method::GET);

    let start_time = Instant::now();

    let mut req_builder = client
        .request(reqwest_method, &target_url)
        .headers(client_headers);

    if !body_bytes.is_empty() {
        req_builder = req_builder.body(body_bytes.clone());
    }

    let upstream_result = req_builder.send().await;
    let duration_ms = start_time.elapsed().as_millis() as u64;

    match upstream_result {
        Ok(resp) => {
            let status_u16 = resp.status().as_u16();
            let status_text = resp
                .status()
                .canonical_reason()
                .unwrap_or("Unknown")
                .to_string();

            let mut resp_headers_list = Vec::new();
            let mut axum_resp_headers = HeaderMap::new();

            for (k, v) in resp.headers() {
                let key_str = k.as_str().to_string();
                let val_str = v.to_str().unwrap_or("").to_string();
                resp_headers_list.push(KeyValue {
                    key: key_str.clone(),
                    value: val_str.clone(),
                });

                if let (Ok(h_name), Ok(h_val)) = (
                    axum::http::HeaderName::from_bytes(key_str.as_bytes()),
                    axum::http::HeaderValue::from_str(&val_str),
                ) {
                    axum_resp_headers.insert(h_name, h_val);
                }
            }

            let resp_bytes = resp.bytes().await.unwrap_or_default();
            let resp_body_str = String::from_utf8(resp_bytes.to_vec()).ok();

            // 存入 TrafficStore
            let entry = TrafficEntry::new(
                method.to_string(),
                path,
                query,
                target_url,
                req_headers_list,
                req_body_str,
                status_u16,
                status_text,
                resp_headers_list,
                resp_body_str,
                duration_ms,
                false,
            );
            config.store.add(entry).await;

            let status_code = StatusCode::from_u16(status_u16).unwrap_or(StatusCode::OK);
            let mut res = Response::new(axum::body::Body::from(resp_bytes));
            *res.status_mut() = status_code;
            *res.headers_mut() = axum_resp_headers;
            res
        }
        Err(err) => {
            // 目标服务不可达，记录 502 Bad Gateway 错误
            let error_msg = format!("WireTap Proxy Error: Failed to reach upstream [{}] - {}", target_url, err);
            let entry = TrafficEntry::new(
                method.to_string(),
                path,
                query,
                target_url,
                req_headers_list,
                req_body_str,
                502,
                "Bad Gateway (Upstream Unreachable)".to_string(),
                vec![KeyValue {
                    key: "Content-Type".to_string(),
                    value: "text/plain".to_string(),
                }],
                Some(error_msg.clone()),
                duration_ms,
                false,
            );
            config.store.add(entry).await;

            (StatusCode::BAD_GATEWAY, error_msg).into_response()
        }
    }
}
