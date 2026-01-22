# File Metadata Reader

## Overview

Extract file metadata (checksums, sizes, paths) from various file types (single files, archives) with a consistent interface that's easily testable.

## Architecture

**New Crate:** `file_metadata`

**Layer:** Core/Infrastructure crate (same level as `file_system`, `dat_file_parser`, `http_downloader`)

**Rationale for separate crate:**
- Metadata reading is a **core capability** distinct from business logic
- **Highly reusable** across import, export, validation, cloud sync, etc.
- Keeps `file_import` focused on import workflow orchestration
- Follows established pattern of focused utility crates

**Dependencies:**
```toml
[dependencies]
core_types = { path = "../core_types" }
file_system = { path = "../file_system" }
utils = { path = "../utils" }
```

**Dependents:**
- `file_import` - Uses metadata reading to validate files before import
- `file_export` (future) - Verify file integrity during export
- `service` - Exposes file metadata APIs
- Other crates needing file inspection without full import/export

## Integration with file_import Crate

**Current state** of `file_import`:
- `FileImporter` struct - Orchestrates full import process (copy, compress, store)
- `import_zip_files()` - Handles zip-specific imports
- Metadata reading is coupled with import logic

**After this implementation:**
- `file_metadata` crate provides metadata reading (read-only inspection)
- `file_import` depends on `file_metadata` for validation
- Clear separation: **inspection** (file_metadata) vs **transformation** (file_import)

**Migration strategy for file_import:**
1. Add `file_metadata = { path = "../file_metadata" }` to dependencies
2. Refactor `FileImporter` to use `FileMetadataReader` for pre-import validation
3. Remove inline metadata reading code from import logic
4. Keep existing public APIs as convenience wrappers

## Design

### Core Types

**Note:** Consider moving `FileMetadata` to `core_types` crate for reusability across export, cloud sync, etc.

```rust
/// Represents metadata for a single file entry
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FileMetadata {
    pub checksum: Sha1Checksum,
    pub size: u64,
    pub relative_path: String, // Path within archive, or filename for single files
}

/// Supported file types for metadata extraction
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FileType {
    Single,
    Zip,
    // Future: SevenZip, Rar, Tar
}

/// Trait for reading file metadata from various sources
pub trait FileMetadataReader: Send + Sync {
    /// Read metadata for all files in this source
    /// 
    /// Returns a Vec since archives can contain multiple files.
    /// Single files return a Vec with one element for consistent interface.
    /// 
    /// Note: This is a blocking operation. Checksumming large files may take time.
    /// Future enhancement: async version with progress callbacks.
    fn read_metadata(&self) -> Result<Vec<FileMetadata>, FileMetadataError>;
}
```

### Factory Function

Instead of a Provider trait, use a simple factory function:

```rust
/// Create appropriate reader based on file type
pub fn create_metadata_reader(path: &Path) -> Result<Box<dyn FileMetadataReader>, FileMetadataError> {
    match detect_file_type(path)? {
        FileType::Single => Ok(Box::new(SingleFileMetadataReader::new(path)?)),
        FileType::Zip => Ok(Box::new(ZipFileMetadataReader::new(path)?)),
        // Future: FileType::SevenZip, FileType::Tar, etc.
    }
}

fn detect_file_type(path: &Path) -> Result<FileType, FileMetadataError> {
    // Check extension and/or magic bytes
    match path.extension().and_then(|s| s.to_str()) {
        Some("zip") => Ok(FileType::Zip),
        _ => Ok(FileType::Single),
    }
}
```

### Implementations

#### SingleFileMetadataReader

```rust
pub struct SingleFileMetadataReader {
    path: PathBuf,
}

impl SingleFileMetadataReader {
    pub fn new(path: &Path) -> Result<Self, FileMetadataError> {
        if !path.exists() {
            return Err(FileMetadataError::FileNotFound(path.to_path_buf()));
        }
        Ok(Self { path: path.to_path_buf() })
    }
}

impl FileMetadataReader for SingleFileMetadataReader {
    fn read_metadata(&self) -> Result<Vec<FileMetadata>, FileMetadataError> {
        // Uses file_system::read_file_checksum() - ensure dependency is added
        let checksum = read_file_checksum(&self.path)
            .map_err(|e| FileMetadataError::ChecksumError {
                path: self.path.clone(),
                source: e,
            })?;
            
        let metadata = std::fs::metadata(&self.path)
            .map_err(|e| FileMetadataError::FileIoError {
                path: self.path.clone(),
                source: e,
            })?;
            
        let filename = self.path
            .file_name()
            .ok_or_else(|| FileMetadataError::InvalidPath(self.path.clone()))?
            .to_string_lossy()
            .to_string();
        
        Ok(vec![FileMetadata {
            checksum,
            size: metadata.len(),
            relative_path: filename,
        }])
    }
}
```

#### ZipFileMetadataReader

