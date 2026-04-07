# Spec 009: Improve Test Coverage for Import Flow

## Status
< Planning | In Progress | Complete | Abandoned -->
Planning

## Affected Crates
- `service` — tests only (no production code changes)

## Problem Statement

The mass import system (`service/src/mass_import/`) has **18 tests** providing approximately **50% coverage** of the documented functionality in `docs/import-flow.md`. Critical gaps exist in:

1. **Pipeline step coverage** — One step (CategorizeFileSetsForImportStep) has zero tests
2. **Error path testing** — Error scenarios (parse failures, linking failures, DAT changes) are largely untested
3. **Edge case verification** — Complex behaviors like file deduplication, PK violation prevention, and archive handling are undocumented or unverified
4. **Integration-level assertions** — Database-level behaviors lack integration tests

These gaps pose significant risk to data integrity and import correctness, particularly in the routing and file set linking logic.

---

## Acceptance Criteria

### Functional Coverage
- ✅ All 7 DAT-guided pipeline steps have at least 2 tests (happy path + 1 error/edge case)
- ✅ All 4 routing cases (NonExisting, ExistingWithReleaseAndLinkedToDat×2, ExistingWithoutReleaseAndWithoutLinkToDat) are fully tested including error paths
- ✅ All 5 `FileSetImportStatus` enum variants are explicitly exercised in tests
- ✅ Edge cases documented in `docs/import-flow.md` (lines 293–307) have corresponding tests

### Test Quality
- ✅ All async tests use `#[async_std::test]` convention
- ✅ Unit tests use mocks (mock file system, mock services); integration tests use real in-memory SQLite
- ✅ All new tests follow the single-`Arc<Mutex<MockState>>` mock pattern
- ✅ All tests have clear Arrange/Act/Assert structure with descriptive names
- ✅ No `unwrap()` or `panic!()` in test assertions (use `assert!`, `assert_eq!`, `expect()`)

### Build & Test
- ✅ `cargo test -p service` passes all 33+ tests (18 existing + 15+ new)
- ✅ `cargo clippy --all-targets` reports no warnings in mass_import module
- ✅ `cargo sqlx prepare --workspace -- --all-targets` succeeds (if any SQL queries added)

---

## Implementation Approach

### Phase 1: High-Risk Path Coverage (4-5 tests)
Target the three areas with zero or minimal error path testing:
1. **ImportDatFileStep parse failure** — Verify pipeline aborts on invalid DAT file
2. **CategorizeFileSetsForImportStep** — Test all 3 status classification branches (4 test cases)
3. **ReadFilesStep edge cases** — Test read failures and directory scan errors

### Phase 2: Routing Logic Completion (4 tests)
Complete test coverage for `RouteAndProcessFileSetsStep` error/edge cases:
1. Case 1 (NonExisting): All ROMs missing → file set created as placeholder
2. Case 4 (Unlinked): Success AND failure paths for linking
3. Cases 3 & 4: Link failure with missing files → `Failed` status

### Phase 3: Integration-Level Edge Cases (4 tests)
Test complex database-level behaviors that cannot be fully verified with unit tests:
1. **Archive handling** — Verify metadata extraction from zip contents
2. **file_info deduplication** — Verify same file reused across multiple file sets
3. **PK violation prevention** — Verify no duplicate entries when re-linking files
4. **DAT changed between runs** — Verify unmatched SHA1s skipped with warning

### Phase 4: Minor Gaps (2-3 tests)
Address remaining gaps:
1. Files-only mode edge case: all duplicates filtered
2. Status enum audit: verify all 5 variants are covered
3. Progress event reporting (if gaps found in audit)

---

## Test Location & Naming Convention

All tests should be added to existing test modules in `service/src/mass_import/`:

| Module | Current Tests | New Tests | Location |
|--------|---------------|-----------|----------|
| `with_dat/steps.rs` | 5 | +1 (parse failure) | `tests` module at end of file |
| `with_dat/steps.rs` | — | +4 (categorize) | New `tests` module for categorize tests |
| `common_steps/steps.rs` | 3 | +1 (read failures) | Existing `tests` module |
| `with_dat/route_and_process_step.rs` | 4 | +4 (error paths) | Existing `tests` module |
| `service.rs` | 3 | +4 (integration) | Existing `tests` module |
| `with_files_only/steps.rs` | 1 | +1 (edge case) | Existing `tests` module |

