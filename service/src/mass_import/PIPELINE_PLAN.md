# Mass Import Pipeline Implementation Plan

## Goal
Create a complete pipeline to import Software Titles with Releases and File Sets from DAT files and associated ROM/game files.

## Architecture Overview

### Data Flow
```
DAT File (DatGame with DatRoms) + File Directory
    ↓
Parse & Match (using SHA1 checksums)
    ↓
Software Title (from DatGame.name/description)
    ↓
Release (from DatRelease or generated)
    ↓
File Set (from DatGame with DatRoms as files)
```

### Layer Placement
- **Pipeline Steps**: Service layer (`service/src/mass_import/steps.rs`)
- **Context & Models**: Service layer (`service/src/mass_import/context.rs`)
- **Repository Calls**: Database layer (already exists)
- **File Operations**: Core utilities (file_metadata, file_import)

## Pipeline Steps (9 Total)

### ✅ Step 1: ImportDatFileStep
**Status**: Already implemented
- Parses DAT file using `DatFileParserOps`
- Stores result in `context.dat_file`

### ✅ Step 2: ReadFilesStep
**Status**: Already implemented
- Discovers files in source directory
- Creates `ImportItem` for each file with `Pending` status

### ⬜ Step 3: ReadFileMetadataStep
**Status**: Partially implemented as `CheckFilesStep`
**Needs**: 
- Extract and store SHA1 checksums in `ImportItem`
- Store file size and other metadata
- Update `ImportItem` structure to hold metadata

**Changes Required**:
```rust
pub struct ImportItem {
    pub path: PathBuf,
    pub sha1_checksum: Option<Sha1Checksum>,
    pub file_size: Option<FileSize>,
    pub release_name: String,
    pub software_title_name: String,
    pub file_set: Option<FileSetImportModel>,
    pub status: ImportItemStatus,
}
```

### ⬜ Step 4: CollectFilesMatchingDatFileStep
**Status**: Stub exists, needs implementation
**Purpose**: Match files to DatGame entries using SHA1 checksums

**Logic**:
1. Build SHA1 → DatGame map from `context.dat_file`
2. For each `ImportItem` with SHA1:
   - Find matching `DatGame` by checking all `DatRom.sha1` values
   - Set `software_title_name` from `DatGame.description` or `DatGame.name`
   - Set `release_name` from `DatRelease[0].name` or generate from game name
   - Group items by `DatGame` for file set creation

**Output**: `ImportItem` instances populated with names

### ⬜ Step 5: CollectExistingFilesStep
**Status**: Stub exists, needs implementation
**Purpose**: Check which files already exist in database

**Logic**:
1. Collect all SHA1 checksums from `ImportItem`s
2. Query `FileInfoRepository` to find existing files
3. Store results in context for use in Step 6

**Changes Required**:
```rust
pub struct MassImportContext {
    // ... existing fields ...
    pub existing_files: Vec<FileInfo>, // Add this
}
```

### ⬜ Step 6: PrepareFileSetImportModelsStep
**Status**: New step needed
**Purpose**: Create `FileSetImportModel` for each DatGame

**Logic**:
1. Group `ImportItem`s by DatGame (using the game name as key)
2. For each game group:
   - Create `FileSetImportModel` with:
     - `file_set_name`: from DatGame.name
     - `selected_files`: SHA1s of all matched ROMs
     - `import_files`: `FileImportSource` for each file
     - `file_type`: Based on file extension or DAT metadata
   - Store model in corresponding `ImportItem.file_set`

**Output**: Each `ImportItem` has populated `file_set` field

### ⬜ Step 7: CreateFileSetsStep
**Status**: Stub exists, needs implementation
**Purpose**: Create file sets in database (imports new files, links existing)

**Logic**:
1. For each unique `FileSetImportModel` (per-game basis):
   - **BEGIN TRANSACTION**
   - Call `FileImportService::create_file_set()`
   - Store resulting `file_set_id` in corresponding `ImportItem`s
   - **COMMIT TRANSACTION**
2. On error: **ROLLBACK**, update `ImportItem.status` to Failed, continue with next

**Changes Required**:
```rust
pub struct ImportItem {
    // ... existing fields ...
    pub file_set_id: Option<i64>, // Add this
}
```

### ⬜ Step 8: CreateSoftwareTitlesStep
**Status**: New step needed
**Purpose**: Create Software Titles for each game

**Logic**:
1. For each unique `software_title_name` (per-game basis):
   - **BEGIN TRANSACTION**
   - Create new Software Title: `SoftwareTitleRepository::create()`
   - Store `software_title_id` in corresponding `ImportItem`s
   - **COMMIT TRANSACTION**
2. On error: **ROLLBACK**, update `ImportItem.status` to Failed, continue with next
3. Note: Duplicates are allowed - deduplication handled separately

**Changes Required**:
```rust
pub struct ImportItem {
    // ... existing fields ...
    pub software_title_id: Option<i64>, // Add this
}
```

