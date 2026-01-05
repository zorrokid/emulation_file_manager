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
2. **File management**: Optionally link file sets to specific items for organization
3. **Flexible linking**: File sets can be linked to multiple items (e.g., combined PDF with multiple items)
4. **Backward compatible**: File sets without item associations continue to work
5. **Migration**: Optionally associate existing file_sets with appropriate items

## Current State

- File sets are linked to releases via `release_file_set` table
- No concept of items yet
- Need to add items as optional metadata/organization layer
- Some FileTypes don't have physical item equivalents (MemorySnapshot, Screenshot, etc.)

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

### 2. File Organization Strategy (Many-to-Many Model)

**Flexible approach**: Optional many-to-many linking between file sets and items
- Release → FileSet (via `release_file_set` - unchanged)
- FileSet ↔ Item (via new `file_set_item` - optional, many-to-many)
- ItemType = physical thing (Disk, Manual, Box, etc.)
- FileType = digital content type (DiskImage, ManualScan, etc.)

**Key benefits**:
- File sets can link to 0, 1, or many items
- Handles complex cases (e.g., one PDF with scans of multiple items)
- Backward compatible (existing file sets have no item links)
- Some file types don't need items (MemorySnapshot, Screenshot, etc.)

**Examples**:

1. **Disk item** with separate file sets:
   - FileSet A (type: DiskImage) → links to Disk item
   - FileSet B (type: MediaScan) → links to Disk item
   - Both also linked to release via `release_file_set`

2. **Combined PDF** with multiple items:
   - FileSet (type: ManualScan, contains all scans) → links to Manual, Box, InlayCard items
   - Also linked to release

3. **Screenshots** (no physical item):
   - FileSet (type: Screenshot) → no item links
   - Just linked to release via `release_file_set`

**File path structure** (unchanged):
```
collection_root/
  rom/
  disk_image/
  tape_image/
  memory_snapshot/
  screenshot/
  loading_screen/
  title_screen/
  manual_scan/
  box_scan/
  inlay_scan/
  media_scan/
  package_scan/
  manual/
  ... (organized by FileType as before)
```

**Key insight**: FileType determines storage location, ItemType represents the physical object. Linking is optional and flexible.

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

**New junction table: `file_set_item` (many-to-many)**
```sql
CREATE TABLE file_set_item (
    file_set_id INTEGER NOT NULL,
    item_id INTEGER NOT NULL,
    PRIMARY KEY (file_set_id, item_id),
    FOREIGN KEY (file_set_id) REFERENCES file_set(id) ON DELETE CASCADE,
    FOREIGN KEY (item_id) REFERENCES release_item(id) ON DELETE CASCADE
);
```

**Existing table unchanged**: `release_file_set` continues to link file sets to releases as before.
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

**Migration note**: `release_file_set` table remains unchanged. New `file_set_item` table provides optional item associations.

### 4. Repository (database crate)

Create `ReleaseItemRepository` with methods:
- `create_item(release_id, item_type, notes)` -> ReleaseItem
- `get_items_for_release(release_id)` -> Vec<ReleaseItem>
- `get_item(item_id)` -> ReleaseItem
- `update_item(item_id, notes)`
- `delete_item(item_id)`
- `link_file_set_to_item(file_set_id, item_id)` -> Result<()>
- `unlink_file_set_from_item(file_set_id, item_id)` -> Result<()>
- `get_file_sets_for_item(item_id)` -> Vec<FileSet>
- `get_items_for_file_set(file_set_id)` -> Vec<ReleaseItem>

### 5. File Import Pipeline Updates (service crate)

The current import pipeline links file_sets to releases via `release_file_set`. Item linking is optional and can be added separately.

**Changes to FileImportData / Context**:
- Add `item_ids: Vec<i64>` field (optional, can be empty)
- After file_set is created and linked to release, optionally link to items
- Backward compatible: existing code continues to work without item links

**Pipeline steps to update**:
- `UpdateDatabaseStep` in `add_file_set`: 
  - First create file_set and link to release (existing behavior)
  - Then optionally link to items via `file_set_item` table
- No directory structure changes needed (still organized by FileType)

**UI flow**:
1. User imports files for a release (existing flow)
2. Optionally: User selects which items the file set represents
3. File set is linked to release (required) and items (optional)

**Backward compatibility**:
- Phase 1: Item linking is optional, all existing code continues to work
- File sets without item links function normally

### 6. Migration Strategy

**Phase 1**: Add new schema alongside existing system
- Add ItemType enum
- Add release_item and file_set_item tables
- Add repository
- Keep existing `release_file_set` table functional (no changes)

**Phase 2**: Optional data migration
- For releases where item tracking is desired:
  - Analyze FileType and create appropriate items:
    - DiskImage, TapeImage, Rom → Disk/Tape/Cartridge items
    - ManualScan, Manual → Manual items
    - BoxScan, PackageScan → Box items
    - InlayScan → InlayCard items
    - MediaScan → Disk/Tape/Cartridge (may need user input)
  - Link file_sets to items via `file_set_item`
  - Keep `release_file_set` links (required for release association)
- Screenshot, MemorySnapshot, etc. file sets have no item links (intentionally)
- **S3 files**: No changes needed (organized by FileType, which doesn't change)
- **Database updates**: Only add links in `file_set_item`, no changes to existing tables

**Phase 3**: UI updates
- Add item management UI
- Update file import UI to optionally select items
- Show item checklist for releases
- Display which items a file set belongs to
- Allow linking/unlinking file sets to/from items

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
- Deprecation of `release_file_set` table (remains essential)

## Success Criteria

- ✅ ItemType enum added to core_types with all conversions
- ✅ Database migrations create release_item and release_item_file_set tables
- ✅ ReleaseItemRepository implements all CRUD operations
- ✅ All tests pass
- ✅ Existing functionality remains unchanged
