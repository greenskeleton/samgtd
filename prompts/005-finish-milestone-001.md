You are the implementation and acceptance agent, using either Codex or Claude Code.

Finish Milestone 001's remaining implementation and local acceptance work, and
produce reproducible evidence of what works. Work through implementation and
verification, not just a plan. Do not declare the milestone complete without
evidence for every required item, including hosted CI.

## Read first

- `AGENTS.md` and any applicable nested instructions.
- `README.md`, `docs/milestone-001.md`, `docs/architecture.md`, `docs/protocol.md`.
- All `docs/adr/*` and `tests/integration/README.md`.
- `.agent/codex-review.md` and `.agent/claude-final-review.md`, if present.
- Current daemon, CRDT, store, tests, and CI implementation.

Inspect git status/diff before editing. This checkout may contain substantial
uncommitted collaborative work. Preserve it and build on it; do not reset,
discard, or overwrite unrelated work. Verify review claims against current code:
older findings may already be fixed or may be incorrect.

## Starting point to verify

The September 11, 2026 assessment found task CRUD, stable node/dataset identity,
explicit provisioning, SQLite durability, native Automerge WebSocket sync,
loopback defaults, logging, and bounded shutdown implemented. All 19 tests and
format/Clippy checks passed locally. Current sync tests use independent stores
and real HTTP/WebSocket handling over in-memory streams. A subprocess test
checks committed-write survival after a kill. This is not yet proof of two
daemon binaries synchronizing over TCP. Hosted CI was not verified.

Replicas are symmetric: a hub is an optional topology, not an authoritative
master. HTTP-only clients are not independent offline replicas. Preserve these
semantics. The daemon currently accepts inbound sync connections; tests supply
a transparent relay. Automatic outbound dialing, discovery, and retry are absent.

## Required work

### 1. Real two-process acceptance test and runnable demo

Add an automated test that launches two actual `samgtdd` binaries with separate
temporary SQLite stores and loopback TCP listeners. Exercise their public HTTP
and WebSocket interfaces over actual sockets. Do not substitute in-process
routers, direct service calls, or snapshot copying for the network exchange.

Provide a small runnable client/relay or demo harness that connects both `/sync`
endpoints and forwards native protocol frames bidirectionally. Reuse the harness
between the test and demo where practical. A relay is sufficient for this
milestone; a production peer manager is not required. Clearly document its role.
Keep Rust/Tokio for any new core or daemon functionality.

The scenario must:

1. Start A and B, wait for readiness, verify health, distinct node UUIDs, and
   loopback listeners. Provision empty B into A's dataset through HTTP.
2. Create a synthetic task on A while disconnected; connect and verify that B
   discovers and receives it through native Automerge sync.
3. Disconnect the sync link. Edit the same shared task independently on A and B
   in different fields; create additional tasks offline on both sides.
4. Reconnect. Assert preservation of both independent field edits, all task IDs,
   complete projected task values, and equal CRDT heads for root and task documents.
5. Exercise a same-field concurrent conflict. Assert equal resolution on both
   peers, without assuming wall-clock last-write-wins or hardcoding an arbitrary
   winner. Preserve the existing lower-layer conflict tests.
6. Verify that a local edit propagates over an already-open idle sync connection.
7. Stop both processes, restart them from their respective stores, and verify
   identity and full state persistence. Reconnect again and assert matching
   heads, no missing tasks, and no duplicate semantic entities.
8. Verify an HTTP-acknowledged write survives abrupt termination of an actual
   daemon process. Verify graceful termination with an active sync connection
   completes within a bounded time.

For document-head and membership assertions unavailable through the public API,
inspect each synthetic store after its daemon exits using a read-only test
helper, or introduce a narrowly justified diagnostic interface. Do not add a
public document dump endpoint merely for testing. Never read or mutate one
replica's database to drive another replica's synchronization.

Use bounded startup/readiness/sync/shutdown timeouts, reliable cleanup on failure,
isolated temporary paths, and robust port allocation. Avoid fixed sleeps as
readiness checks. Include useful failure diagnostics and captured subprocess
output. Failed socket binding must fail or be explicitly reported as blocked;
never silently skip the test or substitute in-memory transport and report success.

