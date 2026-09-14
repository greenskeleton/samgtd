#![forbid(unsafe_code)]

//! `samgtdd`: the local-first GTD daemon.
//!
pub mod config;
pub mod service;
pub mod transport;

use axum::{routing::get, Json, Router};
use samgtd_api::health::HealthResponse;
use std::future::IntoFuture;

pub use config::Config;

/// Build the daemon's HTTP router. Split out from `run` so integration tests
/// can exercise it without binding a real socket.
pub fn build_router<S: Clone + Send + Sync + 'static>() -> Router<S> {
    Router::new().route("/health", get(health))
}

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse::ok())
}

/// Bind and serve until a shutdown signal is received.
pub async fn run(config: Config) -> anyhow::Result<()> {
    let path = config.db_path.clone();
    let service = tokio::task::spawn_blocking(move || service::Service::open(&path)).await??;
    let state = transport::App::new(service);
    let app = transport::router(state.clone());

    tracing::info!(addr = %config.bind_addr, "starting samgtdd");

    let listener = tokio::net::TcpListener::bind(config.bind_addr).await?;
    let local_addr = listener.local_addr()?;
    tracing::info!(addr = %local_addr, "listening");
    if let Some(ready_path) = &config.ready_path {
        announce_ready(ready_path, local_addr)?;
    }
    let (shutdown_started, shutdown_received) = tokio::sync::oneshot::channel();
    let server = axum::serve(listener, app)
        .with_graceful_shutdown(async move {
            shutdown_signal().await;
            state.shutdown();
            let _ = shutdown_started.send(());
        })
        .into_future();
    tokio::pin!(server);
    tokio::select! {
        result = &mut server => result?,
        _ = shutdown_received => {
            tokio::time::timeout(std::time::Duration::from_secs(10), &mut server).await??;
        }
    }

    Ok(())
}

/// Write the real bound address to `path` atomically (write-then-rename), so
/// a reader either sees nothing yet or a complete address, never a partial
/// write.
fn announce_ready(path: &std::path::Path, addr: std::net::SocketAddr) -> anyhow::Result<()> {
    let tmp = path.with_extension("tmp");
    std::fs::write(&tmp, addr.to_string())?;
    std::fs::rename(&tmp, path)?;
    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        if let Err(error) = tokio::signal::ctrl_c().await {
            tracing::error!(%error, "signal handler failed");
        }
    };

    #[cfg(unix)]
    let terminate = async {
        match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
            Ok(mut signal) => {
                signal.recv().await;
            }
            Err(error) => {
                tracing::error!(%error, "signal handler failed");
            }
        }
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    tracing::info!("shutdown signal received");
}
