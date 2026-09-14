# Prompt 012 — MVP hardening and next-milestone plan

You are the hardening/planning agent.

The repository is the source of truth.

Assume TUI and web MVPs exist.

## Goal

Turn the MVP into a stable foundation and define the next milestone without premature expansion.

## Tasks

### Verify whole repository

Run:
- Rust fmt/clippy/tests;
- TUI tests;
- web typecheck/lint/tests/build;
- integration tests.

Fix clear MVP regressions.

### Failure modes

Review:
- daemon unavailable;
- daemon restart;
- stale client state;
- invalid edits;
- API errors;
- network interruption;
- duplicate submissions;
- UI starts before daemon;
- CRDT update arrives while item is selected/edited.

Add reasonable handling for high-impact cases.

### Performance

Test with a realistically sized local dataset.

Look for obvious issues in:
- task list rendering;
- report switching;
- search;
- project tree;
- API chatter.

Do not prematurely optimize CRDT internals.

### Developer workflow

Make local startup and verification simple and documented.

### Next milestone

Propose, but do not implement, priorities among:
- richer report editor;
- project tree improvements;
- task ordering;
- weekly review workflow;
- Android/mobile;
- Tailscale convenience;
- MCP;
- voice agent;
- Redmine external tasks.

Base recommendation on the current repo.

Write a durable roadmap/milestone document.

## Output

Summarize:
- MVP readiness;
- defects;
- tests;
- recommended next milestone;
- explicitly deferred work.

Do not push.
