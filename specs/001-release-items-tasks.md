# Release Items Feature - Task Breakdown


**Branch**: `001-release-items`  
**Spec**: `specs/release-items.md`

## Phase 1: Core Implementation (MVP)

### Task 1: Add ItemType enum to core_types
**Estimate**: 30 min  
**Files**: `core_types/src/lib.rs`

- [x] Create `ItemType` enum with variants:
  - Box, Manual, InlayCard, Disk, Tape, Cartridge
  - Map, RegistrationCard, ReferenceCard, KeyboardOverlay, CodeWheel, Other
- [x] Implement `to_db_int()` -> u8
- [x] Implement `from_db_int(u8)` -> Result<ItemType, CoreTypeError>
- [x] Implement `Display` trait
- [x] Add unit tests for conversions

**Dependencies**: None

---

### Task 2: Create database migrations
**Estimate**: 30 min  
**Files**: `database/migrations/YYYYMMDDHHMMSS_add_release_item_tables.sql`, `database/migrations/YYYYMMDDHHMMSS_add_file_set_ordering.sql`

**Migration 1: Add release_item tables**
- [x] Create migration file with timestamp
- [x] Add `release_item` table:
  - id (INTEGER PRIMARY KEY)
  - release_id (INTEGER NOT NULL, FK to release)
  - item_type (INTEGER NOT NULL)
  - notes (TEXT)
- [x] Add `file_set_item` junction table (many-to-many):
  - file_set_id (INTEGER NOT NULL, FK to file_set)
  - item_id (INTEGER NOT NULL, FK to release_item)
  - PRIMARY KEY on (file_set_id, item_id)
- [x] Add CASCADE DELETE on foreign keys

**Migration 2: Add file ordering**
- [x] Add `sort_order INTEGER` column to `file_set_file_info` table
- [x] Test migrations run successfully

**Dependencies**: Task 1 (ItemType enum)

---

### Task 3: Create ReleaseItem model
**Estimate**: 15 min  
**Files**: `database/src/models.rs` (or new `database/src/models/release_item.rs`)

- [x] Create `ReleaseItem` struct:
  - id: i64
  - release_id: i64
  - item_type: ItemType (use from_db_int for conversion)
  - notes: Option<String>
- [ ] Implement `sqlx::FromRow` if needed (or use query_as! macro)
- [ ] Add to `models/mod.rs` exports

**Dependencies**: Task 1, Task 2

---

### Task 4: Create ReleaseItemRepository
**Estimate**: 2-3 hours  
**Files**: `database/src/repository/release_item_repository.rs`, `database/src/repository/mod.rs`