**Naming pattern:** `test_<component>_<scenario>_<expected_outcome>`

Examples:
- `test_import_dat_file_step_with_parse_failure_aborts_pipeline`
- `test_categorize_file_sets_for_import_step_non_existing_game`
- `test_handle_link_existing_to_dat_with_create_release_failure`

---

## Technical Details

### Mock Pattern
Use single `Arc<Mutex<MockState>>` per service mock:

```rust
pub struct MockFileImportServiceOps {
    state: Arc<Mutex<MockFileImportState>>,
}

struct MockFileImportState {
    create_results: Vec<Result<ImportResult, Error>>,
    create_call_count: usize,
    update_results: Vec<Result<(), Error>>,
    update_call_count: usize,
}

impl MockFileImportServiceOps {
    pub fn new() -> Self { /* ... */ }
    pub fn fail_on_create(self, error: Error) -> Self { /* ... */ }
    pub fn expect_create_calls(self, count: usize) -> Self { /* ... */ }
    pub fn get_call_count(&self) -> usize { /* ... */ }
}
```

### Database Setup
For integration tests requiring real database:

```rust
#[async_std::test]
async fn test_file_info_deduplication_across_multiple_file_sets_integration() {
    // Setup
    let db = setup_test_db().await;
    let repo_manager = Arc::new(setup_test_repository_manager(&db).await);
    // All migrations run automatically
    
    // Create and import first file set with shared ROM
    // Create and import second file set with same ROM
    
    // Assert
    // Query database: only one file_info record for shared SHA1
    // Both file sets linked to same file_info ID
}
```

### Error Injection Patterns
For failure scenario tests:

```rust
// Mock parser returning error
let mock_parser = Arc::new(MockDatFileParserOps::new()
    .with_parse_error(DatFileParserError::InvalidFormat("...".into())));

// Mock file set service returning error
let mock_file_set_ops = Arc::new(MockFileSetServiceOps::new()
    .fail_on_create_release(Error::DatabaseConstraintViolation));
```

---

## Success Metrics

| Metric | Current | Target |
|--------|---------|--------|
| Total tests | 18 | 33+ |
| Pipeline steps with error path tests | 3/7 | 7/7 |
| Routing cases fully covered | 3/4 | 4/4 with error paths |
| Integration tests | 0 | 4 |
| Edge cases from spec (lines 293–307) | 6 covered, 8 untested | 14/14 |
| Test-to-code ratio | 18:7 steps | >4:1 |

---

## Risks & Mitigations

| Risk | Impact | Mitigation |
|------|--------|-----------|
| Mocks diverge from real behavior | False negatives in tests | Review mock implementations against actual service interfaces; add integration tests for critical paths |
| CategorizeFileSetsForImportStep logic is complex | Hard to test all branches | Write 4 unit tests (one per status variant) + integration test with real DB |
| Link failure recovery is critical | Silent data corruption if not tested | Add explicit tests for linking failure with captured error message and `Failed` status |
| Archive decompression black box | Zip files might not work correctly | Add explicit test with real zip file; verify metadata extracted from contents |

---

## Related Documentation

- `docs/import-flow.md` — Specification of all pipeline steps, routing cases, and edge cases
- `docs/patterns/test.md` — Testing conventions for this codebase
- `service/src/mass_import/test_utils.rs` — Existing helper functions for creating test data
- `service/src/file_import/` — Reference implementation for integration tests (real DB + mocks)

---

## Notes

- The existing `test_utils.rs` module has reusable helpers; add new helpers there if needed
- All tests should be deterministic (no file system access; use mocks)
- Integration tests should clean up after themselves (database is in-memory, so auto-cleanup)
- Consider parallelization: unit tests can run in parallel; integration tests may need serialization if they share resources


## As Implemented
_(Pending)_