```rust
pub struct ZipFileMetadataReader {
    path: PathBuf,
}

impl ZipFileMetadataReader {
    pub fn new(path: &Path) -> Result<Self, FileMetadataError> {
        if !path.exists() {
            return Err(FileMetadataError::FileNotFound(path.to_path_buf()));
        }
        Ok(Self { path: path.to_path_buf() })
    }
}

impl FileMetadataReader for ZipFileMetadataReader {
    fn read_metadata(&self) -> Result<Vec<FileMetadata>, FileMetadataError> {
        let entries = read_zip_contents_with_checksums(&self.path)?;
        
        Ok(entries
            .into_iter()
            .map(|(path, checksum, size)| FileMetadata {
                checksum,
                size,
                relative_path: path,
            })
            .collect())
    }
}
```

## Testing Strategy

### Test Architecture Approach

**Two scenarios with different testing strategies:**

#### Scenario 1: Single File Processing (Direct Reader Injection)

When processing a single file where the reader type is known or controlled:

```rust
pub fn import_single_file_with_reader(
    reader: &dyn FileMetadataReader,
    destination: &Path,
) -> Result<ImportResult, FileMetadataError> {
    let metadata = reader.read_metadata()?;
    // ... import logic
}

// Tests inject mock directly
#[test]
fn test_import_single_file() {
    let mock = MockFileMetadataReader { /* ... */ };
    import_single_file_with_reader(&mock, temp_dir.path());
}
```

#### Scenario 2: Batch/Folder Processing (Factory Injection)

When processing multiple files of unknown types, inject the factory function:

```rust
/// Type alias for factory function
pub type ReaderFactoryFn = dyn Fn(&Path) -> Result<Box<dyn FileMetadataReader>, FileMetadataError>;

pub fn import_folder_with_factory(
    folder: &Path,
    destination: &Path,
    factory: &ReaderFactoryFn,
) -> Result<Vec<ImportResult>, FileMetadataError> {
    let mut results = Vec::new();
    
    for entry in std::fs::read_dir(folder)? {
        let path = entry?.path();
        let reader = factory(&path)?;
        let metadata = reader.read_metadata()?;
        // ... process each file
        results.push(/* result */);
    }
    
    Ok(results)
}

// Production code uses real factory
pub fn import_folder(folder: &Path, destination: &Path) -> Result<Vec<ImportResult>, FileMetadataError> {
    import_folder_with_factory(folder, destination, &create_metadata_reader)
}
```

Tests inject a mock factory:

```rust
#[test]
fn test_import_folder_with_mock_factory() {
    let mock_reader = MockFileMetadataReader {
        metadata: vec![/* test data */],
    };
    
    // Mock factory always returns the same mock reader
    let mock_factory = |_path: &Path| -> Result<Box<dyn FileMetadataReader>, FileMetadataError> {
        Ok(Box::new(mock_reader.clone()))
    };
    
    let result = import_folder_with_factory(temp_dir.path(), dest.path(), &mock_factory);
    assert_eq!(result.unwrap().len(), 3); // Verify all files processed
}

#[test]
fn test_import_folder_with_error_factory() {
    // Mock factory that simulates errors
    let error_factory = |_path: &Path| -> Result<Box<dyn FileMetadataReader>, FileMetadataError> {
        Err(FileMetadataError::UnsupportedFormat("test".into()))
    };
    
    let result = import_folder_with_factory(temp_dir.path(), dest.path(), &error_factory);
    assert!(result.is_err());
}
```

### Unit Tests

1. **SingleFileMetadataReader**
   - Test with existing file → returns correct checksum and size
   - Test with non-existent file → returns FileNotFound error
   - Test with various file sizes (empty, small, large)

2. **ZipFileMetadataReader**
   - Test with valid zip containing single file
   - Test with zip containing multiple files
   - Test with zip containing nested directories
   - Test with corrupted/invalid zip → returns appropriate error
   - Test with empty zip

3. **Factory Function**
   - Test .zip extension → creates ZipFileMetadataReader
   - Test other extensions → creates SingleFileMetadataReader
   - Test non-existent file → returns error
   - **Note:** Factory tests use real test fixtures, not mocks

### Integration Tests

Create test fixtures in `file_metadata/tests/fixtures/`:
- `test.txt` - Simple text file
- `archive.zip` - Zip with multiple files
- `nested.zip` - Zip with directory structure

### Mocking Strategy

**MockFileMetadataReader** - For testing code that accepts readers:

```rust
#[cfg(test)]
#[derive(Clone)]
pub struct MockFileMetadataReader {
    pub metadata: Vec<FileMetadata>,
}

#[cfg(test)]
impl FileMetadataReader for MockFileMetadataReader {
    fn read_metadata(&self) -> Result<Vec<FileMetadata>, FileMetadataError> {
        Ok(self.metadata.clone())
    }
}
```

**Mock Factory** - For testing code that processes multiple file types:

```rust
#[cfg(test)]
pub fn create_mock_factory(
    reader: MockFileMetadataReader,
) -> impl Fn(&Path) -> Result<Box<dyn FileMetadataReader>, FileMetadataError> {
    move |_path: &Path| Ok(Box::new(reader.clone()))
}
```

