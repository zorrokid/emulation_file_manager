# Database Synchronization Design for Multiple Devices

## Overview

This document outlines the design for synchronizing SQLite databases across multiple devices while maintaining file availability through cloud storage. The system ensures that users can access their software collection from any device, with files automatically downloaded from cloud storage when needed.

## Current System Architecture

### Existing Components

1. **SQLite Database** - Stores metadata about:
   - File information (SHA1, size, archive name)
   - File sets (collections of files)
   - Releases, systems, software titles
   - File sync logs (tracking cloud upload/deletion status)

2. **Cloud Storage (S3-compatible)** - Stores actual files:
   - Files compressed with zstd
   - Organized by file type (rom/, screenshot/, manual/, etc.)
   - Files tracked via `file_sync_log` table

3. **File Sync Log** - Tracks sync status:
   - `UploadPending`, `UploadInProgress`, `UploadCompleted`, `UploadFailed`
   - `DeletionPending`, `DeletionInProgress`, `DeletionCompleted`, `DeletionFailed`

## Multi-Device Sync Requirements

### Goals

1. **Database Portability** - Users can access their collection from any device
2. **File On-Demand** - Files download from cloud when needed, not all at once
3. **Conflict Resolution** - Handle concurrent modifications from multiple devices
4. **Bandwidth Efficiency** - Only sync database and download files when required
5. **Offline Support** - Basic functionality works without internet connection

### Non-Goals (Initial Version)

- Real-time synchronization (use manual sync trigger)
- Operational transformation for concurrent editing
- Peer-to-peer synchronization

## Proposed Architecture

### 1. Database Sync Strategy

#### Option A: Cloud Database Backup (Recommended for MVP)

**How it works:**
1. Database file stored in cloud storage (e.g., `metadata/collection.db`)
2. Each device maintains local copy
3. Manual sync operation:
   - **Pull**: Download latest database from cloud
   - **Push**: Upload local database to cloud

**Pros:**
- Simple to implement
- Leverages existing S3 infrastructure
- Small database size (only metadata)
- Built-in versioning via S3 versioning feature

**Cons:**
- Last-write-wins (potential data loss)
- Manual conflict resolution
- Requires full database replacement

**Implementation:**
```rust
pub struct DatabaseSyncService {
    repository_manager: Arc<RepositoryManager>,
    settings_service: Arc<SettingsService>,
    cloud_ops: Option<Arc<dyn CloudStorageOps>>,
}

impl DatabaseSyncService {
    pub async fn sync_database_to_cloud(&self) -> Result<(), Error> {
        // 1. Get database file path
        // 2. Create backup with timestamp
        // 3. Upload to cloud: metadata/collection.db
        // 4. Upload backup: metadata/backups/collection_2026-01-04T11-00-00Z.db
        // 5. Update local sync metadata
    }
    
    pub async fn sync_database_from_cloud(&self) -> Result<SyncResult, Error> {
        // 1. Download from cloud: metadata/collection.db
        // 2. Check if local changes exist (compare timestamps)
        // 3. If conflicts exist, provide options:
        //    - Use cloud version (discard local)
        //    - Use local version (will push on next sync)
        //    - Manual merge (advanced)
        // 4. Replace local database or merge
        // 5. Update local sync metadata
    }
    
    pub async fn list_database_backups(&self) -> Result<Vec<DatabaseBackup>, Error> {
        // List all backups from metadata/backups/
        // Useful for rollback scenarios
    }
}

pub struct SyncResult {
    pub sync_type: SyncType, // Pull or Push
    pub conflict_detected: bool,
    pub resolution: ConflictResolution,
    pub cloud_version_timestamp: Option<DateTime>,
    pub local_version_timestamp: Option<DateTime>,
}

pub enum SyncType {
    Pull,
    Push,
}

pub enum ConflictResolution {
    NoConflict,
    UsedCloud,
    UsedLocal,
    ManualMergeRequired,
}

pub struct DatabaseBackup {
    pub filename: String,
    pub timestamp: DateTime,
    pub size: u64,
}
```

#### Option B: Conflict-Free Replicated Data Types (CRDT) - Future Enhancement

**How it works:**
1. Use CRDT-based database (e.g., Automerge, Yjs for SQLite)
2. Automatic conflict resolution
3. Event-based synchronization

**Pros:**
- Automatic conflict resolution
- Can support real-time sync
- No data loss

**Cons:**
- Complex implementation
- Requires significant refactoring
- Performance overhead
- Not all operations map well to CRDTs

**Defer to Future Version** - Focus on Option A for MVP

### 2. Device Identification

Add device tracking to understand sync history:

```sql
-- New migration: add_device_sync_metadata.sql
CREATE TABLE device (
    id TEXT PRIMARY KEY, -- UUID generated on first run
    name TEXT NOT NULL,  -- User-friendly name (e.g., "Laptop", "Desktop")
    last_seen TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE database_sync_log (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    sync_type INTEGER NOT NULL, -- 0=Pull, 1=Push
    sync_time TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    device_id TEXT NOT NULL,
    cloud_version TEXT, -- S3 version ID or timestamp
    local_version TEXT, -- Local database checksum
    conflict_detected INTEGER NOT NULL DEFAULT 0,
    conflict_resolution INTEGER, -- 0=NoConflict, 1=UsedCloud, 2=UsedLocal, 3=Manual
    message TEXT NOT NULL DEFAULT '',
    FOREIGN KEY (device_id) REFERENCES device(id)
);

-- Store local device ID in settings table
INSERT INTO setting (key, value) VALUES ('device_id', '<UUID>');
INSERT INTO setting (key, value) VALUES ('device_name', '<User Device Name>');
INSERT INTO setting (key, value) VALUES ('last_database_sync', '<ISO timestamp>');
```

### 3. File Download on Demand

Files are already tracked via `file_sync_log`. When switching devices:

**Current Behavior** (Already Implemented):
- `FileSetDownloadService` checks if files exist locally
- Downloads from cloud if missing
- Downloads tracked in `file_set_download` pipeline

**Enhancement** - Add sync status awareness:

```rust
pub enum FileAvailability {
    LocalOnly,           // File exists locally but not in cloud
    CloudOnly,           // File in cloud but not downloaded yet
    Synchronized,        // File exists both locally and in cloud
    NotAvailable,        // File doesn't exist anywhere
}

impl FileInfo {
    pub async fn get_availability(
        &self, 
        repo_manager: &RepositoryManager,
        file_system: &dyn FileSystemOps
    ) -> Result<FileAvailability, Error> {
        let local_exists = file_system.file_exists(&self.get_archive_path()).await;
        
        let sync_log = repo_manager
            .get_file_sync_log_repository()
            .get_latest_log_for_file(self.id)
            .await?;
        
        let cloud_exists = sync_log
            .map(|log| log.status == FileSyncStatus::UploadCompleted)
            .unwrap_or(false);
        
        match (local_exists, cloud_exists) {
            (true, true) => Ok(FileAvailability::Synchronized),
            (true, false) => Ok(FileAvailability::LocalOnly),
            (false, true) => Ok(FileAvailability::CloudOnly),
            (false, false) => Ok(FileAvailability::NotAvailable),
        }
    }
}
```

### 4. Sync Workflow

#### Device A: Initial Setup
1. User creates collection, imports files
2. Files uploaded to cloud storage (existing functionality)
3. User manually triggers "Sync Database to Cloud"
4. Database uploaded to `metadata/collection.db`

#### Device B: First Sync
1. User installs app on Device B
2. Configures cloud storage credentials
3. User triggers "Sync Database from Cloud"
4. Database downloaded to local storage
5. User browses collection (metadata available)
6. When user opens a release/file set:
   - Check file availability
   - If `CloudOnly`, download automatically (existing functionality)

#### Subsequent Syncs (Device A and B)

**Before Making Changes:**
1. Optional: Pull latest database from cloud
2. Check for conflicts
3. Resolve if needed

**After Making Changes:**
1. User triggers "Sync Database to Cloud"
2. Create local backup
3. Upload to cloud
4. Update sync log

### 5. Conflict Detection and Resolution

#### Conflict Scenarios

**Scenario 1: Simple - No Conflict**
- Device A last synced at T1
- Device B last synced at T1
- Device A makes changes at T2, pushes database
- Device B pulls at T3 (T3 > T2)
- **Resolution**: Device B accepts changes automatically

**Scenario 2: Conflict - Last Write Wins**
- Device A synced at T1
- Device B synced at T1
- Device A makes changes at T2 (adds Release X)
- Device B makes changes at T3 (adds Release Y, T3 > T2)
- Device B pushes at T4
- Device A tries to push at T5
- **Resolution**: Device A detects conflict (cloud modified since last pull)
  - Option 1: Pull first, lose local changes, manual re-add
  - Option 2: Keep local, overwrite cloud (data loss)
  - Option 3: Show diff, manual merge (future feature)

#### Conflict Detection Implementation

