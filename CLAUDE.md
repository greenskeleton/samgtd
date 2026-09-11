# Claude Code project guidance

Read `AGENTS.md` first and treat it as authoritative shared project guidance.

For architecture work, write durable decisions into `docs/adr/` rather than leaving important rationale only in chat output.

Do not assume Codex's previous implementation is correct. Review repository state and tests independently.

When asked to review, prefer actionable findings with file/line references and do not silently rewrite large areas unless the prompt explicitly asks for fixes.
