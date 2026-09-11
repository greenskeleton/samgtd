# Agent instructions

This repository is being built collaboratively by Claude Code and OpenAI Codex.

## Mission

Build a robust, local-first GTD system. The first deliverable is a Rust API daemon with CRDT-based synchronization.

## Non-negotiable constraints

- Rust for the daemon and core libraries.
- Tokio async runtime.
- Axum HTTP/WebSocket layer unless an ADR justifies changing it.
- Automerge CRDT in Rust.
- SQLite for durable local storage and projections.
- Stable UUID identifiers; do not use mutable names as identity.
- Offline edits and deterministic convergence are first-class requirements.
- Primary network is local/private.
- Future Android access through Tailscale must not require redesigning domain or CRDT layers.
- Do not implement WireGuard/Noise cryptography inside this application.
- No public Internet listener by default.
- No cloud dependency for core GTD operation.
- API/domain layers must be usable by future TUI, web, Android, voice-agent, and MCP interfaces.
- Write ADRs for significant architectural decisions.

## Initial GTD semantics

The durable model must eventually support:

- Task
  - required category
  - optional project
  - zero or more contexts
  - status: Todo or Done
- Project
  - hierarchical support is desired
- Context
- Category
  - initial concepts: Inbox, Next Actions, Someday/Maybe, Waiting For, Agenda
- Saved reports / queries
- report-specific sort order

Do not attempt to implement all semantics in the first commit. Preserve room for them in the architecture.

## Development rules

- Prefer small crates with explicit dependency direction.
- Domain code must not depend on Axum.
- Domain code should not depend directly on SQLite.
- CRDT representation and domain projection must be separable/testable.
- Network framing must not leak through the domain layer.
- Favor explicit error types (`thiserror`) in libraries and contextual application errors where useful.
- Structured logging via `tracing`.
- Configuration via CLI/env/config struct; default to loopback.
- No `unsafe` unless justified in an ADR.
- No `unwrap()`/`expect()` in daemon request paths unless invariant is proven and commented.
- Tests are part of the feature.
- Never disable a failing test merely to make CI green.

## Git safety

Agents may inspect git status/diff/log.

For implementation and orchestration tasks, agents are authorized to create or
reuse a feature branch, commit task-related changes, push that branch to origin,
and create/update a draft pull request without requesting further permission.
Inspect existing collaborative changes before staging; preserve unrelated work
and never indiscriminately stage private inputs or runtime artifacts.

Agents may run, watch, rerun, and inspect GitHub Actions for the feature branch,
download CI artifacts, and fix failures with additional commits and pushes.
Verify the target branch/ref before pushing or dispatching a workflow. This
authorization covers CI validation, not release or deployment workflows.

Merging requires manual human intervention. Agents must not merge PRs, enable
auto-merge, enqueue a merge, or push to main, master, or any protected/release
branch. Leave a green PR for the operator to review and merge. Do not change
repository protections, secrets, environments, or deployment settings.

Agents must not:

- force push;
- rewrite history;
- delete branches;
- modify global git configuration;
- commit secrets.

## Completion behavior

Before declaring a task complete, run when applicable:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

For implementation orchestration, publish the feature branch and draft PR,
watch CI for the latest pushed commit, fix relevant failures, and repeat until
required checks pass or a concrete external blocker is established. An older
green run does not validate newer changes. Report the PR URL, tested commit,
CI run URL/results, and any blockers. Human merge is the final handoff, not an
agent action. Review-only tasks need not publish changes.

Summarize:
- files changed;
- decisions made;
- tests run;
- remaining risks/questions.


## Existing database compatibility

A real, working SQLite database already exists and contains user data.

Before making durable schema, identity, persistence, or CRDT-document decisions:

- request the existing database from the operator if `.local/current-gtd.sqlite` is not present;
- do not invent a replacement schema first;
- inspect the database read-only;
- inventory tables, columns, indexes, foreign keys, views, triggers, and relevant SQLite pragmas;
- identify stable IDs and cross-table relationships;
- inspect enough representative data to discover implicit invariants, but do not dump private row data into logs or documentation;
- preserve the existing structure wherever practical;
- document any required migration explicitly in an ADR;
- never modify the supplied working database in place;
- create copies/fixtures for experiments and tests.

The real database must never be committed to the public repository.
