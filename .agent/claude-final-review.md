# Final architecture/correctness review — Milestone 001

Reviewed 2026-09-11 against the current working tree (post-ADR-0004: `service.rs`,
`transport.rs`, `crates/samgtd-api/src/sync.rs`, `crates/samgtdd/tests/vertical_slice.rs`
added; `persistence.rs` removed). Read `AGENTS.md`, all four ADRs, `.agent/codex-review.md`,
`README.md`, `docs/{architecture,protocol,milestone-001,existing-database}.md`, and the full
source/test tree independently — findings below are based on reading the current code, not
on trusting either prior document's claims.

**Scope of this pass, per the request:** defects that would make future Android/Tailscale/mobile
synchronization painful — CRDT authority, document boundaries, server-centric assumptions, peer
identity/sync-state, schema evolution, persistence/restart, transport/domain coupling, missing
convergence cases. This is not a restatement of `.agent/codex-review.md`; see "Disposition of the
prior review" for how that review's findings map onto the current code.

## Verification results

Run locally, clean workspace, no source changes made during this review:

| Command | Result |
|---|---|
| `cargo fmt --all -- --check` | PASS, no diff |
| `cargo clippy --workspace --all-targets --all-features -- -D warnings` | PASS, no warnings |
| `cargo test --workspace --all-features` | PASS — 19 tests, 0 failed, 0 ignored |

Tests executed: `samgtd-api` (2), `samgtd-crdt` (5), `samgtd-store` (5), `samgtd-store/tests/convergence.rs`
(1), `samgtdd` (1), `samgtdd/tests/health.rs` (1), `samgtdd/tests/vertical_slice.rs` (4, including
the real HTTP+WebSocket duplex-stream relay test and the subprocess-kill durability test). No hosted
CI run was observed in this session; local results are not proof of a green GitHub Actions run.

## Disposition of the prior review (`.agent/codex-review.md`)

Verified directly against current source, not assumed:

- **H1** (no task/sync path) — resolved. `service.rs` + `transport.rs` implement create/read/update
  and a native-sync WebSocket endpoint.
- **H2** (no dataset identity/join) — resolved. `Service::open` persists `Identity{node,dataset}`
  once; `provision`/`join` (`service.rs:49-77`) transfer root history explicitly; `vertical_slice.rs:101-106`
  asserts distinct node UUIDs and a shared dataset UUID after joining.
- **H3** (no atomic multi-doc persistence) — resolved. `Store::save_documents` (`store/src/lib.rs:120-131`)
  is a real transaction, exercised by `batch_rolls_back_when_second_write_fails` and by the
  subprocess-kill test `acknowledged_write_survives_process_kill`.
- **H4** (test bypassed sync) — resolved. `convergence.rs`'s `sync_tasks` (lines 116-159) transfers
  unknown tasks exclusively through `generate_sync_message`/`receive_sync_message`, and
  `assert_equivalent` (lines 276-289) compares full fields and heads for every document on both sides.
- **H5** (foreign database not protected) — resolved. `Store::open` checks `application_id` before
  ever opening read/write (`store/src/lib.rs:49-56`, re-checked at `80-86` after lock acquisition),
  with a test that hashes the file before/after and covers a symlink alias.
- **M4** (CI reproducibility) — partially resolved: `ubuntu-24.04` pinned, `--locked` added to
  clippy/test. Still open: `rustup toolchain install stable` floats (no pinned compiler version),
  and Actions are pinned to major tags (`@v4`, `@v2`) rather than commit SHAs. Low severity, carried
  forward below.
- **M5** (WAL sidecars ungitignored) — resolved; `.gitignore` now lists `*.sqlite-{shm,wal}`,
  `*.sqlite3-{shm,wal}`, `*.db-{shm,wal}`.
- **L1** (config silently swallows bad input) — re-verified as **not actually a bug** in the current
  code: `Config::from_env`'s `.map(...).unwrap_or(Ok(default))?` pattern (`config.rs:23-28`) only
  substitutes the default when the env var is *unset* (outer `Err`); a var that's set but fails to
  parse produces `Ok(Err(_))`, which `unwrap_or` passes through unchanged, so `?` propagates the
  parse error and startup fails loudly. Worth noting since the prior review flagged this and it no
  longer holds.

Given that disposition, this pass focuses on new findings — mostly forward-looking traps that won't
bite at current (two-peer, task-only) scale but will bite as soon as more entity types or more peers
are added, which is exactly the Android/multi-device trajectory this review was asked to protect.

## Findings

### 1. [Medium] The next entity type added to the root document can reproduce the exact data-loss bug ADR 0003 already found once

`docs/adr/0003-existing-database-coexistence.md:95-111` documents a real, empirically-found Automerge
hazard: two peers that each independently call `put_object(ROOT, "tasks", Map)` mint two different
object IDs for the same key name, and merging them silently drops one peer's map. ADR 0004 fixed this
for `tasks` by making `RootIndexDocument::new()` (`crates/samgtd-crdt/src/lib.rs:175-179`) the single
place that object gets created, plus explicit `provision`/`join` to hand a *shared* root history to
new replicas instead of letting them mint their own.

