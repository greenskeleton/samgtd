# ADR 0003: CRDT/store coexistence with the existing SQLite database

- Status: Accepted (identity strategy); one section explicitly Open (live
  coexistence protocol)
- Date: 2026-09-10

## Context

`.local/current-gtd.sqlite` is a real, externally-owned database (see
`docs/existing-database.md`): it has its own `_migrations` history from an
existing, non-Rust application, uses plain `INTEGER PRIMARY KEY
AUTOINCREMENT` identity throughout, and already encodes most of
`AGENTS.md`'s GTD semantics plus one concept it didn't anticipate (`domains`).

`AGENTS.md` requires stable UUID identity and forbids using this inventory
step to invent a replacement schema. It also requires that any required
migration be documented and approved before schema changes are made.

Whether the existing application is still actively writing this file was not
resolved as of this ADR (see "Open question" below). This decision is
written to hold regardless of the answer.

## Decision: additive-only identity mapping

Introduce one new table, added via a normal additive migration, that never
alters any existing table's columns, types, or constraints:

```sql
CREATE TABLE samgtd_identity (
    entity_type TEXT NOT NULL,   -- 'category' | 'domain' | 'label' | 'project' | 'task'
    local_id    INTEGER NOT NULL, -- the existing table's integer id
    uuid        TEXT NOT NULL,
    created_at  TEXT NOT NULL DEFAULT (datetime('now')),
    PRIMARY KEY (entity_type, local_id),
    UNIQUE (uuid)
);
```

Rationale:

- **Purely additive.** No existing column is renamed, retyped, or
  constrained differently. A migration runner belonging to the existing
  application has no reason to notice or object to an unrelated new table.
- **Reversible.** Dropping `samgtd_identity` fully undoes the change; the
  original schema is untouched underneath it.
- **Tolerant of gaps.** `docs/existing-database.md` already shows hard
  deletes (e.g. `domains` id `2` missing) — the mapping is keyed by
  `(entity_type, local_id)`, not by assuming density or ordering.
- Every Automerge document (per the per-entity model accepted below) is
  named by the `uuid` from this table, never by the SQLite integer id
  directly. The integer id remains the FK/join key inside the existing
  relational tables; the UUID is the CRDT-facing, network-stable identity.

UUIDs are backfilled once per existing row (`v4`, generated at migration
time) and assigned to new rows at creation time by `samgtd-store`, which
becomes responsible for writing to `samgtd_identity` whenever it inserts a
row into `categories`/`domains`/`labels`/`projects`/`tasks`.

## Decision: resolves ADR 0001's open question — hybrid root + per-entity documents, accepted

ADR 0001 left document granularity open. Given the real data shape (35
tasks, 13 projects, 8 labels, 5 categories, 2 domains — small today, but a
personal GTD system that accumulates tasks indefinitely and must merge
offline edits from multiple devices):

- **Accept the hybrid model**: one root/index document per dataset, listing
  known entity UUIDs by type, plus one Automerge document per mutable
  entity (task, project, label, category, domain).
- A single giant document would mean every offline edit — even to one
  task — produces a sync payload and merge computation scoped to the whole
  dataset, and unrelated concurrent edits would share one change history.
  Per-entity documents keep sync/merge scoped to what actually changed.
- The root index avoids the opposite problem (thousands of documents with no
  way to enumerate them): it is the thing a peer loads first to discover
  which entity documents exist.
- This scales fine at current volumes and is the right shape before volume
  makes it necessary — but see "Known tradeoffs" below for the concrete cost
  this incurs at Milestone 001's scale.

## Migration requirements

1. Add `samgtd_identity` (additive; see above).
2. Backfill one row per existing `categories`/`domains`/`labels`/`projects`/`tasks`
   record with a freshly generated UUID.
3. `samgtd-store` must generate and record a `samgtd_identity` row inside the
   same transaction as any future insert into those tables.
4. No existing column, index, trigger, or view is modified or dropped.

None of this is implemented yet in code as of this ADR — Milestone 001's
vertical slice (see `docs/milestone-001.md`) only needs a *new*,
CRDT-originated task to prove convergence, so it does not yet require
running this backfill against the real 35 existing tasks. That backfill is
required before the daemon treats the existing rows as the live dataset.

## Known tradeoff, found empirically: documents need a shared genesis

Building the Milestone 001 convergence test (`crates/samgtd-store/tests/convergence.rs`)
surfaced a real Automerge constraint not obvious from the API docs: **two
documents created independently via `AutoCommit::new()`, even with identical
top-level key names, do not merge safely.** Each `put_object` call mints a
new object ID from its own actor, so two peers' independently-created
`"tasks"` map objects are different objects that happen to share a key name.
Merging them produces a *conflict* on that key (Automerge picks one
deterministically for reads), silently dropping the other peer's map — not
an error, just quiet data loss.

The fix (and the only correct approach): a dataset's root index document is
created exactly once; every peer is provisioned from that same initial
document (byte-for-byte, before any peer makes changes), never by calling
`RootIndexDocument::new()` independently per device. This has a direct
implication for device provisioning/pairing (not yet designed): pairing a
new device must transfer the genesis document, not just say "start fresh."

## Known tradeoffs

- Per-entity documents mean the daemon manages up to (currently) ~65 open
  Automerge documents rather than one. `automerge`'s Rust API persists each
  document as its own compact binary blob; this is a storage/bookkeeping
  cost, not a fundamental scaling problem, but it means `samgtd-store` needs
  a real "document table" (id → bytes), not just a single blob column.
- The root index document itself is a single point of write contention: any
  change to *which* entities exist (create/delete) touches it, even though
  per-entity field edits don't. This is accepted as inherent to the hybrid
  model, not a defect to solve here.

## Open question: live coexistence protocol (unresolved)

Whether the existing application is still writing to `current-gtd.sqlite`
was not answered before this ADR was written. Two cases:

- **Still active**: `samgtd-store` needs a way to detect rows changed by the
  other application (e.g. poll `updated_at`, or a trigger writing to a
  changelog table) and feed them into the corresponding Automerge document.
  This is nontrivial and is **explicitly deferred** — no protocol is
  designed here, per `AGENTS.md`'s instruction to avoid speculative
  abstractions not needed for Milestone 001.
- **One-time cutover**: `samgtd-store` does a single import (steps 1–3
  above) and the existing application stops touching the file; no ongoing
  detection is needed.

Milestone 001's vertical slice does not require resolving this — it operates
on a freshly created task document, not on the imported legacy rows. This
question must be resolved before any migration in "Migration requirements"
runs against the real database.
