# samgtd bootstrap

Bootstrap package for a local-first GTD service written in Rust.

## Goal of phase 1

Build a **headless Rust daemon** that:

- owns the GTD domain model and persistence;
- uses **Automerge CRDT documents** as the replication source of truth;
- exposes a versioned local HTTP API using Axum;
- exposes an Automerge sync endpoint over WebSocket;
- is safe to bind to loopback or LAN addresses;
- does **not** require Tailscale, but keeps transport/auth cleanly separated so Tailscale can be used later;
- can eventually support Android/mobile clients that work offline and synchronize when connectivity returns.

The first phase deliberately does **not** build a TUI, web UI, Android app, voice agent, MCP server, Redmine integration, or cloud sync service.

## Architectural stance

### CRDT

Use the Rust `automerge` crate directly. Do not make the daemon dependent on the JavaScript Automerge Repo server protocol.

The daemon owns:

1. document lifecycle;
2. persistence;
3. Automerge sync state;
4. WebSocket framing;
5. domain validation/projection.

This leaves the wire transport replaceable and allows a future Android client to sync over LAN, Tailscale, or another Noise-secured channel.

### Persistence

Phase 1 should use SQLite for durable daemon metadata and Automerge document bytes/change data.

Treat Automerge state as authoritative for replicated GTD entities. SQL tables may be used for indexing/projections, but do not create a second independent mutable source of truth.

### Document granularity

Do **not** put the entire GTD database into one giant Automerge document.

The initial design should make document granularity an explicit decision. Preferred starting point:

- a small root/index document identifying the dataset and known entity documents;
- separate Automerge document per mutable aggregate/entity (task, project, report, etc.);
- references by stable UUID.

The first agent must validate this against Automerge sync/persistence costs and record the decision in an ADR.

### Network

Initial daemon modes:

- default: `127.0.0.1`;
- optional LAN bind: explicit configured address;
- later: Tailscale IP / `tailscale serve` / private DNS.

**Do not implement Noise yourself.** Tailscale already supplies the encrypted peer network. Keep application authentication/authorization separate from transport security.

## Repo layout target

The agents may refine this, but the intended boundary is:

```text
crates/
  samgtd-domain/      Pure GTD domain types/rules
  samgtd-crdt/        Automerge schema, serialization, merge helpers
  samgtd-store/       SQLite persistence/projections
  samgtd-api/         HTTP/WebSocket contracts
  samgtdd/            Daemon binary

docs/
  adr/
  architecture.md
  protocol.md

tests/
  integration/
```

## macOS prerequisites

Install:

```sh
xcode-select --install
brew install rustup-init git gh jq
rustup-init
source "$HOME/.cargo/env"
rustup toolchain install stable
rustup default stable
```

Install/login to the two official agents using the vendors' current instructions, then verify:

```sh
claude --version
codex --version
claude
codex
```

The interactive launches above are the preferred way to complete subscription sign-in before running the scripts.

## Start from this zip

```sh
unzip samgtd-bootstrap.zip
mv samgtd-bootstrap samgtd
cd samgtd

git init -b main
git add .
git commit -m "Bootstrap local-first GTD project"
```

Optional public GitHub repo:

```sh
gh auth login
gh repo create samgtd --public --source=. --remote=origin --push
```

If `samgtd` is already taken under your account, substitute another repository name.


## Existing working SQLite database handoff

**Important:** this project already has a working SQLite database containing real data. Its structure is a compatibility constraint.

The first Claude session must pause before making durable database/CRDT schema decisions and ask the operator to provide the existing SQLite database.

When Claude asks for it:

1. Copy the database into the repository as:

   ```sh
   mkdir -p .local
   cp /path/to/your/current/database.sqlite .local/current-gtd.sqlite
   ```

2. Tell Claude that `.local/current-gtd.sqlite` is available for **read-only analysis**.

3. Do not add or commit the database. `.local/` is ignored by Git.

The agent should inspect the existing schema and representative data before accepting the CRDT/storage ADR. It must preserve compatibility unless an explicit migration is documented and approved.


## Exact command: first agent

Run Claude as the architecture/scaffold agent while macOS is prevented from sleeping:

```sh
./scripts/agent.sh claude prompts/001-claude-bootstrap.md
```

