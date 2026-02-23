# Filename-Based Mass Import — Task Breakdown

**Branch**: `003-filename-import`
**Spec**: `specs/003-filename-import.md`

## Phase 1: Context trait and refactoring

### Task 1: Define `MassImportContext` trait
**Files**: `service/src/mass_import/context.rs`

- [ ] Define `MassImportContext` trait with accessors for shared state:
  - `deps()`, `ops()`, `progress_tx()`, `input()`
  - `read_ok_files_mut()`, `read_failed_files_mut()`, `dir_scan_errors_mut()`
  - `file_metadata_mut()`, `file_metadata()`
  - `import_items()`, `import_items_mut()`
  - `import_results_mut()`
- [ ] Rename existing concrete context struct to `DatImportContext`
- [ ] Implement `MassImportContext` trait for `DatImportContext`
- [ ] `DatImportContext` retains its DAT-specific fields: `dat_file`, `dat_file_id`, `statuses`

**Dependencies**: None

---

### Task 2: Make `ImportItem.dat_game` optional
**Files**: `service/src/mass_import/models.rs`

- [ ] Change `dat_game: DatGame` to `dat_game: Option<DatGame>` in `ImportItem`
- [ ] Update all construction sites in existing steps to wrap with `Some(...)`
- [ ] Update all read sites to use `.as_ref()` or pattern-match

**Dependencies**: None

---

### Task 3: Make shared steps generic over `MassImportContext`
**Files**: `service/src/mass_import/steps.rs`

- [ ] Change `ReadFilesStep` impl from `PipelineStep<MassImportContext>` to `impl<C: MassImportContext> PipelineStep<C>`
- [ ] Change `ReadFileMetadataStep` similarly
- [ ] Change `ImportFileSetsStep` similarly; update any `dat_game` access to handle `Option`
- [ ] Verify DAT-specific steps (`ImportDatFileStep`, `CheckExistingDatFileStep`, `StoreDatFileStep`, `FilterExistingFileSetsStep`, `LinkExistingFileSetsStep`) remain typed against `DatImportContext` — no changes needed

**Test cases:**
- [ ] All existing unit/integration tests for the steps still compile and pass after the generic change

**Dependencies**: Task 1, Task 2

---

### Task 4: Rename existing pipeline to `DatImportPipeline`
**Files**: `service/src/mass_import/pipeline.rs`

- [ ] Rename existing pipeline struct/function to `DatImportPipeline`
- [ ] Update `service.rs` and `mod.rs` references accordingly

**Dependencies**: Task 3

---

## Phase 2: New pipeline and step

### Task 5: Add `FileNameImportContext`
**Files**: `service/src/mass_import/context.rs`

- [ ] Create `FileNameImportContext` struct with only the shared-state fields (no DAT fields)
- [ ] Implement `MassImportContext` trait for `FileNameImportContext`
- [ ] Add `into_result()` method that converts context state to `MassImportResult`

**Dependencies**: Task 1

---

### Task 6: Implement `BuildImportItemsFromFileNamesStep`
**Files**: `service/src/mass_import/steps.rs` (or new `service/src/mass_import/steps_filename.rs`)

- [ ] Implement `PipelineStep<FileNameImportContext>` for `BuildImportItemsFromFileNamesStep`
- [ ] For each entry in `context.file_metadata()`:
  - [ ] Strip extension using `domain::title_normalizer::rules::extension::strip_extension`
  - [ ] Call `domain::title_normalizer::normalizer::get_software_title(stem)` to get both names
  - [ ] Query DB for existing FileSet by SHA1 — skip if already imported
  - [ ] Construct `ImportItem` with `dat_game: None`, empty `dat_roms_available`/`dat_roms_missing`, derived names, and `FileSetImportModel`
  - [ ] Append to `context.import_items_mut()`
- [ ] `should_execute`: only if `file_metadata` is non-empty

**Test cases (unit tests, inline `#[cfg(test)]`):**
- [ ] Single file with no parentheticals → `release_name == stem`, `software_title_name == stem`
- [ ] File with region tag `Game (USA).nes` → `release_name == "Game (USA)"`, `software_title_name == "Game"`
- [ ] File with multiple tags `Game (USA) (v1.1) (Beta).nes` → `software_title_name == "Game"`
- [ ] File whose SHA1 already exists in DB → item is skipped (not added to `import_items`)
- [ ] File metadata empty → step skips (`should_execute` returns false)

**Dependencies**: Task 2, Task 5

---

### Task 7: Implement `FileNameImportPipeline`
**Files**: `service/src/mass_import/pipeline.rs`

- [ ] Add `FileNameImportPipeline` struct
- [ ] Steps in order: `ReadFilesStep`, `ReadFileMetadataStep`, `BuildImportItemsFromFileNamesStep`, `ImportFileSetsStep`
- [ ] Export from `mod.rs`

**Dependencies**: Task 3, Task 6

---

### Task 8: Update `MassImportService::import()` to branch on pipeline
**Files**: `service/src/mass_import/service.rs`

- [ ] If `input.dat_file_path.is_some()`: create `DatImportContext`, run `DatImportPipeline`
- [ ] Else: create `FileNameImportContext`, run `FileNameImportPipeline`
- [ ] Both branches return `MassImportResult`

**Dependencies**: Task 4, Task 7

---

## Phase 3: Integration tests

### Task 9: Integration test for filename-based import
**Files**: `service/tests/mass_import_filename.rs` (or new file in `service/tests/`)

- [ ] Test: directory with a single file → one FileSet, Release, SoftwareTitle created with correct names
- [ ] Test: directory with multiple files → each file gets its own FileSet/Release/SoftwareTitle
- [ ] Test: file whose SHA1 already exists in DB → skipped, not duplicated
- [ ] Test: file with NoIntro-style name `Donkey Kong (USA, Europe).nes` → `software_title_name == "Donkey Kong"`, `release_name == "Donkey Kong (USA, Europe)"`
- [ ] Test: directory with no files → pipeline completes without error, empty result

**Dependencies**: Task 8

---

### Task 10: Regression — existing DAT import tests still pass
**Files**: All existing test files under `service/`

- [ ] Run `cargo test -p service` — all existing tests must pass unmodified
- [ ] Verify `cargo clippy --all-targets` produces no new warnings

**Dependencies**: Task 8

---

## Manual verification checklist

> These items require running the application and confirming behaviour visually.

- [ ] Open import dialog, leave DAT field blank, select a directory with ROM files → import runs and files appear in the collection
- [ ] Each imported file appears as a separate FileSet/Release/SoftwareTitle with names derived from the filename
- [ ] Running the same import again on the same directory → files are skipped (already imported by SHA1), no duplicates created
- [ ] DAT-based import still works correctly after this change

---

## Summary

| Phase | Tasks | Notes |
|---|---|---|
| 1 — Refactoring | 1–4 | Context trait, generics; no behaviour change |
| 2 — New pipeline | 5–8 | New context, step, pipeline, service wiring |
| 3 — Tests | 9–10 | Integration + regression |
