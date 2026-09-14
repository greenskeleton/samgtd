# Prompt 011 — Cross-interface integration and UX convergence

You are the integration/review agent.

The repository is the source of truth.

Read all UI ADRs/docs and both implementations.

## Goal

Make TUI and web feel like two interfaces to the same application without forcing artificial rendering-code sharing.

## Review

### Semantic consistency

Compare:
- action names;
- terminology;
- report/scope behavior;
- Done visibility;
- project/category/context behavior;
- task creation defaults;
- validation/error presentation.

Resolve accidental differences.

### Keymaps

Compare both to `docs/ui/actions.md`.

Update shared spec if reality requires changes.

Ensure both have discoverable in-app help.

### API correctness

Ensure both:
- use daemon;
- do not access SQLite;
- avoid unnecessary duplicated business rules.

Refactor accidental one-frontend API shortcuts.

### End-to-end acceptance

Create the strongest practical test/script for:
1. start daemon;
2. create task in TUI;
3. observe in web;
4. modify/complete in web;
5. observe update in TUI.

Automate as much as practical; otherwise create a deterministic manual acceptance procedure.

### MVP usability

Verify a new user can discover:
- create task;
- navigate;
- search;
- toggle Done;
- get help.

Fix high-impact gaps.

### Documentation

Make docs match current implementation and clearly mark/delete superseded planning docs.

## Do not

- redesign CRDT without evidence;
- replace a working frontend for technology consistency;
- add Android/Tailscale convenience/voice/MCP yet;
- prioritize polish over correctness.

## Output

Write `docs/ui/mvp-review.md`.

Include:
- what works;
- differences;
- acceptance results;
- known issues;
- next-milestone recommendation.

Run all verification.

Do not push.
