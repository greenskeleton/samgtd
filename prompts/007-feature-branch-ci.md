Complete the GitHub CI handoff for the implementation in this working tree.
Read AGENTS.md, the current diff, prior review findings, and the relevant
acceptance documentation. Preserve collaborative work and fix unresolved
acceptance defects within scope. Do not treat a prior agent's success report
as evidence for the current revision.

Follow AGENTS.md's feature-branch authorization: create or reuse a feature
branch, run the required local checks, stage only reviewed task-related files,
commit, push to origin, and open or update a draft PR. Use the repository's
actual default branch as the PR base. Never push to that base branch.

Watch required GitHub Actions checks on the latest pushed commit, inspect logs
and artifacts, fix relevant failures, and push additional commits until checks
pass. CI workflows may be rerun/dispatched against the feature branch when
appropriate; release/deployment workflows are outside this authorization.
If CI does not trigger, inspect workflow triggers and PR state and fix the
cause. Do not claim success from an older green run or disable failing checks.

Update acceptance evidence where applicable. After any further source/docs
commit, ensure CI validates that latest revision. Put the final tested commit
and CI URLs in the PR/handoff to avoid an endless evidence-only commit cycle.
If credentials, network, or another external dependency blocks progress,
finish unaffected work and report the precise blocker without inventing results.

Leave the PR for human review and manual merge. Do not merge, enable auto-merge,
enqueue a merge, deploy, or alter repository protections/settings/secrets.
Report files changed, decisions, local verification, PR URL, tested SHA,
CI run URLs/results, and remaining risks or blockers.