```rust
pub struct ConflictDetector {
    repository_manager: Arc<RepositoryManager>,
}

impl ConflictDetector {
    pub async fn check_for_conflicts(
        &self,
        cloud_ops: &dyn CloudStorageOps
    ) -> Result<ConflictCheckResult, Error> {
        // 1. Get local last sync timestamp
        let local_sync = self.repository_manager
            .get_setting_repository()
            .get("last_database_sync")
            .await?;
        
        // 2. Get cloud database metadata (last modified time)
        let cloud_metadata = cloud_ops
            .get_metadata("metadata/collection.db")
            .await?;
        
        // 3. Compare timestamps
        if cloud_metadata.last_modified > local_sync.timestamp {
            Ok(ConflictCheckResult::Conflict {
                local_timestamp: local_sync.timestamp,
                cloud_timestamp: cloud_metadata.last_modified,
            })
        } else {
            Ok(ConflictCheckResult::NoConflict)
        }
    }
    
    pub async fn create_diff_report(
        &self,
        cloud_db_path: &Path
    ) -> Result<DatabaseDiff, Error> {
        // Advanced feature: Compare two database files
        // Show differences in:
        // - New releases
        // - Modified releases
        // - Deleted releases
        // - New file sets
        // - etc.
    }
}

pub enum ConflictCheckResult {
    NoConflict,
    Conflict {
        local_timestamp: DateTime,
        cloud_timestamp: DateTime,
    },
}

pub struct DatabaseDiff {
    pub releases_added: Vec<Release>,
    pub releases_modified: Vec<(Release, Release)>, // (old, new)
    pub releases_deleted: Vec<Release>,
    pub file_sets_added: Vec<FileSet>,
    pub file_sets_deleted: Vec<FileSet>,
    // ... other entity types
}
```

### 6. User Interface Additions

#### Settings Page Additions
```
[ ] Cloud Storage Configuration (existing)
    Endpoint: _______________
    Region: _________________
    Bucket: _________________
    [x] Enable Sync

[+] Database Synchronization (new section)
    Device Name: [Laptop         ]
    Device ID: abc-123-def (read-only)
    
    Last Sync: 2026-01-04 10:30:00
    Sync Status: ⚫ Up to date | ⚠️ Conflict detected
    
    [Sync from Cloud (Pull)]  [Sync to Cloud (Push)]
    
    [ ] Automatic sync on startup
    [ ] Prompt before overwriting local changes
    
    Backups:
    - 2026-01-04 10:30:00 (current)
    - 2026-01-03 15:20:00 [Restore]
    - 2026-01-02 09:45:00 [Restore]
```

#### File Set / Release View
```
Release: Super Mario Bros (NES)
File Sets:
  📂 ROM Files (2.5 MB) 
     Status: ☁️ In Cloud | 💾 Downloaded
  📂 Manual (15 MB)
     Status: ☁️ In Cloud | ⬇️ Download Required
  📂 Box Art (3 MB)
     Status: ⚠️ Local Only (not synced)
```

### 7. Database Schema Changes

```sql
-- Migration: 20260104_add_database_sync_support.sql

-- Track devices
CREATE TABLE device (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    last_seen TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Track database sync operations
CREATE TABLE database_sync_log (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    sync_type INTEGER NOT NULL, -- 0=Pull, 1=Push
    sync_time TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    device_id TEXT NOT NULL,
    cloud_version TEXT,
    local_version TEXT,
    conflict_detected INTEGER NOT NULL DEFAULT 0,
    conflict_resolution INTEGER,
    message TEXT NOT NULL DEFAULT '',
    FOREIGN KEY (device_id) REFERENCES device(id)
);

-- Add device settings
INSERT INTO setting (key, value) 
VALUES 
    ('device_id', ''), -- Will be generated on first run
    ('device_name', ''), -- User can customize
    ('last_database_sync', ''), -- ISO timestamp
    ('database_cloud_version', ''), -- S3 version or checksum
    ('auto_sync_on_startup', '0');

-- Index for performance
CREATE INDEX idx_database_sync_log_device 
    ON database_sync_log(device_id, sync_time DESC);
```

### 8. Implementation Phases

#### Phase 1: Basic Database Sync (MVP)
- [ ] Create `DatabaseSyncService`
- [ ] Implement database upload to cloud
- [ ] Implement database download from cloud
- [ ] Add database sync log tracking
- [ ] Add device ID generation
- [ ] Add settings UI for manual sync

**Deliverable**: Users can manually push/pull database between devices

#### Phase 2: Conflict Detection
- [ ] Implement timestamp-based conflict detection
- [ ] Add conflict resolution UI
- [ ] Add backup creation before overwrite
- [ ] Add backup restore functionality

**Deliverable**: Users are warned about conflicts and can resolve them

