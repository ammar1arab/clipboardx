---
name: api-contract
description: Keep API producers and clients aligned. Use when adding or changing an endpoint or shared payload.
---

# API contract

Map one existing operation end to end. List method or operation name, path, authentication, request validation, response shape, nullable fields, and error behavior.

Change the canonical contract and every affected producer and consumer. Preserve compatibility for existing callers unless the task includes a coordinated breaking change. Validate inputs at the boundary and keep authorization server-side. Avoid broad casts or fallback values that hide a mismatch.

Check one success response and one relevant failure response at the serialized boundary when possible. Run focused type and contract checks, then state any client or migration work still required.
