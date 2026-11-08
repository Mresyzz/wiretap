use std::time::Instant;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use reqwest::Method;

use crate::model::{KeyValue, TrafficEntry};
use crate::store::TrafficStore;

pub fn generate_curl(entry: &TrafficEntry) -> String {
    let mut parts = vec![format!("curl -X {}", entry.method)];
    
    // Headers (过滤掉 host, content-length 等易冲突头)
    for kv in &entry.request_headers {
        let key_lower = kv.key.to_lowercase();
        if key_lower == "host" || key_lower == "content-length" {
            continue;
        }
        parts.push(format!("-H \"{}: {}\"", kv.key, kv.value.replace('"', "\\\"")));
    }
    
    // Body
    if let Some(body) = &entry.request_body {
        if !body.is_empty() {
            parts.push(format!("-d \"{}\"", body.replace('"', "\\\"").replace('\n', "")));
        }
    }
    
    // URL
    parts.push(format!("\"{}\"", entry.full_url));
    
    parts.join(" ")
}

pub async fn execute_replay(
    entry: &TrafficEntry,
    override_url: Option<&str>,
    store: &TrafficStore,
) -> Result<TrafficEntry, String> {
    let client = reqwest::Client::builder()
        .build()
        .map_err(|e| format!("Failed to create client: {}", e))?;

    let target_url = override_url.unwrap_or(&entry.full_url);
    let method = Method::from_bytes(entry.method.as_bytes())
        .map_err(|e| format!("Invalid HTTP method: {}", e))?;

    let mut headers = HeaderMap::new();
    for kv in &entry.request_headers {
        let key_lower = kv.key.to_lowercase();
        if key_lower == "host" || key_lower == "content-length" {
            continue;
        }
        if let (Ok(name), Ok(val)) = (
            HeaderName::from_bytes(kv.key.as_bytes()),
            HeaderValue::from_str(&kv.value),
        ) {
            headers.insert(name, val);
        }
    }

    let start_time = Instant::now();
    let mut req_builder = client.request(method, target_url).headers(headers);

    if let Some(body) = &entry.request_body {
        req_builder = req_builder.body(body.clone());
    }

    let resp = req_builder
        .send()
        .await
        .map_err(|e| format!("Replay request failed: {}", e))?;

    let duration_ms = start_time.elapsed().as_millis() as u64;
    let status = resp.status().as_u16();
    let status_text = resp
        .status()
        .canonical_reason()
        .unwrap_or("Unknown")
        .to_string();

    let mut resp_headers = Vec::new();
    for (k, v) in resp.headers() {
        if let Ok(v_str) = v.to_str() {
            resp_headers.push(KeyValue {
                key: k.as_str().to_string(),
                value: v_str.to_string(),
            });
        }
    }

    let resp_bytes = resp.bytes().await.unwrap_or_default();
    let resp_body = String::from_utf8(resp_bytes.to_vec()).ok();

    let replay_entry = TrafficEntry::new(
        entry.method.clone(),
        entry.path.clone(),
        entry.query.clone(),
        target_url.to_string(),
        entry.request_headers.clone(),
        entry.request_body.clone(),
        status,
        status_text,
        resp_headers,
        resp_body,
        duration_ms,
        true, // is_replay = true
    );

    store.add(replay_entry.clone()).await;
    Ok(replay_entry)
}
