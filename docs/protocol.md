# Sync protocol v1

Implemented by the Axum /sync WebSocket endpoint; see ADR 0004.

Each JSON **text** frame carries one native Automerge sync message:

~~~json
{"version":1,"dataset":"<dataset-uuid>","kind":"root_index","id":"default","message":[66,1]}
~~~

The message is an array of encoded bytes (illustrative above, not a valid message).
Kind is root_index or task; a task's id is its UUID. Every frame is scoped to the
persisted dataset identity. Mismatched datasets/versions, unknown kinds,
unindexed tasks, malformed documents, and task identity mismatches close the
connection. No custom encryption is used.

Create a dataset by starting with a new store path. To join it, GET /provision
on an existing replica and POST that JSON to /provision on an empty replica.
This transfers root history and dataset identity, not task snapshots. Joining
preserves the receiving node identity. GET /identity returns both IDs.

A connection starts with root sync, then exchanges all indexed task documents
once root membership is acknowledged. Missing tasks start as empty Automerge
receivers. Root discovery can precede task availability; reads then fail until
native sync transfers the task. Reconnect creates fresh per-document sync states
and retries all indexed documents, including already-known tasks.

Writes are persisted before HTTP success or sync replies. Connected sessions
wake after local edits or received changes. WebSocket frames/messages are limited
to 1 MiB, and sends time out after ten seconds. A malformed/oversized message
terminates the connection; a fresh connection can resume from durable history.

The daemon currently accepts inbound connections. A client/relay establishes
peer links; autonomous dialing, retry policy, discovery, authentication, total
session/memory limits, and large-history chunking are not implemented.

`samgtd-testkit::relay` (`crates/samgtd-testkit/src/relay.rs`) is that
client/relay for this milestone's acceptance test and demo: it dials each
daemon's `/sync` endpoint as a real WebSocket client over real loopback TCP
and forwards the JSON text frames above verbatim, in both directions, without
decoding or interpreting them. It is a transparent relay standing in for "some
link between two already-addressable peers exists," not a production peer
manager — see `docs/milestone-001-acceptance.md` for how it's used and what
it does and doesn't prove.

## Test/demo-only readiness mechanism

Setting `SAMGTD_BIND_PORT=0` asks the OS for any free loopback port, which is
the race-free way to run more than one instance on a host without picking
ports by hand. Since the caller then doesn't know which port was chosen, the
daemon optionally announces it: if `SAMGTD_READY_FILE=<path>` is set, the
daemon writes the real bound `ip:port` to that path (atomically — write to a
temp file, then rename) right after `TcpListener::bind` succeeds and before
it starts accepting connections. Both variables are optional and off by
default; neither changes any behavior for a normally-configured daemon
(explicit port, no ready file).

## Task HTTP API

- POST /tasks: JSON fields title, notes, status (TODO/DONE), required category_id
  UUID, optional project_id and domain_id UUIDs. The server assigns uuid; returns
  201 and the full task.
- GET /tasks/{uuid}: full projected task.
- PATCH /tasks/{uuid}: any subset of title, notes, status.
- GET /health: health response.

All current application failures return 400; finer distinction between missing
tasks, invalid input, and storage failures remains an API refinement.
