# File Type Migration Specification

**Feature**: Consolidate specific scan types into generic types  
**Created**: 2026-01-06  
**Related**: `001-release-items.md`

## Overview

Simplify the FileType enum by replacing specific scan types with generic ones. Use ItemType to provide context about what's being scanned. This makes the system more flexible and reduces FileType proliferation.

**Why automated migration is required**: Files are deduplicated and shared across file_sets. We cannot allow individual file_set type changes without coordinating file moves atomically for all file_sets that share the same files.

## Goals

1. **Simplify FileTypes**: Replace multiple specific types with generic `Scan` and `Document`
2. **Consolidate Screenshots**: Use single `Screenshot` type for all screenshot variants
3. **Use Items for context**: ItemType indicates what the scan represents
4. **Atomic migration**: Update all file_sets and move files in coordinated manner
5. **Zero data loss**: All files remain accessible after migration

## Current State

**FileTypes to consolidate**:

*Scan types:*
- ManualScan (10), MediaScan (11), PackageScan (12), InlayScan (13)

*Document types:*
- BoxScan (14) → Document (PDFs of box scans)
- Manual (5) → Document (PDF documents)

*Screenshot types:*
- LoadingScreen (8), TitleScreen (9) → Screenshot (4)

**Problem**: File deduplication means same file can belong to multiple file_sets. Cannot change types independently.

## Migration Mapping

| Old FileType | New FileType | Create Item? | ItemType | Directory Change |
|--------------|--------------|--------------|----------|------------------|
| ManualScan | Scan | Yes | Manual | manual_scan/ → scan/ |
| InlayScan | Scan | Yes | InlayCard | inlay_scan/ → scan/ |
| MediaScan | Scan | Yes | Disk/Tape/Cartridge* | media_scan/ → scan/ |
| PackageScan | Scan | Yes | Box | package_scan/ → scan/ |
| BoxScan | Document | Yes | Box | box_scan/ → document/ |
| Manual | Document | Yes | Manual | manual/ → document/ |
| LoadingScreen | Screenshot | No | - | loading_screen/ → screenshot/ |
| TitleScreen | Screenshot | No | - | title_screen/ → screenshot/ |

\* Requires heuristic or user input

## Architecture: Pipeline Pattern

Following the existing pipeline pattern used in file_import:

```
service/src/file_type_migration/
  ├── mod.rs
  ├── service.rs        # Entry point
  ├── context.rs        # Migration context
  ├── pipeline.rs       # Pipeline definition
  └── steps/
      ├── mod.rs
      ├── analyze.rs    # Find file_sets to migrate
      ├── plan.rs       # Plan migrations and file moves
      ├── database.rs   # Update database
      ├── local.rs      # Move local files
      └── s3.rs         # Move S3 files
```

### Service (Entry Point)

```rust
pub struct FileTypeMigrationService {
    repo_manager: Arc<RepositoryManager>,
    file_system_ops: Arc<dyn FileSystemOps>,
    settings: Arc<Settings>,
    s3_ops: Option<Arc<dyn S3Ops>>,
}

impl FileTypeMigrationService {
    pub async fn migrate_file_types(&self, dry_run: bool) -> Result<MigrationReport> {
        let mut context = MigrationContext::new(
            self.repo_manager.clone(),
            self.file_system_ops.clone(),
            self.settings.clone(),
            self.s3_ops.clone(),
            dry_run,
        );
        
        let pipeline = Pipeline::<MigrationContext>::new();
        pipeline.execute(&mut context).await?;
        
        Ok(context.report)
    }
}
```

### Context

```rust
pub struct MigrationContext {
    // Dependencies
    pub repo_manager: Arc<RepositoryManager>,
    pub file_system_ops: Arc<dyn FileSystemOps>,
    pub settings: Arc<Settings>,
    pub s3_ops: Option<Arc<dyn S3Ops>>,
    pub dry_run: bool,
    
    // Populated by AnalyzeStep
    pub file_sets_to_migrate: Vec<FileSetMigrationPlan>,
    
    // Populated by PlanStep
    pub items_to_create: Vec<ItemCreationPlan>,
    pub files_to_move: Vec<FileMoveInfo>,
    
    // Updated throughout
    pub report: MigrationReport,
}

pub struct FileSetMigrationPlan {
    pub file_set_id: i64,
    pub release_id: i64,
    pub old_type: FileType,
    pub new_type: FileType,
    pub target_item_type: Option<ItemType>,
}

pub struct ItemCreationPlan {
    pub release_id: i64,
    pub item_type: ItemType,
}

pub struct FileMoveInfo {
    pub file_info_id: i64,
    pub old_local_path: PathBuf,
    pub new_local_path: PathBuf,
    pub old_s3_key: Option<String>,
    pub new_s3_key: Option<String>,
}

pub struct MigrationReport {
    pub dry_run: bool,
    pub file_sets_migrated: usize,
    pub items_created: usize,
    pub files_moved_local: usize,
    pub files_moved_s3: usize,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}
```

