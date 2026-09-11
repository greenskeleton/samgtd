#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 2 ]]; then
  echo "usage: $0 {claude|codex} prompt-file" >&2
  exit 2
fi

agent="$1"
prompt_file="$2"

if [[ ! -f "$prompt_file" ]]; then
  echo "prompt not found: $prompt_file" >&2
  exit 2
fi

mkdir -p .agent/runs
stamp="$(date +%Y%m%d-%H%M%S)"
log=".agent/runs/${stamp}-${agent}.log"

case "$agent" in
  claude)
    command -v claude >/dev/null || { echo "claude not found" >&2; exit 127; }
    # Interactive first-run path: permissions remain visible to the operator.
    echo "Starting Claude Code under caffeinate; log: $log"
    # Replace this shell so edits to the launcher during a session cannot
    # cause Bash to resume reading it at a stale file offset on exit.
    exec script -q "$log" caffeinate -dimsu claude "$(cat "$prompt_file")"
    ;;
  codex)
    command -v codex >/dev/null || { echo "codex not found" >&2; exit 127; }
    echo "Starting Codex under caffeinate; log: $log"
    exec script -q "$log" caffeinate -dimsu codex --sandbox workspace-write \
      -c sandbox_workspace_write.network_access=true "$(cat "$prompt_file")"
    ;;
  *)
    echo "unknown agent: $agent" >&2
    exit 2
    ;;
esac
