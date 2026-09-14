//! A transparent sync relay over two real loopback TCP WebSocket
//! connections.
//!
//! This is the "small runnable client/relay" the milestone calls for: it
//! dials each daemon's `/sync` endpoint as a real WebSocket client and
//! forwards native Automerge sync frames (opaque JSON text frames, per
//! `docs/protocol.md`) bidirectionally, verbatim. It never decodes,
//! inspects, or otherwise interprets the frames, and it never reads either
//! peer's SQLite store — the two daemons synchronize each other entirely
//! through the protocol, exactly as two independent real replicas would
//! over a LAN or a future Tailscale link. A production peer manager (dialing
//! policy, retry, discovery, authentication) is explicitly out of scope for
//! this milestone; this relay only stands in for "some link between two
//! already-addressable peers exists."
//!
//! Used by both the two-process acceptance test
//! (`crates/samgtdd/tests/two_process_acceptance.rs`) and the standalone
//! demo (`crates/samgtdd/examples/demo.rs`).

use anyhow::Context;
use futures_util::{SinkExt, StreamExt};
use std::net::SocketAddr;
use tokio::{net::TcpStream, sync::oneshot, task::JoinHandle};
use tokio_tungstenite::{tungstenite::Message, WebSocketStream};

async fn dial(addr: SocketAddr) -> anyhow::Result<WebSocketStream<TcpStream>> {
    let tcp = TcpStream::connect(addr)
        .await
        .with_context(|| format!("connect to {addr}/sync"))?;
    tcp.set_nodelay(true).ok();
    let (ws, _response) = tokio_tungstenite::client_async(format!("ws://{addr}/sync"), tcp)
        .await
        .with_context(|| format!("WebSocket handshake with {addr}/sync"))?;
    Ok(ws)
}

/// A live relay between two daemons' `/sync` endpoints. Dropping this without
/// calling [`Relay::disconnect`] aborts the forwarding task, simulating an
/// unclean network loss rather than a deliberate disconnect.
pub struct Relay {
    stop: oneshot::Sender<()>,
    task: JoinHandle<()>,
}

impl Relay {
    /// Dial both peers' `/sync` endpoints over real TCP and begin forwarding.
    pub async fn connect(a: SocketAddr, b: SocketAddr) -> anyhow::Result<Self> {
        let mut ws_a = dial(a).await?;
        let mut ws_b = dial(b).await?;
        let (stop, mut stop_rx) = oneshot::channel();
        let task = tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = &mut stop_rx => break,
                    next = ws_a.next() => match next {
                        Some(Ok(msg)) if msg.is_text() => { if ws_b.send(msg).await.is_err() { break; } }
                        Some(Ok(Message::Ping(_) | Message::Pong(_))) => {}
                        _ => break,
                    },
                    next = ws_b.next() => match next {
                        Some(Ok(msg)) if msg.is_text() => { if ws_a.send(msg).await.is_err() { break; } }
                        Some(Ok(Message::Ping(_) | Message::Pong(_))) => {}
                        _ => break,
                    },
                }
            }
            let _ = ws_a.close(None).await;
            let _ = ws_b.close(None).await;
        });
        Ok(Self { stop, task })
    }

    /// Deliberately close both connections (simulating the sync link going
    /// down while both daemons keep running) and wait for the forwarding
    /// task to finish, bounded by the caller's own timeout via `tokio::time`.
    pub async fn disconnect(self) -> anyhow::Result<()> {
        let _ = self.stop.send(());
        self.task.await.context("relay task panicked")
    }
}
