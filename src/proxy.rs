use std::collections::VecDeque;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Instant;

use http_body_util::BodyExt;
use hudsucker::certificate_authority::RcgenAuthority;
use hudsucker::hyper::body::Bytes;
use hudsucker::hyper::{Request as HyperRequest, Response as HyperResponse, StatusCode as HyperStatusCode, Uri};
use hudsucker::rcgen::{BasicConstraints, CertificateParams, IsCa, Issuer, KeyPair};
use hudsucker::rustls::crypto::aws_lc_rs;
use hudsucker::tokio_tungstenite::tungstenite::Message;
use hudsucker::{Body, HttpContext, HttpHandler, Proxy, RequestOrResponse, WebSocketContext, WebSocketHandler};

use crate::model::{KeyValue, TrafficEntry};
use crate::store::TrafficStore;

/// 最大用于记录与文本化解析的 Body 大小 (10MB)，超出部分截断，防 OOM DoS
pub const MAX_RECORD_BODY_SIZE: usize = 10 * 1024 * 1024;

#[derive(Clone)]
pub struct WireTapProxyHandler {
    pub target_base_url: Arc<tokio::sync::RwLock<Option<String>>>,
    pub target_process: Arc<tokio::sync::RwLock<Option<String>>>,
    pub store: TrafficStore,
    pending: Arc<tokio::sync::Mutex<VecDeque<PendingRequest>>>,
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
    process_name: Option<String>,
    process_pid: Option<u32>,
    should_record: bool,
}

impl WireTapProxyHandler {
    pub fn new(store: TrafficStore, target_base_url: Option<String>) -> Self {
        Self {
            target_base_url: Arc::new(tokio::sync::RwLock::new(target_base_url)),
            target_process: Arc::new(tokio::sync::RwLock::new(None)),
            store,
            pending: Arc::new(tokio::sync::Mutex::new(VecDeque::new())),
        }
    }

    pub fn new_shared(
        store: TrafficStore,
        target_base_url: Arc<tokio::sync::RwLock<Option<String>>>,
        target_process: Arc<tokio::sync::RwLock<Option<String>>>,
    ) -> Self {
        Self {
            target_base_url,
            target_process,
            store,
            pending: Arc::new(tokio::sync::Mutex::new(VecDeque::new())),
        }
    }

    pub async fn process_request(
        &mut self,
        req: HyperRequest<Body>,
    ) -> RequestOrResponse {
        self.process_request_with_port(req, None).await
    }

