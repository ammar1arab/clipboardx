# Code and contracts

- Trace the affected flow through callers, validation, business logic, persistence, and UI before changing a shared shape. Check existing sibling patterns first.
- Keep boundaries clear: UI should use the project's established data access layer; server authorization and validation belong on the server; persistence details should not leak into presentation code without a project reason.
- When a request or response changes, update all producers, consumers, types, schemas, error behavior, and tests that depend on it. Keep wire values stable unless the change explicitly requires a migration.
- Use the project's language, framework, and typing conventions. Avoid blanket technology bans. Choose an effect, abstraction, comment, or dependency when it solves a real problem and fits the codebase.
- Prefer small, reviewable diffs. Refactor adjacent code only when required for the requested behavior or to remove a directly blocking defect.