Nothing generalizes that fix to the next top-level key. `docs/adr/0004-durable-replica-lifecycle.md:59`
and `docs/architecture.md:24` both already flag that category/project/domain documents are coming and
aren't implemented yet. When they are, the natural implementation mirrors `RootIndexDocument::new()`
and adds e.g. `put_object(ROOT, "projects", Map)` — and if that object is created lazily/independently
by each replica on first upgrade (rather than by exactly one replica, propagated to the rest via sync,
the way `tasks` had to be), two already-provisioned replicas that upgrade before their next sync will
each mint a competing `"projects"` object. The merge will conflict on that key and one replica's entire
project index vanishes without an error — indistinguishable from the bug ADR 0003 already paid to find
once.

**Recommendation:** before implementing any additional root-level entity key, write the rule down (ADR
addendum to 0003/0004 is the natural place) that new top-level keys must be introduced by exactly one
replica and propagated by sync — never lazily/independently initialized per replica — and add a unit
test analogous to `independently_initialized_roots_are_rejected`
(`crates/samgtd-crdt/src/lib.rs:253-262`) for the new key before it ships.

### 2. [Medium] Adding a required field to `TaskFields` will permanently break reading of every already-synced document, with no migration path

`read_str` (`crates/samgtd-crdt/src/lib.rs:130-134`) unconditionally returns
`CrdtError::MissingField` if a key is absent, and `read_fields` (`:146-161`) calls it for `title`,
`notes`, `status`, and `category_id` — every field the domain currently treats as required. There is
no document schema-version marker anywhere in `TaskDocument`, and no code path that backfills a
default into old documents when new required fields are introduced.

Concretely: the day a Milestone 002 change adds a new required field the same way `status` is handled
today, every task document created before that code shipped is missing that key. `fields()`/`read()`
on those documents will start returning `MissingField` immediately, on every replica that receives the
old bytes — not a validation warning, a hard read failure for previously-good data, and it will surface
first via whichever peer happens to sync an old document after the upgrade (i.e., it's a distributed
rollout hazard, not something a single-node test will catch).

Contrast with how `project_id`/`domain_id` are already handled: `read_opt_str` (`:136-144`) treats
absence as `None`, not an error. That pattern only currently covers fields the *domain* model also
treats as optional; there's no equivalent for a field the domain wants to be required but that needs a
default for pre-existing documents.

**Recommendation:** before Milestone 002 adds any field, decide and document (ADR) one of: (a) all new
CRDT-layer fields are read as optional with an explicit in-code default, regardless of whether the
domain type wants them required, or (b) an explicit per-document upgrade-on-read step that writes the
default in as a real CRDT op the first time an old document is loaded by new code. Either is fine;
having neither means schema evolution is a landmine.

### 3. [Medium] Convergence is only tested for exactly two peers; transitive/three-peer propagation is unverified

Both convergence tests (`crates/samgtd-store/tests/convergence.rs` and
`crates/samgtdd/tests/vertical_slice.rs`) exercise exactly two peers/replicas. The realistic
Android/Tailscale target topology is ≥3 devices (phone, laptop, desktop) that are not necessarily
all pairwise-connected at once — e.g. phone↔laptop and laptop↔desktop connect, but phone and desktop
never dial each other directly, and a change made on the phone needs to reach the desktop transitively
through the laptop.

