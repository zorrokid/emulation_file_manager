# Tasks: DAT Import Pipeline Router Refactor

## Problem

The `with_dat` import pipeline has routing logic split across three places,
making it hard to trace what happens to each file set status:

1. **`DatFileMassImportContext::get_import_items()`** — silently filters which
   statuses are passed to `ImportFileSetsStep`. Contains a bug where
   `ExistingWithReleaseAndLinkedToDat { is_missing_files: true }` is re-imported
   as a new file set instead of being updated.
2. **`ImportFileSetsStep`** (generic, `common_steps`) — processes the filtered
   items from above.
3. **`HandleExistingFileSetsStep`** — processes the remaining statuses with a
   complex `should_execute` guard. Has two `is_missing_files: _, // TODO`
   comments where missing-file state is silently dropped.

The `DatGameFileSetStatus` variants each have exactly one correct handling path,
but that path is invisible from the pipeline definition.

## Approach

Replace steps 7 and 8 (`ImportFileSetsStep` + `HandleExistingFileSetsStep`) in
the `with_dat` pipeline with a single **`RouteAndProcessFileSetsStep`** that
contains an explicit `match` over every status variant. Each arm calls a small
dedicated async `handle_*` function.

The `with_files_only` pipeline is **not touched**.

### New `with_dat` pipeline (7 steps)

```
ImportDatFileStep                       (unchanged)
CheckExistingDatFileStep                (unchanged)
StoreDatFileStep                        (unchanged)
ReadFilesStep                           (unchanged)
ReadFileMetadataStep                    (unchanged)
CategorizeFileSetsForImportStep         (simplified — see task 0)
RouteAndProcessFileSetsStep             (NEW — replaces steps 7 & 8)
```

### `DatGameFileSetStatus` enum — 3 variants (was 4)

`NonExistingWithoutLocalFiles` is removed and merged into `NonExisting`.
The distinction only existed to identify games with zero local SHA1 matches,
but `handle_new_file_set` handles that naturally: when the SHA1 map has no
matches for the game's ROMs, all ROMs land in `missing_files` of the
`FileSetImportModel`, producing `SuccessWithWarnings` automatically. The
`scanned_file_sha1s` parameter on `DatGameStatusService::get_status()` existed
solely to drive this distinction and can be removed, simplifying
`CategorizeFileSetsForImportStep`.

`is_missing_files: bool` is replaced by `missing_files: Vec<Sha1Checksum>` in
both existing-file-set variants. The bool is derivable from the vec and loses
information; the SHA1s enable precise warning messages and the
newly-available-file detection needed for re-run completion.

```rust
pub enum DatGameFileSetStatus {
    NonExisting(DatGame),
    ExistingWithReleaseAndLinkedToDat {
        file_set_id: i64,
        game: DatGame,
        missing_files: Vec<Sha1Checksum>,
    },
    ExistingWithoutReleaseAndWithoutLinkToDat {
        file_set_id: i64,
        game: DatGame,
        missing_files: Vec<Sha1Checksum>,
    },
}
```

### `RouteAndProcessFileSetsStep` structure

4 match arms, 4 `handle_*` async functions in `with_dat/steps.rs`:

```rust
async fn execute(&self, context: &mut DatFileMassImportContext) -> StepAction {
    let sha1_map = context.build_sha1_to_file_map();
    let dat_file = ...; // cloned from state

    for status in &context.state.statuses {
        match status {
            NonExisting(game) =>
                handle_new_file_set(game, &dat_file, &sha1_map, context).await,

            ExistingWithoutReleaseAndWithoutLinkToDat { file_set_id, game, missing_files } =>
                handle_link_existing_to_dat(*file_set_id, game, missing_files, &sha1_map, context).await,

            ExistingWithReleaseAndLinkedToDat { file_set_id, game, missing_files }
                if missing_files.is_empty() =>
                handle_already_complete(*file_set_id, game, context),

            ExistingWithReleaseAndLinkedToDat { file_set_id, game, missing_files } =>
                handle_existing_with_missing_files(*file_set_id, game, missing_files, &sha1_map, context).await,
        }
    }
    StepAction::Continue
}
```

