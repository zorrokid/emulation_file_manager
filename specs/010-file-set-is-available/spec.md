# Spec 010: File Set `is_available` — Missing Files Support in DAT Import

## Status
< Planning | In Progress | Complete | Abandoned -->
Complete

## Affected Crates
- `core_types` — `ImportedFile` field `is_available`
- `database` — migration adding `is_available` column, repository updates
- `service` — DAT import pipeline, `DatImportExtras`, `RouteAndProcessFileSetsStep`

## Problem Statement

When a user imports a DAT file but does not have all the referenced ROM files locally available, the import should still create file set records in the database — with `file_info` rows marked `is_available = 0` for any missing ROMs. This allows the catalogue to reflect the complete DAT-described collection even before the user has sourced every file.

This branch (`file_set_is_available`) adds the `is_available` column to `file_info`, threads the missing-files data through the import pipeline via `DatImportExtras`, and routes `NonExistingWithoutLocalFiles` games through a new `RouteAndProcessFileSetsStep`.

---

## Acceptance Criteria

### Functional

1. When a DAT game has **all ROMs locally available**, the file set is imported normally with all `file_info` rows `is_available = 1`.
2. When a DAT game has **some ROMs available and some missing**, the file set is imported with available ROMs as `is_available = 1` and missing ROMs as `is_available = 0`.
3. When a DAT game has **no ROMs locally available**, a placeholder file set is still created with all `file_info` rows `is_available = 0`.
4. When the source directory is **completely empty** (no files scanned at all), DAT games are still categorised and placeholder records are created for all games without local files.
5. Existing file sets already linked to the DAT file are reported as `AlreadyExists` and not re-imported.
6. Existing file sets not yet linked to the DAT are linked and a release is created; `is_missing_files` status is propagated to the import result.
7. The import result status is:
   - `Success` — all ROMs were available
   - `SuccessWithWarnings` — some or all ROMs were missing (list of missing file names included)
   - `Failed` — import pipeline aborted
   - `AlreadyExists` — file set was already linked to this DAT

### Data Integrity

8. `file_info.is_available` is correctly persisted to SQLite for both available (`1`) and unavailable (`0`) records.
9. Missing files do not produce orphaned `file_info` rows — they are linked to the correct `file_set_file_info` record.
10. Re-importing the same DAT after acquiring missing files should update `is_available` from `0` to `1` (covered by the existing-file-set path).

---

## Known Issues / Findings (from branch review)

The following issues were identified during architectural review and must be resolved before merging.

### 🔴 Critical

#### Finding 1: `CategorizeFileSetsForImportStep` guard defeats the feature for empty source dirs

**File:** `service/src/mass_import/with_dat/steps.rs` ~line 238

```rust
fn should_execute(&self, context: &DatFileMassImportContext) -> bool {
    !context.state.common_state.file_metadata.is_empty()  // ← wrong guard
        && context.state.dat_file.is_some()
        && context.state.dat_file_id.is_some()
}
```

When the source directory is empty or has no matching files, `file_metadata` is empty, so `dat_game_statuses` is never populated, and the entire routing chain is silently skipped. No placeholders are created, no error is returned.

**Fix:** Remove the `file_metadata` condition. The guard should be:
```rust
fn should_execute(&self, context: &DatFileMassImportContext) -> bool {
    context.state.dat_file.is_some() && context.state.dat_file_id.is_some()
}
```

The `DatGameStatusService::get_status` already handles the no-local-files case by returning `NonExistingWithoutLocalFiles` when `scanned_file_sha1s` is empty.

**Status: ✅ Fixed**

---

#### Finding 2: `println!` calls in production database code ✅ Fixed

**File:** `database/src/repository/file_set_repository.rs`

Multiple `println!` calls in `add_file_set_with_tx` and `get_file_sets_by_file_type_and_systems` that were left in from debugging. These spam stdout in production and expose internal state.

**Fix:** Added `tracing = "0.1"` to `database/Cargo.toml` and replaced all 7 `println!` calls with `tracing::debug!`.

---

### 🟠 Important

#### Finding 3: `build_file_set_import_model` panics on invalid SHA1 ✅ Fixed

