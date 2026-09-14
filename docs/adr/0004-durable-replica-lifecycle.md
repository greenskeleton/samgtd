# ADR 0004: Durable replica lifecycle and initial transport

- Status: Accepted
- Date: 2026-09-11

## Context

The reviewed scaffold had no application owner, dataset identity, join path,
atomic multi-document writes, or running task/sync routes. The supplied operator
database was inspected again read-only, including schema objects, columns,
indexes, foreign keys, pragmas, and representative data shapes. Its integer
identity and relational relationships remain unchanged. Legacy import and
coexistence remain deferred under ADR 0003.

## Decision

Keep the root/index plus per-task Automerge representation. Task fields and
validation live in the domain crate; CRDT projection is fallible. Invalid UUIDs,
invalid statuses, incorrectly typed optional fields, competing root map objects,
and conflicted immutable task identity are rejected. Mutable field conflicts use
Automerge's deterministic winner; history retains both operations.

A new replica generates independent node and dataset UUIDs and stores local
metadata alongside the initial root in one SQLite transaction. Node identity is
not an Automerge actor ID. GET /provision exports current root history and dataset
UUID; POST /provision explicitly joins an empty replica, preserving its node UUID.
A root may reference tasks not yet received. Task history arrives only through
native sync messages into empty receivers. Original genesis bytes are unnecessary:
the shared map object history must be preserved.

The store recognizes application_id 0x53475444. Existing unmarked databases,
including old scaffold stores, are refused without write initialization. There
is no automatic adoption or legacy migration; use a new path for this milestone.
A canonical-path sidecar advisory lock serializes ownership across samgtd
processes, including symlink aliases. Do not create hard-link aliases of SQLite
stores. The lock file is retained after release and ignored by Git.

A serialized application service runs SQLite/Automerge operations in blocking
tasks. Each operation loads candidate documents from durable state. Task creation
commits entity and root together; mutations commit before HTTP success. Received
changes persist before generating responses. Failed candidate writes cannot
change subsequently served state. SQLite remains an opaque document store.
Remote root discovery can temporarily precede task availability; reconnect
resynchronizes every indexed document to recover incomplete transfers.

Each WebSocket connection owns fresh sync state per document. Protocol v1 uses
JSON text frames with a byte-array payload, dataset UUID, kind, and document ID.
This replaces the scaffold's unimplemented base64 framing. Root membership must
be acknowledged before task frames are emitted. Connected sessions receive
notifications after edits. Messages are limited to 1 MiB and sends to ten seconds.
Shutdown notifies upgraded sessions to exit and bounds server drain to ten seconds.
Dataset changes invalidate sessions.

## Consequences and remaining work

This is a small-dataset vertical slice. Sync currently scans indexed documents
and builds outgoing messages in memory; aggregate memory/session limits and
dirty-document scheduling remain work. Category/project/domain references are
UUID-validated but their target entities are not implemented. Contexts and
remaining GTD semantics remain deferred.

The daemon exposes a sync endpoint; an initiating client or relay must establish
peer connections. Automatic dialing/retry/discovery is not implemented. The
integration test supplies a transparent relay between real WebSocket upgrades
over in-memory byte streams, with independent SQLite files. Socket binding is
prohibited in the implementation environment, so bound TCP two-process behavior
remains unverified. A separate subprocess test checks acknowledged-write survival
after an abrupt kill. Hosted CI status is not established by local checks.

Authentication remains deferred; loopback is the default. This decision adds no
cryptography, external service dependency, or legacy database migration.
