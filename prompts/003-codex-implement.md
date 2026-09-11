You are the implementation agent.

Read:
- AGENTS.md
- all docs/adr/*
- docs/milestone-001.md
- .agent/codex-review.md if present

Implement the highest-value fixes necessary to make Milestone 001 a real vertical slice.

Priority order:

1. Correct CRDT model/sync semantics.
2. Persistence across restart.
3. Two-peer offline concurrent-edit convergence integration test.
4. Clean domain/CRDT/storage/transport boundaries.
5. HTTP/WebSocket daemon behavior.
6. CI/tooling/documentation.

Do not expand scope into UI, Android, Tailscale configuration, voice, MCP, or Redmine.

Do not implement custom encryption.

The convergence test should demonstrate:
- initial shared state;
- peers diverge while disconnected;
- both mutate meaningful Task fields;
- sync resumes;
- both converge;
- reload from disk retains converged state.

Run:
- cargo fmt --all -- --check
- cargo clippy --workspace --all-targets --all-features -- -D warnings
- cargo test --workspace --all-features

Update ADRs when implementation changes an architectural decision.

Do not commit or push.

At the end, summarize changes, verification, and remaining Milestone 001 gaps.
