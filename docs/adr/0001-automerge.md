# ADR 0001: Automerge for replicated state

- Status: Accepted
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

## Resolved: document granularity

Resolved in `docs/adr/0003-existing-database-coexistence.md`, after
inventorying the existing SQLite database: **hybrid root/index document plus
one document per entity**, accepted as-is against the real data shape (see
that ADR for rationale and known tradeoffs).
