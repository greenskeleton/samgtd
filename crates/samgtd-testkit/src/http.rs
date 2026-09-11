//! A deliberately minimal HTTP/1.1 JSON client over a real `TcpStream`.
//!
//! This exists only so the acceptance scenario exercises the daemon's public
//! HTTP surface over an actual socket, without pulling a full HTTP client
//! crate into the dependency graph. Every request sends `Connection: close`
//! and reads to EOF, which is safe because Axum/Hyper's server closes the
//! connection after responding to such a request.

use anyhow::Context;
use serde_json::Value;
use std::{net::SocketAddr, time::Duration};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
};

pub struct Response {
    pub status: u16,
    pub body: Value,
}

pub async fn request(
    addr: SocketAddr,
    method: &str,
    path: &str,
    body: Option<&Value>,
    bound: Duration,
) -> anyhow::Result<Response> {
    tokio::time::timeout(bound, request_inner(addr, method, path, body))
        .await
        .with_context(|| format!("{method} {path} to {addr} timed out after {bound:?}"))?
}

async fn request_inner(
    addr: SocketAddr,
    method: &str,
    path: &str,
    body: Option<&Value>,
) -> anyhow::Result<Response> {
    let mut stream = TcpStream::connect(addr)
        .await
        .with_context(|| format!("connect to {addr}"))?;
    stream.set_nodelay(true).ok();

    let body_str = body.map(|b| b.to_string()).unwrap_or_default();
    let mut request = format!("{method} {path} HTTP/1.1\r\nHost: {addr}\r\nConnection: close\r\n");
    if !body_str.is_empty() {
        request.push_str("Content-Type: application/json\r\n");
    }
    request.push_str(&format!("Content-Length: {}\r\n\r\n", body_str.len()));
    request.push_str(&body_str);

    stream
        .write_all(request.as_bytes())
        .await
        .context("write request")?;
    // No half-close here: the request carries an exact Content-Length, so
    // the server can tell it's complete without needing our EOF, and
    // `Connection: close` makes the server close its side once it has sent
    // the response — that's what `read_to_end` below waits for.

    let mut raw = Vec::new();
    stream
        .read_to_end(&mut raw)
        .await
        .context("read response")?;
    parse_response(&raw)
}

fn parse_response(raw: &[u8]) -> anyhow::Result<Response> {
    let split = raw
        .windows(4)
        .position(|w| w == b"\r\n\r\n")
        .with_context(|| {
            format!(
                "malformed HTTP response: no header terminator in {} bytes: {:?}",
                raw.len(),
                String::from_utf8_lossy(&raw[..raw.len().min(200)])
            )
        })?;
    let header_str =
        std::str::from_utf8(&raw[..split]).context("HTTP response headers are not UTF-8")?;
    let mut declared_body = &raw[split + 4..];
    let mut lines = header_str.split("\r\n");
    let status_line = lines.next().context("missing HTTP status line")?;
    let status: u16 = status_line
        .split_whitespace()
        .nth(1)
        .context("malformed status line")?
        .parse()
        .context("non-numeric status code")?;
    for line in lines {
        if let Some((name, value)) = line.split_once(':') {
            if name.eq_ignore_ascii_case("content-length") {
                if let Ok(len) = value.trim().parse::<usize>() {
                    declared_body = &declared_body[..declared_body.len().min(len)];
                }
            }
        }
    }
    let body = if declared_body.is_empty() {
        Value::Null
    } else {
        serde_json::from_slice(declared_body).with_context(|| {
            format!(
                "response body is not valid JSON: {}",
                String::from_utf8_lossy(declared_body)
            )
        })?
    };
    Ok(Response { status, body })
}

pub async fn get(addr: SocketAddr, path: &str, bound: Duration) -> anyhow::Result<Response> {
    request(addr, "GET", path, None, bound).await
}

pub async fn post(
    addr: SocketAddr,
    path: &str,
    body: &Value,
    bound: Duration,
) -> anyhow::Result<Response> {
    request(addr, "POST", path, Some(body), bound).await
}

pub async fn patch(
    addr: SocketAddr,
    path: &str,
    body: &Value,
    bound: Duration,
) -> anyhow::Result<Response> {
    request(addr, "PATCH", path, Some(body), bound).await
}

/// Poll `GET /health` until it returns 200, bounded — the readiness check
/// used once a daemon's TCP listener is known to be bound.
pub async fn wait_health(addr: SocketAddr, bound: Duration) -> anyhow::Result<()> {
    let start = tokio::time::Instant::now();
    loop {
        let attempt = match get(addr, "/health", Duration::from_secs(2)).await {
            Ok(resp) if resp.status == 200 => return Ok(()),
            Ok(resp) => anyhow::anyhow!("unexpected status {}", resp.status),
            Err(err) => err,
        };
        if start.elapsed() > bound {
            return Err(attempt.context(format!("{addr} did not report healthy within {bound:?}")));
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
}

/// Poll `GET {path}` until the JSON body equals `expected`, bounded.
pub async fn wait_for_value(
    addr: SocketAddr,
    path: &str,
    expected: &Value,
    bound: Duration,
) -> anyhow::Result<Value> {
    let start = tokio::time::Instant::now();
    let mut last_seen: Option<Value> = None;
    loop {
        if let Ok(resp) = get(addr, path, Duration::from_secs(2)).await {
            if resp.status == 200 {
                if &resp.body == expected {
                    return Ok(resp.body);
                }
                last_seen = Some(resp.body);
            }
        }
        if start.elapsed() > bound {
            anyhow::bail!(
                "{addr}{path} did not converge to expected value within {bound:?}\nexpected: {expected}\nlast seen: {}",
                last_seen.map(|v| v.to_string()).unwrap_or_else(|| "<no successful response>".into())
            );
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
}
