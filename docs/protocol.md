# Sync protocol

Status: framing defined; **not yet wired to an Axum WebSocket route** (see
"What's implemented vs. defined" below).

## Scope

This defines how two `samgtdd` peers exchange Automerge sync messages over a
WebSocket. It does not define authentication/authorization (see
`docs/adr/0002-private-network.md`: that's a separate, not-yet-designed
layer — possession of a LAN/tailnet address is not authorization) or the
live-coexistence protocol with the existing SQLite database (explicitly
deferred in `docs/adr/0003-existing-database-coexistence.md`).

## Document model recap

Per `docs/adr/0001-automerge.md` and `docs/adr/0003-existing-database-coexistence.md`,
a dataset is: one root index document (tracks known entity UUIDs by type)
plus one Automerge document per entity. A sync connection must therefore
carry messages for *multiple* documents, not just one.

## Framing

Each WebSocket message is one JSON envelope (binary WS frame containing
UTF-8 JSON with a base64-encoded payload — see "Why JSON+base64" below):

```jsonc
{
  "doc_type": "root_index" | "task",   // more doc_types added as entities are added
  "doc_id": "default" | "<task-uuid>", // "default" for the singleton root index
  "message": "<base64, automerge::sync::Message::encode() bytes>"
}
```

This maps directly onto `samgtd-crdt`'s existing, tested API:
`TaskDocument`/`RootIndexDocument::generate_sync_message()` produce the
`message` bytes; `receive_sync_message()` consumes them. One
`automerge::sync::State` is kept per `(peer, doc_type, doc_id)` tuple on each
side of the connection.

### Connection lifecycle

1. On connect, both sides know only the root index's `doc_id` ("default")
   in advance. Sync the root index first.
2. As `receive_sync_message` for the root index reveals new task UUIDs
   (`RootIndexDocument::task_uuids()` grows), start a sync exchange for each
   new `("task", uuid)` pair the receiving side doesn't yet have — creating
   an empty placeholder to sync into, exactly as
   `crates/samgtd-store/tests/convergence.rs`'s `sync_tasks` does today
   in-process.
3. Either side may send a frame for any `(doc_type, doc_id)` at any time;
   there is no required ordering beyond "the root index must be synced
   before an unfamiliar task's `doc_id` means anything to the receiver."
4. The connection is idle (no frames sent) once every known document's
   `generate_sync_message` returns `None` on both sides. This is the same
   termination condition already proven in the two-peer integration test.

### Why JSON+base64, not a tighter binary framing

`AGENTS.md` says to avoid speculative abstractions; a hand-rolled binary
envelope (length-prefixed doc_type/doc_id/payload) would save bytes but adds
a second wire format to get right and test, for a dataset currently in the
tens of documents. JSON+base64 is debuggable with a browser's WS inspector
and costs nothing we've measured a need to save yet. Revisit if/when message
volume or size becomes a real constraint.

### What this does not solve

- **Peer discovery / connection establishment**: out of scope here: `samgtdd`
  binds loopback by default (`docs/adr/0002-private-network.md`); how a
  second peer's WebSocket client finds and connects to it (LAN address,
  Tailscale hostname, pairing flow) is undesigned.
- **Backpressure / reconnection**: the sync loop as described re-derives
  everything from `automerge::sync::State`, which is designed to resume
  correctly after a dropped connection (a fresh `State` just means a full
  resync), but no reconnection/retry logic is implemented.

## What's implemented vs. defined

Implemented and tested (`crates/samgtd-crdt`, `crates/samgtd-store`):
- Per-document sync message generation/receipt (`TaskDocument`,
  `RootIndexDocument`).
- The exact multi-document sync sequencing described above, exercised
  in-process (no WebSocket) by
  `crates/samgtd-store/tests/convergence.rs`, including restart-from-disk
  and idempotent re-sync.

Defined here but **not implemented**: the Axum WebSocket route that carries
this framing between two `samgtdd` processes. Milestone 001's scaffold
proves the sync *logic* end-to-end; wiring it to a live socket is the next
implementation step (see the "what Codex should review next" list in the
scaffold summary).