Nothing in `Service::exchange` (`crates/samgtdd/src/service.rs:125-188`) is topology-aware — it just
re-shares whatever's in the local store to whichever peer is currently connected, which should make
transitive propagation work in principle (the laptop will re-offer everything it has, including
changes it only just received from the phone, to the desktop on the desktop's next sync round) — but
this is inference from reading the code, not something any test demonstrates. There's also no test for
two peers connecting to a third *simultaneously* and both pushing conflicting edits to the same task
in the same window, which is a much more likely real-world Android scenario than the current
two-peer-serial-offline-edit test covers.

**Recommendation:** add a three-replica test: A and B both connect to C (not to each other), each
independently edits a shared task offline, syncs to C, and C is checked for correct convergence; then
a fresh D provisions from C and is checked for receiving both edits without ever talking to A or B
directly.

### 4. [Low] `Service`'s single-writer guarantee lives one layer up, in `transport::App`, not in `Service` itself

`service.rs`'s module doc comment (`crates/samgtdd/src/service.rs:1-2`) states "documents are loaded
into candidates and committed before returning, so failed writes cannot leak into served state" — an
atomicity/serialization guarantee. In the daemon, that guarantee actually depends on
`transport::App::call` (`crates/samgtdd/src/transport.rs:33-45`) holding a `Mutex<Service>` around
every single operation; `Service` itself has no internal lock and does nothing to prevent concurrent
callers from interleaving, e.g., two threads racing `update()`'s read-modify-write
(`service.rs:103-124`) on the same task.

This matters specifically for mobile/embedding: an Android build is a plausible future consumer of
`samgtd-crdt`/`samgtd-store`/this `Service` type directly (e.g., via FFI) rather than through the Axum
HTTP layer, and there's nothing about `Service`'s own type signature that would stop such a consumer
from calling it concurrently from multiple threads and silently losing the atomicity the header comment
promises.

**Recommendation:** either move the serialization inside `Service` (e.g., wrap its internals or require
`&mut self` be enforced by an internal lock so the guarantee travels with the type), or narrow the doc
comment to state explicitly that callers must serialize access themselves — so a future embedder
doesn't inherit a false assumption from the comment.

### 5. [Low] Task status validation is duplicated, unsynchronized, between two crates

`crates/samgtd-crdt/src/lib.rs:80` (`TaskDocument::set_status`) and
`crates/samgtd-domain/src/lib.rs:37` (`TaskFields::validate`) each independently hardcode
`matches!(status, "TODO" | "DONE")`. `docs/existing-database.md:106-112` already hints at richer
status-like state living in the legacy `reports`/label model, so a future status value is plausible.
If only one of the two sites is updated, `TaskDocument::set_status` could accept a value that
`TaskFields::validate` then rejects the next time the document is read (`read_fields` calls
`fields.validate()` at `crdt/lib.rs:159`) — surfacing as a read failure on a document that was
successfully written moments earlier, easy to mistake for CRDT corruption rather than a validation
mismatch.

**Recommendation:** have `TaskDocument::set_status` call into `samgtd_domain`'s validation (or a shared
constant) instead of re-declaring the allowed set.

### 6. [Low] `Service::join` can silently reassign an already-joined replica's dataset

`join()` (`crates/samgtdd/src/service.rs:56-77`) only guards on the replica currently having zero
tasks (`root().task_uuids()?.is_empty()`); it doesn't check whether the replica has already been
explicitly joined to a dataset. A replica that joined dataset X while X still has no tasks, then
receives a second (e.g. mistaken or stale-retry) `POST /provision` for dataset Y, will silently switch
to Y with no error — plausible exactly during initial multi-device setup, when datasets are most likely
to still be empty and a user is juggling more than one.

**Recommendation:** track "has this replica ever explicitly joined" independent of current task count,
and refuse a second `join()` once set.

### 7. [Low, carried forward] CI toolchain/actions still float

`.github/workflows/ci.yml:22` installs `stable` rather than a pinned compiler version, and Actions are
referenced by major tag (`@v4`, `@v2`) rather than commit SHA. `--locked` and a pinned `ubuntu-24.04`
runner (added since the prior review) meaningfully improve reproducibility, but a `stable` toolchain
bump can still turn a currently-green CI run red (or silently green on different clippy lints) without
a corresponding repo change. Not specific to mobile/sync; noted for completeness since it was raised
before and is only partially addressed.

## Assessment against the review's specific concern list

| Concern | Assessment |
|---|---|
| CRDT authority | Correct — Automerge is the only mutable source of truth for task state; SQLite stores opaque bytes only (`store/src/lib.rs` has no GTD schema). |
| Document boundaries | Hybrid root+per-task model is sound at current scope; the boundary-extension hazard for future entity types (finding 1) is real and unaddressed. |
| Hidden server-centric assumptions | None found that block a symmetric-peer model — both `Service`/`Peer` shapes in tests are fully symmetric, and `/provision` + `/sync` don't assume a privileged "server" beyond who dials whom. |
| Peer identity/sync-state | Node/dataset identity is properly separated from Automerge actor IDs; per-connection fresh `Session` matches ADR 0004. `join()`'s reprovisioning gap (finding 6) is the one real gap. |
| Schema evolution traps | Two concrete, currently-latent traps found (findings 1 and 2) — the most important findings in this pass for the stated Android/mobile focus. |
| Persistence/restart gaps | None found; `save_documents` transactionality and the subprocess-kill test give real evidence here. |
| Transport/domain coupling | Clean — verified via `Cargo.toml` dependency edges (`samgtd-domain`/`samgtd-crdt` have no axum/rusqlite dependency) and via reading `samgtd-api`'s wire types (finding 5 is a validation-duplication issue, not a layering violation). |
| Missing conflict/convergence cases | Same-task/same-field conflict is genuinely tested; three-or-more-peer/transitive propagation is not (finding 3). |

## Note on scope

No source files were modified in this review. Findings above are prioritized by what would actually
hurt when Android/Tailscale/multi-device work begins, per the request — none of them block continuing
Milestone 001/002 work, but findings 1 and 2 should be resolved (at least by ADR decision, not
necessarily code) before any new entity type or new task field ships, since both describe damage that
is silent at write time and only surfaces later, on already-deployed data.
