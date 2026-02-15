# Hybrid Pipeline Pattern Implementation

## Pattern Description

The **Hybrid Pipeline Pattern** is a design pattern that combines sequential data processing with intelligent flow control. It addresses the common challenge of orchestrating complex, multi-step business processes where:

- **Operations must happen in a specific order** - Each step builds on the results of previous steps
- **Steps may be conditional** - Some operations only execute based on accumulated state
- **Early exit is necessary** - The process can stop successfully or abort on errors without completing all steps
- **Context accumulates over time** - Data and state flow through the pipeline, with each step potentially enriching it
- **Reusability is important** - Common steps should be sharable across different pipelines

**Key characteristics:**
- A **Context object** carries all dependencies, configuration, and accumulated state
- **Steps** are independent, testable units that mutate the context
- **Flow control** is explicit through return values (Continue, Skip, Abort)
- **Generic implementation** eliminates boilerplate while allowing type-safe specialization
- **Steps can be generic** and reused across multiple pipeline types

This pattern excels at modeling complex domain workflows where traditional sequential code becomes hard to test, maintain, and reason about.

## Overview

The service layer implements this pattern generically in `service/src/pipeline/` and uses it across **9 different service pipelines** in the application.

The generic `Pipeline<T>` struct provides:
- A shared `execute()` implementation that eliminates code duplication
- Flexible step ordering via configuration
- Consistent error handling and flow control across all pipelines
- Support for conditional execution and early exit
- Reusable generic steps like `ConnectToCloudStep<T>`

## Module Structure

```
service/src/file_set_deletion/
├── mod.rs        - Module declaration
├── context.rs    - DeletionContext and FileDeletionResult types
├── service.rs    - Service implementation  
├── pipeline.rs   - Pipeline configuration (defines step sequence)
└── steps.rs      - All pipeline step implementations

service/src/pipeline/
├── mod.rs                - Module exports
├── generic_pipeline.rs   - Generic Pipeline<T> implementation with execute() logic
├── pipeline_step.rs      - PipelineStep trait and StepAction enum
└── cloud_connection.rs   - CloudConnectionContext trait and generic ConnectToCloudStep<T>
```

## Key Components

### 1. DeletionContext

A context object that flows through the pipeline, accumulating state:

```rust
pub struct DeletionContext {
    pub file_set_id: i64,
    pub repository_manager: Arc<RepositoryManager>,
    pub settings: Arc<Settings>,
    pub fs_ops: Arc<dyn FileSystemOps>,
    
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

### 3. PipelineStep Trait

Each step implements the generic `PipelineStep<T>` trait:

```rust
#[async_trait::async_trait]
pub trait PipelineStep<T>: Send + Sync {
    fn name(&self) -> &'static str;
    
    fn should_execute(&self, context: &T) -> bool {
        true // By default, always execute
    }
    
    async fn execute(&self, context: &mut T) -> StepAction;
}
```

For deletion steps, this becomes `PipelineStep<DeletionContext>`:

```rust
impl PipelineStep<DeletionContext> for ValidateNotInUseStep {
    fn name(&self) -> &'static str { "validate_not_in_use" }
    
