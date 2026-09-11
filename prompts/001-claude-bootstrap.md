You are the architecture/scaffold agent for a brand-new public Rust project.

Read, in order:
1. AGENTS.md
2. README.md
3. docs/architecture.md
4. docs/adr/0001-automerge.md
5. docs/adr/0002-private-network.md
6. docs/milestone-001.md

Goal: produce the first coherent Rust scaffold for Milestone 001.


## REQUIRED DATABASE HANDOFF GATE

This project already has a working SQLite GTD database containing real data.

Before deciding document granularity, SQL schema, identity strategy, migrations, or implementing persistent GTD entities, check for:

`.local/current-gtd.sqlite`

If it does not exist:

1. You MAY inspect the repository, verify the Rust toolchain/dependencies, and create only schema-neutral scaffolding.
2. Then STOP and explicitly ask the operator to copy/provide the current SQLite database at `.local/current-gtd.sqlite`.
3. Give the operator this exact example:

   `mkdir -p .local && cp /path/to/current/database.sqlite .local/current-gtd.sqlite`

4. Wait for the operator to confirm it is present.
5. Do NOT design a replacement GTD schema while waiting.

Once present:

- treat the file as READ ONLY;
- make a disposable copy for any operation that could write;
- inventory schema, indexes, FKs, views, triggers, pragmas, IDs, and relationships;
- inspect representative data only as necessary to infer invariants;
- do not echo private task text or other real row contents into committed docs/logs;
- write `docs/existing-database.md` describing STRUCTURE, not private content;
- write/adjust an ADR describing how CRDT state coexists with and preserves the existing database structure;
- identify migration requirements before changing schema;
- never commit `.local/current-gtd.sqlite`.

This gate takes precedence over the implementation tasks below.


Do not build the entire product. Create a high-quality foundation and a thin vertical slice.

Tasks:

1. Inspect current Rust/Automerge/Axum APIs available to the toolchain rather than relying on stale memory.
2. Decide Cargo workspace/crate boundaries. Prefer the target layout in README unless there is a concrete reason to change it.
3. Evaluate the proposed hybrid root-index + per-entity Automerge document model. Write an ADR accepting or replacing it.
4. Define the minimal Task CRDT schema needed for the convergence test.
5. Define persistence boundaries using SQLite. Automerge remains authoritative for replicated fields.
6. Define the sync WebSocket message framing around Automerge's Rust sync protocol.
7. Scaffold the workspace and implement only enough to:
   - start the daemon;
   - expose `/health`;
   - initialize/load local persistence;
   - create/load a minimal task document;
   - exercise CRDT merge/sync in unit/integration tests.
8. Add/repair GitHub Actions CI.
9. Update docs/architecture.md and docs/protocol.md.
10. Run fmt, clippy, and tests.

Security constraints:
- bind to 127.0.0.1 by default;
- do not write a custom cryptographic protocol;
- do not expose secrets;
- do not push to GitHub.

Important:
- Keep domain, CRDT, storage, and HTTP boundaries explicit.
- Tests proving convergence matter more than endpoint count.
- If current Automerge APIs make the proposed design awkward, document the tradeoff rather than hiding it.
- Avoid speculative abstractions not needed for Milestone 001.

At the end, print:
- architecture decisions;
- workspace layout;
- commands/tests run;
- known risks;
- what Codex should review next.