### 2. Acceptance evidence and operator documentation

Provide one documented command to run the synthetic demo without external
services or the user's real database. The demo should report each assertion and
exit nonzero on failure. Generate:

- A machine-readable results artifact containing scenario/check outcomes,
  timestamps, toolchain, source revision, and whether the working tree is dirty.
  For dirty runs, include a source fingerprint covering relevant modified and
  untracked source inputs, so a commit ID alone does not misrepresent provenance.
- A readable transcript of synthetic API operations, offline divergence,
  convergence, identities, document-head comparisons, and restart results.
- `docs/milestone-001-acceptance.md`, mapping every required milestone item to
  its test/demo evidence, exact reproduction commands, observed results, and
  remaining limitations. Distinguish assertions from untested expectations.

Keep generated logs/databases in an ignored artifact directory. Commit-worthy
documentation must contain no real task data, secrets, or machine-specific
private paths. A terminal recording is optional; executable assertions take
priority. Do not fabricate output, a recording, or a hosted CI result.

Update `README.md`, `docs/milestone-001.md`, `docs/protocol.md`, and
`tests/integration/README.md` as needed to describe the final implementation and
give working launch/provision/connect/demo commands. Remove stale environment
claims only when new evidence supersedes them.

### 3. CI and reproducibility

Run the real TCP/subprocess acceptance test in GitHub Actions, with explicit
timeouts, and retain synthetic evidence/failure logs as artifacts. Keep tests
mandatory; do not ignore or disable failing tests to get green CI.

Pin an available, verified Rust toolchain consistently for local and hosted
checks. Pin third-party Actions to verified commit SHAs with readable version
comments; do not invent hashes. Preserve lockfile enforcement and the explicit
runner release. Explain any reproducibility work blocked by network access.

Publish and validate the feature branch through hosted CI under AGENTS.md. A green run on an older
revision does not validate the current working tree. Record the run URL and
tested commit only when actually verified.

### 4. Review disposition and scope control

Fix defects uncovered by the acceptance work. Document a disposition for each
remaining final-review finding: fixed, invalid with code evidence, or deferred
with a concrete follow-up. In particular, record the shared-root schema
extension and backward-compatible field evolution rules before future entity
types or required fields ship. Use an ADR/addendum for architectural decisions.

Do not expand Milestone 001 into three-peer validation, production scheduling,
global resource budgets, complete GTD semantics, legacy import, UI, Android,
Tailscale automation, authentication, voice, MCP, or Redmine. Track those as
follow-ups unless an actual defect makes a narrow change necessary for this
milestone. Do not introduce a privileged central replica or custom cryptography.

Respect the existing-database rules in `AGENTS.md`. If durable schema, identity,
persistence, or CRDT representation changes become necessary, inspect the supplied
database read-only first; request it if absent. Do not invent a replacement
schema, migrate the working database, or modify it in place. Synthetic acceptance
fixtures must remain separate from that database.

## Verification and completion

Run:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

Also run the documented demo and CI-equivalent locked commands. Check that
the generated evidence agrees with actual results and that no private data or
runtime databases entered the proposed diff.

Preserve existing work. Follow AGENTS.md's authorization to commit and push a
feature branch, create/update a draft PR, and run/fix GitHub CI until the latest
pushed revision passes. See prompts/007-feature-branch-ci.md for the handoff.
If sandbox or network restrictions block validation, use the environment's
authorized approval path where available, complete unaffected work, and report
the precise blocker. Never merge or enable auto-merge; humans merge manually.

Finish with:

- Files changed and key decisions.
- Exact checks run, counts/results, and artifact paths.
- What the evidence proves and what remains untested.
- Review findings resolved or deferred.
- PR URL, latest tested commit, hosted CI results, and remaining human review/merge steps.
- An explicit milestone verdict: complete only if all required evidence exists;
  otherwise locally validated with named external gaps, or incomplete with named
  implementation/verification blockers.
