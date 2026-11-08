pub mod ui;

use std::convert::Infallible;
use std::sync::Arc;
use std::time::Duration;

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::response::{Html, Json};
use axum::routing::{get, post};
use axum::Router;
use futures_util::stream::Stream;
use tokio_stream::wrappers::BroadcastStream;
use tokio_stream::StreamExt;

use crate::replay::{execute_replay, generate_curl};
use crate::store::TrafficStore;

#[derive(Clone)]
pub struct WebState {
    pub store: TrafficStore,
    pub target_base_url: Option<String>,
}

pub fn create_web_router(state: WebState) -> Router {
    Router::new()
        .route("/", get(index_handler))
        .route("/api/list", get(list_handler))
        .route("/api/events", get(sse_handler))
        .route("/api/replay/:id", post(replay_handler))
        .route("/api/curl/:id", get(curl_handler))
        .route("/api/clear", post(clear_handler))
        .with_state(Arc::new(state))
}

async fn index_handler() -> Html<&'static str> {
    Html(ui::INDEX_HTML)
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