### Re-run completion logic

When a status has non-empty `missing_files`, the handler cross-references
against the locally scanned SHA1 map to split into `newly_available` and
`still_missing`. If `newly_available` is non-empty, `update_file_set` is called
to import those files and flip their DB records to `is_available = true`.

`update_file_set` already exists on `FileImportServiceOps` (accepts
`UpdateFileSetModel` with `file_set_id`, `import_files`, `selected_files`).
No new service operation needed.

**Cross-reference helper** shared by both affected handlers:

```rust
fn partition_missing_files(
    missing_files: &[Sha1Checksum],
    sha1_map: &HashMap<Sha1Checksum, PathBuf>,
) -> (Vec<Sha1Checksum>, Vec<Sha1Checksum>) {
    missing_files.iter().cloned().partition(|sha1| sha1_map.contains_key(sha1))
    // (newly_available, still_missing)
}
```

### Full routing table

| Status | `missing_files` | `newly_available` | Action |
|---|---|---|---|
| `NonExisting` | — | — | `create_file_set`; ROMs with no SHA1 match go into missing_files |
| `ExistingWithReleaseAndLinkedToDat` | empty | — | `AlreadyExists` |
| `ExistingWithReleaseAndLinkedToDat` | non-empty | empty | `SuccessWithWarnings` (still missing) |
| `ExistingWithReleaseAndLinkedToDat` | non-empty | non-empty | `update_file_set` → `Success` or `SuccessWithWarnings` |
| `ExistingWithoutReleaseAndWithoutLinkToDat` | empty | — | link + release → `Success` |
| `ExistingWithoutReleaseAndWithoutLinkToDat` | non-empty | empty | link + release → `SuccessWithWarnings` |
| `ExistingWithoutReleaseAndWithoutLinkToDat` | non-empty | non-empty | link + release + `update_file_set` → `Success` or `SuccessWithWarnings` |

---

## Tasks

### 0. Simplify `DatGameFileSetStatus` enum and `DatGameStatusService`
**File:** `service/src/dat_game_status_service.rs`

- Remove `NonExistingWithoutLocalFiles` variant
- Replace `is_missing_files: bool` with `missing_files: Vec<Sha1Checksum>` in
  both `ExistingWith*` variants
- In `get_status()`: pass `find_file_set_result.missing_files` directly (already
  `Vec<Sha1Checksum>`) instead of converting with `.is_empty()`
- Remove `scanned_file_sha1s: &[Sha1Checksum]` parameter from `get_status()` —
  it was only used to distinguish `NonExisting` from `NonExistingWithoutLocalFiles`
- Update all callsites and tests that pattern-match on `is_missing_files` or
  `NonExistingWithoutLocalFiles`

### 1. Add `RouteAndProcessFileSetsStep` + `handle_*` functions
**File:** `service/src/mass_import/with_dat/steps.rs`

- Add `RouteAndProcessFileSetsStep` implementing `PipelineStep<DatFileMassImportContext>`
- `should_execute`: requires `statuses` non-empty and `dat_file` + `dat_file_id` present
- Add `partition_missing_files` helper (see structure above)
- Add 4 private async `handle_*` functions:
  - `handle_new_file_set(game, dat_file, sha1_map, context)` — builds
    `FileSetImportModel` (ROMs with no SHA1 match go into `missing_files`),
    calls `create_file_set`, records `Success` or `SuccessWithWarnings`, sends
    progress event
  - `handle_link_existing_to_dat(file_set_id, game, missing_files, sha1_map, context)`
    — calls `create_release_for_file_set` to link + create release; then if
    `newly_available` non-empty, calls `update_file_set`; records result based
    on `still_missing`
  - `handle_already_complete(file_set_id, game, context)` — records
    `AlreadyExists`, no I/O
  - `handle_existing_with_missing_files(file_set_id, game, missing_files, sha1_map, context)`
    — partitions missing files; if `newly_available` non-empty calls
    `update_file_set`; records `Success` or `SuccessWithWarnings`

