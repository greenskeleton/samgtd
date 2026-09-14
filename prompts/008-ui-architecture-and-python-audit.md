# Prompt 008 — UI architecture and legacy Python audit

You are the next architecture/audit agent for samgtd.

The repository is the source of truth.

Read first:
1. `AGENTS.md`
2. all current ADRs
3. architecture/protocol/API docs
4. Milestone 001 completion notes/tests
5. `PROJECT-KNOWLEDGE-HANDOFF.md`
6. daemon/API code
7. any existing Python UI/TUI code

Do not trust the handoff over the repository.

## Goal

Prepare the repository for an MVP TUI and web UI without undermining the completed Rust daemon/CRDT architecture.

## Tasks

A previous implementation exists outside this repository at:

    ~/Development/samgtd-python

This directory is a READ-ONLY reference implementation.

You MUST inspect it as part of this task even though it is outside the
current repository.

Do not:
- edit files in ~/Development/samgtd-python
- commit anything there
- migrate/copy the entire project into this repository

### Required investigation

Perform both static and runtime investigation.

1. Inspect the source tree and determine:
   - framework and dependencies
   - application entry point
   - screen/layout structure
   - widgets/components
   - keyboard bindings
   - navigation/focus model
   - fuzzy search / MultiSuggest implementation
   - task/project/context/category/report presentation
   - database coupling
   - reusable domain-independent UI code

2. Determine how the legacy application is normally started.

3. Attempt to run the application locally.

   Running the legacy application for observation is explicitly authorized.

   Do not modify its real database or persistent data merely to test it.
   Prefer an existing test/sample database or make a disposable copy if one
   is required.

4. Observe the running UI and compare runtime behavior with what is inferred
   from the source code.

   Specifically inspect:
   - overall screen layout
   - pane proportions
   - focus indication
   - task row presentation
   - scope/report/project navigation
   - dialogs/editors
   - MultiSuggest behavior
   - search
   - help/keymap presentation
   - Done visibility
   - keyboard navigation
   - resizing behavior

5. Where practical, capture textual notes describing each important screen
   and interaction.

6. Treat runtime behavior as useful evidence, but use source inspection to
   understand behavior that cannot easily be exercised.

Write the findings into:

    docs/ui/legacy-python-audit.md

The current samgtd repository remains the source of truth for the new
architecture. The legacy repository is a UX/reference implementation, not
an architectural authority.

### Verify the current application boundary

Document:
- client connection model;
- current HTTP endpoints;
- WebSocket/sync endpoints;
- auth assumptions;
- task/project/context/category/report representations;
- UI-critical operations available;
- UI-critical operations missing.

Create or update durable API-facing documentation.

### Audit existing Python UI

If old Python UI/TUI code exists, create:

`docs/ui/legacy-python-audit.md`

Classify pieces as:
- reusable;
- adaptable;
- discard.

Inspect especially:
- keyboard navigation;
- layouts;
- task list rendering;
- project/scope panes;
- fuzzy search;
- MultiSuggest/multi-select;
- help/keymaps;
- direct SQLite coupling;
- obsolete schema assumptions.

Do not choose Rust merely because the daemon is Rust. Optimize for maintainable MVP speed.

### Recommend TUI path

Evaluate:
- adapting existing Python UI;
- new Python TUI (e.g. Textual if appropriate);
- Rust TUI (e.g. Ratatui if appropriate).

Choose based on reuse, implementation speed, testability, keyboard UX, fuzzy/multi-select needs, API integration, and maintenance.

Record the decision in an ADR.

### Recommend web stack

Default candidate:
- Vite
- React
- TypeScript

Change only for a concrete repository-specific reason.

Document package layout, API client strategy, keyboard command handling, testing, and build integration.

### Define shared semantic actions

Create:

`docs/ui/actions.md`

Define actions independently from keys, including:
- selection.next
- selection.previous
- pane.left
- pane.right
- item.open
- item.edit
- item.complete
- task.new
- task.reorder_up
- task.reorder_down
- view.toggle_done
- help.open
- search.open
- scope.next
- scope.previous

Map proposed TUI and web bindings separately.

### Define MVP

Create:

`docs/ui/mvp.md`

Cover:
- daemon connection;
- useful scope/report;
- browse tasks;
- create/edit/complete;
- project/category/context assignment;
- keyboard-first navigation;
- discoverable help;
- search;
- Done visibility.

List API blockers explicitly.

### Scope discipline

This is primarily an audit/architecture prompt.

Small proof-of-concept code is allowed only to validate a framework choice.

Do not build the entire TUI or web UI yet.

## Final report

Summarize:
- UI-relevant API surface;
- Python salvage recommendation;
- TUI stack choice;
- web stack choice;
- action/keymap strategy;
- API gaps;
- recommended implementation order for 009+.

Do not push.
