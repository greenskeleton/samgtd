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

Agents must not:

- force push;
- rewrite history;
- push directly to a remote unless explicitly instructed;
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
