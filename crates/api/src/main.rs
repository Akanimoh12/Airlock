//! The API server.
//!
//! ```text
//! cargo run -p airlock-api                    # stub Reader, fully offline
//! READER_URL=http://127.0.0.1:8081 \
//!   cargo run -p airlock-api                  # talk to the Reader process
//! ```
//!
//! Stub mode is the default on purpose: the flow has to run with no API key
//! and no second process, so the product surface is never blocked and we
//! keep an offline path if the venue wifi dies.

use airlock_agents::Reader;
use airlock_api::{router, AppState};
use std::net::{IpAddr, Ipv4Addr};
use std::time::Duration;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info".into()),
        )
        .init();

    let reader = match std::env::var("READER_URL") {
        Ok(url) if !url.trim().is_empty() => {
            tracing::info!(%url, "using the Reader process");
            Reader::remote(url, Duration::from_secs(2))
        }
        _ => {
            tracing::info!("using the stub Reader (offline, no API key)");
            Reader::Stub
        }
    };

    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(8080);

    // Loopback locally, all interfaces in a container.
    //
    // Binding `0.0.0.0` unconditionally would quietly expose the demo wallet
    // to anyone on the venue wifi, which is not a thing to discover on stage.
    // A platform that routes traffic to us sets `BIND_ALL`; nothing else
    // does.
    let host: IpAddr = if std::env::var("BIND_ALL").is_ok_and(|v| !v.trim().is_empty()) {
        Ipv4Addr::UNSPECIFIED.into()
    } else {
        Ipv4Addr::LOCALHOST.into()
    };

    let state = AppState::new(reader);
    let listener = tokio::net::TcpListener::bind((host, port))
        .await
        .expect("api could not bind");

    tracing::info!(%host, port, "airlock-api listening");
    println!("airlock-api listening on {host}:{port}");

    axum::serve(listener, router(state))
        .await
        .expect("api stopped");
}
