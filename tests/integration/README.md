# Integration tests

Cargo integration tests live in the relevant crate's tests directory:

- crates/samgtd-store/tests/convergence.rs: independent SQLite replicas,
  native sync-only transfer, offline same-task conflicts, full fields and heads
  after restart, and idempotent resync.
- crates/samgtdd/tests/vertical_slice.rs: HTTP operations and real
  Hyper/Axum WebSocket upgrades over Tokio duplex streams, transparent peer
  relay, offline edits, edit-after-idle notifications, disk reload, and a
  subprocess kill after an acknowledged durable write.
- **crates/samgtdd/tests/two_process_acceptance.rs**: the Milestone 001
  acceptance scenario, over **real loopback TCP** and **real `samgtdd`
  subprocesses** — not in-memory streams or in-process routers. Two (briefly,
  three) actual daemon binaries, each with its own temporary SQLite store;
  HTTP via a minimal client over a real `TcpStream`, sync via a real
  WebSocket relay dialing each daemon's `/sync` endpoint. Covers offline
  task creation/discovery, independent-field and same-field-conflict
  convergence, idle-connection propagation, two separate restart-and-reload
  cycles (with root/per-task Automerge head comparison via read-only,
  post-exit SQLite inspection — never while the owning daemon is still
  running, and never used to drive the other peer's sync), a `SIGKILL`
  durability check against a real acknowledged HTTP write, and a bounded
  graceful shutdown with an active sync connection. See
  `docs/milestone-001-acceptance.md` for the full check-by-check mapping and
  `crates/samgtd-testkit/` for the reusable spawn/HTTP/relay/report harness
  it shares with `crates/samgtdd/examples/demo.rs` (`cargo run -p samgtdd
  --example demo` — the standalone, human-runnable version of the same
  scenario, which also writes the acceptance results artifact/transcript).

Crate integration tests can span crates and launch subprocesses. Bound TCP,
two-process sync was previously believed blocked by socket-binding
restrictions in the implementation environment (see the now-superseded note
in `docs/milestone-001.md`'s earlier revisions); this was re-verified and
found not to hold in the environment this work ran in — real loopback TCP
bind/connect/accept all work, and `two_process_acceptance.rs` above is the
result. If a future environment genuinely denies socket binding, that test
will fail loudly with captured process diagnostics (see
`samgtd_testkit::process::Daemon::wait_ready`) rather than silently skip.
