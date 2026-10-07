---
name: change-review
description: Find actionable defects in a diff, branch, PR, or staged change. Use for code review or a second pass.
---

# Change review

Establish the exact review range first. For staged work, read the staged diff only unless the user broadens scope. Distinguish pre-existing problems and unrelated working changes from the reviewed change.

Trace modified behavior through callers, contracts, permissions, and error paths. Prioritize concrete correctness, security, data loss, regression, accessibility, and localization defects. A finding needs a triggering scenario, impact, and narrow file/line evidence. Do not inflate style preferences into defects.

Run a focused check when it resolves a real uncertainty. Return findings in severity order, then state what was checked and any material limit. If none are actionable, say so plainly.