### Pipeline Definition

```rust
impl Pipeline<MigrationContext> {
    pub fn new() -> Self {
        Self::with_steps(vec![
            Box::new(AnalyzeFileTypesStep),
            Box::new(PlanMigrationStep),
            Box::new(UpdateDatabaseStep),
            Box::new(MoveFilesLocalStep),
            Box::new(MoveFilesS3Step),
        ])
    }
}
```

## Pipeline Steps

### Step 1: AnalyzeFileTypesStep

**Purpose**: Find all file_sets with deprecated FileTypes

```rust
pub struct AnalyzeFileTypesStep;

impl PipelineStep<MigrationContext> for AnalyzeFileTypesStep {
    async fn execute(&self, context: &mut MigrationContext) -> StepAction {
        // 1. Query all file_sets
        // 2. Filter those with deprecated types
        // 3. For each, determine new type and target item type
        // 4. Populate context.file_sets_to_migrate
        // 5. If dry_run, log what would be done
    }
}
```

**Output**: `context.file_sets_to_migrate` populated

### Step 2: PlanMigrationStep

**Purpose**: Plan item creation and file moves

```rust
pub struct PlanMigrationStep;

impl PipelineStep<MigrationContext> for PlanMigrationStep {
    async fn execute(&self, context: &mut MigrationContext) -> StepAction {
        // For each file_set to migrate:
        // 1. Check if target item exists for release
        // 2. If not, add to items_to_create
        // 3. Get all files in file_set
        // 4. For each file, calculate old and new paths
        // 5. Populate context.files_to_move
        // 6. Handle MediaScan ambiguity (heuristic or prompt)
        // 7. If dry_run, log detailed plan
    }
}
```

**Output**: 
- `context.items_to_create` populated
- `context.files_to_move` populated

### Step 3: UpdateDatabaseStep

**Purpose**: Update database in single transaction

```rust
pub struct UpdateDatabaseStep;

impl PipelineStep<MigrationContext> for UpdateDatabaseStep {
    async fn execute(&self, context: &mut MigrationContext) -> StepAction {
        if context.dry_run {
            return StepAction::Continue; // Skip in dry run
        }
        
        // BEGIN TRANSACTION
        
        // 1. Create all items from items_to_create
        // 2. For each file_set:
        //    - Link to item via file_set_item
        //    - UPDATE file_set SET file_type = new_type
        // 3. Record item_ids for later
        
        // COMMIT (or ROLLBACK on error)
    }
}
```

**Changes**: Database updated atomically

### Step 4: MoveFilesLocalStep

**Purpose**: Move files in local filesystem

```rust
pub struct MoveFilesLocalStep;

impl PipelineStep<MigrationContext> for MoveFilesLocalStep {
    async fn execute(&self, context: &mut MigrationContext) -> StepAction {
        if context.dry_run {
            return StepAction::Continue;
        }
        
        // Group files by target directory
        // For each directory:
        //   1. Ensure target directory exists
        //   2. For each file:
        //      - Copy file to new location
        //      - Verify copy succeeded
        //      - Delete original
        //      - Update counter
        //      - Log on error, continue with next
    }
}
```

**Changes**: Files moved in filesystem

### Step 5: MoveFilesS3Step

**Purpose**: Move files in S3 using server-side copy

```rust
pub struct MoveFilesS3Step;

impl PipelineStep<MigrationContext> for MoveFilesS3Step {
    async fn execute(&self, context: &mut MigrationContext) -> StepAction {
        if context.dry_run || context.s3_ops.is_none() {
            return StepAction::Continue;
        }
        
        // For each file with S3 key:
        //   1. CopyObject from old key to new key (server-side)
        //   2. HeadObject to verify new exists
        //   3. DeleteObject old key
        //   4. Update counter
        //   5. Log on error, continue with next
    }
}
```