- [x] Create `ReleaseItemRepository` struct with pool
- [x] Implement methods:
  - `create_item(release_id: i64, item_type: ItemType, notes: Option<String>)` -> Result<ReleaseItem>
  - `get_item(item_id: i64)` -> Result<ReleaseItem>
  - `get_items_for_release(release_id: i64)` -> Result<Vec<ReleaseItem>>`
  - `update_item(item_id: i64, notes: Option<String>)` -> Result<()>
  - `delete_item(item_id: i64)` -> Result<()>
  - `link_file_set_to_item(file_set_id: i64, item_id: i64)` -> Result<()>
  - `unlink_file_set_from_item(file_set_id: i64, item_id: i64)` -> Result<()>
  - `get_file_sets_for_item(item_id: i64)` -> Result<Vec<FileSet>>`
  - `get_items_for_file_set(file_set_id: i64)` -> Result<Vec<ReleaseItem>>`
- [x] Use SQLx with compile-time verification (sqlx::query_as!)
- [x] Add proper error handling
- [x] Export from repository/mod.rs

**Dependencies**: Task 3
- [x] Use SQLx with compile-time verification (sqlx::query_as!)
- [x] Add proper error handling
- [x] Export from repository/mod.rs

**Dependencies**: Task 3

---

### Task 5: Add ReleaseItemRepository to RepositoryManager
**Estimate**: 10 min  
**Files**: `database/src/repository_manager.rs` (or wherever RepositoryManager is defined)

- [x] Add `release_item_repository` field to RepositoryManager
- [x] Initialize in constructor
- [x] Add getter method `pub fn release_item_repository(&self) -> &ReleaseItemRepository`

**Dependencies**: Task 4

---

### Task 6: Write integration tests for ReleaseItemRepository
**Estimate**: 45 min - 1 hour  
**Files**: `database/src/repository/release_item_repository.rs` (test module) or `database/tests/`

- [x] Test `create_item` creates item successfully
- [x] Test `get_item` retrieves correct item
- [x] Test `get_items_for_release` returns all items for release
- [x] Test `update_item` updates notes
- [x] Test `delete_item` removes item and cascades to file_set links
- [x] Test `link_file_set_to_item` creates link in file_set_item table
- [x] Test `unlink_file_set_from_item` removes link
- [x] Test `get_file_sets_for_item` returns linked file sets
- [x] Test `get_items_for_file_set` returns linked items
- [x] Test file_set can link to multiple items (many-to-many)
- [x] Test foreign key constraints work
- [x] Test multiple items per release
- [x] Test multiple file sets per item

**Dependencies**: Task 4

---

### Task 7: Update file set repository for ordering
**Estimate**: 1 hour  
**Files**: `database/src/repository/file_set_repository.rs`

- [x] Update queries to include `sort_order` column
- [x] Add method to update file ordering: `update_file_set_file_info_sort_order(file_set_id, file_info_id, new_order)`
- [x] Update `get_files_for_file_set` to order by `sort_order`
- [x] Add method to reorder all files in a file set: `update_file_set_file_infos_sort_order(file_set_id, Vec<(file_info_id, order)>)`
- [x] Add tests for ordering functionality

**Dependencies**: Task 2

---

### Task 8: Run all tests and verify
**Estimate**: 15 min

- [x] Run `cargo test` in database crate
- [x] Run `cargo test` in workspace root
- [x] Verify all existing tests still pass
- [x] Verify new tests pass
- [x] Check for compiler warnings
- [x] Run `cargo clippy`

**Dependencies**: All previous tasks

---

## Phase 2: File Import Pipeline Updates

### Task 9: Update FileImportData/Context for optional item linking
**Estimate**: 1 hour  
**Files**: `service/src/file_import/model.rs`, context files

- [ ] Add `item_ids: Vec<i64>` field to FileImportData (optional, can be empty)
- [ ] Update all context structs (AddFileSetContext, etc.) to include item_ids
- [ ] Update existing code to pass item_ids through pipeline
- [ ] Default to empty vec for backward compatibility

**Dependencies**: Phase 1 complete

---

### Task 10: Update import pipeline to optionally link to items
**Estimate**: 1-2 hours  
**Files**: `service/src/file_import/add_file_set/steps.rs`, `service/src/file_import/update_file_set/steps.rs`

- [ ] Update `UpdateDatabaseStep` in add_file_set:
  - First create file_set and link to release (existing behavior via `release_file_set`)
  - Then loop through item_ids and link via `file_set_item` table
- [ ] Update UpdateFileSet pipeline similarly
- [ ] Add error handling for missing items
- [ ] Update rollback logic if needed (remove file_set_item links on failure)

**Dependencies**: Task 9

---

### Task 11: Write tests for updated import pipeline
**Estimate**: 30-45 min  
**Files**: Test files in `service/src/file_import/`

- [ ] Test file import with empty item_ids works (backward compatible)
- [ ] Test file import with single item_id creates link in file_set_item
- [ ] Test file import with multiple item_ids creates multiple links
- [ ] Test error handling when item doesn't exist
- [ ] Verify existing tests still pass with changes

**Dependencies**: Task 10

---

## Phase 3: Data Migration (Future)

### Task 12: Create optional data migration script/service
**Estimate**: 2-3 hours  
**Files**: New migration utility or service method

- [ ] Create migration service to optionally associate file_sets with items:
  - For releases where user wants item tracking:
    - Determine appropriate ItemType from FileType:
      - DiskImage, TapeImage, Rom → Disk/Tape/Cartridge
      - ManualScan, Manual → Manual
      - BoxScan, PackageScan → Box
      - InlayScan → InlayCard
      - MediaScan → needs determination (Disk/Tape/Cartridge)
      - Screenshot, MemorySnapshot, etc. → skip (no item)
    - Create release_item if doesn't exist for that type
    - Link file_set to item via `file_set_item` table
    - Keep `release_file_set` link (required for release association)
- [ ] Add dry-run mode to preview changes
- [ ] Add progress reporting
- [ ] Add verification step
- [ ] Handle edge cases (combined PDFs, etc.)

**Dependencies**: Phase 2 complete

---

### Task 13: UI for item-to-file-set linking
**Estimate**: 1-2 hours  
**Files**: Existing file management UI

- [ ] Add UI to view which items a file_set is linked to
- [ ] Add ability to link/unlink file_sets to/from items
- [ ] Show in file set detail view
- [ ] Allow multi-select for linking to multiple items

**Dependencies**: Phase 3 Task 12 (or can be done independently)

---

## Phase 4: UI Updates (Future)

### Task 14: Add item management UI
**Estimate**: 4-6 hours  
**Files**: `relm4-ui/` crate, new components

- [ ] Create ItemListComponent to show items for a release
- [ ] Create ItemDetailComponent to show/edit item details
- [ ] Add "Add Item" button/dialog with ItemType selector
- [ ] Add notes field for items
- [ ] Add file set list for each item
- [ ] Add "Attach File Set" functionality
- [ ] Integrate into release detail view
- [ ] Show item type icon/label

**Dependencies**: Phase 1 complete

---

### Task 15: Update file import UI for optional item selection
**Estimate**: 2-3 hours  
**Files**: File import components in `relm4-ui/`

- [ ] Add optional item selection (multi-select) in file import dialog
- [ ] Allow creating new items during import
- [ ] Update import flow to pass item_ids (can be empty)
- [ ] Show which items files will be linked to
- [ ] Make item selection optional (backward compatible)

**Dependencies**: Task 14

---

### Task 16: Add file ordering UI
**Estimate**: 2-3 hours  
**Files**: File viewer/editor components

- [ ] Add drag-and-drop or up/down buttons for file ordering
- [ ] Show current order visually (numbered list)
- [ ] Add "Auto-order by filename" button
- [ ] Update file list display to respect sort_order
- [ ] Add save button to persist new ordering

**Dependencies**: Task 14

---

### Task 17: Update game launching (if needed)
**Estimate**: 30 min - 1 hour  
**Files**: Game launch logic in service/UI

- [ ] Verify game launching still works (should be unaffected since release_file_set unchanged)
- [ ] Optionally: Show which item the launched file belongs to
- [ ] No changes needed if release_file_set remains the primary mechanism

**Dependencies**: Task 14

---

## Summary

**Phase 1 (MVP)**: ~5-7 hours total
- Core types, database schema, repositories
- Tests
- No UI changes, no migration, backward compatible
- `release_file_set` unchanged, items are optional metadata layer

**Phase 2 (Import Pipeline)**: ~3-4 hours
- Update import to optionally link file sets to items
- Fully backward compatible (item linking is optional)

**Phase 3 (Migration)**: ~3-5 hours  
- Optional data migration tool
- UI for managing file_set ↔ item links

**Phase 4 (UI)**: ~7-11 hours  
- Complete UI implementation
- Item management, file ordering, optional item selection in import

**Total**: ~18-27 hours for complete feature

## Notes

- Each phase can be developed and deployed independently
- Phase 1 can be merged without breaking existing functionality
- `release_file_set` table remains essential (not deprecated)
- Items are optional organizational metadata
- File sets can exist without item associations
- File ordering can be implemented separately from items if needed
- Remove deprecated table

**Phase 4 (UI)**: ~9-14 hours  
- Complete UI implementation
- Item management, file ordering, game launching

**Total**: ~20-29 hours for complete feature

## Notes

- Each phase can be developed and deployed independently
- Phase 1 can be merged without breaking existing functionality
- UI can start development during Phase 2/3
- File ordering can be implemented separately from items if needed
