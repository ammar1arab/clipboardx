---
name: ship-check
description: Check a change for handoff, PR, or release readiness. Use when asked to verify or ship work.
---

# Ship check

Read the changed-file list and the project's real scripts. Select checks for the affected behavior: type or build, tests, lint, localization, accessibility, API contract, and runtime flow as applicable. Avoid mutating `--fix` commands during a review-only check.

Run focused checks first. Exercise a representative success path and relevant error or permission path when the environment allows. Inspect the final diff for secrets, generated artifacts, unintended files, and unrelated edits.

Report each selected check as pass, fail, or skipped with its reason. Distinguish source checks from runtime observations. State residual risk and whether the change is ready for the requested next step. Commit, push, deploy, or publish only if separately authorized by the task.
