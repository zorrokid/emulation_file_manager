# Spec 009 Tasks: Improve Test Coverage for Import Flow

## Task Breakdown

### Phase 1: High-Risk Path Coverage (Est. 4-5 tests)

#### Task 1.1: ImportDatFileStep Parse Failure
- **File:** `service/src/mass_import/with_dat/steps.rs`
- **Test name:** `test_import_dat_file_step_with_parse_failure_aborts_pipeline`
- **Description:** Verify that when the DAT parser returns an error, the pipeline aborts gracefully
- **Setup:**
  - Create `MockDatFileParserOps` that returns `Err(DatFileParserError::...)`
  - Create a `DatFileMassImportContext` with input pointing to invalid DAT file path
  - Create `ImportDatFileStep` instance
- **Action:**
  - Call `should_execute()` — should return `true`
  - Call `execute()` — should return `StepAction::Abort`
- **Assertions:**
  - Verify `context.state.dat_file` is `None`
  - Verify `context.state.dat_file_id` is `None`
  - Verify no database writes occurred
- **Risk level:** HIGH
- **Status:** Pending

---

#### Task 1.2: CategorizeFileSetsForImportStep - NonExisting Branch
- **File:** `service/src/mass_import/with_dat/steps.rs` (or new file)
- **Test name:** `test_categorize_file_sets_for_import_step_non_existing_game`
- **Description:** Verify that a game with no matching file set is classified as `NonExisting`
- **Setup:**
  - Create real `RepositoryManager` with test database
  - Create a DAT file in database
  - Do NOT create any file sets
  - Create `DatFile` with one game (ROM with SHA1)
  - Create `CategorizeFileSetsForImportStep` instance
  - Create context with empty file sets
- **Action:**
  - Call `should_execute()` — should return `true`
  - Call `execute()` on context
- **Assertions:**
  - `context.state.dat_game_statuses.len()` == 1
  - `context.state.dat_game_statuses[0]` matches `DatGameFileSetStatus::NonExisting`
- **Risk level:** HIGH
- **Status:** Pending

---

#### Task 1.3: CategorizeFileSetsForImportStep - ExistingWithReleaseAndLinkedToDat (no missing)
- **File:** `service/src/mass_import/with_dat/steps.rs` (or new file)
- **Test name:** `test_categorize_file_sets_for_import_step_existing_with_release_and_linked_to_dat_no_missing`
- **Description:** Verify that a game with a complete, linked file set is classified correctly
- **Setup:**
  - Create real `RepositoryManager` with test database
  - Create system, software title, release, file set with all ROMs present
  - Create DAT file in database
  - Link file set to DAT file via release
  - Create `DatFile` with matching game
- **Action:**
  - Call `execute()` on `CategorizeFileSetsForImportStep`
- **Assertions:**
  - Status matches `ExistingWithReleaseAndLinkedToDat { file_set_id: _, game: _, missing_files: [] }`
  - `missing_files` is empty
- **Risk level:** HIGH
- **Status:** Pending

---

#### Task 1.4: CategorizeFileSetsForImportStep - ExistingWithReleaseAndLinkedToDat (with missing)
- **File:** `service/src/mass_import/with_dat/steps.rs` (or new file)
- **Test name:** `test_categorize_file_sets_for_import_step_existing_with_release_and_linked_to_dat_with_missing`
- **Description:** Verify that a game with an incomplete but linked file set returns missing file list
- **Setup:**
  - Create real `RepositoryManager` with test database
  - Create system, software title, release, file set
  - Create file_info record for one ROM with `is_available = false`
  - Create DAT file in database, link to file set
  - Create `DatFile` with matching game (including unavailable ROM)
- **Action:**
  - Call `execute()` on `CategorizeFileSetsForImportStep`
- **Assertions:**
  - Status matches `ExistingWithReleaseAndLinkedToDat { file_set_id: _, game: _, missing_files: [sha1_1] }`
  - `missing_files` contains the unavailable SHA1
- **Risk level:** HIGH
- **Status:** Pending

---

#### Task 1.5: CategorizeFileSetsForImportStep - ExistingWithoutReleaseAndWithoutLinkToDat
- **File:** `service/src/mass_import/with_dat/steps.rs` (or new file)
- **Test name:** `test_categorize_file_sets_for_import_step_existing_without_release_and_without_link_to_dat`
- **Description:** Verify that a file set matching the DAT but not linked is classified correctly
- **Setup:**
  - Create real `RepositoryManager` with test database
  - Create system, file set with ROMs matching DAT game
  - Do NOT create release or link to any DAT file
  - Create DAT file in database (separate from file set)
  - Create `DatFile` with matching game