**S3 Move Implementation**:
```rust
async fn move_s3_object(
    s3_ops: &dyn S3Ops,
    bucket: &str,
    old_key: &str,
    new_key: &str,
) -> Result<()> {
    // Server-side copy (no bandwidth, fast)
    s3_ops.copy_object(bucket, old_key, new_key).await?;
    
    // Verify
    s3_ops.head_object(bucket, new_key).await?;
    
    // Delete old
    s3_ops.delete_object(bucket, old_key).await?;
    
    Ok(())
}
```

## MediaScan Heuristic

```rust
fn determine_media_item_type(
    file_set: &FileSet,
    release: &Release,
) -> Result<ItemType> {
    // 1. Check file_set name for keywords
    let name_lower = file_set.name.to_lowercase();
    if name_lower.contains("disk") || name_lower.contains("floppy") {
        return Ok(ItemType::Disk);
    }
    if name_lower.contains("tape") || name_lower.contains("cassette") {
        return Ok(ItemType::Tape);
    }
    if name_lower.contains("cartridge") || name_lower.contains("cart") {
        return Ok(ItemType::Cartridge);
    }
    
    // 2. Check system type
    // (Requires system info - may not be available)
    // C64, Amiga → likely Disk
    // NES, SNES, Genesis → likely Cartridge
    // ZX Spectrum, C64 → could be Tape
    
    // 3. Default or prompt
    Ok(ItemType::Disk) // Default, can be corrected later in UI
}
```

## Error Handling

**Step failures**:
- AnalyzeStep / PlanStep: Safe to fail, no changes made
- UpdateDatabaseStep: Transaction rollback on failure
- MoveFilesLocalStep: Log errors, continue with remaining files
- MoveFilesS3Step: Log errors, continue with remaining files

**Partial failure recovery**:
- Database updated but file move failed → Files still in old location but DB says new
  - Solution: Manual file move or rollback DB
- Local move succeeded but S3 failed → Inconsistent state
  - Solution: Re-run S3 step only, or manual S3 fix

**Best practice**: Always run dry-run first, verify plan carefully

## Dry Run Output Example

```
File Type Migration - DRY RUN
==============================

File Sets to Migrate: 45

FileType Changes:
  ManualScan → Scan (12 file_sets)
  BoxScan → Document (8 file_sets)
  Manual → Document (15 file_sets)
  MediaScan → Scan (7 file_sets)
  LoadingScreen → Screenshot (2 file_sets)
  TitleScreen → Screenshot (1 file_set)

Items to Create: 23
  Manual: 8 items
  Box: 6 items
  Disk: 5 items
  Tape: 2 items
  Cartridge: 2 items

Files to Move: 127
  Local filesystem: 127 files
  S3 storage: 127 objects

Directory Changes:
  manual_scan/ → scan/ (34 files)
  box_scan/ → document/ (18 files)
  manual/ → document/ (52 files)
  media_scan/ → scan/ (19 files)
  loading_screen/ → screenshot/ (2 files)
  title_screen/ → screenshot/ (2 files)

Estimated Time: 5-10 minutes
Estimated S3 Operations: 254 (127 copy + 127 delete)

Warnings:
  - MediaScan file_set "Game Photos" mapped to Disk (verify manually)

Ready to proceed? Review carefully before running actual migration.
```

## Usage

```rust
// In service layer or CLI
let migration_service = FileTypeMigrationService::new(
    repo_manager,
    file_system_ops,
    settings,
    s3_ops,
);

// 1. Dry run
let report = migration_service.migrate_file_types(true).await?;
println!("Dry run complete: {:?}", report);

// 2. Review, then actual migration
let report = migration_service.migrate_file_types(false).await?;
println!("Migration complete: {:?}", report);
```

## Testing Strategy

1. **Unit tests**: FileType mapping logic, heuristics
2. **Integration tests**: Pipeline with test database and mock filesystem
3. **Manual testing**: Small test collection, verify all steps

## Success Criteria

- ✅ All deprecated FileTypes updated
- ✅ Items created and linked
- ✅ Local files moved to new directories
- ✅ S3 objects moved to new keys
- ✅ All files accessible after migration
- ✅ No data loss
- ✅ Dry run accurately predicts changes

## Future: Cleanup

After migration verified (weeks later):
1. Remove deprecated FileType variants from enum
2. Delete empty old directories
3. Update documentation
