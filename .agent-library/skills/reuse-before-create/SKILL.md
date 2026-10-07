---
name: reuse-before-create
description: Find an existing component, hook, helper, or type before creating another. Use when adding a reusable code unit.
---

# Reuse before create

Search the relevant local, shared, and platform layers for the needed behavior, including likely synonyms. Read candidate APIs and current call sites; names alone do not establish a fit.

Choose the smallest sound action: reuse, compatibly extend, promote to a shared layer, or create a new unit. Reuse when the current API covers the need. Extend when a backward-compatible option fits the same responsibility. Promote only when code is genuinely shared and can leave feature-specific services, permissions, and state at its callers. Create new when sharing would distort a boundary or break existing behavior.

Keep the decision brief. Implement within the requested scope, then check old and new call sites and the affected build or test. Do not create a wrapper or abstraction solely to satisfy this skill.
