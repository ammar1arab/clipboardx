# Portable Agent Library

Reusable coding-agent guidance drawn from five local projects: `clinic-platform`, `volunteer-platform`, `medical-website`, `ammar-portfolio`, and `nexa_lms_frontend`. It is neutral about language, framework, architecture, package manager, model vendor, and available tools.

## Install now, customize later

From the extracted library folder, run:

```text
node scripts/install.mjs "D:\path\to\project" --dry-run
node scripts/install.mjs "D:\path\to\project"
```

The installer copies this folder to `<project>/.agent-library/` and adds a short pointer to root `AGENTS.md`. It preserves existing instructions and refuses to overwrite a prior installation. **It does not create an active project profile.** The generic rules work immediately; agents inspect the actual project when a task needs project facts.

Later, if repeated work would benefit from durable context, copy `.agent-library/PROJECT_PROFILE.template.md` to `.agent-library/PROJECT_PROFILE.md` and fill it with verified facts. The profile is optional.

Without Node.js, copy the folder to `<project>/.agent-library/` and add this short line to the project's instruction entry (or to a new root `AGENTS.md`):

> Follow `.agent-library/rules/00-core.md`. For specialized work, consult `.agent-library/CATALOG.md` and load only the matching file.

## Keep context lean

- Only `rules/00-core.md` is a default. Read `CATALOG.md` when a task calls for a specialized workflow, then load one matching skill and any directly relevant rule.
- Skill bodies and supporting files are never needed just because they exist. No dependency, model, MCP, or browser tool is enabled by this library.
- Do not install every host adapter or expose every skill automatically. Duplicate always-on instructions and overlapping skill descriptions cost context and can conflict.
- Verify affected behavior with focused checks. Token savings never justify inventing facts or claiming an unrun test passed.

## Optional automatic skill discovery

Every skill is already usable by explicitly telling an agent to read `.agent-library/skills/<name>/SKILL.md`. If a host supports project skills, expose only the few skills you use often:

```text
node .agent-library/scripts/expose.mjs "D:\path\to\project" --host shared --skills bug-triage,change-review
```

`--host shared` creates small discovery pointers in `.agents/skills/`, used by current Cursor, Gemini CLI, GitHub Copilot, and supported Codex setups. `--host claude` uses `.claude/skills/` for Claude Code. The canonical bodies stay in `.agent-library/skills/`. Check your host's discovery UI after exposure. If a host does not recognize these locations, use explicit paths or its documented skill location. Avoid exposing both `shared` and `claude` sets in the same repo when Cursor scans both, since that can duplicate skill metadata.

Optional instruction pointers are in `adapters/`: `CLAUDE.md`, `GEMINI.md`, and `copilot-instructions.md`. Add one only when the host does not already read `AGENTS.md`; merge with an existing file instead of overwriting it. For another vendor or custom agent, point its project instruction entry to `AGENTS.md` or give the library paths explicitly. Universal content does not imply universal automatic discovery.

## Use the library

Read [CATALOG.md](CATALOG.md) to choose a skill. For example:

> Read `.agent-library/skills/reuse-before-create/SKILL.md` and implement this component using the project's existing patterns.

Run `node .agent-library/scripts/validate.mjs` to check library structure and links. Node is needed only for installation and validation; the rules and skills are plain Markdown.

The library does not authorize commits, deployments, production changes, outbound messages, or destructive maintenance. Project-specific instructions and the current user's request control the task.

See [SOURCES.md](SOURCES.md) for what was adopted from the local projects and current agent documentation, and what was deliberately excluded.
