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

## Implementation status (2026-09-11)

Implemented: workspace, loopback default, health, persisted node/dataset UUIDs,
explicit empty-replica provisioning, task create/read/update, atomic creation,
restart persistence, native Automerge WebSocket sync, structured logs, and
shutdown notification for sync sessions with bounded server drain.

Convergence is tested at two levels:

- Lower-layer, in-process (`crates/samgtd-store/tests/convergence.rs`,
  `crates/samgtd-crdt/src/lib.rs` unit tests): separate SQLite files, shared
  state, disconnected same-task edits (plus a same-field conflict), native
  sync, equal full projections/document heads, reload, and repeated sync.
- **Real two-process acceptance** (`crates/samgtdd/tests/two_process_acceptance.rs`,
  and the runnable demo at `crates/samgtdd/examples/demo.rs`, via
  `cargo run -p samgtdd --example demo`): two (briefly, three) actual
  `samgtdd` binaries, each with its own temporary SQLite store, bound to real
  loopback TCP ports. HTTP and `/sync` WebSocket traffic goes over real
  sockets — no in-process router, direct `Service` call, or snapshot
  copying. This exercises exactly the offline-edit/reconnect/conflict/
  restart/kill scenario `README.md`'s "First milestone definition" and this
  work's acceptance instructions describe. Full mapping of every required
  item to specific evidence, exact reproduction commands, and observed
  results: `docs/milestone-001-acceptance.md`.

The pre-existing daemon-level test (`crates/samgtdd/tests/vertical_slice.rs`)
still exercises real HTTP upgrades/WebSocket framing over in-memory duplex
streams, and a subprocess test there still verifies committed task/identity
survival after `kill`; both are preserved unchanged and remain useful
lower-fidelity/faster coverage alongside the real-TCP acceptance test.

Remaining acceptance/operational gaps (see `docs/milestone-001-acceptance.md`
for the full, evidenced list):
- Hosted GitHub CI has not been observed for this changeset; this session had
  no network/`gh` access. Local checks (including the real two-process test)
  are separate evidence, not a substitute.
- Automatic outbound peer connection/retry is not implemented; a client or
  relay connects sync endpoints — `samgtd-testkit`'s relay (used by the
  acceptance test and demo) is that relay, documented as a demo/test harness,
  not a production peer manager.
- Large-dataset scheduling, total session/memory limits and refined HTTP error
  status codes remain follow-up hardening.
- Third-party GitHub Actions remain pinned to major-version tags rather than
  commit SHAs (no network/`gh` access in this session to look up and verify
  current SHAs); the Rust toolchain itself is now pinned to an exact verified
  version (`1.98.1`) in both `rust-toolchain.toml` and CI, no longer floating
  `stable`.
