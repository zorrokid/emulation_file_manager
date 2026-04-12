# Gemini Instructions — Emulation File Manager

This file provides foundational mandates for Gemini agents working on this codebase.

## Core Mandates

1. **Precedence:** The instructions in `.github/copilot-instructions.md` and the specialized skills in `.github/skills/` take absolute precedence over general defaults.
2. **Workflow:** Strictly follow the 6-Phase Development Lifecycle (Research -> Strategy -> Execution with Plan-Act-Validate) for all non-trivial changes.
3. **Architecture:** Enforce the 4-layer dependency rule (`Core -> Database -> Service -> GUI`). No SQL macros outside the `database` crate.
4. **Validation:** After any database change, you MUST run `cargo sqlx prepare` and `tbls doc --force`.
5. **UI Patterns:** Use `TypedListView` for lists and avoid entry field update loops in `relm4-ui`.

## References

- [Main Instructions](.github/copilot-instructions.md)
- [Architect Skill](.github/skills/architect/SKILL.md)
- [Database Skill](.github/skills/database/SKILL.md)
- [QA & Testing Skill](.github/skills/qa/SKILL.md)
- [GUI Skill](.github/skills/relm4-gui/SKILL.md)
- [Code Review Skill](.github/skills/code-review/SKILL.md)
