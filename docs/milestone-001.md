# Milestone 001 — CRDT daemon vertical slice

## Required

- Cargo workspace.
- Daemon starts on `127.0.0.1` by default.
- Health endpoint.
- Stable node/dataset identity persisted locally.
- Create/read/update one minimal Task aggregate.
- Automerge document persisted across restart.
- WebSocket sync endpoint using Automerge sync messages.
- Two-peer integration test proving convergence after offline concurrent edits.
- Structured logs.
- Graceful shutdown.
- No public network bind by default.
- GitHub CI green.

## Nice to have

- OpenAPI generation.
- read-only projections in relational tables.
- metrics endpoint.

## Explicitly deferred

- complete GTD schema;
- TUI/web UI;
- Android;
- Tailscale automation;
- voice;
- MCP;
- Redmine;
- multi-user authorization;
- Internet-facing deployment.
