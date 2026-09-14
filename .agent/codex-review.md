# Independent Rust/local-first review

Reviewed 2026-09-10 at `2d0b162` (`Scaffold Rust local-first daemon`), against bootstrap `96e79b5`. The working tree was clean initially. Read AGENTS.md and all three ADRs first, then inspected the architecture commit's full source/configuration changes, tests, and surrounding repository documentation/tooling. Inspected the supplied database schema read-only (`mode=ro`, `query_only=ON`); no private row contents are included here.

**Verdict:** a useful scaffold with genuine Automerge operations, but not a completed Milestone 001. Retain the per-entity document direction and crate boundaries; resolve lifecycle, identity, durability, and test gaps before treating this as a syncing daemon. The first architecture prompt explicitly permitted a narrower scaffold; missing milestone features below are completion blockers, not evidence that every deferred feature violated that prompt.

## Critical

No confirmed critical defect in the currently exposed, health-only daemon. High findings below include data integrity risks that must be addressed before enabling task writes and peer sync. Passing checks do not establish production safety.

## High

### H1. The daemon has no task or sync application path

References: `crates/samgtdd/src/lib.rs:21`, `:30`; `crates/samgtdd/src/persistence.rs:15`; `docs/milestone-001.md:5`; `docs/architecture.md:3`.

The router exposes only `/health`. Startup loads a root, logs its task count, and builds a stateless router; the store is dropped when initialization returns. There is no task create/read/update service, loaded task repository, WebSocket route, or live sync-state owner. Consequently two running daemons cannot perform the required offline-edit/reconnect/restart scenario. The architecture document's claim that the vertical slice is implemented overstates the result.

Implement a small application service owning document mutation/persistence and sync coordination, then expose it through HTTP/WebSocket. Keep HTTP framing out of the CRDT/domain crates. Revise milestone status until the daemon-level acceptance test passes.

### H2. Dataset identity and safe joining are absent despite a known root conflict

References: `crates/samgtdd/src/persistence.rs:9-23`; `crates/samgtd-crdt/src/lib.rs:152-155`, `:197-200`; `docs/adr/0003-existing-database-coexistence.md` (shared genesis); `docs/protocol.md:27-44`.

Every fresh daemon creates its own nested `tasks` map, and all stores/envelopes call the root `default`. There is no persisted node UUID or dataset UUID and no join operation. If future transport simply connects two fresh daemons as the protocol suggests, independently created root maps conflict and the ordinary `get` projection selects only one map. The test avoids this by supplying identical genesis bytes externally; startup does not enforce that prerequisite.

Persist stable node/dataset IDs, distinguish creating a dataset from joining one, provision an existing root history on join, and reject mismatched datasets before sync. Do not equate Automerge actor identity with application node/dataset identity. Include the dataset identity in a connection handshake or envelope scope.

The ADR's “only correct approach”/“byte-for-byte, before any peer makes changes” wording is too restrictive: a joining replica can receive an existing root's current history; it need not be handed the original genesis snapshot. The essential requirement for this representation is preserving the shared map object identity, not independently initializing competing maps.

### H3. Related root/entity writes have no atomic persistence boundary

References: `crates/samgtd-store/src/lib.rs:59-70`; `crates/samgtd-store/tests/convergence.rs:79-88`; `crates/samgtdd/src/persistence.rs:18-23`.

Each blob upsert commits independently. The only implemented multi-document persistence flow saves the root first and tasks afterward. A crash or write failure between those saves leaves a durable task reference without its document. SQLite WAL protects each transaction; it does not make this sequence atomic. The test's drop/reopen occurs only after every save succeeds, so it cannot detect this failure.

Add a transaction/batch boundary for local create operations (entity plus root, and projections when introduced). Define when local writes and received changes become durable relative to API success and outbound sync messages. For remote discovery, explicitly represent temporarily unavailable documents and recover them on reconnect. Test injected failure between writes and abrupt subprocess termination; assert no acknowledged task loss or permanently dangling index entries.

Also serialize initialization/mutations or reject concurrent owners of one store: the current load-then-save initialization can race across processes, and unconditional snapshot replacement has no stale-writer protection.

### H4. The integration test bypasses initial task synchronization

References: `crates/samgtd-store/tests/convergence.rs:124-143`, `:178-217`; `docs/protocol.md:45-50`, `:83-90`.

For a missing task, the helper directly reads the other peer's full saved document and loads it locally before exchanging any task sync messages. Contrary to its comments and protocol documentation, it does not create an empty receiver and discover the task through sync. Root synchronization is real, but the central new-document transfer is bypassed. Post-restart checks inspect each originating peer's own task title, not every task's complete fields on both peers; no document heads are compared.

Replace direct cross-peer snapshot access with receiver initialization and actual message delivery. Assert complete projected fields and equal heads for every document on both sides, before and after restart. Exercise same-task offline changes, same-field conflicts, interrupted sync, and repeated reconnects. The existing CRDT unit test at `crates/samgtd-crdt/src/lib.rs:236-270` genuinely proves concurrent edits to different fields survive and converge; keep it, but do not substitute it for a two-daemon durable sync test.

