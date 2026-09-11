# Architecture direction

Status: bootstrap / intentionally incomplete.

```text
                         future clients
                ┌───────────┬───────────┐
                │           │           │
              TUI/Web    Android     Voice/MCP
                │           │           │
                └───────────┼───────────┘
                            │
                 HTTP + CRDT sync API
                            │
                    ┌───────▼───────┐
                    │    samgtdd    │
                    │  Axum/Tokio   │
                    └───────┬───────┘
                            │
              ┌─────────────┼─────────────┐
              │             │             │
              ▼             ▼             ▼
          GTD domain    CRDT engine     storage
                        Automerge        SQLite

Remote mobile path (future):

Android client
      │
      │ HTTP/WebSocket over tailnet
      ▼
Tailscale / WireGuard+Noise control plane
      │
      ▼
private samgtdd listener
```

## Boundary rule

Tailscale is a transport/access mechanism, not the domain protocol. `samgtdd` should function perfectly on loopback/LAN without Tailscale.

## Sync rule

CRDT synchronization must be transport-independent above a reliable ordered byte/message stream. WebSocket is the initial transport.

## Authentication

Phase 1 may run unauthenticated on loopback.

Before LAN/Tailscale write access is considered production-ready, add an explicit authentication/authorization design. Do not equate possession of a LAN address with authorization.
