# ADR 0005: Root-key introduction and field evolution rules

- Status: Accepted
- Date: 2026-09-11

## Context

`docs/adr/0003-existing-database-coexistence.md` documents a real, empirically
found Automerge hazard: two peers that each independently call
`put_object(ROOT, "tasks", Map)` mint two different object IDs for the same
key name, and merging them silently conflicts on that key — one peer's map
disappears from reads, with no error. ADR 0004 fixed this specifically for
`"tasks"` by making `RootIndexDocument::new()` the single place that object is
created, plus explicit `GET`/`POST /provision` to hand a joining replica the
*current shared history* of an existing root rather than letting it mint its
own.

Milestone 001's final review (`.agent/claude-final-review.md`, findings 1 and
2) pointed out that neither fix generalizes automatically:

1. Nothing stops a future top-level root key (`"projects"`, `"categories"`,
   `"domains"`, …) from being introduced the way `"tasks"` almost was —
   lazily, independently, by each replica's first-upgrade code path —
   silently reproducing the exact bug ADR 0003 already paid to find once.
2. `TaskDocument::fields()`'s field readers (`read_str`) unconditionally
   return `CrdtError::MissingField` for an absent key. Every field
   `TaskFields` currently treats as required (`title`, `notes`, `status`,
   `category_id`) is read this way. A future required field added the same
   way would make every already-synced document written before that change
   fail to read on any replica that receives it — a distributed rollout
   hazard, not something a single-node test catches.

Both are latent: they cost nothing today (there is exactly one root key and
today's field set), and only bite the day Milestone 002+ adds another entity
type or another required task field. This ADR exists so that day doesn't
rediscover ADR 0003's bug or invent an ad hoc fix under time pressure.

## Decision 1: root-level keys are introduced by exactly one code path, never lazily per replica

A new top-level key on the root document (a new entity-type index, following
the `"tasks"` pattern) **must** be created in exactly one place — the
equivalent of `RootIndexDocument::new()` — and reach every other replica
*only* through sync or `/provision`, never by a replica's own upgraded code
independently deciding "this key doesn't exist yet, let me create it."

Concretely, before any code adds a second top-level key:

- The constructor that creates the root document (or an explicit, one-time
  "upgrade" operation gated the same way schema migrations are) is the only
  call site allowed to `put_object(ROOT, <new key>, Map)`.
- Any other code path that encounters a root document missing the new key
  must treat that as "not yet synced/upgraded" (e.g. behave as if the
  collection is empty, or fail closed), never as "initialize it myself."
- A unit test analogous to
  `independently_initialized_roots_are_rejected` (`crates/samgtd-crdt/src/lib.rs`)
  must exist for the new key before it ships: merge two independently
  constructed documents that both created the key, and assert the merge is
  detected as a conflict rather than silently resolved.

This rule applies to root-level keys specifically (the shared map ADR 0003
identified as the hazard). It does not by itself say how per-entity documents
of the new type are created — that follows the existing per-task pattern
(`TaskDocument::new`), which is not the part of the design ADR 0003 found
unsafe.

## Decision 2: new CRDT-layer fields default to optional-with-explicit-default, not unconditionally required

When a future milestone adds a field to `TaskFields` (or a future entity's
fields) that the *domain* wants to treat as required, the **CRDT read path**
must not assume the field is present. Two shapes are acceptable; this ADR
picks (a) as the default:

**(a) Read as optional, default in code (default policy).** The field's CRDT
reader uses the `read_opt_str`-style pattern already used for
`project_id`/`domain_id` (absence is `Ok(None)`, not `Err`), and the
application layer supplies an explicit, documented default value when the
key is absent, before handing the value to domain validation. This requires
no CRDT-level migration and works uniformly regardless of which replica
authored a given document or when it last upgraded.

**(b) Explicit upgrade-on-read.** For a field where a *silent* default would
be actively wrong (rare — most GTD fields tolerate a sensible default), the
reader instead performs a real, explicit Automerge write the first time an
old document is loaded by new code, recording the backfilled value as a
genuine CRDT op (so it propagates on next sync) rather than only existing
in that process's in-memory projection. This is more invasive and should be
justified per field, not used by default.

Either way: the domain type (`TaskFields`, or a future entity's fields
struct) may still declare the field as required/non-`Option` in Rust — this
decision only constrains the *CRDT layer's read path*, so that "domain wants
it required" and "every previously-synced document already has it" are
handled as two separate concerns instead of one unconditional `MissingField`
error. `TaskDocument::set_status`'s delegation to
`samgtd_domain::is_valid_status` (added alongside this ADR) is the pattern to
follow for keeping a single validity definition shared between the two
crates when a new field's value has a closed set of valid values.

## Consequences

- No code change ships with this ADR — Milestone 001 has exactly one root key
  and no new required fields. This records the rule for whoever adds the
  next one (Milestone 002+), per the final review's recommendation to decide
  this in an ADR before it's needed under time pressure.
- Future root-key or required-field PRs should link back to this ADR in their
  description and include the tests these two decisions call for.
- This does not change wire compatibility, persistence, or identity; it is a
  process/implementation constraint on future schema growth.
