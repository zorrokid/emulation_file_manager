# Copilot Instructions — Emulation File Manager

**Emulation File Manager** is a GTK4 desktop application (Linux) for managing emulation software collections (ROMs, disk images, manuals, cover art), launching them with emulators, and syncing to cloud storage. It is a Rust workspace with ~14 crates.

## Recommended Model

Use **Claude Sonnet** (or better) for all sessions in this project. The codebase involves async Rust, SQLx query macros, complex trait hierarchies, and pipeline patterns — smaller models (e.g. Haiku) make significantly more errors and require more rework. Switch with `/model` if needed.

When using the `task` tool or `/model`, choose based on complexity:
- **Claude Haiku** (fast/cheap): Simple searches, file reads, straightforward grep/view operations
- **Claude Sonnet** (default): Most coding work — refactoring, bug fixes, implementations, tests
- **Claude Opus** (premium): Complex architecture decisions, multi-layer feature planning, intricate debugging

## Commands

```bash
cargo build                          # Build
cargo run --bin efm-relm4-ui         # Run the application
cargo test --verbose                 # Run all tests
cargo test -p <crate>                # Test a specific crate (e.g. -p database)
cargo test <test_name>               # Run a single test by name
cargo fmt                            # Format
cargo clippy --all-targets           # Lint
```

### Critical: SQLx Offline Mode

After **any** change to SQL queries or database schema, regenerate `.sqlx/` metadata — **CI will fail without it**:

```bash
cargo sqlx prepare --workspace -- --all-targets
```

After schema migrations, also update ER diagrams:

```bash
tbls doc --force
```

Commit the migration file, `.sqlx/` metadata, and `docs/schema/` together.

## Architecture

Dependencies flow strictly upward — upper layers depend on lower layers, never the reverse:

```
Core Crates  →  Database  →  Service  →  GUI (relm4-ui)
```

| Crate | Role |
|---|---|
| `core_types` | Domain types: `FileType`, `DocumentType`, `Sha1Checksum`, … |
| `domain` | Naming conventions, title normalization |
| `file_system` | Path resolution (directories-next) |
| `database` | SQLx repositories, migrations, `RepositoryManager` |
| `service` | Orchestration, `AppServices`, sync, import/export pipelines |
| `relm4-ui` | GTK4 UI, relm4 components, `AppModel` |
| `file_import` | File compression/import logic |
| `file_export` | File export logic |
| `cloud_storage` | S3-compatible sync |
| `dat_file_parser` | DAT file parsing |
| `credentials_storage` | Credential management |
| `executable_runner` | Launching external executables |
| `thumbnails` | Thumbnail generation |
| `ui-components` | Reusable relm4 UI components |

**Hard rules:**
- Core crates have **no** dependencies on other project crates
- SQL queries (`sqlx::query!`) live in `database` only — never in GUI
- GUI calls services (`AppServices`), never repositories directly
- Business logic lives in `service`, not in repositories or UI

**Where to place new code:**
- Domain type / value object → `core_types` or `domain`
- SQL query / data access → `database` crate
- Business logic / orchestration → `service` crate
- User interaction / widget → `relm4-ui` or `ui-components`

## Key Conventions

### Async Runtime

The project uses **async-std** as its primary runtime. A migration to Tokio is in progress — when modifying a crate, follow the convention already established in that crate. New crates and new tests default to async-std until explicitly migrated.

### Database

- Repository pattern: one struct per entity holding `Arc<Pool<Sqlite>>`, all aggregated in `RepositoryManager`
- Use `query!` / `query_as!` macros for static queries; `QueryBuilder` for dynamic IN clauses
- Test databases: `database::setup_test_repository_manager().await` — in-memory SQLite, all migrations run automatically
- See the `database` skill for schema conventions, type conversions, and the full migration workflow

### Service Layer: Pipeline Pattern

Complex multi-step operations use a pipeline with `should_execute` / `execute` steps. Pipeline steps may use `.expect()` (not `.unwrap()`) — the `should_execute` guard is tested first and guarantees the value is present. This is the one exception to the no-panic rule.

### Error Handling

- All public APIs return `Result<T, Error>`
- Use `?` for propagation; `unwrap()` only in tests

### Code Conventions

- Edition: Rust 2024; no `unwrap()` in production code (tests OK); prefer `?` over `unwrap`/`expect`
- Pipeline steps in the service layer may use `expect()` when the value was already verified in `should_execute` — this is the one exception to the no-panic rule
- Do **not** use `// ---` or similar divider comments to split code into sections — split into separate files instead
- Public functions, structs, traits, and methods **must** have doc comments (`///`)
- **Single responsibility, no side effects** — each function does one thing and returns its result; never write computed results to shared state as a side effect inside the function; callers are responsible for persisting or sending results
- **DRY** — extract shared logic into a helper at the nearest common ancestor; do not duplicate logic across files; place the abstraction at the lowest level both consumers can reach without increasing visibility unnecessarily