#### Phase 3: Enhanced File Availability
- [ ] Add file availability status to UI
- [ ] Implement automatic download prompts
- [ ] Add "Download All" functionality for offline use
- [ ] Add storage space indicators

**Deliverable**: Clear visibility of which files are local vs. cloud

#### Phase 4: Advanced Features (Future)
- [ ] Automatic sync on startup (optional)
- [ ] Database diff viewer
- [ ] Three-way merge for conflicts
- [ ] Selective sync (filter by system, file type)
- [ ] CRDT-based synchronization

### 9. Edge Cases and Considerations

#### Storage Concerns
- **Database Size**: SQLite databases are typically small (< 50MB for large collections)
- **Bandwidth**: Only metadata syncs, not files (unless explicitly downloaded)
- **Versioning**: S3 versioning enabled to prevent accidental data loss

#### Network Reliability
- **Offline Mode**: App works with local database, sync when online
- **Partial Sync**: If download fails, can retry
- **Timeout Handling**: Cloud operations have configurable timeouts

#### Data Integrity
- **Checksums**: Verify database integrity after download
- **Backups**: Automatic backup before any destructive operation
- **Transaction Safety**: SQLite ACID properties maintained

#### Security
- **Credentials**: Already handled by existing `credentials_storage` crate
- **Encryption**: Use HTTPS for cloud transport
- **Database Encryption**: Future consideration (SQLCipher)

### 10. Testing Strategy

#### Unit Tests
- Database upload/download operations
- Conflict detection logic
- Device ID generation and storage
- Backup creation and restoration

#### Integration Tests
- Full sync workflow (push and pull)
- Conflict resolution scenarios
- File download after database sync
- Multiple device simulation

#### Manual Testing Scenarios
1. **Fresh Device Setup**
   - Install on Device B
   - Configure cloud storage
   - Pull database
   - Verify collection appears
   - Open file set, verify download prompts

2. **Concurrent Modifications**
   - Make changes on Device A
   - Make different changes on Device B
   - Attempt sync from both
   - Verify conflict detection

3. **Offline/Online Transitions**
   - Use app offline
   - Make changes
   - Go online
   - Sync and verify changes persist

### 11. Migration Path for Existing Users

For users already using the app:

1. **No Action Required** - Database sync is opt-in
2. **Enable Sync** - Configure cloud storage (may already be done for file sync)
3. **First Push** - Upload current database to cloud
4. **Other Devices** - Pull database and start using

Existing `file_sync_log` data is preserved and continues to work.

## API Reference

### DatabaseSyncService

```rust
impl DatabaseSyncService {
    /// Push local database to cloud storage
    pub async fn push_to_cloud(&self) -> Result<PushResult, Error>;
    
    /// Pull database from cloud storage
    pub async fn pull_from_cloud(&self, resolution: ConflictResolution) -> Result<PullResult, Error>;
    
    /// Check for conflicts without pulling
    pub async fn check_conflicts(&self) -> Result<ConflictCheckResult, Error>;
    
    /// List available database backups
    pub async fn list_backups(&self) -> Result<Vec<DatabaseBackup>, Error>;
    
    /// Restore from a specific backup
    pub async fn restore_backup(&self, backup: &DatabaseBackup) -> Result<(), Error>;
    
    /// Get current device info
    pub async fn get_device_info(&self) -> Result<Device, Error>;
    
    /// Update device name
    pub async fn update_device_name(&self, name: String) -> Result<(), Error>;
}
```

### CloudStorageOps Extension

```rust
#[async_trait]
pub trait CloudStorageOps: Send + Sync {
    // ... existing methods ...
    
    /// Get metadata for a file (size, last modified, version)
    async fn get_metadata(&self, key: &str) -> Result<ObjectMetadata, CloudStorageError>;
    
    /// List objects with prefix
    async fn list_objects(&self, prefix: &str) -> Result<Vec<ObjectInfo>, CloudStorageError>;
}

pub struct ObjectMetadata {
    pub size: u64,
    pub last_modified: DateTime,
    pub version_id: Option<String>,
    pub etag: String,
}

pub struct ObjectInfo {
    pub key: String,
    pub size: u64,
    pub last_modified: DateTime,
}
```

## Conclusion

This design provides a pragmatic approach to multi-device synchronization:

1. **Simple MVP** - Manual push/pull with conflict warnings
2. **Scalable** - Can evolve to more sophisticated sync later
3. **Leverages Existing Infrastructure** - Uses current S3 and file sync systems
4. **User-Friendly** - Clear conflict resolution, backup safety net
5. **Bandwidth Efficient** - Only syncs metadata, files on-demand

The phased approach allows for incremental development and user feedback before implementing more complex features like CRDTs or automatic synchronization.
