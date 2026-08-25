//! The Reader process.
//!
//! A separate binary on purpose. The brief's beat six kills it on stage and
//! the transfer must still hold — so there has to be a real PID to kill and
//! a real socket for the API to fail to reach. An in-process task with a
//! debug switch would make the recovery a story we tell rather than one the
//! system performs, and rule three says triggering the failure is fine but
//! faking the response is not.
//!
//! It is also the least-privileged component in the system, and its
//! dependency list is the proof: `airlock-core` and `airlock-agents`, no
//! ledger, no transaction store, no policy decisions. It reads text and
//! returns a typed signal. Nothing it returns is trusted — the API
//! re-validates every field before anything looks at it.
//!
//! ```text
//! cargo run -p airlock-reader     # :8081
//! ```

use airlock_core::PressureSignal;
use axum::{extract::Json, http::StatusCode, routing::post, Router};
use serde::Deserialize;

#[derive(Deserialize)]
struct ReadRequest {
    text: String,
}

/// Longest message we will look at. Past this, it is not an SMS.
const MAX_MESSAGE_BYTES: usize = 4_096;

async fn read(Json(req): Json<ReadRequest>) -> Result<Json<PressureSignal>, StatusCode> {
    if req.text.len() > MAX_MESSAGE_BYTES {
        return Err(StatusCode::PAYLOAD_TOO_LARGE);
    }
    // Length only. Message content is never written to traces.
    tracing::info!(agent = "reader", bytes = req.text.len(), "reading message");
    Ok(Json(airlock_agents::analyse_message(&req.text)))
}

async fn health() -> &'static str {
    "ok"
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info".into()),
        )
        .init();

    let port: u16 = std::env::var("READER_PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(8081);

    let app = Router::new()
        .route("/read", post(read))
        .route("/health", axum::routing::get(health));

    let listener = tokio::net::TcpListener::bind(("127.0.0.1", port))
        .await
        .expect("reader could not bind");

    // Printed so the demo can kill it by PID without hunting for it.
    tracing::info!(pid = std::process::id(), port, "reader listening");
    println!("airlock-reader listening on :{port} (pid {})", std::process::id());

    axum::serve(listener, app).await.expect("reader stopped");
}