### H5. The “separate database” protection is only documentation

References: `crates/samgtdd/src/config.rs:33-35`; `crates/samgtd-store/src/lib.rs:44-48`; `AGENTS.md` (never modify the working database in place).

`SAMGTD_DB_PATH` accepts any path. `Store::open` immediately opens read/write, changes pragmas, and creates `documents`, without checking database ownership. Configuring the supplied working database (or another existing application database) therefore mutates it despite the crate's claim that it does not touch that database. No such mutation was performed during this review.

Inspect an existing file before write initialization and refuse foreign/unrecognized databases. Introduce an explicit store ownership/schema marker with a documented compatibility path for existing scaffold stores. Test refusal against a synthetic legacy fixture, including path aliases; never test writes against the supplied original.

## Medium

### M1. Sync-state scoping is correct in the test, not implemented in the daemon

References: `docs/protocol.md:35-56`; `crates/samgtd-store/tests/convergence.rs:100-102`, `:146-147`; `crates/samgtd-crdt/src/lib.rs:79-94`.

The documented `(peer, doc_type, doc_id)` scope is appropriate within one dataset. Tests use separate directional states for each document/peer pair. Production has no state registry, however, and the wrapper accepts any caller-provided state without enforcing scope. Specify connection lifetime as well: simultaneous connections must not accidentally share in-flight state, and reconnects may safely start fresh states against durable documents.

The lifecycle emphasizes starting exchanges for newly discovered IDs. It must also sync already-known documents on reconnect and notify connected peers after local edits; unchanged root membership does not imply unchanged task content. Add interleaved multi-document/three-peer tests plus an edit-after-idle test. Bound message sizes, queues, and active exchanges when implementing transport.

### M2. Domain separation exists as directories, but not as validated semantics

References: `crates/samgtd-domain/src/lib.rs:1-12`; `crates/samgtd-crdt/src/lib.rs:23-37`, `:72-74`, `:126-140`.

The store correctly treats Automerge bytes as opaque and has no production CRDT dependency; neither domain nor CRDT depends on Axum/SQLite. However, the only task model lives in CRDT and accepts arbitrary strings for UUIDs, category references, and status. `set_status("anything")` succeeds. Optional fields of a wrong type silently become absent. A received document is not validated for identity matching its envelope/store key.

Introduce only the minimal domain task/status/UUID types needed now, with a separate fallible CRDT projection. Define handling of malformed or conflicted replicated state without turning SQL into another authority. Enforce immutable document identity and validate category representation; existence checks must accommodate documents arriving in different orders. Full GTD semantics and SQL projections can remain deferred.

### M3. Granularity is sound, but the demonstrated lifecycle still performs whole-dataset work

References: `crates/samgtd-crdt/src/lib.rs:67-74`, `:169-177`; `crates/samgtd-store/tests/convergence.rs:63-67`, `:79-88`, `:128`; ADR 0003 (known tradeoffs).

Field edits touch one task document and do not update the root: this satisfies the key granularity requirement. Creates touch the shared index, an acceptable initial tradeoff. But the only peer implementation loads all tasks, saves every document after a create, and scans/syncs all tasks. That is test code, not proof of a production scaling defect, but it must not become the daemon's per-edit algorithm.

Persist dirty documents, schedule sync by changed document, and bound loaded document/state memory. Measure thousands of tasks and accumulated edit history before asserting scaling is proven. A per-task document can also grow with history. No giant-document redesign is needed now.

### M4. CI is useful but not reproducible

References: `rust-toolchain.toml:2`; `.github/workflows/ci.yml:17-29`, `:31-52`; `Cargo.lock`.

The lockfile and bundled SQLite improve repeatability. However, Rust `stable`, `ubuntu-latest`, and action major tags move; build/test commands omit `--locked`. The bootstrap workspace-existence condition can now turn accidental removal of Cargo.toml into a green job that performs no checks.

Pin the compiler, select an explicit runner release, pin actions to reviewed commits, use `--locked` for Clippy/tests, and remove the obsolete conditional skip. Local passes below do not establish that a hosted GitHub run is green; no hosted run was verified.

### M5. SQLite WAL sidecars are not ignored at the default data location

References: `.gitignore:5-10`; `crates/samgtdd/src/config.rs:5`; `crates/samgtd-store/src/lib.rs:46`.

The default database is in the working directory. `*.sqlite` excludes its main file, but does not exclude `samgtd-data.sqlite-wal` or `samgtd-data.sqlite-shm`. WAL can contain task data and may remain after a crash. Add SQLite/SQLite3 sidecar patterns or put the entire runtime state directory outside version-controlled paths. The supplied `.local/` database is already covered by its directory ignore.

## Low

### L1. Configuration errors and shutdown need clearer failure behavior

References: `crates/samgtdd/src/config.rs:22-35`; `crates/samgtdd/src/lib.rs:42-76`; `crates/samgtdd/src/main.rs:4-10`.

Invalid bind IP/port values silently fall back to defaults, obscuring operator mistakes. Return contextual configuration errors. Libraries already use typed errors and the daemon propagates failures with `anyhow`, but startup errors would benefit from operation/path context.

