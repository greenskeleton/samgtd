#!/usr/bin/env python3
"""
Small subscription-friendly local orchestrator.

- Uses the official `claude` and `codex` CLIs.
- Keeps all shared state in the repository.
- Captures stdout/stderr per run.
- Never pushes or merges.
- Intended to be wrapped by scripts/orchestrate.sh, which runs under caffeinate.

This is deliberately not an LLM itself.
"""

from __future__ import annotations
import datetime as dt
import os
from pathlib import Path
import shutil
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[1]
RUNS = ROOT / ".agent" / "runs"

STEPS = [
    ("claude", ROOT / "prompts/001-claude-bootstrap.md"),
    ("codex-review", ROOT / "prompts/002-codex-review.md"),
    ("codex-implement", ROOT / "prompts/003-codex-implement.md"),
    ("claude-review", ROOT / "prompts/004-claude-review.md"),
]

def require(cmd: str) -> None:
    if shutil.which(cmd) is None:
        raise SystemExit(f"required command not found: {cmd}")

def read_prompt(path: Path) -> str:
    return path.read_text(encoding="utf-8")

def command_for(step: str, prompt: str) -> list[str]:
    if step.startswith("claude"):
        # Project .claude/settings.json contains the bounded command allow-list.
        # dontAsk prevents a headless run from hanging on a permission prompt:
        # unmatched actions are denied rather than implicitly approved.
        return [
            "claude",
            "-p",
            "--permission-mode", "dontAsk",
            prompt,
        ]

    if step.startswith("codex"):
        # Workspace writes are allowed; commands outside the sandbox are never
        # auto-escalated. No danger-full-access / yolo mode is used.
        return [
            "codex", "exec",
            "--sandbox", "workspace-write",
            "--ask-for-approval", "never",
            prompt,
        ]

    raise ValueError(step)

def run_step(step: str, prompt_path: Path) -> None:
    stamp = dt.datetime.now().strftime("%Y%m%d-%H%M%S")
    log = RUNS / f"{stamp}-{step}.log"
    prompt = read_prompt(prompt_path)
    cmd = command_for(step, prompt)

    print(f"\n=== {step} ===")
    print(f"prompt: {prompt_path.relative_to(ROOT)}")
    print(f"log:    {log.relative_to(ROOT)}")
    print("command:", " ".join(cmd[:4]), "...")

    with log.open("w", encoding="utf-8") as fh:
        proc = subprocess.Popen(
            cmd,
            cwd=ROOT,
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            text=True,
            bufsize=1,
        )
        assert proc.stdout is not None
        for line in proc.stdout:
            sys.stdout.write(line)
            fh.write(line)
            fh.flush()
        rc = proc.wait()

    if rc != 0:
        raise SystemExit(f"{step} failed with exit status {rc}; see {log}")

def main() -> None:
    require("claude")
    require("codex")
    RUNS.mkdir(parents=True, exist_ok=True)

    if not (ROOT / ".git").exists():
        raise SystemExit("initialize this directory as a git repository first")

    start = 0
    if len(sys.argv) == 3 and sys.argv[1] == "--from":
        names = [name for name, _ in STEPS]
        try:
            start = names.index(sys.argv[2])
        except ValueError:
            raise SystemExit(f"unknown step {sys.argv[2]!r}; choose: {', '.join(names)}")

    for step, prompt_path in STEPS[start:]:
        run_step(step, prompt_path)

    print("\nAll configured agent steps completed.")
    print("Review git diff and .agent/runs/ before committing.")

if __name__ == "__main__":
    main()
