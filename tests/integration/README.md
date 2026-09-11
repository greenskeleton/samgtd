# Integration tests

The Milestone 001 two-peer convergence test lives at
`crates/samgtd-store/tests/convergence.rs`, not here: Cargo integration
tests must live inside a crate's own `tests/` directory to run under
`cargo test`, and this test exercises `samgtd-store` + `samgtd-crdt`
together, so that's the right crate boundary for it (see `AGENTS.md`,
"CRDT representation and domain projection must be separable/testable").

This top-level directory is kept empty, matching the layout in `README.md`,
for tests that genuinely span multiple crates end-to-end in a way no single
crate's `tests/` directory can host (e.g. a future test that drives the
actual `samgtdd` binary as a subprocess over the real WebSocket sync
endpoint once `docs/protocol.md`'s framing is wired up).
