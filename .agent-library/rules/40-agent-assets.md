# Agent asset maintenance

- Keep one canonical copy of each rule and skill. Editor-specific files should point to it or contain only the minimum syntax needed for discovery.
- Keep always-loaded guidance short. Put conditional workflows in narrowly described skills and project facts in `PROJECT_PROFILE.md`.
- Do not copy another project's routes, roles, styling paths, model choices, credentials, tool availability, or one-off permissions into reusable guidance.
- After editing agent assets, check metadata, links, duplicate or conflicting triggers, and the host's actual discovery behavior. A valid file does not prove that every agent loaded or obeyed it.
- Update guidance when repeated, observed failures justify it. Prefer fixing a stale instruction to adding another competing instruction.
