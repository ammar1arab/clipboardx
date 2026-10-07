---
name: security-boundary
description: Check who can read, write, or trigger protected behavior. Use for auth, tenant, permission, or sensitive-action changes.
---

# Security boundary

Trace the protected operation from request to data or side effect. Identify the trusted identity source, server-side permission decision, resource owner or tenant scope, input validation, and audit or logging behavior already used by the project.

Check both an allowed path and a denied path. Include cross-user or cross-tenant attempts when the product has those boundaries. For state changes, consider concurrent requests, replay, and transaction behavior only where relevant.

Do not use real secrets or private records in tests. Do not weaken a server rule to make a UI flow pass. Report the precise boundary checked and any environment-dependent behavior that remains unverified.
