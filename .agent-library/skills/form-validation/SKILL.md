---
name: form-validation
description: Align form input, validation, payload, and feedback. Use when building or fixing a form.
---

# Form validation

Inspect the existing form, schema, default values, API payload, and server validator. Write down how blank text, missing values, `null`, zero, dates, and numeric strings are meant to behave in this form.

Keep client feedback consistent with server rules without relying on client checks for security. Resolve messages in the active locale if the app supports localization. Preserve accessible labels, field associations, and focus behavior after submit.

Test representative blank, invalid, boundary, and valid values. Verify the serialized payload and one server rejection, not just the visible message. Avoid adding state synchronization merely to derive a value that can be computed from the existing form state.
