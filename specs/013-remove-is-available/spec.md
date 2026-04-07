# Spec 013: Remove `is_available` from `file_info`

## Status
<!-- Planning | In Progress | Complete | Abandoned -->
Planning

## Affected Crates
- `core_types` — remove `ImportedFile.is_available` field, remove `FileSyncStatus::UploadSkipped`
- `database` — migration dropping `is_available` column, repository cleanup
- `service` — remove all `is_available` field access and cloud-sync guard
- `file_import` — `ImportedFile` constructors

## Problem Statement

`file_info` carries a redundant `is_available` column (and corresponding Rust field) that
duplicates information already encoded in `archive_file_name`:

> **invariant already enforced everywhere:** `is_available = true ↔ archive_file_name = Some(...)`

This invariant is upheld in every repository method (`add_file_info` computes
`is_available = archive_file_name.is_some()`; `update_is_available` always sets both
columns together). The separate flag therefore creates maintenance overhead:

- Every struct initialiser must spell out `is_available: true` or `is_available: false`
  (dozens of test fixtures).
- Introducing the optional `archive_file_name` (spec 011) required the `UploadSkipped`
  `FileSyncStatus` workaround to prevent infinite retry loops when the invariant was
  accidentally violated.
- Cloud sync queries duplicate the filter `is_available = 1 AND archive_file_name IS NOT
  NULL` where a single `archive_file_name IS NOT NULL` would suffice.

Now that `archive_file_name` is properly nullable (spec 011) and the invariant is fully
enforced, the `is_available` column is pure redundancy. Removing it also eliminates the
conditions under which `UploadSkipped` could be produced, making that `FileSyncStatus`
variant dead code that can be removed too.

---

## Proposed Solution

Remove `is_available` as a stored column / struct field everywhere and replace it with a
computed method `is_available() -> bool { archive_file_name.is_some() }` on `FileInfo`,
`FileSetFileInfo`, and `ImportedFile`. Replace the DB filter `is_available = 1` with
`archive_file_name IS NOT NULL` in all queries.

Also remove `FileSyncStatus::UploadSkipped` (never in production use; the invariant
violation it guarded against is now structurally impossible once the DB column is gone).

This is a **pure structural refactoring** — no new behaviour is introduced.

---

## Acceptance Criteria

### Structural

1. The `file_info` table has no `is_available` column.
2. `FileInfo`, `FileSetFileInfo`, and `ImportedFile` have no `is_available` field.
3. All three structs expose a computed `pub fn is_available(&self) -> bool` that returns
   `self.archive_file_name.is_some()`.
4. `FileSyncStatus::UploadSkipped` variant does not exist in `core_types`.

### Queries

5. All queries that previously filtered by `is_available = 1` now filter by
   `archive_file_name IS NOT NULL`.
6. No query references the `is_available` column.

### Quality

7. `cargo test --workspace` passes with no regressions.
8. `cargo clippy --all-targets` produces no new warnings.
9. `.sqlx/` metadata is regenerated and committed.
10. ER diagrams (`docs/schema/`) are updated via `tbls doc --force`.

---

## Scope

### In scope

- DB migration to drop the `is_available` column
- `core_types`: `ImportedFile` field, `FileSyncStatus::UploadSkipped` variant
- `database`: `FileInfo`, `FileSetFileInfo` models; `FileInfoRepository` and
  `FileSetRepository` queries; rename `update_is_available` → `set_archive_file_name`
- `service`: all struct initialisers, field accesses, `update_is_available` call sites,
  cloud-sync `UploadSkipped` guard
- `file_import`: `ImportedFile` constructors

### Out of scope

- Changing how missing files are detected or recorded (already handled in specs 010–012)
- GUI display of availability status
- Any other `FileSyncStatus` variants

---

## Key Decisions

| Decision | Rationale |
|---|---|
| Keep `is_available()` as a **method** (not field) | Zero semantic change; avoids mass rename at call sites; compiler catches all stale field access attempts |
| Rename `update_is_available` → `set_archive_file_name` + add `_with_tx` variant | The method no longer touches an `is_available` column; a `_with_tx` variant is required because `FileSetRepository` calls this inside a transaction |
| Remove `FileSyncStatus::UploadSkipped` entirely | Never in production use; no DB records store value 8; the condition it handled is now structurally impossible |

---

## Migration Design

SQLite does not support `DROP COLUMN` in older versions (this project's SQLite may predate
v3.35). Use the standard recreate-table pattern:

```sql
PRAGMA foreign_keys = OFF;

CREATE TABLE file_info_new (
    id                INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
    sha1_checksum     BLOB    NOT NULL,
    file_size         INTEGER NOT NULL,
    archive_file_name TEXT,
    file_type         INTEGER,
    cloud_sync_status INTEGER NOT NULL DEFAULT 0
);

INSERT INTO file_info_new (id, sha1_checksum, file_size, archive_file_name, file_type, cloud_sync_status)
    SELECT id, sha1_checksum, file_size, archive_file_name, file_type, cloud_sync_status
    FROM file_info;

DROP TABLE file_info;
ALTER TABLE file_info_new RENAME TO file_info;

PRAGMA foreign_keys = ON;
```

## As Implemented
_(Pending)_
