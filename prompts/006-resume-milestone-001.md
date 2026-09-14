Continue the Milestone 001 work from prompt 005 and your existing session.
Read AGENTS.md, the current working diff, prompts/005-finish-milestone-001.md,
and docs/milestone-001-acceptance.md. Preserve completed work; do not restart
the implementation. Verify the current report rather than assuming its claims.

The operator has enabled gh command access for Claude and outbound networking
in the repository's Codex launchers. Retry previously blocked GitHub operations;
do not assume a previous environment restriction still applies. Do not print
tokens or other credentials. Check gh auth status and read repository CI state.
If authentication fails, distinguish network/keychain access from an invalid
credential and report the specific operator action needed.

Finish the remaining authorized local work from prompt 005, especially resolving
and verifying third-party Action commit SHAs, checking toolchain availability,
and correcting CI configuration based on actual GitHub evidence. Inspect runs,
logs, and artifacts, and run/rerun feature-branch CI as needed. Do not treat an older green run as evidence for
this uncommitted changeset. Run the required checks and demo after relevant
changes, and update the acceptance report with accurate evidence and remaining
gaps. Never fabricate a hosted CI result.

The operator now authorizes feature-branch commits, pushes, draft PR creation,
and CI runs/fixes under AGENTS.md. Continue through the GitHub CI handoff in
prompts/007-feature-branch-ci.md; do not stop at local verification. Preserve
collaborative changes and publish only reviewed task-related files. Human
intervention is required to merge: do not merge, enable auto-merge, enqueue a
merge, deploy, alter secrets, or change repository protections/settings.

Summarize changes, verification, evidence paths, and the precise remaining
milestone acceptance items.
