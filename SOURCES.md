# Bootstrap research notes

This package intentionally avoids pinning application dependencies before the first agent evaluates current APIs.

Research checked on 2026-09-10:

- Automerge Rust crate: current 0.11.x line; CRDT document with compact binary persistence and a sync protocol designed for reliable in-order transport.
  - https://docs.rs/automerge/latest/automerge/
  - https://docs.rs/automerge/latest/automerge/sync/
- Axum: current 0.8.x line; Tokio/Hyper based and Tower middleware compatible.
  - https://docs.rs/axum/latest/axum/
- Tailscale: `tailscaled`/platform network service supplies the private encrypted network; application should not reimplement its cryptography.
  - https://tailscale.com/docs/reference/tailscaled
- Claude Code:
  - `-p`/`--print` supports noninteractive operation.
  - project permission rules and `dontAsk` can bound unattended execution.
  - https://code.claude.com/docs/en/cli-usage
  - https://code.claude.com/docs/en/permissions
- OpenAI Codex:
  - `codex exec` provides noninteractive execution.
  - workspace-write + never approval provides unattended repository-scoped work without using unrestricted/yolo mode.
  - https://github.com/openai/codex

Agents should verify APIs again when they begin implementation.
