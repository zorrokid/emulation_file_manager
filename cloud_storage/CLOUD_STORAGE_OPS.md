# CloudStorageOps Trait - Usage Guide

## Overview

The `CloudStorageOps` trait provides an abstraction over cloud storage operations, enabling easy testing without requiring real S3 credentials or network access.

## Implementations

### Production: `S3CloudStorage`

Uses real S3-compatible storage (AWS S3, Backblaze B2, MinIO, etc.)

```rust
use cloud_storage::{CloudStorageOps, S3CloudStorage};

// Connect to cloud storage
let cloud_ops = S3CloudStorage::connect(
    "s3.eu-central-003.backblazeb2.com",  // endpoint
    "eu-central-003",                       // region  
    "my-bucket-name",                       // bucket
)
.await?;

// Upload a file
cloud_ops
    .upload_file(
        Path::new("/local/path/file.zst"),
        "rom/game.zst",  // cloud key
        Some(&progress_tx),
    )
    .await?;

// Delete a file
cloud_ops.delete_file("rom/game.zst").await?;

// Check if file exists
if cloud_ops.file_exists("rom/game.zst").await? {
    println!("File exists in cloud");
}
```

### Testing: `MockCloudStorage`

Simulates cloud storage operations without network or credentials.

```rust
use cloud_storage::mock::MockCloudStorage;

// Create mock
let mock = Arc::new(MockCloudStorage::new());

// Simulate existing files
mock.add_file("rom/existing.zst", b"content".to_vec());

// Upload a file
mock.upload_file(
    Path::new("/test/file.zst"),
    "rom/game.zst",
    None,
)
.await?;

// Verify upload
assert!(mock.was_uploaded("rom/game.zst"));
assert_eq!(mock.uploaded_count(), 2);  // existing + new

// Delete a file
mock.delete_file("rom/game.zst").await?;

// Verify deletion
assert!(mock.was_deleted("rom/game.zst"));
assert!(!mock.was_uploaded("rom/game.zst"));
```

## Mock Features

### Setup Test Data

```rust
let mock = MockCloudStorage::new();

// Add file with specific content
mock.add_file("rom/game.zst", vec![1, 2, 3, 4]);

// Add file with dummy content
mock.add_file_dummy("rom/another.zst");
```

### Simulate Failures

```rust
// Make specific upload fail
mock.fail_upload_for("rom/game.zst");

let result = mock.upload_file(Path::new("/test"), "rom/game.zst", None).await;
assert!(result.is_err());

// Make specific deletion fail
mock.fail_delete_for("rom/protected.zst");

let result = mock.delete_file("rom/protected.zst").await;
assert!(result.is_err());
```

### Configure Multipart Behavior

```rust
// Simulate more parts in multipart upload
mock.set_part_count(10);  // Default is 3

let (tx, rx) = async_std::channel::unbounded();
mock.upload_file(Path::new("/test"), "rom/game.zst", Some(&tx)).await?;

// Count part events
let mut parts = 0;
while let Ok(SyncEvent::PartUploaded { .. }) = rx.try_recv() {
    parts += 1;
}
assert_eq!(parts, 10);
```

### Verify Operations

```rust
// Check what was uploaded
assert!(mock.was_uploaded("rom/game1.zst"));
assert!(!mock.was_uploaded("rom/game2.zst"));

// Get list of all uploaded files
let keys = mock.get_uploaded_keys();
assert_eq!(keys.len(), 1);

// Get uploaded content
let content = mock.get_uploaded_content("rom/game1.zst");
assert!(content.is_some());

// Check deletions
assert!(mock.was_deleted("rom/old-game.zst"));
let deleted = mock.get_deleted_keys();
assert_eq!(deleted.len(), 1);

// Get counts
assert_eq!(mock.uploaded_count(), 1);
assert_eq!(mock.deleted_count(), 1);
```

### Clean State Between Tests

```rust
#[async_std::test]
async fn test_1() {
    let mock = MockCloudStorage::new();
    // ... test code ...
    mock.clear();  // Reset for next test
}
```

## Usage in Services

### Generic Service with Default

```rust
pub struct CloudSyncService<C: CloudStorageOps = S3CloudStorage> {
    cloud_ops: Arc<C>,
    // ... other fields
}

impl CloudSyncService<S3CloudStorage> {
    pub fn new(...) -> Self {
        // Will be initialized lazily when needed
    }
}

impl<C: CloudStorageOps> CloudSyncService<C> {
    pub fn new_with_cloud_ops(cloud_ops: Arc<C>) -> Self {
        Self { cloud_ops }
    }
}
```

### Testing the Service

```rust
#[async_std::test]
async fn test_upload_step() {
    let mock_cloud = Arc::new(MockCloudStorage::new());
    
    let service = CloudSyncService::new_with_cloud_ops(mock_cloud.clone());
    
    service.sync_files().await?;
    
    // Verify
    assert!(mock_cloud.was_uploaded("rom/game.zst"));
}

#[async_std::test]
async fn test_upload_failure_handling() {
    let mock_cloud = Arc::new(MockCloudStorage::new());
    mock_cloud.fail_upload_for("rom/game.zst");
    
    let service = CloudSyncService::new_with_cloud_ops(mock_cloud.clone());
    
    let result = service.sync_files().await;
    
    // Service should handle the failure gracefully
    assert!(!mock_cloud.was_uploaded("rom/game.zst"));
}
```

## Complete Test Example

```rust
#[async_std::test]
async fn test_sync_with_uploads_and_deletions() {
    // Setup
    let mock_cloud = Arc::new(MockCloudStorage::new());
    
    // Simulate existing files in cloud
    mock_cloud.add_file_dummy("rom/old-version.zst");
    
    // Simulate failure for one file
    mock_cloud.fail_upload_for("rom/corrupted.zst");
    
    // Create service with mock
    let service = CloudSyncService::new_with_cloud_ops(mock_cloud.clone());
    
    // Execute sync
    let result = service.sync_files().await?;
    
    // Verify uploads
    assert!(mock_cloud.was_uploaded("rom/new-game.zst"));
    assert!(!mock_cloud.was_uploaded("rom/corrupted.zst"));
    
    // Verify deletions
    assert!(mock_cloud.was_deleted("rom/old-version.zst"));
    
    // Check results
    assert_eq!(result.successful_uploads, 1);
    assert_eq!(result.failed_uploads, 1);
    assert_eq!(result.successful_deletions, 1);
}
```

## Benefits

✅ **No Network Required** - Tests run offline and fast  
✅ **No Credentials** - No need for AWS keys or S3 setup  
✅ **Deterministic** - Same results every time  
✅ **Failure Simulation** - Test error handling easily  
✅ **Inspection** - Verify exactly what operations occurred  
✅ **Parallel Testing** - Multiple tests can run simultaneously  
✅ **Realistic** - Mock sends same progress events as production  

## Pattern Consistency

This follows the same pattern as `FileSystemOps`:

| Trait | Production | Mock |
|-------|-----------|------|
| `FileSystemOps` | `StdFileSystemOps` | `MockFileSystemOps` |
| `CloudStorageOps` | `S3CloudStorage` | `MockCloudStorage` |

Both use:
- Trait with `Send + Sync`
- Default generic parameter
- `new()` for production
- `new_with_X()` for testing
- Rich mock with verification methods
