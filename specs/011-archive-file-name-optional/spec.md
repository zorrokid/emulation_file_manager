# Spec 011: Make `archive_file_name` Optional

## Status
< Planning | In Progress | Complete | Abandoned -->
Complete

## Affected Crates
- `core_types` — `ImportedFile.archive_file_name: Option<String>`, `FileSyncStatus::UploadSkipped` variant
- `database` — migration making `archive_file_name` nullable, `is_available` default fix
- `service` — all pipelines guarding `archive_file_name.is_some()` before use
- `file_import` — `ImportedFile` constructors

## Problem Statement

`archive_file_name` is stored as `TEXT NOT NULL` in the database and `String` in all
Rust structs, but it has no meaningful value when `is_available = false`. The codebase
currently uses an empty string (`""`) as a sentinel for missing files, which:

- Hides the absence of a real value behind an opaque convention
- Required a manual workaround (the bug fixed in this branch where `""` was
  accidentally preserved on re-import instead of being replaced with the real name)
- Forces every callsite that constructs a path to double-check both `is_available` and
  `!archive_file_name.is_empty()`

There is an existing `// TODO: make optional?` comment on `FileInfo.archive_file_name`
in `database/src/models.rs` that confirms this is a known issue.

Making `archive_file_name: Option<String>` (NULL in the database) is a pure mechanical
refactoring — it introduces no new behaviour, only cleaner semantics.

---

## Acceptance Criteria

### Semantic

1. `archive_file_name` is `None` for all `file_info` records where `is_available = false`.
2. `archive_file_name` is `Some(name)` for all `file_info` records where `is_available = true`.
3. No callsite constructs a file path from `archive_file_name` without first unwrapping it.
4. `generate_cloud_key()` returns `None` when `archive_file_name` is `None`.
5. The `""` empty-string sentinel no longer appears anywhere in production code.

### Data Integrity

6. The migration converts all existing `archive_file_name = ''` records to NULL.
7. All existing records where `archive_file_name` is non-empty are preserved unchanged.
8. No foreign key constraints or indexes on `file_info` are dropped by the migration.

### Quality

9. All existing tests pass after the refactoring.
10. `cargo clippy --all-targets` reports no warnings introduced by this change.
11. `.sqlx/` metadata is regenerated and committed.
12. ER diagrams (`docs/schema/`) are updated via `tbls doc`.

---

## Scope

This is a **pure refactoring**. No new pipeline steps, no new database columns, no
new behaviour. All changes are mechanical type propagation.

### In scope

- DB schema migration
- All Rust struct definitions that contain `archive_file_name`
- Repository method signatures and SQL queries
- All callsites in the service layer
- Test data construction sites

### Out of scope

- Renaming `is_available` (separate concern)
- Changing how missing files are detected or recorded (already handled in spec 010)
- GUI display of missing files

---

## Affected Files

### Database

| File | Change |
|---|---|
| `database/migrations/<new>.sql` | Recreate `file_info` with nullable column; migrate `''` → NULL |
| `database/src/models.rs` | `FileInfo`, `FileSetFileInfo`, `FileSyncLogWithFileInfo` field types |
| `database/src/repository/file_info_repository.rs` | `add_file_info`, `update_is_available` signatures; `FromRow` impl |

### Core types

| File | Change |
|---|---|
| `core_types/src/lib.rs` | `ImportedFile.archive_file_name: Option<String>` |

### Service

| File | Change |
|---|---|
| `service/src/view_models.rs` | `FileInfoViewModel.archive_file_name: Option<String>` |
| `service/src/cloud_sync/steps.rs` | `generate_cloud_key` + `get_file_path` callsites |
| `service/src/file_import/add_file_set/context.rs` | `""` sentinel → `None` |
| `service/src/file_import/add_file_set/steps.rs` | `get_file_path` callsite |
| `service/src/file_import/common_steps/file_deletion_steps.rs` | `get_file_path` callsites (~4) |
| `service/src/file_import/service.rs` | `generate_cloud_key` callsites |
| `service/src/file_import/update_file_set/steps.rs` | `generate_cloud_key` + construction sites |
| `service/src/file_set_deletion/service.rs` | `get_file_path` callsite |
| `service/src/file_set_download/steps.rs` | `generate_cloud_key` + `get_file_path` callsites |
| `service/src/file_type_migration/steps.rs` | `generate_cloud_key` + `get_file_path` callsites |

### file_import crate

| File | Change |
|---|---|
| `file_import/src/lib.rs` | `assert!(!is_empty())` → `assert!(is_some())` |

---

## Implementation Order

1. **T1** — DB migration (schema)
2. **T2** — Struct field changes + `generate_cloud_key` return type
3. **T3** — Repository method signatures
4. **T4** — Service callsites: `generate_cloud_key` (~12)
5. **T5** — Service callsites: `get_file_path` (~12)
6. **T6** — Sentinel replacement (`""` → `None`, `is_empty` → `is_none`)
7. **T7** — Test construction sites (~61 string literals → `Some(...)`)
8. **T8** — `cargo sqlx prepare --workspace` + `tbls doc`
9. **T9** — `cargo test -p database -p service` + `cargo clippy --all-targets`

Tasks T4–T7 can be done in parallel once T3 is complete.

---

## Findings from Code Review

Two bugs were identified during code review of the implementation branch.

