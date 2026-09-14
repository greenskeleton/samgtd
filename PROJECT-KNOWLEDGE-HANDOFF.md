# samgtd project knowledge handoff

This file is a planning/context handoff only. The **repository is the source of truth**.

Agents must read the current repository, ADRs, docs, API contracts, tests, and Milestone 001 implementation before acting. If this file conflicts with the repository, the repository wins.

## Current project state

- Milestone 001 is considered complete by the prior agent.
- The Rust API daemon and CRDT synchronization are believed to be functioning between instances.
- The current priority is to make the system usable.
- Next goal: functional MVP interfaces:
  - TUI
  - web UI
- Both interfaces should consume the existing daemon/API rather than bypassing it.
- Preserve the local-first / private-network architecture already established.
- Do not redesign Milestone 001 unless a concrete incompatibility is discovered.

## Product direction

The GTD application is intended to be:

- keyboard-first;
- fast to navigate;
- local-first;
- usable through multiple front ends;
- compatible with private-LAN usage first;
- compatible with future Android/mobile access over Tailscale;
- compatible with future voice-agent and MCP access.

The daemon/API should remain the common application boundary.

## Existing GTD concepts

### Tasks

Historically:
- status: Todo / Done
- required category
- optional project
- zero or more contexts
- stable identity
- report-specific ordering is important

### Projects

- optional on a task
- hierarchical/tree presentation is desirable
- project-centric views are important

### Contexts

Historical examples:
- Home
- Computer
- Bank
- Car

Tasks may have multiple contexts.

### Categories

Historically:
- Inbox
- Next Actions
- Someday/Maybe
- Waiting For
- Agenda

Treat actual repository/database/API definitions as authoritative.

### Reports

Saved reports/queries are central to the product.

Historically they have filtered combinations of:
- status
- context(s)
- category/categories
- project(s)
- no-project
- no-context

Report-specific sort order has also been important.

## Interaction model / UX preferences

Historical keyboard ideas include:
- `j` / `k` — selection down/up
- `h` / `l` — pane movement
- `J` / `K` — reorder selected task
- `Enter` — activate/open
- `Space` — collapse/expand where applicable
- `<` / `>` — mode/scope navigation
- `zd` — global Done visibility toggle
- fast search
- fuzzy selection
- multi-select context input
- visible focus indicator
- counts beside reports/projects/scopes
- left-side scope picker
- project tree
- built-in discoverable help / keymap reference

Exact bindings may vary by interface, but shared conceptual actions should be named consistently.

## Semantic action principle

Define semantic actions first, then bind keys per interface.

Suggested vocabulary:
- `selection.next`
- `selection.previous`
- `pane.left`
- `pane.right`
- `item.open`
- `item.edit`
- `item.complete`
- `task.new`
- `task.reorder_up`
- `task.reorder_down`
- `view.toggle_done`
- `help.open`
- `search.open`
- `scope.next`
- `scope.previous`

## TUI direction

Current priority is **functional MVP speed**, not language purity.

It is acceptable to salvage/adapt an existing Python TUI if:
- it can cleanly consume the Rust daemon/API;
- it does not directly mutate SQLite;
- its architecture is maintainable;
- reuse materially accelerates MVP delivery.

Historically the Python UI work included:
- SQLite-driven GTD views;
- fuzzy finder / MultiSuggest-style controls;
- keyboard navigation;
- VisiData experiments;
- Textual-style UI ideas.

Do not assume old Python code is usable until inspected.

If present, classify old code as:
1. reusable;
2. adaptable;
3. discard.

Useful candidates:
- widgets
- navigation
- keyboard bindings
- layouts
- fuzzy/multi-select controls
- display formatting

Discard candidates:
- direct SQLite mutation
- obsolete schema assumptions
- duplicated business rules
- dead experiments

## Web direction

Reasonable default:
- Vite
- React
- TypeScript
- minimal state layer unless complexity requires more
- typed API client
- browser keyboard handling
- accessible semantics
- responsive layout

The web UI should be architecturally independent from TUI rendering, but share terminology and semantic actions.

## MVP scope

Prioritize:
- daemon connection/status
- list tasks
- scope/report selection
- project selection
- task detail/open
- add task
- edit task
- Todo/Done
- show/hide Done
- category assignment
- optional project assignment
- multi-context assignment
- search/filter
- built-in help/keymap

If supported by the API:
- ordering/reordering
- project tree
- saved reports

Do not silently bypass missing API functionality. Add the smallest appropriate API change.

## Architecture constraints

- Repository is the source of truth.
- UI must not directly edit SQLite.
- UI must not independently implement CRDT behavior.
- Daemon/API owns persistence, domain behavior, and synchronization.
- Do not expose the daemon publicly by default.
- Preserve compatibility with LAN/Tailscale access.
- Avoid cloud dependencies for basic operation.
- Avoid duplicating domain rules in each frontend.

## Testing expectations

TUI:
- command/action mapping tests where practical
- API adapter tests
- startup smoke test
- critical navigation/flow tests if framework supports them

Web:
- TypeScript typecheck
- lint
- action/key tests
- component tests
- production build
- API client tests/mocks

Integration acceptance:
1. start daemon;
2. create task from TUI;
3. observe task in web UI;
4. modify/complete from web UI;
5. observe update in TUI;
6. restart interfaces without data loss.

## Agent workflow

Use repository files to communicate durable decisions:
- ADRs
- architecture docs
- shared action/keymap spec
- API gaps
- test plans

Do not depend on hidden conversation state.

## Immediate planning priority

Before substantial UI coding:
1. inspect repository;
2. inspect API;
3. locate existing Python UI;
4. inventory reusable code;
5. define semantic actions/keymaps;
6. identify API gaps;
7. choose MVP implementation path;
8. implement thin vertical slices.
