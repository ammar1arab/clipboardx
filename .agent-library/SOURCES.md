# Design notes and sources

Reviewed on 2026-09-26. This file is for maintainers; agents do not need to load it during ordinary coding tasks. The skills are original, short syntheses rather than copies of third-party skill bodies.

## Local projects

| Source | Reusable idea | Kept local instead |
| --- | --- | --- |
| `volunteer-platform` | Truth-first bug triage, staged/diff review, focused verification, canonical rule source | Basmat routes, chat providers, Arabic brand copy, IronBee-only browser rule |
| `clinic-platform` | Cross-layer contracts, tenant/security boundary review, localization, form validation, asset validation | Cureva roles, Prisma paths, OTP limits, theme token names |
| `medical-website` | Component and form conventions, translation sync, pre-ship checks | Copied Portfolio paths and SCSS instructions that do not fit its current Tailwind structure |
| `ammar-portfolio` | Scoped Cursor rules, project skills, concise prompt/context discipline | Starfield identity, section numbering, model routing, 91 KB prose handbook |
| `nexa_lms_frontend` | Search/reuse decision before new components or hooks, additive accessibility pass, module registration checks, branch summary | Tenant paths, `lmsui` imports, Vite/React bans, unconditional plan approval, duplicated Portfolio skills; ADHD output style is opt-in |

## Current public guidance

- [OpenAI, Rethinking skills and prompts for GPT-6 Astra](https://developers.openai.com/blog/rethinking-skills-and-prompts-for-gpt-6-astra): short descriptions, narrow triggers, small always-on files, progressive disclosure, and avoiding obsolete over-specific rules.
- [Anthropic, Agent Skills overview](https://platform.claude.com/docs/en/agents-and-tools/agent-skills/overview): metadata is discoverable first; full instructions and references load only when needed.
- [Agent Skills specification](https://agentskills.io/specification): portable `SKILL.md` structure and metadata.
- [Cursor Skills](https://prod.cursor.com/help/customization/skills), [Gemini CLI Skills](https://geminicli.com/docs/cli/tutorials/skills-getting-started/), [GitHub Copilot Skills](https://docs.github.com/en/copilot/how-tos/copilot-on-github/customize-copilot/customize-cloud-agent/add-skills), and [Claude Code Skills](https://code.claude.com/docs/en/skills): host discovery differs. The installer exposes selected skills only in a documented host path; explicit file references remain the fallback.
- [obra/superpowers](https://github.com/obra/superpowers): community workflow examples for planning, test-driven development, systematic debugging, and review. This library uses those themes selectively, without adopting its mandatory full-process or subagent setup.
- [Vercel Labs agent-skills](https://github.com/vercel-labs/agent-skills): example of specialized skills. React and web-specific advice belongs in a project extension, not the universal core.

## Constraints

No instruction can guarantee accuracy or a fixed token count across OpenAI, Anthropic, Google, Meta, xAI, or a custom agent. This library keeps default context small, avoids duplicated instructions, exposes skills selectively, and still requires evidence for consequential claims. Auto-discovery must be checked in each actual host and version.
