use chrono::Local;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyValue {
    pub key: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrafficEntry {
    pub id: String,
    pub timestamp: i64,
    pub time_str: String,
    pub method: String,
    pub path: String,
    pub query: Option<String>,
    pub full_url: String,
    pub request_headers: Vec<KeyValue>,
    pub request_body: Option<String>,
    pub request_size_bytes: usize,
    
    pub status: u16,
    pub status_text: String,
    pub response_headers: Vec<KeyValue>,
    pub response_body: Option<String>,
    pub response_content_type: Option<String>,
    pub response_size_bytes: usize,
    
    pub duration_ms: u64,
    pub is_replay: bool,
}

impl TrafficEntry {
    pub fn new(
        method: String,
        path: String,
        query: Option<String>,
        full_url: String,
        request_headers: Vec<KeyValue>,
        request_body: Option<String>,
        status: u16,
        status_text: String,
        response_headers: Vec<KeyValue>,
        response_body: Option<String>,
        duration_ms: u64,
        is_replay: bool,
    ) -> Self {
        let now = Local::now();
        let timestamp = now.timestamp_millis();
        let time_str = now.format("%H:%M:%S%.3f").to_string();
        
        let request_size_bytes = request_body.as_ref().map(|b| b.len()).unwrap_or(0);
        let response_size_bytes = response_body.as_ref().map(|b| b.len()).unwrap_or(0);
        
        let response_content_type = response_headers
            .iter()
            .find(|kv| kv.key.eq_ignore_ascii_case("content-type"))
            .map(|kv| kv.value.clone());

        Self {
            id: Uuid::new_v4().to_string(),
            timestamp,
            time_str,
            method,
            path,
            query,
            full_url,
            request_headers,
            request_body,
            request_size_bytes,
            status,
            status_text,
            response_headers,
            response_body,
            response_content_type,
            response_size_bytes,
            duration_ms,
            is_replay,
        }
    }
}
