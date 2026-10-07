---
name: test-first
description: Use a failing regression or acceptance test to guide a behavior change. Use when the user asks for TDD or a risky defect has a testable failure.
---

# Test first

Describe the behavior from a user's or caller's perspective, then choose the smallest stable test boundary. Reproduce the defect or missing behavior with a test that fails for the expected reason. Do not write a test that merely repeats the proposed implementation.

Implement the minimal change, run the new test and relevant neighboring checks, and remove accidental scaffolding. If a test cannot represent the behavior well, use another observable check and explain why.

Report the red-to-green evidence. Do not claim TDD if the test was written after the fix or passed before the change.
