# Release Items Feature Specification

**Branch**: `001-release-items`  
**Created**: 2026-01-04

## Overview

Add support for tracking items that are included in software release packaging. Users can track which items they own and attach files to each item.

## What are Items?

Items are the physical/digital components of a software release:

- **Packaging**: box, manual, inlay card
- **Media**: disk(s), tape(s), cartridge(s)
- **Documentation**: map, registration card, reference card, keyboard overlay, code wheel
- **Other**: promotional materials, posters, stickers, etc.

## Goals

1. **Track item ownership**: Users can mark which items they have for each release
2. **File management**: Attach file sets to specific items (e.g., disk images to disk item, manual scans to manual item)
3. **Unified model**: All files (including executable media) linked through items
4. **Migration**: Convert existing file_set links from release to appropriate items

## Current State

- File sets are currently linked directly to releases via `release_file_set` table
- FileType enum has both media types (Rom, DiskImage) and scan types (ManualScan, BoxScan)
- Need to introduce items as intermediary between releases and file sets

## Requirements

### 1. Item Type Enumeration (core_types)

Create `ItemType` enum with variants:
- Box
- Manual
- InlayCard
- Disk (physical disk media)
- Tape (cassette tape)
- Cartridge
- Map
- RegistrationCard
- ReferenceCard
- KeyboardOverlay
- CodeWheel
- Other

Methods needed:
- `to_db_int()` / `from_db_int()` for database storage
- `Display` trait for UI display

**Note**: File organization remains by FileType (disk_image/, rom/, manual_scan/), not ItemType.

### 2. File Organization Strategy (Unified Model)

**Unified approach**: All files linked through items
- Release → Item → FileSet → Files
- ItemType = physical thing (Disk, Manual, Box, etc.)
- FileType = digital content type (DiskImage, ManualScan, etc.)
- One item can have multiple file sets of different types

**Examples**:

1. **Disk item** can have:
   - FileSet (type: DiskImage) - executable dump of the disk
   - FileSet (type: MediaScan) - photos/scans of physical disk

2. **Manual item** can have:
   - FileSet (type: ManualScan) - scanned pages
   - FileSet (type: Manual) - PDF document

3. **Cartridge item** can have:
   - FileSet (type: Rom) - ROM dump from cartridge
   - FileSet (type: MediaScan) - photos of cartridge

**File path structure** (unchanged):
```
collection_root/
  rom/
  disk_image/
  tape_image/
  memory_snapshot/
  manual_scan/
  box_scan/
  inlay_scan/
  media_scan/
  package_scan/
  manual/
  ... (organized by FileType as before)
```

**Key insight**: FileType determines storage location, ItemType represents the physical object.

### 3. Database Schema

**New table: `release_item`**
```sql
CREATE TABLE release_item (
    id INTEGER PRIMARY KEY,
    release_id INTEGER NOT NULL,
    item_type INTEGER NOT NULL,  -- ItemType as int
    notes TEXT,  -- Optional notes about condition, completeness, etc.
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (release_id) REFERENCES release(id) ON DELETE CASCADE
);
```

**New junction table: `release_item_file_set`**
```sql
CREATE TABLE release_item_file_set (
    release_item_id INTEGER NOT NULL,
    file_set_id INTEGER NOT NULL,
    PRIMARY KEY (release_item_id, file_set_id),
    FOREIGN KEY (release_item_id) REFERENCES release_item(id) ON DELETE CASCADE,
    FOREIGN KEY (file_set_id) REFERENCES file_set(id) ON DELETE CASCADE
);
```

**Update existing table: `file_set_file_info`**
Add ordering column to support file ordering within file sets:
```sql
ALTER TABLE file_set_file_info ADD COLUMN sort_order INTEGER;
```

**Ordering logic**:
- Default: Arbitrary order (or alphabetical by file_name)
- User can manually reorder files
- Can auto-order by filename postfix (e.g., "disk1.d64", "disk2.d64")
- Lower sort_order = earlier in sequence

**Migration note**: Existing `release_file_set` table will be deprecated and eventually removed after data migration.

### 4. Repository (database crate)

Create `ReleaseItemRepository` with methods:
- `create_item(release_id, item_type, notes)` -> ReleaseItem
- `get_items_for_release(release_id)` -> Vec<ReleaseItem>
- `get_item(item_id)` -> ReleaseItem
- `update_item(item_id, notes)`
- `delete_item(item_id)`
- `add_file_set_to_item(item_id, file_set_id)`
- `remove_file_set_from_item(item_id, file_set_id)`
- `get_file_sets_for_item(item_id)` -> Vec<FileSet>

### 5. File Import Pipeline Updates (service crate)

The current import pipeline links file_sets directly to releases via `release_file_set`. We need to update it to link through items.

**Changes to FileImportData / Context**:
- Add `item_id: i64` field (required)
- All file sets must be linked to an item
- Remove direct release → file_set linking

**Pipeline steps to update**:
- `UpdateDatabaseStep` in `add_file_set`: Link file_set to item via `release_item_file_set` instead of to release
- No directory structure changes needed (still organized by FileType)

**UI flow**:
1. User creates/selects an item (e.g., "Disk 1")
2. User imports files for that item
3. Files are linked to the item via file_set

**Backward compatibility**:
- Phase 1: Keep `release_file_set` table for existing data, add new item-based linking
- Phase 2: Migrate existing links to items

### 6. Migration Strategy

**Phase 1**: Add new schema alongside existing system
- Add ItemType enum
- Add release_item and release_item_file_set tables
- Add repository
- Keep existing `release_file_set` table functional

**Phase 2**: Data migration
- For each release with file_sets:
  - Analyze FileType and create appropriate items
  - Create Disk/Tape/Cartridge items for media file sets (DiskImage, TapeImage, Rom)
  - Create Manual items for ManualScan file sets
  - Create Box items for BoxScan/PackageScan file sets
  - Create InlayCard items for InlayScan file sets
  - Handle MediaScan (could belong to Disk, Tape, or Cartridge - may need user input)
  - Link file_sets to new items via `release_item_file_set`
  - Remove old links from `release_file_set`
- **S3 files**: No changes needed (organized by FileType, which doesn't change)
- **Database updates**: Only table linking changes, no file path updates needed

**Phase 3**: UI updates
- Add item management UI
- Update file attachment UI to work with items
- Show item checklist for releases
- Update game launching to find executable files through items

## Technical Notes

- Follow layered architecture (Core -> Database -> Service -> UI)
- Use SQLx with compile-time verification
- Use async-std runtime
- Add proper error handling with thiserror
- Write unit tests for conversions and repository methods

## Out of Scope (for initial implementation)

- UI changes (can be added later)
- Automatic migration of existing file_sets to items
- Item condition tracking (good/fair/poor)
- Item completeness tracking (complete/incomplete)
- Multiple instances of same item type with labels (e.g., "Disk 1", "Disk 2")
- Removal of deprecated `release_file_set` table

## Success Criteria

- ✅ ItemType enum added to core_types with all conversions
- ✅ Database migrations create release_item and release_item_file_set tables
- ✅ ReleaseItemRepository implements all CRUD operations
- ✅ All tests pass
- ✅ Existing functionality remains unchanged
