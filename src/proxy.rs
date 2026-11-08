use std::sync::Arc;
use std::time::Instant;

use axum::extract::{Request, State};
use axum::http::{HeaderMap, HeaderName, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use reqwest::Method;

use crate::model::{KeyValue, TrafficEntry};
use crate::store::TrafficStore;

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
