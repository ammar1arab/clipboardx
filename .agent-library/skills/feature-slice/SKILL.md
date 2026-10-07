---
name: feature-slice
description: Implement a feature across existing project layers. Use for a requested change spanning multiple files or boundaries.
---

# Feature slice

Trace a comparable feature from entry point to data source. Identify the user-visible acceptance criteria and the existing contracts. Mark unresolved business decisions before coding, while continuing work that does not depend on them.

Implement the smallest complete path through the layers this project actually uses. Reuse its components, services, schemas, repositories, and route conventions. Add or change shared contracts before relying on invented fields. Include loading, empty, error, and permission behavior when the feature requires them.

Run focused static checks and meaningful tests, then exercise the affected flow in the available runtime. Inspect the final diff for unrelated changes. Report behavior delivered, evidence, and remaining limitations.
