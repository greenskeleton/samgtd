Act as an independent senior Rust/local-first reviewer.

Read AGENTS.md and all ADRs first. Then inspect the full diff/repository produced by the first architecture agent.

Do not assume the architecture is correct because another agent proposed it.

Review specifically:

1. Is Automerge being used as a genuine replicated source of truth rather than as decoration around conventional CRUD?
2. Will the chosen document granularity scale to thousands of GTD tasks without forcing every edit through one giant document?
3. Is Automerge's sync state scoped correctly per peer and document?
4. Is persistence crash/restart safe enough for Milestone 001?
5. Is there a clear separation among:
   - GTD domain;
   - CRDT representation;
   - SQLite persistence/projections;
   - HTTP/WebSocket transport?
6. Does default networking remain loopback/private?
7. Could a future Android client connect over Tailscale without redesigning the sync/domain layers?
8. Are concurrent offline edits actually tested?
9. Do tests prove convergence, not merely successful request/response behavior?
10. Are error handling, cancellation, graceful shutdown, and logging appropriate for a daemon?
11. Is CI reproducible?

Do not make broad changes yet unless needed to run/verify tests.

Write a review to `.agent/codex-review.md` containing:
- Critical
- High
- Medium
- Low
- Recommended implementation order

Include file/line references where possible.

Run the project's verification commands and include results.

This is a review-only step; leave findings for the implementation/CI handoff.
Feature-branch publication is authorized under AGENTS.md; merging is manual.
