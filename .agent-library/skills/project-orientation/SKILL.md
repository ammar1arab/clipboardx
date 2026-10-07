---
name: project-orientation
description: Map an unfamiliar project's structure, commands, and conventions. Use when entering a repo or its facts are unclear.
---

# Project orientation

Read the root entry files, package manifests, relevant directories, and existing tests. Inspect only enough code to trace one representative path for the requested area.

Record verified facts in `PROJECT_PROFILE.md` when the user wants durable project guidance. If no edit was requested, report a compact map instead. Keep uncertain or changing facts labeled as such; do not turn guesses into rules.

Identify the actual package manager, focused checks, runtime tools, design source, localization source, and security boundary where applicable. Distinguish generated files from maintained sources.

Finish with the smallest set of files and checks needed for the current task. Do not load the whole repository into context or rewrite existing project guidance merely to conform to this template.