This is intentionally interactive for the first pass, so you can inspect/approve any command Claude wants to run.

When it completes:

```sh
git status
git diff
cargo fmt --all -- --check
cargo test --workspace
```

For a manual checkpoint, you can commit the first pass yourself:

```sh
git add .
git commit -m "Scaffold Rust local-first daemon"
```

Then use Codex as an independent reviewer:

```sh
./scripts/agent.sh codex prompts/002-codex-review.md
```

After addressing review findings, the first implementation pass is:

```sh
./scripts/agent.sh codex prompts/003-codex-implement.md
```

Then have Claude review the resulting code:

```sh
./scripts/agent.sh claude prompts/004-claude-review.md
```

## Unattended sequence

After the repo is stable enough for unattended agents:

```sh
./scripts/orchestrate.sh
```

This keeps the Mac awake for the entire sequence and records logs under `.agent/runs/`.

The default sequence is:

1. Claude architecture/scaffold
2. Codex independent review
3. Codex implementation/fixes
4. Claude final review
5. Codex feature-branch publication and GitHub CI handoff

The orchestrator starts from step 1 by default; it does not automatically detect
steps completed in earlier runs or through `agent.sh`. To resume after the
bootstrap and Codex review have completed (with `.agent/codex-review.md` saved):

```sh
./scripts/orchestrate.sh --from codex-implement
```

To rerun the Codex review first, use `--from codex-review`. The selected step and
all subsequent steps will run.

Implementation agents may commit and push feature branches, open/update draft
PRs, run GitHub CI, and fix failures until the latest pushed commit passes.
The final `codex-ci` orchestration step performs this handoff. Agents must never
merge, enable auto-merge, or push to the default/protected branch; a human
reviews and merges the PR. Release/deployment workflows remain outside scope.

To run only the publication/CI handoff after implementation and review:

```sh
./scripts/orchestrate.sh --from codex-ci
```

To resume the existing Claude milestone session with this policy:

```sh
claude --continue "$(cat prompts/006-resume-milestone-001.md)"
```

Claude's command allowlist permits Git operations and `gh`; AGENTS.md governs
allowed branch targets and actions. Both Codex launchers enable outbound
networking while retaining the workspace filesystem sandbox. These settings
are not GitHub branch protection; human-merge policy also applies to API calls.

## CI

The GitHub Actions workflow (`.github/workflows/ci.yml`) runs, on a pinned
Rust toolchain (`1.98.1`, matching `rust-toolchain.toml`) and pinned
`ubuntu-24.04` runner:

- `cargo fmt --all -- --check`
- `cargo clippy --locked --workspace --all-targets --all-features -- -D warnings`
- `cargo test --locked --workspace --all-features` (includes the real
  two-process acceptance test)
- `cargo run --locked -p samgtdd --example demo`, uploading
  `artifacts/acceptance/` (results JSON + transcript) as a workflow artifact
  on every run, including failed ones.

The agents are required to keep CI green. Hosted CI has been observed green
for the current changeset (PR #1, run
https://github.com/greenskeleton/samgtd/actions/runs/34630462929) — see
`docs/milestone-001-acceptance.md`, "Hosted CI run".

## First milestone definition

Phase 1 is complete when two daemon/client processes can demonstrate:

1. create a task while peer B is disconnected;
2. make a concurrent edit on peer B;
3. reconnect;
4. synchronize using Automerge;
5. converge to equivalent document heads/state;
6. restart both processes;
7. load the converged state from disk;
8. repeat sync without losing or duplicating semantic entities.

That test matters more than building CRUD endpoints quickly.

## Milestone 001 acceptance evidence

Run the synthetic two-process acceptance demo (real loopback TCP, real
`samgtdd` subprocesses, temporary SQLite stores only — never a real/user
database):

```sh
cargo run -p samgtdd --example demo
```

It prints a transcript of every check, writes a machine-readable results
JSON and a human-readable transcript under the gitignored `artifacts/`
directory, and exits nonzero if anything failed. The same scenario also runs
as an automated test (`cargo test -p samgtdd --test two_process_acceptance`,
included in `cargo test --workspace`). See `docs/milestone-001-acceptance.md`
for the full requirement-by-requirement evidence mapping, exact reproduction
commands, the hosted CI run, and named remaining gaps.