SIGINT/SIGTERM and Axum graceful shutdown are present; structured tracing fields are present; no request-path unwrap was found. Signal registration still panics on failure, and graceful drain has no deadline. Before adding long-lived sync sessions, arrange cancellation, persistence-worker draining, task joining, and a bounded shutdown timeout; test shutdown with an active client. Synchronous SQLite startup is limited today, but future request/sync writes should run in a dedicated blocking worker rather than on Tokio executor threads.

### L2. Documentation overstates evidence and contains database-handling inaccuracies

References: `docs/architecture.md:3`, `:36`; `docs/existing-database.md` (Pragmas, Tables); `crates/samgtd-domain/src/lib.rs:8-12`; `tests/integration/README.md`.

Mark scaffold versus milestone completion consistently and remove stale statements that inventory has not happened. Complete the inventory with exact column types/nullability/defaults, indexes, and explicit absence/presence of views/triggers. This review observed no user-defined views/triggers. `foreign_keys` is connection-local: this review's read-only connection reported 0, so the earlier tool's value 1 does not prove the legacy writer enforces FKs.

Copying a live main/WAL/SHM trio sequentially is not itself a consistent backup guarantee. Document a SQLite backup operation or a coordinated, quiescent snapshot instead. The integration README also incorrectly implies a crate's tests cannot host cross-crate/subprocess tests; `samgtdd/tests` can host the required daemon integration test.

Add iteration/message bounds to convergence loops (`crates/samgtd-crdt/src/lib.rs:247`, `:283`; store test `:103`, `:148`) so regressions fail diagnostically instead of hanging CI.

## Recommended implementation order

1. Correct milestone claims and immediately protect foreign databases and SQLite sidecars (H5, M5).
2. Define minimal typed identities/domain projection, dataset creation/join, and persisted node/dataset metadata in an ADR (H2, M2). Preserve the separate store and defer legacy import until live-writer ownership is resolved; future UUID backfill must happen once and its mapping be replicated, not independently regenerated per imported copy.
3. Add transactional document persistence and a serialized application owner, with write-failure/restart tests and an explicit durability acknowledgment contract (H3).
4. Fix the in-process test to transfer unknown tasks exclusively through sync; compare all fields and heads, including same-task offline conflicts (H4).
5. Implement the application task API and versioned WebSocket contract with correctly scoped state, reconnect synchronization of existing documents, local-change notifications, limits, and cancellation (H1, M1, L1). Keep loopback as default; define application authentication before enabling remote write access.
6. Add a real two-process offline-edit/sync/restart test and abrupt-termination cases. Measure thousands of tasks and optimize document scheduling/loading based on results (M3).
7. Pin CI inputs and remove bootstrap skips; finish documentation corrections (M4, L2).

## Requested assessment coverage

| Question | Assessment |
|---|---|
| Automerge source of truth? | Yes in the implemented library: edits are Automerge operations, saved blobs retain CRDT state, and no independent SQL CRUD model competes with it. Daemon integration is missing. |
| Document granularity? | Suitable per-task isolation; root changes only for membership. Large-dataset lifecycle performance is unproven. |
| Peer/document sync state? | Correct in the pairwise tests and design text; not yet implemented/enforced in production. |
| Crash/restart safety? | Individual blob persistence and clean reopen tested; atomic aggregate writes and crash recovery not established. |
| Layer separation? | Transport/CRDT/store dependencies are clean; meaningful domain types and validation are missing. |
| Default networking? | Loopback IPv4 by default; non-loopback binding requires explicit configuration. No custom cryptography or cloud dependency. |
| Android over Tailscale? | No fundamental domain/CRDT redesign appears necessary. IP/WebSocket is compatible in direction, but transport, dataset joining, protocol versioning, and auth must be implemented; interoperability is not tested. |
| Concurrent offline edits tested? | Yes, genuinely, in the task CRDT unit test; no real two-daemon test. |
| Convergence proven? | Unit test proves equal fields and preservation of independent edits; integration bypasses initial task transfer and lacks full-state/head assertions. |
| Daemon lifecycle quality? | Reasonable health-only scaffold; lifecycle of mutation, sync, cancellation, and durable shutdown remains unimplemented. |
| Reproducible CI? | No: lockfile helps, but compiler/runner/actions float and lock enforcement is absent. |

## Verification results and review changes

Executed locally on 2026-09-10 with `rustc 1.98.1 (48a229cea 2026-09-01) (Homebrew)` and `cargo 1.98.1 (797e8a9bc 2026-08-05) (Homebrew)`:

| Command | Result |
|---|---|
| `cargo fmt --all -- --check` | PASS, exit 0 |
| `cargo clippy --workspace --all-targets --all-features -- -D warnings` | PASS, exit 0 |
| `cargo test --workspace --all-features` | PASS, exit 0: 11 tests passed, none failed/ignored; doc-tests contained no tests |

Verification used the existing local dependency/build cache. No new tests, source fixes, schema changes, commits, or pushes were made. Only `.agent/codex-review.md` was added. The working database was inspected read-only. Remaining release risks are listed above; whether the legacy application remains an active writer is still unresolved and must be answered before import/coexistence work.
