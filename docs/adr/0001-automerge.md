# ADR 0001: Automerge for replicated state

- Status: Proposed
- Date: 2026-09-10

## Context

The GTD application should be local-first and eventually support multiple local/mobile clients that may edit offline.

## Proposed decision

Use the Rust `automerge` crate as the CRDT engine and use its native sync protocol as the semantic basis for peer synchronization.

Do not require the JavaScript `automerge-repo` sync server protocol in phase 1.

## Consequences

Positive:

- offline mutation;
- deterministic merge behavior;
- Rust-native daemon;
- transport independence;
- future mobile peers can synchronize opportunistically.

Costs:

- we own persistence/document lifecycle and transport framing;
- schema evolution requires deliberate design;
- projections/indexes must be kept consistent with CRDT state.

## Open question for first agent

Decide and document document granularity:
- one dataset document;
- one document per aggregate/entity;
- hybrid index/root plus entity documents.

The bootstrap recommendation is hybrid/root + per-entity docs, but this ADR must not be accepted until tested against the current Automerge Rust APIs.