### 2. Remove `HandleExistingFileSetsStep`
**File:** `service/src/mass_import/with_dat/steps.rs`

- Delete `HandleExistingFileSetsStep` struct and its `PipelineStep` impl
- Delete its unit tests (around lines 798 and 918)

### 3. Remove `get_import_items` / `get_import_item` from context
**File:** `service/src/mass_import/with_dat/context.rs`

- Delete `get_import_items()` and `get_import_item()` — logic moves into handler
  functions
- Remove `can_import_file_sets()` and `get_import_file_sets()` from the
  `MassImportContextOps` impl (no longer used by `with_dat`)
- Remove associated unit tests (`test_get_import_items`, `test_get_non_failed_files`
  if it references the above, etc.)
- **Keep** `build_sha1_to_file_map()` — it is called by `RouteAndProcessFileSetsStep`

### 4. Update `with_dat` pipeline definition
**File:** `service/src/mass_import/with_dat/pipeline.rs`

- Remove `ImportFileSetsStep` and `HandleExistingFileSetsStep`
- Add `RouteAndProcessFileSetsStep` as the final step
- Pipeline shrinks from 8 to 7 steps

### 5. Update `CategorizeFileSetsForImportStep`
**File:** `service/src/mass_import/with_dat/steps.rs`

- Remove the `scanned_file_sha1s` / `file_sha1s` build and the
  `scanned_file_sha1s` argument passed to `get_status()` (removed in task 0)
- Update tests for this step

### 6. Add update mock support to `MockFileImportServiceOps`
**File:** `service/src/file_import/file_import_service_ops.rs`

- Add `UpdateMockState` struct and `with_update_mock()` constructor to
  `MockFileImportServiceOps`, mirroring the existing `setup_create_mock` /
  `with_create_mock()` pattern
- The mock `update_file_set` already records calls in `update_calls`; this just
  adds a configurable success return value so re-run tests can assert on it

### 7. Write tests
**Files:** `service/src/mass_import/with_dat/steps.rs`,
`service/src/mass_import/service.rs`

Unit tests for `RouteAndProcessFileSetsStep` — one test per routing branch:
- `NonExisting`, all files available → `create_file_set`, `Success`
- `NonExisting`, no files available → `create_file_set`, `SuccessWithWarnings`
- `ExistingWithReleaseAndLinkedToDat`, `missing_files` empty → `AlreadyExists`
- `ExistingWithReleaseAndLinkedToDat`, none locally available → `SuccessWithWarnings`, no `update_file_set` call
- `ExistingWithReleaseAndLinkedToDat`, all now locally available → `update_file_set` called, `Success`
- `ExistingWithReleaseAndLinkedToDat`, some locally available → `update_file_set` called, `SuccessWithWarnings`
- `ExistingWithoutReleaseAndWithoutLinkToDat`, `missing_files` empty → linked, `Success`
- `ExistingWithoutReleaseAndWithoutLinkToDat`, none locally available → linked, `SuccessWithWarnings`
- `ExistingWithoutReleaseAndWithoutLinkToDat`, all now locally available → linked + `update_file_set`, `Success`

Integration test in `service.rs` for the re-run scenario: first import
produces `SuccessWithWarnings` (missing file); second import with file now
present produces `Success` and `update_file_set` is called.

Verify existing `service.rs` tests still pass.

### 8. (Optional) Clean up `MassImportContextOps` trait
**Files:** `service/src/mass_import/common_steps/context.rs`,
`service/src/mass_import/with_files_only/context.rs`,
`service/src/mass_import/common_steps/steps.rs`

- Remove `can_import_file_sets()` and `get_import_file_sets()` from the trait
- Implement them as concrete methods on `FilesOnlyMassImportContext` only
- Narrow `ImportFileSetsStep` generic bound accordingly

Polish — can be a follow-up.

---

## Files Affected

