# Prompt 010 — Web MVP vertical slice

You are the web UI implementation agent.

The repository is the source of truth.

Read:
- `AGENTS.md`
- current ADRs
- `PROJECT-KNOWLEDGE-HANDOFF.md`
- `docs/ui/actions.md`
- `docs/ui/mvp.md`
- current API docs
- TUI implementation from Prompt 009

The web UI is an independent frontend, not a port of TUI rendering.

Use the same semantic action vocabulary where practical.

## Goal

Deliver a functional keyboard-driven web MVP against the real daemon/API.

Default stack unless repository decisions say otherwise:
- Vite
- React
- TypeScript

Never access SQLite directly.

## Required MVP

Implement:
1. daemon connection/status;
2. left-side scope/project navigation;
3. task list;
4. selected task detail/editing;
5. create;
6. edit;
7. Todo/Done;
8. show/hide Done;
9. category;
10. optional project;
11. multiple contexts;
12. search/filter;
13. keyboard navigation;
14. built-in help/keymap;
15. responsive laptop/tablet/mobile-browser layout.

## Keyboard/accessibility

Use `docs/ui/actions.md`.

Requirements:
- don't hijack typing in text inputs;
- visible focus state;
- preserve Tab semantics where practical;
- shortcuts discoverable;
- pointer/mouse still usable.

Preserve j/k, h/l, Enter, `zd`, and `?` when practical.

## API client

Centralize API access.

Prefer:
- typed client module;
- clear request/response types;
- minimal duplicate domain logic.

Evaluate OpenAPI-generated types only if current API/docs make it worthwhile.

## State management

Start simple. Prefer React state/hooks plus a small data-fetching layer unless real complexity justifies more.

Do not build a frontend shadow database.

## Tests

At minimum:
- typecheck;
- lint;
- semantic action/key unit tests;
- major component-flow tests;
- API client mocks/tests;
- production build.

If practical add browser smoke coverage for:
- load list;
- create;
- complete;
- toggle Done;
- open help.

## Documentation

Add local development/build instructions, connectivity assumptions, shortcuts, and limitations.

## Completion

A developer should be able to start daemon + web dev server and manage actual GTD tasks without touching SQLite.

Do not push.

Summarize:
- features;
- tests/build;
- API gaps/changes;
- differences from TUI;
- remaining MVP issues.
