pub mod ui;

use std::convert::Infallible;
use std::sync::Arc;
use std::time::Duration;

use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::response::{IntoResponse, Json};
use axum::routing::{get, post};
use axum::Router;
use futures_util::stream::Stream;
use rust_embed::RustEmbed;
use tokio_stream::wrappers::BroadcastStream;
use tokio_stream::StreamExt;

use crate::replay::{execute_replay, generate_curl};
use crate::store::TrafficStore;

#[derive(RustEmbed)]
#[folder = "static/"]
pub struct Assets;

#[derive(Clone)]
pub struct WebState {
    pub store: TrafficStore,
    pub target_base_url: Option<String>,
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

    let replay_result = execute_replay(
        &entry,
        state.target_base_url.as_deref(),
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

async fn ca_cert_handler() -> impl IntoResponse {
    let cert_pem = "-----BEGIN CERTIFICATE-----\r\nMIIDXTCCAkWgAwIBAgIJALa891WireTapLocalRootCA001MA0GCSqGSIb3DQEBCwUA\r\nMBoxCzAJBgNVBAYTAkNOMRAwDgYDVQQIDAdCZWlqaW5nMRAwDgYDVQQKDAdXaXJl\r\nVGFwMRMwEQYDVQQDDApXaXJlVGFwIENBMB4XDTI2MDEwMTAwMDAwMFoXDTM2MDEw\r\nMTAwMDAwMFowGzEZMBcGA1UEAwwQV2lyZVRhcCBMb2NhbCBDQTAggZ8wDQYJKoZI\r\nhvcNAQEBBQADgY0AMIGJAoGBAMl5eY3U50w9i3FqK2vD1g3fX9mY9X9y0w3u8vX7\r\n-----END CERTIFICATE-----\r\n";
    (
        [
            (axum::http::header::CONTENT_TYPE, "application/x-x509-ca-cert"),
            (axum::http::header::CONTENT_DISPOSITION, "attachment; filename=\"wiretap-ca.crt\""),
        ],
        cert_pem,
    )
}

async fn status_handler(State(state): State<Arc<WebState>>) -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "status": "running",
        "target": state.target_base_url,
        "traffic_count": state.store.list().await.len(),
    }))
}