### ⬜ Step 9: CreateReleasesStep
**Status**: New step needed
**Purpose**: Link Software Titles to File Sets via Releases

**Logic**:
1. For each `ImportItem` with both `software_title_id` and `file_set_id` (per-game basis):
   - **BEGIN TRANSACTION**
   - Create `Release`: `ReleaseRepository::create()`
   - Link to Software Title: `ReleaseItemRepository::create()`
   - Link Release to File Set (verify relationship method)
   - **COMMIT TRANSACTION**
2. On error: **ROLLBACK**, update `ImportItem.status` to Failed, continue with next
3. Use `release_name` from `ImportItem`

**Note**: Each step processes items individually with its own transaction for maximum resilience

## Key Design Decisions

### 1. Software Title Naming
**Question**: Use `DatGame.name` or `DatGame.description`?
- `name`: Often technical/ID-like (e.g., "[BIOS] ColecoVision (USA, Europe)")
- `description`: Usually human-readable

**Recommendation**: Use `description` as primary, fall back to `name`

### 2. Release Naming
**Options**:
- From `DatRelease.name` + `DatRelease.region` (if available)
- Generate from `DatGame.name` 
- Use file set name

**Recommendation**: Use `DatRelease[0].name` if available, else use `DatGame.name`

### 3. Multiple ROMs per Game
**Scenario**: Some DatGames have multiple DatRoms
**Approach**: All ROMs → single File Set (matches your existing model)

### 4. System Association
**Decision**: User selects system(s) when initiating the import
- System ID(s) passed to the service that triggers the pipeline
- Stored in `MassImportContext.system_ids`
- Applied to all File Sets created during import

### 5. File Type Detection
**Decision**: User provides the file type when initiating import
- File type passed to the service that triggers the pipeline
- Stored in `MassImportContext.file_type`
- Applied to all File Sets created during import

### 6. Duplicate Detection Strategy
**Decision**: Allow duplicates in initial implementation
- Import will create new Software Titles even if same name exists
- Separate merge functionality will be implemented in UI later
- Simplifies initial import logic

### 7. Transaction Handling Strategy
**Decision**: Per-game transactions for resilient partial imports
- Each DatGame processed in its own transaction
- Transaction includes: File Set creation + Software Title creation + Release creation + all linkages
- On error: Rollback that game only, mark `ImportItem` as Failed, continue with next
- Allows partial success and granular error reporting

**Benefits:**
- User can retry only failed items
- Hours of import work not lost due to one bad file
- Clear progress tracking via `ImportItemStatus`
- Aligns with existing error handling design

### 8. Error Handling Strategy
**Approach**: Fail gracefully per-item
- Continue pipeline even if individual items fail
- Track failures in `ImportItem.status`
- Provide summary report at end

## Implementation Checklist

### Context & Models
- [ ] Add `system_ids: Vec<i64>` to `MassImportContext` constructor
- [ ] Add `file_type: FileType` to `MassImportContext` constructor
- [ ] Add `sha1_checksum` and `file_size` to `ImportItem`
- [ ] Add `file_set_id` and `software_title_id` to `ImportItem`
- [ ] Add `existing_files: Vec<FileInfo>` to `MassImportContext`
- [ ] Add helper method to group items by DatGame

### Step Implementations
- [ ] Implement `ReadFileMetadataStep` (refactor `CheckFilesStep`)
- [ ] Implement `CollectFilesMatchingDatFileStep`
- [ ] Implement `CollectExistingFilesStep`
- [ ] Implement `PrepareFileSetImportModelsStep`
- [ ] Implement `CreateFileSetsStep`
- [ ] Implement `CreateSoftwareTitlesStep`
- [ ] Implement `CreateReleasesStep`

### Pipeline Definition
- [ ] Update `pipeline.rs` with all 9 steps in order

### Testing
- [ ] Test with real ColecoVision DAT file
- [ ] Test error cases (missing files, parse errors)
- [ ] Test with games having multiple ROMs
- [ ] Test with existing files (deduplication)

## Open Questions

1. ~~**System linking**: How should systems be determined/selected?~~ ✅ **Resolved**: User selects during import
2. ~~**File type detection**: Should we use file extension, DAT metadata, or user input?~~ ✅ **Resolved**: User provides during import
3. ~~**Release ↔ FileSet relationship**: Verify database schema supports this~~ ✅ **Resolved**: Already supported
4. ~~**Duplicate detection**: What if a Software Title with same name already exists?~~ ✅ **Resolved**: Allow duplicates, merge later via UI
5. ~~**Transaction handling**: Should entire import be atomic, or per-game?~~ ✅ **Resolved**: Per-game transactions for resilience

## All Design Decisions Resolved! ✅

Ready for implementation.

## Next Steps

1. Review this plan and answer open questions
2. Verify database schema supports the data flow
3. Start implementation with Step 3 (ReadFileMetadataStep)
