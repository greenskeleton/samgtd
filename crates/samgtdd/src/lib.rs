#![forbid(unsafe_code)]

//! `samgtdd`: the local-first GTD daemon.
//!
//! This binary currently wires up only the HTTP boundary (`/health`). It
//! does not yet load or persist any GTD entities: `AGENTS.md` ("Existing
//! database compatibility") requires inventorying the existing SQLite
//! database before this daemon initializes persistence or CRDT documents.

pub mod config;
mod persistence;

use axum::{routing::get, Json, Router};
use samgtd_api::health::HealthResponse;

pub use config::Config;
pub use persistence::init_persistence;

/// Build the daemon's HTTP router. Split out from `run` so integration tests
/// can exercise it without binding a real socket.
pub fn build_router() -> Router {
    Router::new().route("/health", get(health))
}

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse::ok())
}

/// Bind and serve until a shutdown signal is received.
pub async fn run(config: Config) -> anyhow::Result<()> {
    let root = init_persistence(&config.db_path)?;
    tracing::info!(
        db_path = %config.db_path.display(),
        known_tasks = root.task_uuids()?.len(),
        "persistence initialized",
    );

    let app = build_router();

    tracing::info!(addr = %config.bind_addr, "starting samgtdd");

    let listener = tokio::net::TcpListener::bind(config.bind_addr).await?;
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        // Startup-time signal registration, not a request path: failure here
        // is an unrecoverable environment error (AGENTS.md's unwrap/expect
        // rule targets request paths, not process bring-up).
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    tracing::info!("shutdown signal received");
}