| File | Change |
|------|--------|
| `service/src/dat_game_status_service.rs` | Remove `NonExistingWithoutLocalFiles`, replace `is_missing_files: bool` → `missing_files: Vec<Sha1Checksum>`, remove `scanned_file_sha1s` param |
| `service/src/mass_import/with_dat/steps.rs` | Add `RouteAndProcessFileSetsStep` + 4 `handle_*` fns + `partition_missing_files`. Remove `HandleExistingFileSetsStep`. Simplify `CategorizeFileSetsForImportStep`. |
| `service/src/mass_import/with_dat/pipeline.rs` | 8 → 7 steps; swap out old steps 7–8 for `RouteAndProcessFileSetsStep` |
| `service/src/mass_import/with_dat/context.rs` | Remove `get_import_items`, `get_import_item`. Keep `build_sha1_to_file_map`. |
| `service/src/file_import/file_import_service_ops.rs` | Add `UpdateMockState` + `with_update_mock()` to `MockFileImportServiceOps` |
| `service/src/mass_import/common_steps/context.rs` | (Optional) Remove `can_import_file_sets` / `get_import_file_sets` from trait |
| `service/src/mass_import/common_steps/steps.rs` | (Optional) Narrow `ImportFileSetsStep` bound |
| `service/src/mass_import/with_files_only/context.rs` | (Optional) Make `get_import_file_sets` / `can_import_file_sets` concrete |

---

## Acceptance Criteria

- [ ] `cargo test -p service` passes with no regressions
- [ ] `with_dat` pipeline definition lists 7 steps, all clearly named
- [ ] `DatGameFileSetStatus` has 3 variants; `NonExistingWithoutLocalFiles` is gone
- [ ] Every status variant is handled in a single `match` in `RouteAndProcessFileSetsStep`
- [ ] No more `get_import_items()` in the context
- [ ] No more `HandleExistingFileSetsStep`
- [ ] Re-running import when previously missing files are now locally present calls `update_file_set` and produces `Success`
- [ ] `with_files_only` pipeline is unchanged

## Out of Scope

- `with_files_only` pipeline
- `CategorizeFileSetsForImportStep` logic (signature change only from task 0)
- `DatGameStatusService` business logic (enum + signature changes only)

---

## Post-Implementation Architect Review

Findings from code review after tasks 0–7 + bug fix were completed.

### Finding 1 (Medium): Handlers should return status, not call `record_and_send` directly ✅ Fixed

Every handler (`handle_new_file_set`, `handle_already_complete`, etc.) ended with a
`record_and_send(...)` call, mixing *computing* the outcome with *recording* it.

**Fix applied:** Handlers return `(Option<i64>, FileSetImportStatus)`. The routing loop in
`RouteAndProcessFileSetsStep::execute` calls `record_and_send` once per iteration.

### Finding 2 (Medium): DRY violation — `build_file_set_import_model` vs `handle_new_file_set` ✅ Fixed

Both built `FileSetImportModel` from a `DatGame` by iterating `game.roms`, matching SHA1s to
local files, and collecting `ImportFileContent`. The ROM-iteration loop was duplicated.

**Fix applied:** `build_file_set_import_model` moved to `with_dat/mod.rs` (shared parent module).
`handle_new_file_set` now calls it and adds only warning generation on top.

### Finding 3 (Medium): `existing_files` in `UpdateFileSetState` had misleading semantics ✅ Fixed

`CheckExistingFilesStep` was returning **all** `file_info` records matching selected SHA1s —
including records already in `files_in_file_set` with `is_available = false`. This overlap caused
the `already_linked` id-based workaround in `get_file_info_ids_with_file_names`.

**Fix applied:**
- `get_sha1_checksums` in `UpdateFileSetContext` now filters out SHA1s already in `files_in_file_set` before querying the DB.
- `get_file_info_ids_with_file_names` now skips by SHA1 before the id lookup (early `continue`), making the panic structurally impossible for already-linked files.
- `already_linked` HashSet removed.
- `existing_files` field comment updated to accurately describe its contents.