Usage:

```rust
#[test]
fn test_batch_import() {
    let mock = MockFileMetadataReader {
        metadata: vec![FileMetadata { /* ... */ }],
    };
    let factory = create_mock_factory(mock);
    
    let result = import_folder_with_factory(folder, dest, &factory);
    assert!(result.is_ok());
}
```

## Future Enhancements

- **7-Zip support**: Add `SevenZipFileMetadataReader`
- **RAR support**: Add `RarFileMetadataReader`
- **Tar archives**: Add `TarFileMetadataReader`
- **Magic byte detection**: Improve file type detection beyond extensions
- **Parallel processing**: Checksum calculation for multiple archive entries concurrently

## Open Questions

1. Should `FileMetadata` include timestamps (modified date)?
2. Should we validate checksums against expected values in this layer?
3. How should we handle password-protected archives?
4. Should nested archives be extracted recursively or treated as single files?

---

## Implementation Task List

### Phase 1: Crate Setup
- [ ] Create `file_metadata/` directory structure
- [ ] Create `file_metadata/Cargo.toml` with dependencies (core_types, file_system, utils)
- [ ] Create `file_metadata/src/lib.rs` with module structure
- [ ] Add workspace member to root `Cargo.toml`
- [ ] Verify `cargo build` succeeds for empty crate

### Phase 2: Core Types & Traits
- [ ] Define `FileMetadata` struct in `lib.rs`
- [ ] Define `FileType` enum (Single, Zip)
- [ ] Define `FileMetadataError` enum with path context
- [ ] Define `FileMetadataReader` trait
- [ ] Define `ReaderFactoryFn` type alias
- [ ] Add comprehensive documentation to all public items
- [ ] Run `cargo doc --open` to verify docs

### Phase 3: SingleFileMetadataReader
- [ ] Create `SingleFileMetadataReader` struct
- [ ] Implement `new()` constructor with validation
- [ ] Implement `FileMetadataReader` trait
- [ ] Add unit tests for single file reading
- [ ] Add unit tests for error cases (missing file, invalid path)
- [ ] Test with various file sizes (empty, small, large)

### Phase 4: ZipFileMetadataReader
- [ ] Create `ZipFileMetadataReader` struct
- [ ] Implement `new()` constructor with validation
- [ ] Implement `FileMetadataReader` trait
- [ ] Add unit tests for single-file zips
- [ ] Add unit tests for multi-file zips
- [ ] Add unit tests for nested directory structures
- [ ] Add unit tests for error cases (corrupt zip, invalid zip)

### Phase 5: Factory Function (after readers are implemented)
- [ ] Implement `detect_file_type()` function
- [ ] Implement `create_metadata_reader()` factory function
- [ ] Add tests for file type detection (zip vs non-zip)
- [ ] Add tests for factory with various file extensions
- [ ] Test factory error cases (non-existent files)

### Phase 6: Test Infrastructure
- [ ] Create `tests/fixtures/` directory
- [ ] Add `test.txt` fixture
- [ ] Add `archive.zip` fixture (multiple files)
- [ ] Add `nested.zip` fixture (directory structure)
- [ ] Create integration tests using fixtures
- [ ] Implement `MockFileMetadataReader` in test module
- [ ] Implement `create_mock_factory()` helper
- [ ] Add tests demonstrating mock usage

### Phase 7: Documentation & Examples
- [ ] Add module-level documentation with usage examples
- [ ] Add examples showing single file scenario
- [ ] Add examples showing batch/folder scenario with factory injection
- [ ] Document blocking behavior and future async plans
- [ ] Add README.md to file_metadata crate

### Phase 8: Integration with file_import
- [ ] Add `file_metadata = { path = "../file_metadata" }` to `file_import/Cargo.toml`
- [ ] Update `FileImporter` to use `FileMetadataReader` for validation
- [ ] Refactor inline metadata reading code
- [ ] Update tests to use new abstractions
- [ ] Verify existing `file_import` APIs still work

### Phase 9: Validation
- [ ] Run `cargo test --workspace` - all tests pass
- [ ] Run `cargo clippy --workspace` - no warnings
- [ ] Run `cargo fmt --workspace --check` - formatted correctly
- [ ] Regenerate `.sqlx/` if any queries changed: `cargo sqlx prepare --workspace -- --all-targets`
- [ ] Verify layer boundaries - no violations
- [ ] Spot-check critical functionality manually

### Phase 10: Cleanup & Documentation
- [ ] Remove any temporary test files
- [ ] Update project documentation if needed
- [ ] Archive or remove old metadata reading code from `file_import`
- [ ] Commit changes with clear message

**Estimated effort:** 8-12 hours for experienced Rust developer

**Critical path:** Phases 1-2-3-4-5 must be done in sequence. Phase 6 can start after Phase 3.

**Key change from original:** Factory implementation (Phase 5) now happens AFTER both readers are implemented (Phases 3-4).
