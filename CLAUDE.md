# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

**Emulation File Manager** — a GTK4 desktop application (Linux) for managing emulation software collections (ROMs, disk images, manuals, cover art, etc.), launching them with emulators, and syncing to cloud storage.

- **Language:** Rust (Edition 2024)
- **UI Framework:** relm4 0.9.1 (GTK4 bindings)
- **Database:** SQLx 0.8.6 + SQLite
- **Async Runtime:** async-std (not tokio — use `#[async_std::test]` in tests)
- **Cloud Storage:** rust-s3 (S3-compatible)

## Common Commands

```bash
cargo build                    # Build
cargo build --release          # Optimized build
cargo run --bin efm-relm4-ui   # Run the application
cargo test --verbose           # Run all tests
cargo test -p <crate>          # Test a specific crate (e.g. -p file_export)
cargo test <test_name>         # Run a specific test by name
cargo fmt                      # Format code
cargo clippy --all-targets     # Lint
```

### SQLx Offline Mode (Critical)

After **any** change to SQL queries or database schema, regenerate the `.sqlx/` metadata or CI will fail:

```bash
cargo sqlx prepare --workspace -- --all-targets
```

After schema migrations, also regenerate database documentation:

```bash
tbls doc
```

## 4-Layer Architecture (Non-Negotiable)

Dependencies flow strictly upward — upper layers depend on lower layers, never the reverse:

```
Core Crates   →   Database   →   Service   →   GUI (relm4-ui)
```

**Quick placement guide:**
- Domain type / value object → `core_types` or `domain`
- Data access / SQL query → `database` crate only
- Business logic / orchestration → `service` crate
- User interaction / component → `relm4-ui` crate

**Hard rules:**
- Core crates have **no** dependencies on other project crates
- SQLx queries **never** appear in GUI code
- GUI **never** calls database repositories directly (always goes through service)
- Business logic **never** lives in database repositories

## Crate Responsibilities

| Crate | Role |
|---|---|
| `core_types` | Domain types (FileType, DocumentType, Sha1Checksum, …) |
| `domain` | Naming conventions, title normalization |
| `file_system` | Path resolution (directories-next) |
| `database` | SQLx repositories, migrations, `RepositoryManager` |
| `service` | Orchestration, `AppServices`, sync, import/export pipelines |
| `relm4-ui` | GTK4 UI, relm4 Components, `AppModel` |
| `file_import` | File compression/import logic |
| `file_export` | File export logic |
| `cloud_storage` | S3-compatible sync |
| `dat_file_parser` | DAT file parsing |
| `credentials_storage` | Credential management |
| `executable_runner` | Launching external executables |
| `thumbnails` | Thumbnail generation |
| `ui-components` | Reusable relm4 UI components |

## Key Patterns

### Pipeline Pattern

Complex multi-step operations in the service layer use a pipeline pattern with `should_execute` / `execute` steps. Pipeline steps may use `.expect()` (not `.unwrap()`) because the `should_execute` guard is tested first — this is the one exception to the no-panic rule.

### Error Handling

- All public APIs return `Result<T, Error>`
- Use `?` operator for propagation
- `unwrap()` is only acceptable in tests

## Pattern References

Detailed implementation patterns live in `docs/patterns/`:

| Doc | Covers |
|---|---|
| `docs/patterns/architect.md` | Domain model, layer boundaries, crate placement decisions |
| `docs/patterns/database.md` | Migrations, repositories, SQLx queries, offline mode |
| `docs/patterns/gui.md` | relm4 components, `AppServices`, async commands, GTK4 patterns |
| `docs/patterns/test.md` | Mock pattern, test DB setup, coverage expectations |

## Development Practices

### Temporary Files

**Always clean up temporary files immediately after use.** Do not leave debug scripts, test files, or temporary artifacts in the repository:

- After debugging or testing a hypothesis, delete the temporary file
- Examples: test scripts, debug binaries, temporary .rs files, exploration attempts
- If a temporary file is created and the approach is abandoned, clean it up before completing the task

## Spec-Driven Development

Any change that introduces or modifies behavior requires a spec. **Do not start implementing without a spec in place.**

### When a spec is required

- New features
- Bug fixes with non-trivial logic
- Changes to existing behavior

Refactoring does **not** require a spec — the existing tests define correct behavior. If they stay green, the refactoring is safe.

### Spec files

Specs live in `specs/` and come in pairs:

- `specs/<N>-feature.md` — behavior description, requirements, acceptance criteria
- `specs/<N>-feature-tasks.md` — task breakdown with explicit test cases and manual verification checklist

### Before implementing

1. Confirm `specs/<N>-feature.md` exists and is complete
2. Confirm `specs/<N>-feature-tasks.md` exists with test cases listed
3. If either is missing, create them first

### Implementation order

1. Write failing test stubs derived from the task list (use `todo!()`)
2. Implement until tests pass
3. Complete manual verification checklist for any GUI changes

### After implementing

Verify the implementation is complete by checking all three:

1. **Code review** — read through the implementation and confirm it matches the spec requirements
2. **Automated tests** — `cargo test` must pass, covering all test cases listed in the task file
3. **Manual checklist** — all GUI verification items in the task file must be ticked off

### Test vs manual verification split

- Backend behavior (repositories, pipelines, domain logic) → automated tests in `<crate>/tests/` or inline `#[cfg(test)]`
- GUI behavior (dialogs, layout, widget state) → manual verification checklist in the task file

## Build Dependencies (for local setup)

```
build-essential pkg-config libglib2.0-dev libgtk-4-dev libcairo2-dev
libpango1.0-dev libgdk-pixbuf-2.0-dev libgraphene-1.0-dev libdbus-1-dev
```
