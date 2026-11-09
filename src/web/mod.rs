pub mod ui;

use std::convert::Infallible;
use std::sync::Arc;
use std::time::Duration;

use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::response::{IntoResponse, Json};
use axum::routing::{get, post};
use axum::Router;
use futures_util::stream::Stream;
use rust_embed::RustEmbed;
use tokio_stream::wrappers::BroadcastStream;
use tokio_stream::StreamExt;

use crate::config::WireTapConfig;
use crate::replay::{execute_replay, generate_curl};
use crate::scanner::{get_dev_ports, scan_ports_list, scan_ports_range, ScanQuery, ScannedPort};
use crate::store::TrafficStore;

#[derive(RustEmbed)]
#[folder = "static/"]
pub struct Assets;

#[derive(Clone)]
pub struct WebState {
    pub store: TrafficStore,
    pub target_base_url: Arc<tokio::sync::RwLock<Option<String>>>,
    pub target_process: Arc<tokio::sync::RwLock<Option<String>>>,
    pub config: Arc<tokio::sync::RwLock<WireTapConfig>>,
    pub config_path: String,
    pub ca_cert_pem: Arc<tokio::sync::RwLock<String>>,
}

impl WebState {
    pub fn new(store: TrafficStore, target_base_url: Option<String>) -> Self {
        let cfg = WireTapConfig {
            target_url: target_base_url.clone().unwrap_or_else(|| "http://localhost:3000".to_string()),
            ..Default::default()
        };
        let (_, cert_pem) = crate::proxy::build_ca();
        let target_proc = cfg.target_process.clone();
        Self {
            store,
            target_base_url: Arc::new(tokio::sync::RwLock::new(target_base_url)),
            target_process: Arc::new(tokio::sync::RwLock::new(target_proc)),
            config: Arc::new(tokio::sync::RwLock::new(cfg)),
            config_path: crate::config::DEFAULT_CONFIG_FILE.to_string(),
            ca_cert_pem: Arc::new(tokio::sync::RwLock::new(cert_pem)),
        }
    }
}

pub fn create_web_router(state: WebState) -> Router {
    Router::new()
        .route("/", get(index_handler))
        .route("/api/ws", get(ws_handler))
        .route("/api/list", get(list_handler))
        .route("/api/events", get(sse_handler))
        .route("/api/replay/:id", post(replay_handler))
        .route("/api/curl/:id", get(curl_handler))
        .route("/api/clear", post(clear_handler))
        .route("/api/ca.crt", get(ca_cert_handler))
        .route("/api/status", get(status_handler))
        .route("/api/config", get(get_config_handler).post(update_config_handler))
        .route("/api/processes", get(list_processes_handler))
        .route("/api/target-process", get(get_target_process_handler).post(set_target_process_handler))
        .route("/api/scan-ports", get(scan_ports_handler))
        .route("/static/*file", get(static_handler))
        .with_state(Arc::new(state))
}

async fn index_handler() -> impl IntoResponse {
    match Assets::get("index.html") {
        Some(content) => (
            [(axum::http::header::CONTENT_TYPE, "text/html; charset=utf-8")],
            content.data.into_owned(),
        )
            .into_response(),
        None => axum::response::Html(ui::INDEX_HTML).into_response(),
    }
}

async fn static_handler(Path(file): Path<String>) -> impl IntoResponse {
    match Assets::get(&file) {
        Some(content) => {
            let mime = mime_guess::from_path(&file).first_or_octet_stream();
            (
                [(axum::http::header::CONTENT_TYPE, mime.as_ref())],
                content.data.into_owned(),
            )
                .into_response()
        }
        None => (StatusCode::NOT_FOUND, "File not found").into_response(),
    }
}

async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<WebState>>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

async fn handle_socket(mut socket: WebSocket, state: Arc<WebState>) {
    let mut rx = state.store.subscribe();
    while let Ok(entry) = rx.recv().await {
        if let Ok(json) = serde_json::to_string(&entry) {
            if socket.send(Message::Text(json)).await.is_err() {
                break;
            }
        }
    }
}

async fn list_handler(State(state): State<Arc<WebState>>) -> Json<Vec<crate::model::TrafficEntry>> {
    let list = state.store.list().await;
    Json(list)
}

async fn sse_handler(
    State(state): State<Arc<WebState>>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let rx = state.store.subscribe();
    let stream = BroadcastStream::new(rx).filter_map(|res| match res {
        Ok(entry) => {
            if let Ok(json) = serde_json::to_string(&entry) {
                Some(Ok(Event::default().data(json)))
            } else {
                None
            }
        }
        Err(_) => None,
    });

    Sse::new(stream).keep_alive(KeepAlive::new().interval(Duration::from_secs(15)))
}

async fn replay_handler(
    State(state): State<Arc<WebState>>,
    Path(id): Path<String>,
) -> Result<Json<crate::model::TrafficEntry>, (StatusCode, String)> {
    let entry = state
        .store
        .get(&id)
        .await
        .ok_or_else(|| (StatusCode::NOT_FOUND, "Traffic entry not found".to_string()))?;

    let target_opt = state.target_base_url.read().await.clone();
    let replay_result = execute_replay(
        &entry,
        target_opt.as_deref(),
        &state.store,
    )
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;

    Ok(Json(replay_result))
}

