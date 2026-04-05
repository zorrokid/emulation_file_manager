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
tbls doc
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

Use **async-std** throughout — not tokio. Tests use `#[async_std::test]`.

### Database

- Repository pattern: each entity has a struct holding `Arc<Pool<Sqlite>>`
- All repositories are aggregated in `RepositoryManager`
- Test databases use `database::setup_test_db()` + `setup_test_repository_manager()` — in-memory SQLite, migrations run automatically
- Table naming: `snake_case`; junction tables: `table1_table2`; foreign keys: `{table}_id`
- Use `query!` / `query_as!` macros for static queries; `QueryBuilder` for dynamic IN clauses
- Custom type conversions: `FileType` ↔ `u8` via `to_db_int()` / `from_db_int()`; `Sha1Checksum` ↔ `Vec<u8>`

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

**Entry field update loop** — never combine `#[watch]` + `connect_changed` on the same entry. Use `update_with_view` with manual widget updates, or `#[block_signal]`. Prefer manual updates because `set_text` with `#[watch]` causes a cursor jump.

**Always call `self.update_view(widgets, sender)`** at the end of `update_with_view` — omitting it leaves `#[watch]` attributes stale.

**Window reuse** — use `root.hide()`, never `root.close()`. Close requests should send a `Hide` message, not close the window directly.

**Async commands** — use `sender.oneshot_command(async move { … })`. `update_cmd` has no access to `root` or `widgets`; send a message back to self for any UI updates.

**Lists** — use `TypedListView` (not raw `gtk::ListView`). It supports filtering, sorting, and typed access.

**Error display** — use `show_error_dialog` / `show_info_dialog` from `crate::utils::dialog_utils` inside `update_with_view`.

### Testing: Mock Pattern

- Real `RepositoryManager` with in-memory SQLite — never mock the database layer
- Mock service traits (`FileSetServiceOps`, `CloudStorageOps`, etc.) using a single `Arc<Mutex<MockState>>` struct — not one `Arc<Mutex<>>` per field
- Reference examples: `cloud_storage/src/mock.rs`, `service/src/file_set/mock_file_set_service.rs`
- Use `BTreeSet` (not `Vec` or `HashSet`) as HashMap keys for order-independent collections in mock state

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

## Detailed Pattern References

| Doc | Covers |
|---|---|
| `docs/patterns/architect.md` | Domain model, layer placement decisions |
| `docs/patterns/database.md` | Migrations, repository pattern, SQLx offline mode |
| `docs/patterns/gui.md` | relm4 components, async commands, shutdown coordination |
| `docs/patterns/test.md` | Mock structure, test DB setup, coverage expectations |
