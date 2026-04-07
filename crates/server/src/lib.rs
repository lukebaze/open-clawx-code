//! Axum HTTP+SSE server for headless OCX mode.
//!
//! Exposes the runtime via `POST /command`, `GET /events` (SSE),
//! `GET /health`, and `GET /models`.

use std::convert::Infallible;
use std::sync::Arc;

use axum::{
    extract::State,
    response::{
        sse::{Event as SseEvent, KeepAlive},
        Json, Sse,
    },
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;
use tokio_stream::wrappers::BroadcastStream;
use tokio_stream::StreamExt;

/// Server shared state.
pub struct ServerState {
    /// Broadcast channel for SSE events.
    event_tx: broadcast::Sender<String>,
}

/// Health check response.
#[derive(Serialize)]
struct HealthResponse {
    status: &'static str,
    version: &'static str,
}

/// Command received via POST.
#[derive(Deserialize)]
struct CommandPayload {
    #[serde(rename = "type")]
    cmd_type: String,
    #[serde(default)]
    text: String,
}

/// Build the axum router.
pub fn build_router(state: Arc<ServerState>) -> Router {
    Router::new()
        .route("/health", get(health_handler))
        .route("/events", get(sse_handler))
        .route("/command", post(command_handler))
        .with_state(state)
}

/// Create server state with a broadcast channel.
#[must_use]
pub fn create_state() -> (Arc<ServerState>, broadcast::Sender<String>) {
    let (event_tx, _) = broadcast::channel::<String>(256);
    let state = Arc::new(ServerState {
        event_tx: event_tx.clone(),
    });
    (state, event_tx)
}

/// Start the server on the given address.
pub async fn start_server(bind_addr: &str) -> anyhow::Result<()> {
    let (state, event_tx) = create_state();
    let app = build_router(state);

    let listener = tokio::net::TcpListener::bind(bind_addr).await?;
    eprintln!("OCX server listening on {bind_addr}");
    eprintln!("  POST /command  — send commands");
    eprintln!("  GET  /events   — SSE event stream");
    eprintln!("  GET  /health   — health check");

    // Keep event_tx alive for the lifetime of the server
    let _tx = event_tx;

    axum::serve(listener, app).await?;
    Ok(())
}

async fn health_handler() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok",
        version: env!("CARGO_PKG_VERSION"),
    })
}

async fn sse_handler(
    State(state): State<Arc<ServerState>>,
) -> Sse<impl tokio_stream::Stream<Item = Result<SseEvent, Infallible>>> {
    let rx = state.event_tx.subscribe();
    let stream = BroadcastStream::new(rx).filter_map(|result| match result {
        Ok(data) => Some(Ok(SseEvent::default().data(data))),
        Err(_) => None, // skip lagged messages
    });

    Sse::new(stream).keep_alive(KeepAlive::default())
}

async fn command_handler(
    State(state): State<Arc<ServerState>>,
    Json(payload): Json<CommandPayload>,
) -> Json<serde_json::Value> {
    // Echo back as an event for now (real wiring connects to orchestrator)
    let event_json = serde_json::json!({
        "type": "command_received",
        "cmd_type": payload.cmd_type,
        "text": payload.text,
    });
    let _ = state.event_tx.send(event_json.to_string());

    Json(serde_json::json!({
        "status": "accepted",
        "cmd_type": payload.cmd_type,
    }))
}