    async fn execute(&self, context: &mut DeletionContext) -> StepAction {
        // Implementation
    }
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

### Pipeline Construction and Execution

The generic `Pipeline<T>` struct is defined in `service/src/pipeline/generic_pipeline.rs` with a shared `execute()` implementation. Each specific pipeline (like deletion) just configures the steps:

```rust
// service/src/file_set_deletion/pipeline.rs
impl Pipeline<DeletionContext> {
    pub fn new() -> Self {
        Self::with_steps(vec![
            Box::new(ValidateNotInUseStep),
            Box::new(FetchFileInfosStep),
            Box::new(DeleteFileSetStep),
            Box::new(FilterDeletableFilesStep),
            Box::new(MarkForCloudDeletionStep),
            Box::new(DeleteLocalFilesStep),
        ])
    }
}

// service/src/file_set_deletion/service.rs
pub async fn delete_file_set(&self, file_set_id: i64) -> Result<(), Error> {
    let mut context = DeletionContext { /* ... */ };
    let pipeline = Pipeline::<DeletionContext<F>>::new();
    pipeline.execute(&mut context).await?;  // execute() is in base Pipeline<T>
    
    // Access detailed results
    let successful = context.deletion_results.values()
        .filter(|r| r.file_deletion_success && r.was_deleted_from_db)
        .count();
}
```

**Advantages:**

1. **Modular Structure** - Separated into context, pipeline configuration, steps, and service
2. **Testability** - Each step can be tested in isolation
3. **Clarity** - Clear sequence of operations with descriptive names
4. **Flexibility** - Steps can be conditional (`should_execute`)
5. **Observability** - Detailed tracking via `FileDeletionResult`
6. **Error Handling** - Centralized in base `Pipeline<T>::execute()`, errors are `StepAction::Abort`
7. **Extensibility** - Easy to add/remove/reorder steps
8. **Debugging** - Step names in logs make debugging easier
9. **Reusability** - Generic `Pipeline<T>` eliminates code duplication across different pipelines

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
impl<F: FileSystemOps> PipelineStep<DeletionContext<F>> for MarkForCloudDeletionStep {
    fn should_execute(&self, context: &DeletionContext<F>) -> bool {
        // Skip if no deletable files
        context.deletion_results.values().any(|r| r.is_deletable)
    }
    
