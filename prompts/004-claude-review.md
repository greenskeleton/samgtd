Perform a final architecture and correctness review of the current Milestone 001 implementation.

Read AGENTS.md, ADRs, `.agent/codex-review.md`, and current source/tests.

Focus on defects that would make future Android/Tailscale/mobile synchronization painful:
- incorrect CRDT authority;
- wrong document boundaries;
- hidden server-centric assumptions;
- peer identity/sync-state mistakes;
- schema evolution traps;
- persistence/restart gaps;
- transport/domain coupling;
- missing conflict/convergence cases.

Run the verification suite.

Write findings to `.agent/claude-final-review.md`.

Do not rewrite the project merely to match personal style.
This is a review-only step; leave findings for the implementation/CI handoff.
Feature-branch publication is authorized under AGENTS.md; merging is manual.
