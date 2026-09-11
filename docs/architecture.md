# Architecture

Milestone 001 now has a durable application and HTTP/WebSocket path. See
milestone-001.md for verification limits and ADR 0004 for lifecycle decisions.

- samgtd-domain: task values and UUID/status validation; no transport/storage.
- samgtd-crdt: per-task and root Automerge documents, fallible projection,
  serialization, and native sync; depends on domain only.
- samgtd-store: opaque document blobs, atomic batches, ownership marker and
  process lock; no domain/CRDT/transport dependency.
- samgtd-api: health, identity, provisioning, patch and sync contracts.
- samgtdd: serialized application operations in blocking tasks, Axum routes,
  connection-scoped sync states and change/shutdown notifications.

Automerge is authoritative. No independent SQL task model competes with it.
New task creation saves root membership and entity atomically. A failed operation
cannot leave changed application documents visible in memory. Replicas retain
stable node/dataset identity across restart; explicit provisioning preserves
shared root map history.

The supplied legacy database is read-only input, never the daemon's store.
Unrecognized existing files are refused. Import and live coexistence remain
deferred. Category UUID references are validated without requiring category
documents, which are not yet implemented.

Tests cover offline same-task edits, same-field conflicts, native unknown-task
transfer, complete fields/heads after disk reload, transaction rollback, store
ownership, HTTP task writes, real WebSocket upgrades over duplex streams and
acknowledged-write survival after subprocess termination — plus, in
`crates/samgtdd/tests/two_process_acceptance.rs`, the same scenario again
over **real loopback TCP and real subprocesses** (two, briefly three,
independent `samgtdd` binaries), using the `samgtd-testkit` crate
(`crates/samgtd-testkit/`) for process spawn/readiness, a minimal HTTP client,
a real WebSocket sync relay, post-mortem read-only store inspection, and
acceptance reporting. That crate is a `[dev-dependencies]`-only harness,
never a production dependency of `samgtdd`; it's shared between that test and
the runnable demo (`crates/samgtdd/examples/demo.rs`,
`cargo run -p samgtdd --example demo`). See
`docs/milestone-001-acceptance.md` for the full evidence mapping.
Hosted CI has not been observed for this changeset (no network/`gh` access in
this session). No production-scale memory or throughput claim is made.

Loopback remains the default. Authentication is required before production remote
write access; no custom networking cryptography is implemented.