    async fn execute(&self, context: &mut DeletionContext<F>) -> StepAction {
        // Only runs if should_execute returns true
        StepAction::Continue
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
The base `Pipeline<T>::execute()` method can be enhanced to add metrics:

```rust
// In service/src/pipeline/generic_pipeline.rs
pub async fn execute(&self, context: &mut T) -> Result<(), Error> {
    for step in &self.steps {
        let start = Instant::now();
        
        if step.should_execute(context) {
            log::info!("Executing step: {}", step.name());
            match step.execute(context).await {
                StepAction::Continue => {
                    log::info!("Step {} completed in {:?}", step.name(), start.elapsed());
                    continue;
                }
                // ... handle other actions
            }
        }
    }
}
```

### Dry Run Mode
```rust
pub struct DeletionContext {
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

impl PipelineStep<DeletionContext> for TransactionalDeletionStep {
    fn name(&self) -> &'static str { "transactional_deletion" }
    
    async fn execute(&self, context: &mut DeletionContext) -> StepAction {
        // Begin transaction
        // Execute sub-steps
        // Commit or rollback
        StepAction::Continue
    }
}
```

## Summary

The Hybrid Pipeline Pattern provides:
- **Pipeline's** sequential flow and data transformation
- **Chain of Responsibility's** conditional execution and early exit
- **Generic implementation** that eliminates code duplication
- Clear separation of concerns
- Excellent testability
- Easy extension and maintenance

It's particularly well-suited for complex business processes with multiple sequential steps that need to be testable, observable, and maintainable.

## All Pipelines Using This Pattern

The application has **9 pipeline implementations** following this pattern:

### 1. FileSetDeletionService
```rust
// service/src/file_set_deletion/pipeline.rs
impl Pipeline<DeletionContext> {
    pub fn new() -> Self {
        Self::with_steps(vec![
            Box::new(ValidateNotInUseStep),
            Box::new(FetchFileInfosStep),
            Box::new(DeleteFileSetStep),
            Box::new(FilterDeletableFilesStep),
            Box::new(MarkForCloudDeletionStep),
            Box::new(DeleteLocalFilesStep),
        ])
    }
}
```

### 2. CloudStorageSyncService
```rust
// service/src/cloud_sync/pipeline.rs
impl Pipeline<SyncContext> {
    pub fn new() -> Self {
        Self::with_steps(vec![
            Box::new(PrepareFilesForUploadStep),
            Box::new(GetSyncFileCountsStep),
            Box::new(ConnectToCloudStep::<SyncContext>::new()),
            Box::new(UploadPendingFilesStep),
            Box::new(DeleteMarkedFilesStep),
        ])
    }
}
```

### 3. FileSetDownloadService
```rust
// service/src/file_set_download/pipeline.rs
impl Pipeline<DownloadContext> {
    pub fn new() -> Self {
        Self::with_steps(vec![
            Box::new(FetchFileSetStep),
            Box::new(FetchFileSetFileInfoStep),
            Box::new(PrepareFileForDownloadStep),
            Box::new(ConnectToCloudStep::<DownloadContext>::new()),
            Box::new(DownloadFilesStep),
            Box::new(ExportFilesStep),
        ])
    }
}
```

### 4. ExternalExecutableRunnerService
```rust
// service/src/external_executable_runner/pipeline.rs
impl Pipeline<ExternalExecutableRunnerContext> {
    pub fn new() -> Self {
        Self::with_steps(vec![
            Box::new(PrepareFilesStep),
            Box::new(StartExecutableStep),
            Box::new(CleanupFilesStep),
        ])
    }
}
```

### 5. FileTypeMigrationService
```rust
// service/src/file_type_migration/pipeline.rs
impl Pipeline<FileTypeMigrationContext> {
    pub fn new() -> Self {
        Self::with_steps(vec![
            Box::new(CollectFileSetsStep),
            Box::new(CollectCloudFileSetsStep),
            Box::new(MoveLocalFilesStep),
            Box::new(ConnectToCloudStep::<FileTypeMigrationContext>::new()),
            Box::new(MoveCloudFilesStep),
            Box::new(UpdateFileInfosStep),
            Box::new(UpdateFileSetsStep),
            Box::new(AddItemsToFileSetsStep),
        ])
    }
}
```

### 6. PrepareFileImportService
```rust
// service/src/file_import/prepare/pipeline.rs
impl Pipeline<PrepareFileImportContext> {
    pub fn new() -> Self {
        Self::with_steps(vec![
            Box::new(CollectFileMetadataStep),
            Box::new(CollectFileInfoStep::<PrepareFileImportContext>::new()),
        ])
    }
}
```

### 7. UpdateFileSetService
```rust
// service/src/file_import/update_file_set/pipeline.rs
impl Pipeline<UpdateFileSetContext> {
    pub fn new() -> Self {
        Self::with_steps(vec![
            // Preparation steps
            Box::new(FetchFileSetStep),
            Box::new(FetchFilesInFileSetStep),
            // File deletion steps
            Box::new(CollectDeletionCandidatesStep),
            Box::new(FilterDeletableFilesStep::<UpdateFileSetContext>::new()),
            Box::new(DeleteLocalFilesStep::<UpdateFileSetContext>::new()),
            Box::new(MarkForCloudDeletionStep::<UpdateFileSetContext>::new()),
            Box::new(UnlinkFilesFromFileSetStep),
            Box::new(DeleteFileInfosStep::<UpdateFileSetContext>::new()),
            // Import new files
            Box::new(CheckExistingFilesStep::<UpdateFileSetContext>::new()),
            Box::new(ImportFilesStep::<UpdateFileSetContext>::new()),
            Box::new(UpdateFileInfoToDatabaseStep),
            Box::new(UpdateFileSetFilesStep),
            Box::new(UpdateFileSetStep),
            Box::new(MarkNewFilesForCloudSyncStep),
        ])
    }
}
```

### 8. AddFileSetService
```rust
// service/src/file_import/add_file_set/pipeline.rs
impl Pipeline<AddFileSetContext> {
    pub fn new() -> Self {
        Self::with_steps(vec![
            Box::new(CheckExistingFilesStep::<AddFileSetContext>::new()),
            Box::new(ImportFilesStep::<AddFileSetContext>::new()),
            Box::new(UpdateDatabaseStep),
        ])
    }
}
```

## Generic and Reusable Steps

### Generic Cloud Connection Step

`ConnectToCloudStep<T>` is a **generic step** defined in `service/src/pipeline/cloud_connection.rs` that can be used by any pipeline needing cloud connectivity. It works with any context that implements the `CloudConnectionContext` trait:

```rust
pub trait CloudConnectionContext {
    fn settings(&self) -> &Arc<Settings>;
    fn settings_service(&self) -> &Arc<SettingsService>;
    fn cloud_ops_mut(&mut self) -> &mut Option<Arc<dyn CloudStorageOps>>;
    fn should_connect(&self) -> bool { true }
}
```

**Pipelines using this step:**
- `CloudStorageSyncService` - `ConnectToCloudStep::<SyncContext>`
- `FileSetDownloadService` - `ConnectToCloudStep::<DownloadContext>`
- `FileTypeMigrationService` - `ConnectToCloudStep::<FileTypeMigrationContext>`

### Generic File Import Steps

Several steps in the `file_import/common_steps/` directory are generic and reused across multiple import pipelines:

- **`CollectFileInfoStep<T>`** - Used by `PrepareFileImportContext`
- **`CheckExistingFilesStep<T>`** - Used by `AddFileSetContext` and `UpdateFileSetContext`
- **`ImportFilesStep<T>`** - Used by `AddFileSetContext` and `UpdateFileSetContext`

### Generic File Deletion Steps

File deletion logic is extracted into generic steps in `file_import/common_steps/file_deletion_steps/`:

- **`FilterDeletableFilesStep<T>`** - Identifies files safe to delete
- **`DeleteLocalFilesStep<T>`** - Deletes files from local storage
- **`MarkForCloudDeletionStep<T>`** - Marks synced files for cloud deletion
- **`DeleteFileInfosStep<T>`** - Removes file_info records from database

These are reused by both `FileSetDeletionService` and `UpdateFileSetService`.

## Pipeline Pattern Principles

Each pipeline follows these principles:

1. **Defines its own context type** (e.g., `SyncContext`, `DownloadContext`, `DeletionContext`)
   - Context holds all dependencies (repository, settings, ops traits)
   - Context accumulates state as pipeline executes
   - Context is passed mutably through each step

2. **Implements steps via `PipelineStep<ContextType>` trait**
   - Each step is a separate struct implementing the trait
   - Steps are stateless - all state lives in the context
   - Steps can be tested in isolation

3. **Configures step sequence in `Pipeline<ContextType>::new()`**
   - Step order is explicitly defined
   - Easy to reorder, add, or remove steps
   - Clear documentation of the process flow

4. **Uses the shared `Pipeline<T>::execute()` for execution logic**
   - No duplication of pipeline execution code
   - Consistent error handling across all pipelines
   - Built-in support for conditional execution via `should_execute()`

5. **Can optionally implement `CloudConnectionContext`**
   - To use the generic `ConnectToCloudStep<T>`
   - Or implement other shared traits for reusable generic steps

6. **Uses trait objects for dependency injection**
   - All external dependencies injected as `Arc<dyn Trait>`
   - Enables easy testing with mocks
   - Runtime flexibility without generic complexity

### Dependency Injection with Trait Objects

All pipelines use trait objects (`dyn Trait`) for dependency injection instead of generics. This approach provides:
- **Cleaner code**: No generic parameters propagating through the entire codebase
- **Flexible testing**: Easy to swap between real implementations and mocks
- **Runtime flexibility**: Different implementations can be injected at runtime

```rust
pub struct DownloadContext {
    pub fs_ops: Arc<dyn FileSystemOps>,
    pub export_ops: Arc<dyn FileExportOps>,
    pub thumbnail_generator: Arc<dyn ThumbnailOps>,
    pub cloud_ops: Option<Arc<dyn CloudStorageOps>>,
    // ...
}
```

While generics offer zero-cost abstraction, trait objects are preferred here because:
- The virtual dispatch overhead is negligible compared to I/O operations
- The code is significantly more maintainable and readable
- Testing is straightforward with mock implementations

# Mapping to Agent concepts to Pipeline:


Agent Concept	Your Pipeline Concept
Agent state / memory	Context
Tools	Context-held services (DB repos, LLM client)
Reasoning	Steps that call the LLM
Planning	Pipeline configuration (step order)
Observation	Context mutations
Action	DB writes / file writes
Reflection	Optional post-step validation
Stop condition	Skip / Abort
