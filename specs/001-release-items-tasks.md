# Release Items Feature - Task Breakdown

**Branch**: `001-release-items`  
**Spec**: `specs/release-items.md`

## Phase 1: Core Implementation (MVP)

### Task 1: Add ItemType enum to core_types
**Estimate**: 30 min  
**Files**: `core_types/src/lib.rs`

- [ ] Create `ItemType` enum with variants:
  - Box, Manual, InlayCard, Disk, Tape, Cartridge
  - Map, RegistrationCard, ReferenceCard, KeyboardOverlay, CodeWheel, Other
- [ ] Implement `to_db_int()` -> u8
- [ ] Implement `from_db_int(u8)` -> Result<ItemType, CoreTypeError>
- [ ] Implement `Display` trait
- [ ] Add unit tests for conversions

**Dependencies**: None

---

### Task 2: Create database migrations
**Estimate**: 30 min  
**Files**: `database/migrations/YYYYMMDDHHMMSS_add_release_item_tables.sql`, `database/migrations/YYYYMMDDHHMMSS_add_file_set_ordering.sql`

**Migration 1: Add release_item tables**
- [ ] Create migration file with timestamp
- [ ] Add `release_item` table:
  - id (INTEGER PRIMARY KEY)
  - release_id (INTEGER NOT NULL, FK to release)
  - item_type (INTEGER NOT NULL)
  - notes (TEXT)
  - created_at (TEXT, default CURRENT_TIMESTAMP)
- [ ] Add `release_item_file_set` junction table:
  - release_item_id (INTEGER NOT NULL, FK to release_item)
  - file_set_id (INTEGER NOT NULL, FK to file_set)
  - PRIMARY KEY on (release_item_id, file_set_id)
- [ ] Add CASCADE DELETE on foreign keys

**Migration 2: Add file ordering**
- [ ] Add `sort_order INTEGER` column to `file_set_file_info` table
- [ ] Test migrations run successfully

**Dependencies**: Task 1 (ItemType enum)

---

### Task 3: Create ReleaseItem model
**Estimate**: 15 min  
**Files**: `database/src/models.rs` (or new `database/src/models/release_item.rs`)

- [ ] Create `ReleaseItem` struct:
  - id: i64
  - release_id: i64
  - item_type: ItemType (use from_db_int for conversion)
  - notes: Option<String>
  - created_at: String
- [ ] Implement `sqlx::FromRow` if needed (or use query_as! macro)
- [ ] Add to `models/mod.rs` exports

**Dependencies**: Task 1, Task 2

---

### Task 4: Create ReleaseItemRepository
**Estimate**: 2-3 hours  
**Files**: `database/src/repository/release_item_repository.rs`, `database/src/repository/mod.rs`

- [ ] Create `ReleaseItemRepository` struct with pool
- [ ] Implement methods:
  - `create_item(release_id: i64, item_type: ItemType, notes: Option<String>)` -> Result<ReleaseItem>
  - `get_item(item_id: i64)` -> Result<ReleaseItem>
  - `get_items_for_release(release_id: i64)` -> Result<Vec<ReleaseItem>>`
  - `update_item(item_id: i64, notes: Option<String>)` -> Result<()>
  - `delete_item(item_id: i64)` -> Result<()>
  - `add_file_set_to_item(item_id: i64, file_set_id: i64)` -> Result<()>
  - `remove_file_set_from_item(item_id: i64, file_set_id: i64)` -> Result<()>
  - `get_file_sets_for_item(item_id: i64)` -> Result<Vec<FileSet>>`
- [ ] Use SQLx with compile-time verification (sqlx::query_as!)
- [ ] Add proper error handling
- [ ] Export from repository/mod.rs

**Dependencies**: Task 3

---

### Task 5: Add ReleaseItemRepository to RepositoryManager
**Estimate**: 10 min  
**Files**: `database/src/repository_manager.rs` (or wherever RepositoryManager is defined)

- [ ] Add `release_item_repository` field to RepositoryManager
- [ ] Initialize in constructor
- [ ] Add getter method `pub fn release_item_repository(&self) -> &ReleaseItemRepository`

**Dependencies**: Task 4

---

### Task 6: Write integration tests for ReleaseItemRepository
**Estimate**: 45 min - 1 hour  
**Files**: `database/src/repository/release_item_repository.rs` (test module) or `database/tests/`

- [ ] Test `create_item` creates item successfully
- [ ] Test `get_item` retrieves correct item
- [ ] Test `get_items_for_release` returns all items for release
- [ ] Test `update_item` updates notes
- [ ] Test `delete_item` removes item and cascades to file_set links
- [ ] Test `add_file_set_to_item` creates link
- [ ] Test `remove_file_set_from_item` removes link
- [ ] Test `get_file_sets_for_item` returns linked file sets
- [ ] Test foreign key constraints work
- [ ] Test multiple items per release
- [ ] Test multiple file sets per item

