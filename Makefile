.PHONY: verify agent-claude agent-codex-review agent-codex-implement agent-claude-review orchestrate

verify:
	cargo fmt --all -- --check
	cargo clippy --workspace --all-targets --all-features -- -D warnings
	cargo test --workspace --all-features

agent-claude:
	./scripts/agent.sh claude prompts/001-claude-bootstrap.md

agent-codex-review:
	./scripts/agent.sh codex prompts/002-codex-review.md

agent-codex-implement:
	./scripts/agent.sh codex prompts/003-codex-implement.md

agent-claude-review:
	./scripts/agent.sh claude prompts/004-claude-review.md

orchestrate:
	./scripts/orchestrate.sh
