# Catalog

`rules/00-core.md` is the only default. If `PROJECT_PROFILE.md` exists, use it when its facts matter to the task. Load only the matching files below.

## Rules

| Rule | Load for |
| --- | --- |
| [Code and contracts](rules/10-code-contracts.md) | Implementation, API, data shapes, refactors |
| [UI and localization](rules/20-ui-localization.md) | UI, accessibility, responsive layout, copy, i18n |
| [Security and data](rules/30-security-data.md) | Auth, permissions, private data, migrations, external actions |
| [Agent asset maintenance](rules/40-agent-assets.md) | Editing rules, skills, adapters, or project memory |

## Skills

| Skill | Use when |
| --- | --- |
| [project-orientation](skills/project-orientation/SKILL.md) | First work in a project or its structure is unclear |
| [change-plan](skills/change-plan/SKILL.md) | Planning a complex or ambiguous change |
| [feature-slice](skills/feature-slice/SKILL.md) | Implementing an end-to-end feature or cross-layer change |
| [bug-triage](skills/bug-triage/SKILL.md) | Diagnosing a bug before choosing a fix |
| [test-first](skills/test-first/SKILL.md) | TDD or a risky behavior change with a testable failure |
| [reuse-before-create](skills/reuse-before-create/SKILL.md) | Adding a component, hook, helper, or type |
| [module-scaffold](skills/module-scaffold/SKILL.md) | Starting a new module or package |
| [api-contract](skills/api-contract/SKILL.md) | Adding or changing an endpoint or shared payload |
| [form-validation](skills/form-validation/SKILL.md) | Building or fixing forms and boundary validation |
| [sync-localization](skills/sync-localization/SKILL.md) | Changing localized UI copy or RTL behavior |
| [ui-change](skills/ui-change/SKILL.md) | Changing appearance or interaction in a UI |
| [security-boundary](skills/security-boundary/SKILL.md) | Auth, roles, tenant access, or sensitive actions |
| [change-review](skills/change-review/SKILL.md) | Reviewing a diff, PR, staged files, or second pass |
| [ship-check](skills/ship-check/SKILL.md) | Checking whether a change is ready to hand off or ship |
| [change-summary](skills/change-summary/SKILL.md) | Writing an accurate PR, ticket, or handoff summary |
| [agent-assets](skills/agent-assets/SKILL.md) | Creating or maintaining this project's AI guidance |
| [focus-mode](skills/focus-mode/SKILL.md) | User requests focused or ADHD-friendly output |

Do not invoke every skill for every task. Prefer one directly matching skill. Add a second only when it changes a decision or verification step.