**File:** `service/src/mass_import/with_dat/mod.rs` ~line 30

```rust
sha1_from_hex_string(&rom.sha1).expect("Invalid SHA1 in DAT")
```

`DatGameStatusService` handles the same parse with a proper `Err` return. This function relies on an invisible ordering guarantee (earlier step already validated SHA1s) not enforced by the type system. If called from any other context it will panic.

**Fix:** Return `Result<FileSetImportModel, Error>` and propagate with `?`. Callers handle the error: `route_and_process_step.rs` returns `FileSetImportStatus::Failed`; `context.rs` logs a warning and skips the game.

---

#### Finding 4: `DatFileMassImportContext::get_import_file_sets` is dead and silently incomplete ✅ Fixed

**File:** `service/src/mass_import/with_dat/context.rs` ~line 84

The dead `get_import_file_sets()` and `can_import_file_sets()` methods were removed from `DatFileMassImportContext`. These methods were extracted into a new `ImportableFileSets` sub-trait which `DatFileMassImportContext` does not implement, enforced at compile time.

---

#### Finding 5: `DatGameStatusService` is hardcoded in `CategorizeFileSetsForImportStep`

**File:** `service/src/mass_import/with_dat/steps.rs` ~line 248

```rust
// TODO: add to context if needs injection for mocking in tests
let dat_game_status_service = DatGameStatusService::new(context.deps.repository_manager.clone());
```

The DB error path in this step cannot be tested in isolation.

**Fix:** Add `dat_game_status_ops: Arc<dyn DatGameStatusServiceOps>` to `DatFileMassImportOps`, following the same injection pattern used for all other service ops.

---

#### Finding 6: Full clone of `dat_game_statuses` in `RouteAndProcessFileSetsStep` ✅ Fixed

Replaced `.clone()` with `std::mem::take`, moving the Vec out of context with zero allocation.

---

### 🟡 Minor

#### Finding 7: TODO markers that should become tracked tasks

- `dat_game_status_service.rs:111` — assumes DAT-linked file sets are always complete; could produce partially-linked states without detection
- `file_set_repository.rs:478` — `sort_order` always set to `0`; produces unpredictable display order for multi-ROM games
- `steps.rs:87` — `header.name` used as DAT type identifier; fragile if names change

#### Finding 8: Doc comment on `handle_new_file_set` omits `Failed` return path

**File:** `service/src/mass_import/with_dat/route_and_process_step.rs` ~line 87

The doc comment lists only `Success` and `SuccessWithWarnings` but the function can also return `Failed`.

#### Finding 9: `dat_file_path: Option` as implicit mode discriminant ✅ Fixed

`MassImportInput` renamed to `DatMassImportInput` and `dat_file_path` changed from `Option<PathBuf>` to `PathBuf`. The two import modes are now fully separated at the type level: `DatMassImportInput` for DAT-driven imports, `FilesOnlyMassImportInput` for files-only imports.

---

## Implementation Order

Findings 1 and 2 are blocking — fix before merge.

Findings 3–6 are important quality improvements that should be completed in this branch if possible, or tracked as immediate follow-up.

Findings 7–9 are non-blocking and can be tracked as separate tasks.

---

## Manual Verification Checklist

After implementing all fixes:

- [ ] Import DAT file with all ROMs present → all `file_info` rows have `is_available = 1`
- [ ] Import DAT file with some ROMs missing → mixed `is_available` values, result status `SuccessWithWarnings`
- [ ] Import DAT file with **no** ROMs present (empty source folder) → placeholder file sets created, all `file_info` rows `is_available = 0`
- [ ] Re-import same DAT → existing entries show `AlreadyExists`, no duplicate records
- [ ] Import DAT with existing unlinked file sets → sets get linked, releases created
- [ ] `cargo test -p service` passes
- [ ] `cargo test -p database` passes
- [ ] `cargo clippy --all-targets` clean
- [ ] No `println!` or `dbg!` in production code

## As Implemented
Implementation completed as specified. The `is_available` column was added to `file_info` and threaded through the DAT import pipeline. `NonExistingWithoutLocalFiles` games are routed to a new pipeline step that creates placeholder file set records.