async fn curl_handler(
    State(state): State<Arc<WebState>>,
    Path(id): Path<String>,
) -> Result<String, (StatusCode, String)> {
    let entry = state
        .store
        .get(&id)
        .await
        .ok_or_else(|| (StatusCode::NOT_FOUND, "Traffic entry not found".to_string()))?;

    Ok(generate_curl(&entry))
}

async fn clear_handler(State(state): State<Arc<WebState>>) -> StatusCode {
    state.store.clear().await;
    StatusCode::OK
}

async fn ca_cert_handler(State(state): State<Arc<WebState>>) -> impl IntoResponse {
    let cert_pem = state.ca_cert_pem.read().await.clone();
    (
        [
            (axum::http::header::CONTENT_TYPE, "application/x-x509-ca-cert"),
            (axum::http::header::CONTENT_DISPOSITION, "attachment; filename=\"wiretap-ca.crt\""),
        ],
        cert_pem,
    )
}

async fn status_handler(State(state): State<Arc<WebState>>) -> Json<serde_json::Value> {
    let target = state.target_base_url.read().await.clone();
    let target_proc = state.target_process.read().await.clone();
    let cfg = state.config.read().await.clone();
    Json(serde_json::json!({
        "status": "running",
        "target": target,
        "target_process": target_proc,
        "listen_port": cfg.listen_port,
        "ui_port": cfg.ui_port,
        "max_history": cfg.max_history,
        "traffic_count": state.store.list().await.len(),
    }))
}

async fn get_config_handler(State(state): State<Arc<WebState>>) -> Json<WireTapConfig> {
    let cfg = state.config.read().await.clone();
    Json(cfg)
}

#[derive(serde::Deserialize)]
pub struct UpdateConfigPayload {
    pub target_url: Option<String>,
    pub listen_port: Option<u16>,
    pub ui_port: Option<u16>,
    pub max_history: Option<usize>,
    pub target_process: Option<String>,
}

async fn update_config_handler(
    State(state): State<Arc<WebState>>,
    Json(payload): Json<UpdateConfigPayload>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let mut cfg = state.config.write().await;

    if let Some(target) = payload.target_url {
        let trimmed = target.trim();
        let normalized = if trimmed.starts_with("http://") || trimmed.starts_with("https://") {
            trimmed.to_string()
        } else {
            format!("http://{}", trimmed)
        };
        *state.target_base_url.write().await = Some(normalized.clone());
        cfg.target_url = normalized;
    }

    if let Some(port) = payload.listen_port {
        cfg.listen_port = port;
    }
    if let Some(ui_port) = payload.ui_port {
        cfg.ui_port = ui_port;
    }
    if let Some(max_history) = payload.max_history {
        cfg.max_history = max_history;
    }
    if let Some(target_proc) = payload.target_process {
        let trimmed = target_proc.trim();
        let val = if trimmed.is_empty() || trimmed.eq_ignore_ascii_case("ALL") {
            None
        } else {
            Some(trimmed.to_string())
        };
        *state.target_process.write().await = val.clone();
        cfg.target_process = val;
    }

    if let Err(e) = cfg.save_to_file(&state.config_path) {
        eprintln!("Warning: failed to auto-save config file: {}", e);
    }

    Ok(Json(serde_json::json!({
        "status": "ok",
        "message": "配置已自动保存并实时生效",
        "config": *cfg
    })))
}

async fn list_processes_handler() -> Json<Vec<crate::process::ProcessInfo>> {
    let processes = tokio::task::spawn_blocking(crate::process::list_active_network_processes)
        .await
        .unwrap_or_default();
    Json(processes)
}

#[derive(serde::Deserialize, serde::Serialize)]
pub struct TargetProcessPayload {
    pub process: Option<String>,
}

async fn get_target_process_handler(State(state): State<Arc<WebState>>) -> Json<TargetProcessPayload> {
    let proc = state.target_process.read().await.clone();
    Json(TargetProcessPayload { process: proc })
}

async fn set_target_process_handler(
    State(state): State<Arc<WebState>>,
    Json(payload): Json<TargetProcessPayload>,
) -> Json<serde_json::Value> {
    let target = match payload.process.as_deref().map(str::trim) {
        None | Some("") | Some("ALL") => None,
        Some(name) => Some(name.to_string()),
    };
    *state.target_process.write().await = target.clone();

    let mut cfg = state.config.write().await;
    cfg.target_process = target.clone();
    let _ = cfg.save_to_file(&state.config_path);

    Json(serde_json::json!({
        "status": "ok",
        "target_process": target
    }))
}

async fn scan_ports_handler(
    Query(query): Query<ScanQuery>,
) -> Json<Vec<ScannedPort>> {
    let mode = query.mode.as_deref().unwrap_or("dev");
    let ports = match mode {
        "all" => {
            // Concurrent full range scan 1..=65535 with 500 tasks
            scan_ports_range(1, 65535, 500).await
        }
        "custom" => {
            let start = query.start.unwrap_or(1);
            let end = query.end.unwrap_or(10000);
            scan_ports_range(start, end, 300).await
        }
        _ => {
            // Dev mode: fast scan of common dev ports
            let dev_ports = get_dev_ports();
            scan_ports_list(dev_ports, 50).await
        }
    };
    Json(ports)
}