**Dependencies**: Task 4

---

### Task 7: Update file set repository for ordering
**Estimate**: 1 hour  
**Files**: `database/src/repository/file_set_repository.rs`

- [ ] Update queries to include `sort_order` column
- [ ] Add method to update file ordering: `update_file_order(file_set_id, file_info_id, new_order)`
- [ ] Update `get_files_for_file_set` to order by `sort_order`
- [ ] Add method to reorder all files in a file set: `reorder_files(file_set_id, Vec<(file_info_id, order)>)`
- [ ] Add tests for ordering functionality

**Dependencies**: Task 2

---

### Task 8: Run all tests and verify
**Estimate**: 15 min

- [ ] Run `cargo test` in database crate
- [ ] Run `cargo test` in workspace root
- [ ] Verify all existing tests still pass
- [ ] Verify new tests pass
- [ ] Check for compiler warnings
- [ ] Run `cargo clippy`

**Dependencies**: All previous tasks

---

## Phase 2: File Import Pipeline Updates

### Task 9: Update FileImportData/Context for item linking
**Estimate**: 1 hour  
**Files**: `service/src/file_import/model.rs`, context files

- [ ] Add `item_id: i64` field to FileImportData
- [ ] Update all context structs (AddFileSetContext, etc.) to include item_id
- [ ] Update existing code to pass item_id through pipeline
- [ ] For now, can use placeholder/default value for backward compatibility

**Dependencies**: Phase 1 complete

---

### Task 10: Update import pipeline to link via items
**Estimate**: 1-2 hours  
**Files**: `service/src/file_import/add_file_set/steps.rs`, `service/src/file_import/update_file_set/steps.rs`

- [ ] Update `UpdateDatabaseStep` in add_file_set:
  - After creating file_set, link to item via `release_item_file_set`
  - Keep temporary support for old `release_file_set` linking
- [ ] Update UpdateFileSet pipeline similarly
- [ ] Add error handling for missing items
- [ ] Update rollback logic if needed

**Dependencies**: Task 9

---

### Task 11: Write tests for updated import pipeline
**Estimate**: 30-45 min  
**Files**: Test files in `service/src/file_import/`

- [ ] Test file import with item_id links to correct item
- [ ] Test file import creates entry in release_item_file_set
- [ ] Test error handling when item doesn't exist
- [ ] Verify existing tests still pass with changes

**Dependencies**: Task 10

---

## Phase 3: Data Migration (Future)

### Task 12: Create migration script/service
**Estimate**: 2-3 hours  
**Files**: New migration utility or service method

- [ ] Create migration service to convert existing data:
  - For each release with file_sets in `release_file_set`:
    - Determine appropriate ItemType from FileType:
      - DiskImage, TapeImage, Rom, MemorySnapshot → Disk/Tape/Cartridge
      - ManualScan, Manual → Manual
      - BoxScan, PackageScan → Box
      - InlayScan → InlayCard
      - MediaScan → needs determination (Disk/Tape/Cartridge)
    - Create release_item if doesn't exist for that type
    - Link file_set to item via release_item_file_set
    - Remove old link from release_file_set
- [ ] Add dry-run mode to preview changes
- [ ] Add progress reporting
- [ ] Add verification step
- [ ] Handle edge cases (multiple items of same type, etc.)

**Dependencies**: Phase 2 complete

---

### Task 13: Remove deprecated release_file_set table
**Estimate**: 30 min  
**Files**: Database migration, code cleanup

- [ ] Create migration to drop `release_file_set` table
- [ ] Remove any remaining code references to release_file_set
- [ ] Update documentation
- [ ] Run all tests to verify nothing breaks

**Dependencies**: Task 12 (migration complete)

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

### Task 15: Update file import UI to require item selection
**Estimate**: 2-3 hours  
**Files**: File import components in `relm4-ui/`

- [ ] Add item selection dropdown/list in file import dialog
- [ ] Allow creating new item during import
- [ ] Update import flow to pass item_id
- [ ] Show which item files will be attached to
- [ ] Handle case where no items exist yet

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

### Task 17: Update game launching to work with items
**Estimate**: 1-2 hours  
**Files**: Game launch logic in service/UI

- [ ] Update executable file discovery to look through items
- [ ] Find Disk/Tape/Cartridge items and their DiskImage/Rom file sets
- [ ] Maintain existing launch functionality
- [ ] Handle case where multiple executable file sets exist
- [ ] Show which item/file set is being launched

**Dependencies**: Task 14

---

## Summary

**Phase 1 (MVP)**: ~5-7 hours total
- Core types, database schema, repositories
- Tests
- No UI changes, no migration, backward compatible

**Phase 2 (Import Pipeline)**: ~3-4 hours
- Update import to link through items
- Keep backward compatibility temporarily

**Phase 3 (Migration)**: ~3-4 hours  
- Data migration from old structure
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