### Finding 4 (Low): Step name `"add_file_info_to_database"` is stale ✅ Fixed

`UpdateFileInfoToDatabaseStep::name()` returns `"add_file_info_to_database"` but the step now
also updates (restores) existing records with `is_available = false`. Rename to
`"update_file_info_in_database"`.

**File:** `service/src/file_import/update_file_set/steps.rs`

### Finding 5 (Low): No guard for empty `selected_files` after SHA1 matching in `complete_missing_files` ✅ Fixed

After the `build_update_model` fix, if all `newly_available` SHA1s failed to match a ROM in
`game.roms`, `update_file_set` would be called with empty `import_files` and `selected_files`.

**Fix applied:** `complete_missing_files` has an early return when `newly_available` is empty,
returning `SuccessWithWarnings` with the still-missing file names.

---

## Post-Implementation Architect Review — Round 2

Findings from code review after all Round 1 findings were addressed.

### Finding 6 (Low): `pub(crate)` over-exposes `build_file_set_import_model` ✅ Fixed

In `with_dat/mod.rs`, `build_file_set_import_model` is `pub(crate)` — visible to the entire
`service` crate. Both callers (`context.rs` and `route_and_process_step.rs`) are child modules of
`with_dat`. In Rust, child modules can access private items from their parent module via `super::`,
so no visibility annotation is needed.

**Suggested fix:** Remove `pub(crate)` — leave the function with no visibility qualifier.

**File:** `service/src/mass_import/with_dat/mod.rs`

### Finding 7 (Low): Inline `std::collections::HashSet` qualification ✅ Fixed

`get_file_info_ids_with_file_names` and `get_sha1_checksums` in `update_file_set/context.rs`
qualify `HashSet` inline (`std::collections::HashSet`) rather than using a `use` statement.
Inline qualification is a sign of a missing import.

**Suggested fix:** Add `use std::collections::HashSet;` to the top of the file.

**File:** `service/src/file_import/update_file_set/context.rs`

### Finding 8 (Medium): `is_new_files_to_be_imported` is semantically imprecise after Finding 3 fix ✅ Fixed

Before the Finding 3 fix, `existing_files` contained already-linked-but-unavailable records. The
method correctly returned `true` only when there were genuinely new records to INSERT. After the
fix, `existing_files` no longer includes already-linked SHA1s — so the method now returns `true`
even when the only work needed is an `UPDATE is_available` (not an INSERT). The name implies
"new records" but the meaning is now "any file_info work pending."

**Suggested fix:** Rename to `needs_file_info_upsert` in the `AddFileSetContextOps` trait and
all implementations.

**Files:** `service/src/file_import/common_steps/import.rs` (trait), `service/src/file_import/update_file_set/context.rs` (impl), `service/src/file_import/update_file_set/steps.rs` (caller)

### Finding 9 (Medium): `FileSetImportStatus::SuccessWithWarnings` misused for "nothing imported" ✅ Fixed

In `complete_missing_files`, when `newly_available` is empty:
```rust
return FileSetImportStatus::SuccessWithWarnings(sha1s_to_warning_messages(...));
```
`SuccessWithWarnings` implies something was imported but with caveats. Here nothing was imported —
the re-run simply found the files still absent. The GUI would show this as a partial success, which
is misleading. A dedicated variant would express the intent precisely and allow distinct UI
treatment (different icon or colour for "still waiting" vs "imported with warnings").

**Suggested fix:** Add a new variant:
```rust
pub enum FileSetImportStatus {
    Success,
    SuccessWithWarnings(Vec<String>),
    StillMissingFiles(Vec<String>),   // re-run attempted, files not yet locally available
    Failed(String),
    AlreadyExists,
}
```
Update `complete_missing_files` to return `StillMissingFiles` when `newly_available` is empty, and
handle the new variant in the GUI progress display.

**Files:** `service/src/mass_import/models.rs`, `service/src/mass_import/with_dat/route_and_process_step.rs`, `relm4-ui` (progress display)
