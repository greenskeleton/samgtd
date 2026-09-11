# Architecture direction

Status: Milestone 001 vertical slice implemented. See `docs/milestone-001.md`
for scope and `docs/protocol.md` for what's implemented vs. still designed.

## Implemented crates

```text
crates/
  samgtd-domain/   empty stub — no GTD entity types yet (see "Existing
                    database compatibility" below)
  samgtd-crdt/      Task + RootIndex Automerge documents, sync message
                    generate/receive (docs/adr/0001, docs/adr/0003)
  samgtd-store/     SQLite-backed document blob store (its own database
                    file, NOT .local/current-gtd.sqlite)
  samgtd-api/       HealthResponse contract type
  samgtdd/          Axum/Tokio daemon: /health, persistence init on
                    startup, graceful shutdown
```

`samgtd-domain` stays empty: GTD entity types (Task/Project/Category/etc.)
are deferred until they're needed for real data import, per
`docs/existing-database.md` and `docs/adr/0003-existing-database-coexistence.md`.
Milestone 001's Task representation lives directly in `samgtd-crdt` as
`TaskFields`/`TaskDocument`.

## Existing database compatibility

`.local/current-gtd.sqlite` is a real, externally-owned database (see
`docs/existing-database.md`). `samgtd-store` does not open it or touch its
schema. `docs/adr/0003-existing-database-coexistence.md` defines a purely
additive identity-mapping strategy (`samgtd_identity` table) for eventually
importing its rows, and explicitly defers the live-coexistence question
(is the owning legacy app still writing to it?) pending operator
confirmation.

## Proven by test

`crates/samgtd-store/tests/convergence.rs` exercises the full Milestone 001
scenario end-to-end: two peers create different tasks offline, sync via
Automerge sync messages, converge, survive a simulated restart (drop +
reopen `Store` from the same file), and re-sync idempotently without
duplication.

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
