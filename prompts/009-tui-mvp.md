# Prompt 009 — TUI MVP vertical slice

You are the TUI implementation agent.

The repository is the source of truth.

Read:
- `AGENTS.md`
- current ADRs
- `PROJECT-KNOWLEDGE-HANDOFF.md`
- `docs/ui/legacy-python-audit.md`
- `docs/ui/actions.md`
- `docs/ui/mvp.md`
- current API docs
- code selected for reuse by Prompt 008

Follow the Prompt 008 TUI decision unless a concrete blocker is found. If blocked, update the ADR before changing direction.

## Goal

Deliver a functional keyboard-first TUI MVP against the real daemon/API.

Never access SQLite directly.

## Required vertical slice

Implement:
1. daemon connection/status;
2. task list for a useful default scope;
3. keyboard selection;
4. open/view task;
5. create task;
6. edit task;
7. Todo/Done;
8. show/hide Done;
9. category assignment;
10. optional project assignment;
11. multiple contexts;
12. search/filter;
13. built-in keyboard help/keymap.

If the API lacks required functionality:
- add the smallest correct API enhancement;
- keep business logic server-side;
- test it;
- document it.

## Keyboard goals

Use `docs/ui/actions.md`.

Preserve when practical:
- j/k
- h/l
- Enter
- Space for tree expansion
- J/K reorder
- `zd` toggle Done
- fast search
- fuzzy picker behavior
- multi-select contexts
- `?` for help if available

Help should display active keybindings.

## Layout

Prefer a simple MVP layout:

```text
┌ Scopes / Projects ┐ ┌ Task List ───────────────────────┐
│ Reports           │ │                                  │
│ Projects          │ │ selected task                    │
└───────────────────┘ └──────────────────────────────────┘
┌ Status / command / hints ───────────────────────────────┐
└──────────────────────────────────────────────────────────┘
```

Prioritize interaction over polish.

## Reuse policy

If adapting Python:
- salvage useful widgets/UX;
- replace direct DB access with API adapters;
- isolate API access;
- do not preserve obsolete architecture just for compatibility.

## Tests

At minimum verify:
- API adapter/client;
- semantic action/key mapping;
- create/edit/complete;
- Done toggle;
- context multi-selection;
- app startup against local/test daemon or mock.

Run repository verification plus TUI-specific tests.

## Documentation

Update setup/run docs, keybindings, TUI architecture, and known limitations.

## Completion

A developer must be able to start daemon + TUI and manage real tasks without touching SQLite.

Do not push.

Report:
- files changed;
- working features;
- tests;
- API changes;
- UX limitations;
- recommendation for Prompt 010.
