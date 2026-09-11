# Milestone 001 — acceptance evidence

This document maps every required Milestone 001 item to the test/demo
evidence that backs it, gives exact reproduction commands, records what was
actually observed running them in this session, and states what remains
untested. It supersedes the "Implementation status" summary in
`docs/milestone-001.md` as the acceptance record; that file still holds the
milestone's requirement list itself.

Distinguish two kinds of statement below: **Assertion** (an automated check
actually ran and passed, reproducibly, evidenced by a named test/check id)
and **Expectation** (believed true from reading the code, but not directly
exercised by an automated check). Nothing in this document is a claim about
GitHub Actions unless that run is named explicitly, with its URL and tested
commit, under "Hosted CI run" — a prior green run on an older revision (e.g.
`main`'s `2d0b162`, which predates every change described here) is never
treated as evidence for the current changeset.

## How to reproduce everything in this document

```sh
cargo fmt --all -- --check
cargo clippy --locked --workspace --all-targets --all-features -- -D warnings
cargo test --locked --workspace --all-features
cargo run -p samgtdd --example demo
```

The first three are the standard checks from `AGENTS.md`/`CLAUDE.md`. The
fourth is the **one documented command to run the synthetic acceptance
demo** the milestone requires: it builds `samgtdd` if needed, launches the
full scenario below against temporary SQLite stores under the OS temp
directory (never `.local/current-gtd.sqlite` or any path outside a
just-created temp directory), prints a transcript of every check as it runs,
writes `artifacts/acceptance/results-<timestamp>.json` (machine-readable) and
`artifacts/acceptance/transcript-<timestamp>.txt` (human-readable) plus
stable `results-latest.json`/`transcript-latest.txt` copies, and exits
nonzero if any check failed. `artifacts/` is gitignored — it is run output,
not source.

Override the daemon binary path with `SAMGTD_DAEMON_BIN=/path/to/samgtdd` and
the artifact directory with `SAMGTD_DEMO_OUT_DIR=/path/to/dir` if needed;
neither is required for the default invocation above.

## Observed results (this session)

Run on 2026-09-11 against this working tree (dirty; see "Provenance" below),
toolchain `rustc 1.98.1 (48a229cea 2026-09-01) (Homebrew)` /
`cargo 1.98.1 (797e8a9bc 2026-08-05) (Homebrew)`:

| Command | Result |
|---|---|
| `cargo fmt --all -- --check` | PASS, no diff |
| `cargo clippy --locked --workspace --all-targets --all-features -- -D warnings` | PASS, no warnings |
| `cargo test --locked --workspace --all-features` | PASS — 22 tests, 0 failed, 0 ignored |
| `cargo run -p samgtdd --example demo` | PASS — 60/60 named checks passed |

The 22 `cargo test` tests, by target: `samgtd-api` health unit tests (2),
`samgtd-crdt` unit tests (5), `samgtd-domain` (0 — pure validation exercised
transitively via the other crates' tests), `samgtd-store` unit tests (5),
`samgtd-store/tests/convergence.rs` (1, the pre-existing lower-layer
convergence/conflict test — preserved unchanged), `samgtd-testkit` unit
tests (1), `samgtdd` unit tests (2, including the new
`join_is_rejected_after_the_replica_has_already_joined_once`),
`samgtdd/tests/health.rs` (1), **`samgtdd/tests/two_process_acceptance.rs`
(1 test containing the 60 named checks below)**, and
`samgtdd/tests/vertical_slice.rs` (4, the pre-existing duplex-stream
HTTP+WebSocket relay tests — preserved unchanged).

The demo (`cargo run -p samgtdd --example demo`) and the test
(`two_process_acceptance.rs`) both call the same
`samgtd_testkit::scenario::run_full` — they are evidenced together below
because they are, deliberately, the same scenario.

## Required item → evidence

From `docs/milestone-001.md`, "## Required":

| Requirement | Evidence | Kind |
|---|---|---|
| Cargo workspace | This workspace builds/tests as one (`cargo test --workspace`) | Assertion |
| Daemon starts on `127.0.0.1` by default | `Config::default()`; `crates/samgtdd/src/config.rs::tests::default_binds_to_loopback` | Assertion |
| Health endpoint | `samgtd-api` health tests; check `1a-health` below (real HTTP, two real processes) | Assertion |
| Stable node/dataset identity persisted locally | Checks `1a-identity-a/b`, `4h-identity-persisted`, `7d-identity-and-state-persisted` (identity unchanged across two separate real-process restarts) | Assertion |
| Create/read/update one minimal Task aggregate | Checks `2a-create-offline`, `3b`/`3c` (update), all real HTTP against a real process | Assertion |
| Automerge document persisted across restart | Checks `4g`–`4i`, `7c`–`7d` (two independent restart-and-reload cycles of real processes) | Assertion |
| WebSocket sync endpoint using Automerge sync messages | `crate::relay` in `samgtd-testkit` dials real `/sync` WebSocket endpoints over real TCP and forwards native protocol frames (see `docs/protocol.md`) verbatim; checks `2b`–`2c`, `4a`–`4c`, `5c`–`5d`, `6a`–`6b`, `7e`–`7f` | Assertion |
| Two-peer integration test proving convergence after offline concurrent edits | The full `two_process_acceptance.rs` scenario (see "Scenario step → checks" below) | Assertion |
| Structured logs | `tracing`/`tracing-subscriber` in `crates/samgtdd/src/main.rs`; captured daemon stdout in `samgtd_testkit::process::Daemon::captured_output` (used for failure diagnostics) | Assertion (logs are produced and captured; not asserted on for content) |
| Graceful shutdown | Checks `4e`, `7a` (bounded graceful shutdown of real processes, `7a` specifically with an **active** sync connection) | Assertion |
| No public network bind by default | `Config` defaults to loopback; check `1a-loopback` confirms both real daemons actually bound loopback addresses | Assertion |
| GitHub CI green | Run https://github.com/greenskeleton/samgtd/actions/runs/34630462929 on PR #1, commit `6bef60deb069d71435d819e3ad1066231edcb9ef` (checked out as merge commit `3da218904a634f74ea371147031e413043bb1215`): `conclusion: success`. See "Hosted CI run". | Assertion |

## Scenario step → checks

Each numbered item is from this work's assignment ("The scenario must…").
Check ids are from `two_process_acceptance.rs` / the demo transcript; all
passed in the run recorded above. All of this happens over **real loopback
TCP sockets and real `samgtdd` child processes** — `samgtd-testkit::process`
spawns actual binaries (`env!("CARGO_BIN_EXE_samgtdd")` for the test,
a `cargo build`-then-locate for the demo), `samgtd-testkit::http` speaks
real HTTP/1.1 over a real `TcpStream`, and `samgtd-testkit::relay` dials real
WebSocket connections to each daemon's `/sync` endpoint — never an in-process
router, direct `Service` call, or snapshot copy.

1. **Start A and B, readiness, health, distinct node UUIDs, loopback
   listeners; provision empty B into A's dataset.** Checks `1a-spawn-a/b`,
   `1a-ready-a/b` (bounded polling on a daemon-announced real bound address —
   see "Port allocation" below — no fixed sleep), `1a-loopback`,
   `1a-health`, `1a-identity-a/b`, `1a-distinct-nodes`, `1b-provision` (real
   `GET`/`POST /provision` over HTTP).
2. **Create a task on A while disconnected; connect; B discovers and
   receives it via native sync.** Checks `2a-create-offline`,
   `2b-connect-relay` (relay dials both real `/sync` endpoints),
   `2c-b-receives-task`.
3. **Disconnect; edit the same task in different fields on A and B; create
   additional tasks offline on both.** Checks `3a-disconnect`,
   `3b-edit-a-notes`, `3c-edit-b-title`, `3d-create-a-offline`,
   `3e-create-b-offline`.
4. **Reconnect; assert both independent edits, all task IDs, complete
   projected values, and equal CRDT heads.** Checks `4a-reconnect`,
   `4b-both-edits-preserved-a/b`, `4c-task2-reaches-b`/`4c-task3-reaches-a`
   (complete projected fields via `wait_for_value`, an exact-JSON-equality
   poll). Heads: the public API has no document-dump endpoint (deliberately —
   see `crates/samgtd-testkit/src/store.rs`'s module doc), so both daemons are stopped
   (`4e-stop-a/b`, graceful, bounded) and check `4f-heads-equal` then opens
   each now-unlocked SQLite store read-only via the same `samgtd-store`/
   `samgtd-crdt` libraries the daemon itself uses, and compares root and
   per-task Automerge heads directly — this never reads one replica's store
   to drive the other's sync; it only verifies what already converged over
   the wire. Both daemons are then restarted from the same stores
   (`4g-restart-a/b`, `4g-ready-a/b`) and checks `4h-identity-persisted`,
   `4i-state-persisted` re-verify identity and full state over HTTP again.
5. **Same-field concurrent conflict; equal resolution on both peers, no
   hardcoded winner.** Checks `5a-conflict-edit-a`/`5b-conflict-edit-b` write
   two *different* values to the same field (`notes`) on the same task,
   offline, on each peer. Check `5d-conflict-resolved-equally` polls both
   peers until they return an **identical** value and only then asserts that
   value is one of the two concurrently-written candidates — it does not
   assume which one wins, matching the existing lower-layer conflict test's
   approach (`crates/samgtd-store/tests/convergence.rs`, preserved
   unchanged). The lower-layer CRDT conflict unit test
   (`concurrent_field_edits_converge_via_sync`,
   `crates/samgtd-crdt/src/lib.rs`) is also preserved unchanged.
6. **A local edit propagates over an already-open, idle sync connection.**
   Check `6a-idle-edit` patches A over the *same* connection `5c-reconnect`
   opened (no intervening disconnect/reconnect); `6b-idle-propagation` polls
   B until the edit arrives on that still-open connection.
7. **Stop both, restart, verify identity/state; reconnect; assert matching
   heads, no missing/duplicate tasks.** Checks `7a-graceful-shutdown-active-a/b`
   (see item 8's second half — this is the same shutdown), `7c-restart-a/b`,
   `7c-ready-a/b`, `7d-identity-and-state-persisted`, `7e-reconnect`,
   `7f-no-op-convergence`, and `7h-heads-no-duplicates` (final store
   inspection: exactly the 3 expected task UUIDs on both peers, equal root
   heads, equal per-task heads).
8. **HTTP-acknowledged write survives abrupt kill; graceful shutdown with an
   active connection is bounded.** The abrupt-kill half runs as its own
   sub-scenario against a third, independent daemon (checks `8a`–`8e`
   in the transcript): `8b-http-write` gets a real `201 Created` from a real
   HTTP POST, `8c-sigkill` sends real `SIGKILL` (not a graceful stop) to the
   real process and reaps it, `8d-restart` starts a *new* process against the
   *same* store path, and `8e-write-survived` confirms the restarted
   process's `GET` returns exactly the previously-acknowledged JSON. The
   graceful-with-active-connection half is checks `7a-graceful-shutdown-active-a/b`:
   both A and B are sent `SIGTERM` (not `SIGKILL`) while relay `5c`'s
   connection is still open and idle, and each is asserted to exit
   successfully within a 15-second bound (observed: both exited in single-digit
   milliseconds in this session — well inside the bound; the bound exists for
   CI-runner variance, not because shutdown is expected to be slow).

## Port allocation and readiness (no fixed sleeps)

Each daemon is started with `SAMGTD_BIND_PORT=0` (OS-assigned ephemeral port —
no reserve-then-hope-it's-still-free race) and `SAMGTD_READY_FILE=<path>`.
`samgtdd` binds first, then writes the real bound address to that file
atomically (write-to-temp, then rename) before starting its accept loop; the
test/demo polls for that file (25ms interval, 10s bound) rather than sleeping
a fixed duration, and treats an early process exit as an immediate,
diagnostic-carrying failure rather than continuing to poll. `SAMGTD_READY_FILE`
is optional and only meaningful together with port 0; it changes no default
behavior for a normally-configured daemon.

## Provenance in the results artifact

`artifacts/acceptance/results-latest.json` (schema documented inline via its
`schema_version` field) always includes: `generated_at`, the `toolchain`
(`rustc`/`cargo` `--version` output), and `source.commit`
(`git rev-parse HEAD`) plus `source.dirty` (`git status --porcelain` is
non-empty). When dirty — the normal state for in-progress work, including
this session's changes prior to commit — it additionally includes
`source.fingerprint`: a `sha256:` hash over every modified/untracked path
`git status --porcelain=v1 -uall --no-renames` reports, paired with each
file's contents (or an explicit "absent" marker for deletions), sorted for
determinism. This exists so a dirty run's evidence is tied to what was
actually on disk, not just the last commit.

## CI and reproducibility

- **Toolchain**: `rust-toolchain.toml` and `.github/workflows/ci.yml` both
  now pin the exact Rust version verified locally this session (`1.98.1`),
  replacing the previously-floating `stable` channel. A compiler bump is now
  a deliberate, reviewed change to `rust-toolchain.toml`, not something that
  can silently change what CI checks.
- **Lockfile enforcement**: `--locked` is used for clippy, test, and the new
  demo-run CI step (`cargo run --locked -p samgtdd --example demo`).
- **Runner**: pinned `ubuntu-24.04` (unchanged from before this work).
- **Action pinning**: resolved. `actions/checkout`, `Swatinem/rust-cache`, and
  `actions/upload-artifact` are now pinned to commit SHAs, each with a
  readable version comment, verified via `gh api` on 2026-09-11 once network/
  `gh` access became available in this session:
  - `actions/checkout` → `11d5960a326750d5838078e36cf38b85af677262` (tag
    `v4.4.0`, resolved via `gh api repos/actions/checkout/git/refs/tags/v4`).
  - `Swatinem/rust-cache` → `6323deb102c322ba6fcbdcafc7e3dddab59af2b6` (tag
    `v2` → release `2.9.2`; `v2` is a PGP-signed annotated tag, resolved to
    its underlying commit via `gh api .../git/tags/<tag-object-sha>`, and the
    signature verification in that response reports `"verified": true`).
  - `actions/upload-artifact` → `ea165f8d65b6e75b540449e92b4886f43607fa02`
    (tag `v4.6.2`).
  No SHA here was invented; each was read directly from GitHub's API against
  the real tag.
- **Toolchain availability**: `rustc`/`cargo` 1.98.1 is a real, published
  `rust-lang/rust` release (tag `1.98.1`, published 2026-09-03), confirmed via
  `gh api repos/rust-lang/rust/releases` — not assumed from the locally
  installed Homebrew copy alone. Whether `rustup toolchain install 1.98.1`
  itself succeeds on the `ubuntu-24.04` runner (i.e. whether rustup's
  distribution server has published the corresponding installable artifacts)
  is confirmed by the CI run below, not by this check alone.
- **Explicit timeouts**: the job has `timeout-minutes: 20`, and every step has
  its own `timeout-minutes` bound, so a hang (rather than a clean failure)
  can't consume unbounded CI time.
- **Evidence retention**: the acceptance demo runs in CI (`if: always()`, so
  it still runs and uploads even if an earlier step failed) and
  `artifacts/acceptance/` is uploaded via `actions/upload-artifact` on every
  run.
- **Hosted CI status**: see "Hosted CI run" below.

## Hosted CI run

Observed directly via `gh` (not assumed, not carried over from an earlier
revision):

- **PR**: https://github.com/greenskeleton/samgtd/pull/1
  (`milestone-001/acceptance-and-ci` → `main`, draft)
- **Pushed branch head**: `6bef60deb069d71435d819e3ad1066231edcb9ef`
  (`git rev-parse HEAD` on the branch before push)
- **Run**: https://github.com/greenskeleton/samgtd/actions/runs/34630462929 —
  `conclusion: success`, 41s, triggered by the `pull_request` event on the
  commit above.
- **Checked-out commit inside the run**: `3da218904a634f74ea371147031e413043bb1215`.
  This differs from the pushed head SHA because `actions/checkout` on a
  `pull_request` event checks out GitHub's synthetic PR **merge commit**
  (branch content merged onto `main`), not the raw branch head — standard,
  expected GitHub Actions behavior, not a mismatch to be concerned about.
  The downloaded results artifact's `source.commit` field records exactly
  this merge commit, with `source.dirty: false` and `source.fingerprint:
  null` — i.e. the run tested a clean checkout of this changeset, not a
  dirty/ambiguous tree.
- **Hosted results artifact** (downloaded via `gh run download 34630462929`
  and inspected): `result: "pass"`, 60/60 checks `passed: true`, 0
  `passed: false` — the same 60 checks enumerated above, now independently
  reproduced on a hosted `ubuntu-24.04` runner rather than only locally.
  `toolchain.rustc`/`toolchain.cargo` both report `1.98.1`, confirming
  `rustup toolchain install 1.98.1` succeeded on the runner (not just that
  the version exists upstream, per the toolchain-availability check above).
- **Note**: the run logged one informational annotation — "Node.js 20 is
  deprecated… forced to run on Node.js 24" for `actions/checkout` and
  `actions/upload-artifact` at the pinned SHAs above. This did not fail the
  run; it's a heads-up that the next time these Actions are re-pinned, a
  newer release with native Node 24 support should be preferred.

This satisfies the milestone's hosted-CI requirement for this changeset. A
prior green run on `main` (commit `2d0b162`, predating every change in this
document) remains explicitly not evidence for this revision — this run,
against this commit, is.

## Review disposition

Findings from `.agent/claude-final-review.md` (the most recent independent
review), in its numbering:

| # | Severity | Finding | Disposition |
|---|---|---|---|
| 1 | Medium | Next root-level entity key could reproduce ADR 0003's shared-object-identity bug | **Fixed by decision, not code** — `docs/adr/0005-schema-evolution-rules.md` records the rule (single-creator, sync-propagated root keys) and requires an analogous rejection test before any new root key ships. No new root key exists yet in Milestone 001, so there is nothing to add the test *for* yet. |
| 2 | Medium | A future required `TaskFields` field would break reading of every already-synced older document | **Fixed by decision, not code** — same ADR, decision 2: new CRDT-layer fields default to optional-with-explicit-default. No new field exists yet in Milestone 001. |
| 3 | Medium | Convergence tested for exactly two peers; three-peer/transitive propagation unverified | **Deferred, explicitly out of scope for this milestone** — this work's instructions explicitly exclude three-peer validation from Milestone 001. Follow-up: a three-replica test (A and B both connect to C, not to each other; independently edit a shared task; a fresh D provisions from C and receives both edits without talking to A or B) belongs in a follow-up milestone. |
| 4 | Low | `Service`'s single-writer guarantee lives in `transport::App`, not `Service` itself | **Fixed (documentation only)** — `crates/samgtdd/src/service.rs`'s module doc now states explicitly that `Service` does not serialize concurrent callers itself and that `transport::App::call`'s `Mutex<Service>` is what provides that guarantee today; a future direct embedder (e.g. FFI) must provide its own serialization. No behavior changed. |
| 5 | Low | Task status validity duplicated, unsynchronized, between `samgtd-crdt` and `samgtd-domain` | **Fixed** — `samgtd_domain::is_valid_status` is now the single definition; `TaskDocument::set_status` (`samgtd-crdt`) and `TaskFields::validate` (`samgtd-domain`) both call it. |
| 6 | Low | `Service::join` could silently reassign an already-joined-but-still-empty replica to a different dataset | **Fixed** — `join()` now writes a sticky `local/joined` marker and refuses a second call regardless of current task count. New test: `crates/samgtdd/src/service.rs::tests::join_is_rejected_after_the_replica_has_already_joined_once`. |
| 7 | Low | CI toolchain/actions still float | **Fixed** — `rust-toolchain.toml` and CI both pin the exact verified toolchain (`1.98.1`, confirmed as a real published `rust-lang/rust` release via `gh api`) instead of `stable`. All three third-party Actions (`actions/checkout`, `Swatinem/rust-cache`, `actions/upload-artifact`) are now pinned to `gh api`-verified commit SHAs with version comments. |

## What remains untested / operator follow-ups

- **Three-or-more-peer / transitive propagation** is explicitly out of scope
  for this milestone (review finding 3) and remains a named follow-up, not a
  defect in what Milestone 001 claims.
- **Large-dataset scheduling, aggregate session/memory limits, and refined
  HTTP error status codes** remain deferred hardening, per
  `docs/milestone-001.md` and ADR 0004 — unchanged by this work, not claimed
  as done.
- **Legacy database import/coexistence** remains fully deferred per ADR 0003;
  this work did not open, read, or modify `.local/current-gtd.sqlite` — it
  ran with no such file present, and all synthetic fixtures use fresh
  temporary SQLite stores under the OS temp directory, never a path the
  operator's real database could occupy.
- **Log content** is produced and captured for diagnostics (see above) but
  this work does not assert on specific structured-log fields/content.
