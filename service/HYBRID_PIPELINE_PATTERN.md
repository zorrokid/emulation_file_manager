# Hybrid Pipeline Pattern Implementation

## Overview

The `FileSetDeletionService` uses a **Hybrid Pipeline Pattern** that combines the best of both Pipeline and Chain of Responsibility patterns. The service has been modularized into separate files for better organization and testability.

## Module Structure

```
service/src/file_set_deletion/
├── mod.rs        - Module declaration
├── context.rs    - DeletionContext and FileDeletionResult types
├── service.rs    - Service implementation  
├── executor.rs   - Pipeline executor
└── steps.rs      - All pipeline step implementations
```

## Key Components

### 1. DeletionContext

A context object that flows through the pipeline, accumulating state:

```rust
pub struct DeletionContext<F: FileSystemOps> {
    pub file_set_id: i64,
    pub repository_manager: Arc<RepositoryManager>,
    pub settings: Arc<Settings>,
    pub fs_ops: Arc<F>,
    
    // Accumulated state (keyed by SHA1 checksum)
    pub deletion_results: HashMap<Vec<u8>, FileDeletionResult>,
}

pub struct FileDeletionResult {
    pub file_info: FileInfo,
    pub file_path: Option<String>,
    pub file_deletion_success: bool,
    pub error_messages: Vec<String>,
    pub is_deletable: bool,
    pub was_deleted_from_db: bool,
    pub cloud_sync_marked: bool,
}
```

### 2. StepAction

Each step returns an action that controls pipeline flow:

```rust
pub enum StepAction {
    Continue,      // Proceed to next step
    Skip,          // Skip remaining steps (successful early exit)
    Abort(Error),  // Stop with error
}
```

### 3. DeletionStep Trait

Each step implements this trait:

```rust
#[async_trait::async_trait]
pub trait DeletionStep<F: FileSystemOps>: Send + Sync {
    fn name(&self) -> &'static str;
    
    fn should_execute(&self, context: &DeletionContext<F>) -> bool {
        true // By default, always execute
    }
    
    async fn execute(&self, context: &mut DeletionContext<F>) -> StepAction;
}
```

## Pipeline Steps

The deletion process has 6 clear steps:

1. **ValidateNotInUseStep** - Check if file set is in use by releases
   - Returns `Abort` if in use
   
2. **FetchFileInfosStep** - Fetch all file infos for the file set
   - Stores results in `context.deletion_results` (HashMap keyed by checksum)
   - Creates `FileDeletionResult` for each file with initial state
   
3. **DeleteFileSetStep** - Delete the file_set record from database
   - Removes file_set and cascades to file_set_file_info entries
   - Executed before filtering to handle foreign keys properly
   
4. **FilterDeletableFilesStep** - Identify files safe to delete
   - Checks if each file is used in other file sets
   - Marks files as `is_deletable` if they're only in this file set
   
5. **MarkForCloudDeletionStep** - Mark synced files for cloud deletion
   - Only executes if there are deletable files
   - Updates sync log with `DeletionPending` status
   
6. **DeleteLocalFilesStep** - Delete files from local storage
   - Only processes files marked as deletable
   - Continues on individual file failures
   - Removes file_info from database on successful deletion
   - Tracks detailed results per file

### Modular Pipeline 
```rust
// service/src/file_set_deletion/service.rs
pub async fn delete_file_set(&self, file_set_id: i64) -> Result<(), Error> {
    let mut context = DeletionContext { /* ... */ };
    let pipeline = DeletionPipeline::new();
    pipeline.execute(&mut context).await?;
    
    // Access detailed results
    let successful = context.deletion_results.values()
        .filter(|r| r.file_deletion_success && r.was_deleted_from_db)
        .count();
}
```

**Advantages:**

1. **Modular Structure** - Separated into context, executor, steps, and service
2. **Testability** - Each step can be tested in isolation
3. **Clarity** - Clear sequence of operations with descriptive names
4. **Flexibility** - Steps can be conditional (`should_execute`)
5. **Observability** - Detailed tracking via `FileDeletionResult`
6. **Error Handling** - Centralized in executor, errors are `StepAction::Abort`
7. **Extensibility** - Easy to add/remove/reorder steps
8. **Debugging** - Step names in logs make debugging easier

## Example: Testing Individual Steps

```rust
#[async_std::test]
async fn test_validate_not_in_use_step() {
    let mut context = DeletionContext { /* ... */ };
    let step = ValidateNotInUseStep;
    
    // Mock: file set is in use
    // ...
    
    let result = step.execute(&mut context).await.unwrap();
    assert!(matches!(result, StepAction::Abort(_)));
}

#[async_std::test]
async fn test_filter_deletable_files_step() {
    let mut context = DeletionContext {
        files_to_delete: vec![file1, file2, file3],
        // ...
    };
    
    let step = FilterDeletableFilesStep;
    step.execute(&mut context).await.unwrap();
    
    // Verify only files used in this file set remain
    assert_eq!(context.files_to_delete.len(), 1);
}
```

## Conditional Execution

Steps can decide whether to run based on context:

```rust
impl<F: FileSystemOps> DeletionStep<F> for MarkForCloudDeletionStep {
    fn should_execute(&self, context: &DeletionContext<F>) -> bool {
        !context.files_to_delete.is_empty()  // Skip if no files
    }
    
    async fn execute(&self, context: &mut DeletionContext<F>) -> Result<StepAction, Error> {
        // Only runs if should_execute returns true
    }
}
```

## Flow Control

### Normal Flow
```
ValidateNotInUse → FetchFileInfos → FilterDeletable → MarkForCloud → DeleteLocal → DeleteFileSet
     ↓                  ↓                  ↓                ↓             ↓             ↓
  Continue           Continue           Continue         Continue      Continue      Continue
```

### Early Abort (File Set In Use)
```
ValidateNotInUse → STOP
     ↓
  Abort(Error)
```

### Skip Empty File Set
```
ValidateNotInUse → FetchFileInfos → FilterDeletable → MarkForCloud → STOP
     ↓                  ↓                  ↓                ↓
  Continue           Continue           Continue         Skip
                                     (0 files to delete)
```

## Future Enhancements

The pipeline pattern makes it easy to add:

### Logging/Metrics
```rust
async fn execute(&self, context: &mut DeletionContext<F>) -> Result<(), Error> {
    for step in &self.steps {
        let start = Instant::now();
        
        if step.should_execute(context) {
            log::info!("Executing step: {}", step.name());
            step.execute(context).await?;
            log::info!("Step {} completed in {:?}", step.name(), start.elapsed());
        }
    }
}
```

### Dry Run Mode
```rust
pub struct DeletionContext<F: FileSystemOps> {
    pub dry_run: bool,  // Add this field
    // ...
}

// Steps check dry_run before making changes
if !context.dry_run {
    context.fs_ops.remove_file(&file_path)?;
}
```

### Transaction Support
```rust
struct TransactionalDeletionStep;

impl<F: FileSystemOps> DeletionStep<F> for TransactionalDeletionStep {
    async fn execute(&self, context: &mut DeletionContext<F>) -> Result<StepAction, Error> {
        // Begin transaction
        // Execute sub-steps
        // Commit or rollback
    }
}
```

## Summary

The Hybrid Pipeline Pattern provides:
- **Pipeline's** sequential flow and data transformation
- **Chain of Responsibility's** conditional execution and early exit
- Clear separation of concerns
- Excellent testability
- Easy extension and maintenance

It's particularly well-suited for complex business processes with multiple sequential steps that need to be testable, observable, and maintainable.
