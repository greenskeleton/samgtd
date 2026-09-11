#!/usr/bin/env bash
set -euo pipefail

# Keep the Mac awake for the entire multi-agent sequence, including the small
# gaps while the orchestrator checks results and starts the next process.
exec caffeinate -dimsu python3 scripts/orchestrate.py "$@"
