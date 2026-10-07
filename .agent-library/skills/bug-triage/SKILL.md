---
name: bug-triage
description: Diagnose a bug from observed behavior to verified cause. Use before fixing an unexplained failure.
---

# Bug triage

Reproduce the symptom or establish the best available evidence. Write down what is observed, what is assumed, and the smallest next check that can separate likely causes.

Trace the failing path across the relevant layers and compare it with a working sibling or expected contract. Use logs, tests, runtime inspection, and versioned source only as needed. Do not treat a nearby error or a plausible story as proof of cause.

Once the cause is supported, make the smallest fix and add a regression check when it would catch this failure again. Re-run the original scenario. If reproduction is unavailable, explain the evidence limit and avoid claiming the bug is fixed solely from reading code.