### GUI (relm4)

Data flow: `User action → AppMsg → AppModel → AppServices → RepositoryManager → SQLite`

Critical gotchas (see the `relm4-gui` skill for full patterns):
- **Entry fields** — never combine `#[watch]` + `connect_changed`; use `update_with_view` with manual `widget.set_text()` to avoid cursor jump
- **Window close** — use `root.hide()`, never `root.close()`; close requests should send a `Hide` message
- **`update_with_view`** — always call `self.update_view(widgets, sender)` at the end or `#[watch]` attributes go stale
- **Async** — `update_cmd` has no access to `root` or `widgets`; route UI changes back via `sender.input`
- **Lists** — use `TypedListView`, not raw `gtk::ListView`
- **Errors** — use `show_error_dialog` / `show_info_dialog` from `crate::utils::dialog_utils`

### Testing: Mock Pattern

- Never mock `RepositoryManager` — use `database::setup_test_repository_manager().await` (real in-memory SQLite)
- Mock service traits using a single `Arc<Mutex<MockState>>` struct — not one `Arc<Mutex<>>` per field
- Reference implementations: `cloud_storage/src/mock.rs`, `service/src/file_set/mock_file_set_service.rs`
- See the `qa` skill for full mock structure, coverage expectations, and test naming conventions

### Spec-Driven Development

Any new feature or non-trivial behavior change requires a spec **before** implementation. Refactoring does **not** require a spec — green tests are sufficient.

#### Spec Files

- `specs/<N>-feature.md` — problem statement, proposed solution, and acceptance criteria
- `specs/<N>-feature-tasks.md` — task breakdown, test cases, manual verification checklist

Every planned code change must have a task in the tasks file. If implementation deviates from the plan, sync both spec files before continuing. Tasks are marked `[x]` as they are completed.

#### Development Phases

Each phase requires explicit user confirmation before moving to the next.

**Phase 1 — Specification**
- Gather requirements through conversation; use the `architect` skill for design decisions
- Create `specs/<N>-feature.md` with problem statement, proposed solution, and acceptance criteria
- ✋ User confirms spec

**Phase 2 — Task Breakdown**
- Create `specs/<N>-feature-tasks.md` with all implementation tasks, test cases, and (for GUI changes) a manual verification checklist
- Every planned code change must appear as a task
- ✋ User confirms task list before any code is written

**Phase 3 — Implementation**
- Work through tasks in order; mark each `[x]` as done
- If implementation deviates from the plan, update both spec files before continuing
- ✋ User confirms implementation is complete

**Phase 4 — QA / Test Coverage Review**
- Invoke the `qa` skill to analyse coverage against the spec's acceptance criteria
- QA produces a list of missing or insufficient test cases; add these as tasks to the tasks file
- ✋ User confirms the test plan

**Phase 5 — Test Implementation**
- Implement the tests identified in Phase 4; mark test tasks `[x]` when done
- ✋ User confirms all tests pass and are satisfactory

**Phase 6 — Code Review**
- Invoke the `architect` skill for a code review
- Fix each finding; add fix tasks to the tasks file and mark them done
- Re-request review; repeat until only minor/acceptable findings remain
- ✋ User decides when the review cycle ends

### Code Change Transparency

Before making any edits to existing files, show the user exactly what will change (old → new, or a clear description) and wait for confirmation. After all edits are applied, run:

```bash
git --no-pager diff HEAD <file1> <file2> ...
```

and display the output so the user can verify the final result matches what was agreed. Do not apply edits and then show the diff after the fact — confirmation must come **before** changes are written.

### Temporary Files

Delete temporary files immediately after use — do not leave debug scripts, test files, or scratch `.rs` files in the repository. If an approach is abandoned, clean up before completing the task.

## Build Dependencies (Linux)

```
build-essential pkg-config libglib2.0-dev libgtk-4-dev libcairo2-dev
libpango1.0-dev libgdk-pixbuf-2.0-dev libgraphene-1.0-dev libdbus-1-dev
```

## Validation Checklist

After any code change:
- [ ] `cargo check` succeeds (use `cargo check` over `cargo build` for compilation verification)
- [ ] `cargo test -p <crate>` passes for all affected crates
- [ ] `cargo clippy --all-targets` produces no new warnings
- [ ] Regenerated `.sqlx/` if queries changed
- [ ] Applied migration to live DB + ran `tbls doc --force` if schema changed
- [ ] No layer boundary violations
- [ ] Changes committed in small increments to keep diffs focused and reviewable

## Skills

Invoke the appropriate skill for domain-specific work:

| Skill | Invoke for |
|---|---|
| `architect` | Feature planning, design decisions, architecture review, code review |
| `database` | Migrations, new repositories, SQLx queries, schema changes |
| `relm4-gui` | GTK4 components, dialogs, forms, list views |
| `qa` | Test coverage analysis, writing tests, mock design |