### Finding 1 — `UploadPendingFilesStep`: silent `continue` after writing `UploadInProgress` to DB

**File:** `service/src/cloud_sync/steps.rs`

**Problem:** `UploadPendingFilesStep` writes a `FileSyncStatus::UploadInProgress` log entry
before checking whether `archive_file_name` is `Some`. If the check fails, the code does a
silent `continue` — no warning is logged, no failure status is written, and the file is
permanently stuck with `UploadInProgress` in the sync log with no recovery path.

In practice `PrepareFilesForUploadStep` prevents files without an archive name from entering
`files_pending_upload`, so this is a defensive case — but it should still warn and write a
terminal status entry rather than leaving `UploadInProgress` as the permanent state.

**Initial fix attempt:** Writing `FileSyncStatus::UploadFailed` was tried, but this causes an
**infinite loop**: `UploadFailed` is one of the statuses queried on every loop iteration
(`[UploadPending, UploadFailed]`) with a fixed offset of 0. The file is immediately re-fetched,
processed again, written `UploadFailed` again, and so on forever.

**Correct fix:** Add a new `FileSyncStatus::UploadSkipped` variant (DB int value `8`). This
status is not in the retry set, is distinct from `UploadCompleted` (the file was not actually
uploaded), and self-documents the invariant violation in the DB. The fix is two parts:

1. **`core_types`** — add `UploadSkipped = 8` to `FileSyncStatus` enum with `to_db_int` /
   `from_db_int` mappings.

2. **`UploadPendingFilesStep`** — when `archive_file_name` is `None`, log `tracing::warn!` and
   write `FileSyncStatus::UploadSkipped` (with message `"missing archive_file_name"`) before
   the `continue`.

### Finding 2 — `UpdateFileInfoToDatabaseStep`: `update_is_available(id, None)` creates `is_available=1, archive_file_name=NULL`

**File:** `service/src/file_import/update_file_set/steps.rs`

**Problem:** When a re-import encounters a SHA1 that already exists in the DB as unavailable
(`is_available = false`), the step calls `update_is_available(existing_id, archive_file_name)`
unconditionally. The SQL always sets `is_available = 1`. If the incoming `ImportedFile` is
**still unavailable** (e.g., the user re-imports with a DAT file and still doesn't have the
file), this writes `is_available = 1, archive_file_name = NULL` — directly violating the
invariant that `is_available = true ↔ archive_file_name = Some(...)`.

After this write, the record appears available to all `is_available` checks but has no archive
name, causing silent skips in every downstream operation (cloud sync, download, migration).

The `FileInfo` pushed to `state.new_files` also hardcodes `is_available: true`, compounding
the issue.

**Fix:** Guard the `update_is_available` call with `if !imported_file.is_available { continue; }`.
If the file is still missing, skip the entire block — the existing DB record is already correct.

### Finding 3 — `is_available` column DEFAULT 1 enables silent invariant violation in `add_file_info`

**Files:** `database/migrations/`, `database/src/repository/file_info_repository.rs`

**Problem:** The `is_available` column has `DEFAULT 1` — set originally because all files were
available at migration time. `add_file_info` does not include `is_available` in its INSERT, so
it silently relies on this default. Now that `archive_file_name` can be `NULL`, calling
`add_file_info(sha1, size, None, file_type)` creates a record with
`is_available = 1, archive_file_name = NULL` — directly violating the core invariant:

> `is_available = true` ↔ `archive_file_name = Some(...)`

**Root cause:** The column default was never updated when spec 010 introduced the concept of
unavailable files. It should be `DEFAULT 0` — a new record is unavailable until confirmed.

**Fix (two parts):**

1. **Migration** — recreate `file_info` with `is_available DEFAULT 0` (SQLite requires
   recreate-table to change a column DEFAULT).

2. **`add_file_info`** — explicitly set `is_available = archive_file_name IS NOT NULL` in the
   INSERT rather than relying on the column default. This enforces the invariant at the DB call
   level and makes the intent explicit to readers.

---

## Migration Design Note

SQLite does not support `ALTER COLUMN` to remove a `NOT NULL` constraint. The standard
pattern is:

```sql
PRAGMA foreign_keys = OFF;

CREATE TABLE file_info_new (
    id                INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
    sha1_checksum     BLOB    NOT NULL,
    file_size         INTEGER NOT NULL,
    archive_file_name TEXT,               -- nullable
    file_type         INTEGER NOT NULL,
    is_available      INTEGER NOT NULL DEFAULT 1
);

INSERT INTO file_info_new
    SELECT id, sha1_checksum, file_size,
           NULLIF(archive_file_name, ''),  -- convert '' to NULL
           file_type, is_available
    FROM file_info;

DROP TABLE file_info;
ALTER TABLE file_info_new RENAME TO file_info;

PRAGMA foreign_keys = ON;
```

Any indexes on `file_info` must be recreated after the rename. Check the existing
migrations for current index definitions.

## As Implemented
Implementation completed as specified. All `archive_file_name` fields changed to `Option<String>`. Two bugs found during code review and fixed (T12–T15, T25): the `UploadFailed` infinite-retry problem required the new `FileSyncStatus::UploadSkipped` variant, and `update_is_available` needed a guard to prevent writing `is_available=1, archive_file_name=NULL`. Second review round (T26–T30) found minor issues, all resolved.