- **Action:**
  - Call `execute()` on `CategorizeFileSetsForImportStep`
- **Assertions:**
  - Status matches `ExistingWithoutReleaseAndWithoutLinkToDat { file_set_id: 123, game: _, missing_files: [] }`
  - `file_set_id` is set to the created file set ID
- **Risk level:** HIGH
- **Status:** Pending

---

#### Task 1.5b: CategorizeFileSetsForImportStep - should_execute Skip Conditions
- **File:** `service/src/mass_import/with_dat/steps.rs`
- **Test names:**
  - `test_categorize_file_sets_for_import_step_skips_when_file_metadata_empty`
  - `test_categorize_file_sets_for_import_step_skips_when_dat_file_not_loaded`
  - `test_categorize_file_sets_for_import_step_skips_when_dat_file_id_not_set`
- **Description:** Verify each of the three guard conditions in `should_execute` causes the step to be skipped. All existing tests only exercise the `true` path; none verify skip behaviour.
- **Setup:** Use `make_categorize_context` helper with one field cleared per test:
  1. `common_state.file_metadata` cleared to empty `HashMap`
  2. `state.dat_file` set to `None`
  3. `state.dat_file_id` set to `None`
- **Action:**
  - Call `step.should_execute(&context)` for each variant
- **Assertions:**
  - Returns `false` in all three cases
- **Risk level:** LOW
- **Status:** Pending

---

#### Task 1.6: ReadFilesStep with Read Failures and Directory Scan Errors
- **File:** `service/src/mass_import/common_steps/steps.rs`
- **Test name:** `test_read_files_step_with_read_failures_and_scan_errors`
- **Description:** Verify that file read failures and directory scan errors are tracked but don't abort the pipeline
- **Setup:**
  - Create `MockFileSystemOps` that:
    - Returns 3 files on `find_files()`
    - Returns error on `read_file()` for path 1
    - Returns success for paths 2, 3
    - Fails on `list_dir()` for a subdirectory
  - Create context with this mock
- **Action:**
  - Call `execute()` on `ReadFilesStep`
- **Assertions:**
  - `context.state.common_state.read_ok_files.len()` == 2
  - `context.state.common_state.read_failed_files.len()` == 1
  - `context.state.common_state.dir_scan_errors.len()` >= 1
  - Pipeline continues (returns `StepAction::Continue`)
- **Risk level:** MEDIUM
- **Status:** Pending

---

### Phase 2: Routing Logic Completion (4 tests)

#### Task 2.1: Handle New File Set with All ROMs Missing
- **File:** `service/src/mass_import/with_dat/route_and_process_step.rs`
- **Test name:** `test_handle_new_file_set_all_roms_missing_creates_placeholder`
- **Description:** Verify that a new import with no local files creates a file set with all files marked unavailable
- **Setup:**
  - Create DAT file with 3 ROMs (SHA1s: A, B, C)
  - Mock file import service with success
  - Create context with empty file metadata (no local files)
  - Create status: `NonExisting(game)`
- **Action:**
  - Call `execute()` on `RouteAndProcessFileSetsStep`
- **Assertions:**
  - Import result status is `SuccessWithWarnings`
  - Warning messages contain all 3 file names
  - File set ID is returned
  - `create_file_set` called with all ROMs in `dat_extras.missing_files`
- **Risk level:** MEDIUM
- **Status:** Pending

---

#### Task 2.2: Handle Link Existing to DAT - Success
- **File:** `service/src/mass_import/with_dat/route_and_process_step.rs`
- **Test name:** `test_handle_link_existing_to_dat_success`
- **Description:** Verify successful linking of an unlinked file set to a DAT file
- **Setup:**
  - Existing file set ID: 42
  - Mock file set service returns success on `create_release_for_file_set`
  - No missing files
  - Create status: `ExistingWithoutReleaseAndWithoutLinkToDat { file_set_id: 42, game, missing_files: [] }`
- **Action:**
  - Call `execute()` on `RouteAndProcessFileSetsStep`
