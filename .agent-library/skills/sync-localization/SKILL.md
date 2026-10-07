---
name: sync-localization
description: Keep supported locales and direction behavior aligned. Use when changing localized UI text or layout.
---

# Sync localization

Find the project's real dictionaries and localization API. Identify each changed key, placeholder, plural form, and user-facing string. Update all supported locales with natural copy; retain product names and technical terms according to project conventions.

Check key shape and placeholder parity, then inspect callers for hardcoded fallback text or strings frozen at module load. For RTL layouts, use the project's direction system and verify the affected interaction, not only text alignment.

Run an existing localization check if available and view the changed UI in at least the relevant locales. Report which locale and direction states were actually checked.