    pub async fn process_request_with_port(
        &mut self,
        mut req: HyperRequest<Body>,
        client_port: Option<u16>,
    ) -> RequestOrResponse {
        let start_time = Instant::now();
        let method = req.method().to_string();
        let original_uri = req.uri().clone();
        let path = original_uri.path().to_string();
        let query = original_uri.query().map(|q| q.to_string());

        let proc_info = client_port.and_then(crate::process::get_process_by_port);
        let (proc_name, proc_pid) = match &proc_info {
            Some(p) => (Some(p.name.clone()), Some(p.pid)),
            None => (None, None),
        };

        let filter_guard = self.target_process.read().await;
        let should_record = match filter_guard.as_deref() {
            None => true,
            Some(f) if f.is_empty() || f.eq_ignore_ascii_case("ALL") => true,
            Some(f) => {
                if let Some(ref p) = proc_info {
                    p.name.to_lowercase().contains(&f.to_lowercase()) || p.pid.to_string() == f
                } else {
                    false
                }
            }
        };
        drop(filter_guard);

        let target_opt = self.target_base_url.read().await.clone();

        let full_url = match &target_opt {
            Some(target) => {
                let target = target.trim_end_matches('/');
                match &query {
                    Some(q) => format!("{}{}{}?{}", target, if path.starts_with('/') { "" } else { "/" }, path, q),
                    None => format!("{}{}{}", target, if path.starts_with('/') { "" } else { "/" }, path),
                }
            }
            None => {
                if original_uri.host().is_some() {
                    original_uri.to_string()
                } else {
                    let host_header = req.headers().get(hudsucker::hyper::header::HOST)
                        .and_then(|v| v.to_str().ok())
                        .unwrap_or("localhost");
                    match &query {
                        Some(q) => format!("http://{}{}{}?{}", host_header, if path.starts_with('/') { "" } else { "/" }, path, q),
                        None => format!("http://{}{}{}", host_header, if path.starts_with('/') { "" } else { "/" }, path),
                    }
                }
            }
        };

        if let Some(target) = &target_opt {
            let target = target.trim_end_matches('/');
            let target_uri_str = match &query {
                Some(q) => format!("{}{}{}?{}", target, if path.starts_with('/') { "" } else { "/" }, path, q),
                None => format!("{}{}{}", target, if path.starts_with('/') { "" } else { "/" }, path),
            };
            if let Ok(new_uri) = target_uri_str.parse::<Uri>() {
                *req.uri_mut() = new_uri.clone();
                // 自动更新 Host 请求头
                if let Some(host) = new_uri.host() {
                    let host_val = if let Some(p) = new_uri.port_u16() {
                        format!("{}:{}", host, p)
                    } else {
                        host.to_string()
                    };
                    if let Ok(v) = hudsucker::hyper::header::HeaderValue::from_str(&host_val) {
                        req.headers_mut().insert(hudsucker::hyper::header::HOST, v);
                    }
                }
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
        } else if body_bytes.len() > MAX_RECORD_BODY_SIZE {
            Some(format!("[Large Body: {} bytes, truncated for memory safety]", body_bytes.len()))
        } else {
            String::from_utf8(body_bytes.to_vec()).ok()
        };

        self.pending.lock().await.push_back(PendingRequest {
            start_time,
            method,
            path,
            query,
            full_url,
            request_headers: req_headers_list,
            request_body: req_body_str,
            process_name: proc_name,
            process_pid: proc_pid,
            should_record,
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
        } else if body_bytes.len() > MAX_RECORD_BODY_SIZE {
            Some(format!("[Large Body: {} bytes, truncated for memory safety]", body_bytes.len()))
        } else {
            String::from_utf8(body_bytes.to_vec()).ok()
        };

        if let Some(pending) = self.pending.lock().await.pop_front() {
            if pending.should_record {
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
                ).with_process(pending.process_name, pending.process_pid);
                self.store.add(entry).await;
            }
        }

        HyperResponse::from_parts(parts, Body::from(body_bytes))
    }
}

impl HttpHandler for WireTapProxyHandler {
    async fn handle_request(
        &mut self,
        ctx: &HttpContext,
        req: HyperRequest<Body>,
    ) -> RequestOrResponse {
        let client_port = ctx.client_addr.port();
        self.process_request_with_port(req, Some(client_port)).await
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
        if let Some(pending) = self.pending.lock().await.pop_front() {
            if pending.should_record {
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
                ).with_process(pending.process_name, pending.process_pid);
                self.store.add(entry).await;
            }
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

/// 构建本地自签 CA 根证书与私钥，返回 (RcgenAuthority, PEM文本)
pub fn build_ca() -> (RcgenAuthority, String) {
    let key_pair = KeyPair::generate().expect("generate ca key");
    let mut params = CertificateParams::default();
    params.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
    params.distinguished_name.push(hudsucker::rcgen::DnType::CommonName, "WireTap CA");
    let cert = params.self_signed(&key_pair).expect("generate self-signed ca cert");
    let pem = cert.pem();
    let issuer = Issuer::from_ca_cert_pem(&pem, key_pair).expect("create issuer");
    let authority = RcgenAuthority::new(issuer, 1000, aws_lc_rs::default_provider());
    (authority, pem)
}

pub async fn start_hudsucker_proxy(
    listen_port: u16,
    target_base_url: Arc<tokio::sync::RwLock<Option<String>>>,
    target_process: Arc<tokio::sync::RwLock<Option<String>>>,
    store: TrafficStore,
    ca: RcgenAuthority,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let addr = SocketAddr::from(([0, 0, 0, 0], listen_port));
    let handler = WireTapProxyHandler::new_shared(store, target_base_url, target_process);

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