- **Assertions:**
  - Import result status is `Success`
  - File set ID is 42
  - `create_release_for_file_set` called with correct parameters
- **Risk level:** HIGH
- **Status:** Pending

---

#### Task 2.3: Handle Link Existing to DAT - Failure
- **File:** `service/src/mass_import/with_dat/route_and_process_step.rs`
- **Test name:** `test_handle_link_existing_to_dat_with_failure`
- **Description:** Verify that linking failure is recorded as `Failed` and pipeline continues
- **Setup:**
  - Existing file set ID: 42
  - Mock file set service returns error on `create_release_for_file_set` (e.g., database constraint)
  - No missing files
  - Create status: `ExistingWithoutReleaseAndWithoutLinkToDat { file_set_id: 42, game, missing_files: [] }`
- **Action:**
  - Call `execute()` on `RouteAndProcessFileSetsStep`
- **Assertions:**
  - Import result status is `Failed`
  - Status message contains error details
  - File set ID is still returned (Some(42))
  - Pipeline returns `StepAction::Continue` (doesn't abort)
- **Risk level:** HIGH
- **Status:** Pending

---

#### Task 2.4: Handle Existing with Missing Files - Link Fails After Completion Attempt
- **File:** `service/src/mass_import/with_dat/route_and_process_step.rs`
- **Test name:** `test_handle_existing_with_missing_files_link_fails`
- **Description:** Verify failure when linking fails after finding newly available files
- **Setup:**
  - File set ID: 99, linked but with missing files
  - Missing files: [SHA1_A, SHA1_B]
  - Local files available: SHA1_A (so "newly available")
  - Mock file import service: `update_file_set` returns error
  - Create status: `ExistingWithReleaseAndLinkedToDat { file_set_id: 99, game, missing_files: [A, B] }`
- **Action:**
  - Call `execute()` on `RouteAndProcessFileSetsStep`
- **Assertions:**
  - Import result status is `Failed`
  - `update_file_set` was called (attempted to complete)
  - Pipeline continues
- **Risk level:** MEDIUM
- **Status:** Pending

---

### Phase 3: Integration-Level Edge Cases (4 tests)

#### Task 3.1: Read File Metadata Step Extracts Metadata from ZIP Contents
- **File:** `service/src/mass_import/common_steps/steps.rs`
- **Test name:** `test_read_file_metadata_step_extracts_metadata_from_zip_contents`
- **Description:** Verify that archives are transparent: metadata is extracted from contents, not the archive itself
- **Setup:**
  - Create mock file system with:
    - Path `/roms/game.zip`
    - Mock reader factory configured to return metadata for files inside the zip:
      - `game.bin` (SHA1_A, 1024 bytes)
      - `game.cue` (SHA1_B, 512 bytes)
  - File metadata map: `/roms/game.zip` → [ReadFile for game.bin, ReadFile for game.cue]
- **Action:**
  - Call `execute()` on `ReadFileMetadataStep`
- **Assertions:**
  - `context.state.common_state.file_metadata` contains `/roms/game.zip` entry
  - Entry contains 2 files (bin and cue), not the archive itself
  - Each file has correct SHA1 and size
- **Risk level:** MEDIUM
- **Status:** Pending

---

#### ~~Task 3.2: Build Update Model Skips Unmatched SHA1s with Warning~~ *(Removed)*

> **Removed:** DAT files are versioned — each import references a specific DAT version, so a SHA1
> recorded in `missing_files` will always have a matching ROM in the same DAT game. The
> unmatched-SHA1 branch in `build_update_model` is defensive dead code; writing a test for it
> would mean constructing an impossible runtime state. The defensive `else` branch and its warning
> log can be considered for removal in a future clean-up.

---

#### Task 3.3: File Info Deduplication Across Multiple File Sets (Integration)
- **File:** `service/src/mass_import/service.rs`
- **Test name:** `test_file_info_deduplication_across_multiple_file_sets_integration`
- **Description:** Verify that file_info records are reused when the same ROM is imported for different file sets
- **Setup:**
  - Create real `RepositoryManager` with test database
  - Import file set A with ROM X (SHA1_shared)
  - Import file set B with same ROM X (SHA1_shared)
- **Action:**
  - Execute mass import for both file sets
- **Assertions:**
  - Database query: `SELECT COUNT(*) FROM file_info WHERE sha1_checksum = SHA1_shared`
  - Result is 1 (not 2)
  - Both file sets linked to same file_info ID via `file_set_file_info` junction
  - `is_available` flag updated correctly on both links
- **Risk level:** MEDIUM
- **Status:** Pending

---

#### Task 3.4: Update File Set with Already-Linked Files Avoids PK Violation (Integration)
- **File:** `service/src/mass_import/service.rs`
- **Test name:** `test_update_file_set_with_already_linked_files_avoids_pk_violation_integration`
- **Description:** Verify that re-linking a file that was previously unavailable doesn't create PK violations
- **Setup:**
  - Create real `RepositoryManager` with test database
  - Create file set with file_info A (is_available = false)
  - File set linked to file_info A in junction table
  - Import same file_info A again (now available locally)
- **Action:**
  - Call `update_file_set()` to restore file_info A
- **Assertions:**
  - No PK violation on `file_set_file_info(file_set_id, file_info_id)`
  - file_info A updated: `is_available = true`
  - No duplicate junction entry created
  - Junction table still has only one entry for (file_set_id, file_info_id)
- **Risk level:** MEDIUM
- **Status:** Pending

---

### Phase 4: Minor Gaps (2-3 tests)

#### Task 4.1: Files-Only Mode with All Duplicates Filtered
- **File:** `service/src/mass_import/with_files_only/steps.rs`
- **Test name:** `test_files_only_mode_with_all_duplicate_file_sets`
- **Description:** Verify that when all scanned files are duplicates, the pipeline completes without error
- **Setup:**
  - Import file set A with ROM X, Y
  - Scan source with same ROM X, Y (already in database)
- **Action:**
  - Call `import_files_only()` on same source
- **Assertions:**
  - Pipeline completes successfully
  - No new file sets created
  - Result shows 0 imports
- **Risk level:** LOW
- **Status:** Pending

---

#### Task 4.2: Status Enum Coverage Audit
- **File:** Multiple test files (audit task)
- **Test name:** N/A (audit)
- **Description:** Verify all 5 `FileSetImportStatus` enum variants are exercised in tests
- **Variants to check:**
  1. `Success` — ✅ covered
  2. `SuccessWithWarnings` — ✅ covered
  3. `StillMissingFiles` — ✅ covered
  4. `AlreadyExists` — ✅ covered
  5. `Failed` — ✅ covered
- **Action:**
  - Search test files for all 5 variants
  - Verify each is asserted at least once
  - Add tests for any missing variants
- **Risk level:** LOW
- **Status:** Pending

---

## Manual Verification Checklist

After implementing all tests, verify the following:

- [ ] All 18 existing tests still pass
- [ ] All 15+ new tests pass
- [ ] `cargo clippy --all-targets` reports no warnings in `service/src/mass_import/`
- [ ] No new `unwrap()` or `panic!()` in test code
- [ ] No file system access in unit tests (all mocked)
- [ ] All async tests use `#[async_std::test]`
- [ ] Test names follow `test_<component>_<scenario>_<outcome>` pattern
- [ ] Each test has Arrange/Act/Assert comments
- [ ] Integration tests use real `RepositoryManager` with in-memory SQLite
- [ ] Mock tests use mock file systems and services
- [ ] Total test count ≥ 33 (18 + 15)

---

## Notes for Implementer

1. **Reuse test utilities:** Check `test_utils.rs` for existing helpers (make_context, make_ops, etc.)
2. **Mock setup:** All new mocks should follow the single-`Arc<Mutex<State>>` pattern used in existing code
3. **Error types:** Use actual `Error` enum from `crate::error` module for realistic scenarios
4. **Database:** Integration tests use `setup_test_db()` and `setup_test_repository_manager()` — migrations run automatically
5. **Async:** Use `#[async_std::test]` and `async fn`, not `#[tokio::test]` (crate uses async-std)
6. **Assertions:** Prefer specific assertions (`assert_eq!`, `assert_matches!`) over generic `assert!()`

---

## Dependencies & Prerequisites

- All tasks depend on reviewing existing test patterns in `service/src/mass_import/`
- Task 1.2–1.5 (categorize step) can be done in parallel once test utilities are understood
- Task 2.1–2.4 (routing) can be done in parallel once route_and_process mock setup is complete
- Task 3.1–3.4 (integration) depend on understanding real DB setup but can be done in parallel
- Task 4.x (audit) is independent and can be done anytime

