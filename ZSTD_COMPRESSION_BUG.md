# File Import/Export Bugs - Critical Issues

## Issue Date
2025-12-26

## FOUR CRITICAL BUGS FOUND

### Bug #1: Cloud Download Saves Error Responses as Data Files
**Severity:** CRITICAL  
**Impact:** Files become corrupted with XML error messages

### Bug #2: CloudFileWriter Double-Compresses Files
**Severity:** CRITICAL  
**Impact:** Files cannot be decompressed

### Bug #3: Files Saved to Wrong Directory Path
**Severity:** HIGH  
**Impact:** File organization broken, files in wrong locations

### Bug #4: Credentials Not Persisted Between Sessions
**Severity:** HIGH  
**Impact:** Users must re-enter credentials every application restart

---

## Bug #1: Cloud Download Error Response Bug

### Problem
Cloud downloads that fail (e.g., invalid credentials) save the **XML error response** as if it were the actual file data, creating a corrupted `.zst` file.

### Evidence
File at `/home/mikko/.local/share/efm/files/manual/61392fe1-3cb6-45ed-9b94-b5d149c428c5.zst` contains:
```xml
<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Error>
    <Code>InvalidAccessKeyId</Code>
    <Message>Malformed Access Key Id</Message>
</Error>
```

This causes "Unknown frame descriptor" error when trying to decompress because zstd sees XML instead of the magic number `28 b5 2f fd`.

### Root Cause
The cloud download code doesn't:
1. Check HTTP response status codes before saving
2. Validate downloaded content is actually zstd data
3. Verify zstd magic number after download
4. Fail appropriately when credentials are invalid

### Fix Required
Must validate HTTP responses and content before saving files.

---

## Bug #2: CloudFileWriter Double-Compression Bug

### Problem
Files compressed using `CloudFileWriter::output_zstd_compressed` are **corrupted and cannot be decompressed**. This causes "Unknown frame descriptor" errors when attempting to decompress these files.

## Root Cause
The `CloudFileWriter::output_zstd_compressed` method in `file_import/src/file_writer.rs` has a critical bug where it **double-compresses data**:

### Current Buggy Flow (lines 78-175):

1. **First Compression (lines 83-110)**: 
   - Creates a temp zstd file
   - Compresses the input file to this temp file
   - Calculates checksum of **uncompressed data**
   - **BUT** this temp file is never used after creation!

2. **Second Compression (lines 136-167)**:
   - Reads from the **original uncompressed input file** again
   - For each 8MB chunk:
     - Creates a NEW zstd encoder
     - Compresses the chunk independently
     - Uploads the compressed chunk
   - This creates **multiple independent zstd frames** concatenated together

### Why This Causes "Unknown frame descriptor" Error:
- The file ends up with multiple independent zstd compression frames
- Each chunk was compressed separately, creating fragmented frames
- The zstd decoder may not properly handle these fragmented/concatenated frames
- Results in: `Zip error: Failed decompressing zstd file: Unknown frame descriptor`

## Impact
- **Any file uploaded using CloudFileWriter is CORRUPTED**
- Files can be uploaded successfully to cloud storage
- Files stored locally appear fine
- **Decompression fails** when trying to export/download these files
- Error occurs during `export_files` step in `file_export/src/lib.rs`

## Affected Code
File: `file_import/src/file_writer.rs`
Method: `CloudFileWriter::output_zstd_compressed` (lines 78-175)

## The Fix
The method should:
1. Compress the input file ONCE to a temp file (lines 83-110) ✓ Already done
2. Upload the **already-compressed temp file** to cloud storage
3. Remove the duplicate compression loop (lines 144-150)

### Corrected Flow:
```rust
// Step 1: Compress to temp file (keep existing code lines 83-110)
let zstd_file_path = system_temp_dir.join(archive_file_name).with_extension("zst");
let zstd_file = File::create(&zstd_file_path)?;
let mut encoder = Encoder::new(zstd_file, compression_level.to_zstd_level())?;
// ... compress input file ...
encoder.finish()?;

// Step 2: Upload the COMPRESSED temp file (not the original!)
let mut compressed_file = File::open(&zstd_file_path)?;
loop {
    let bytes_read = compressed_file.read(&mut buffer)?;
    // Upload directly - NO ADDITIONAL COMPRESSION
    bucket.put_multipart_chunk(buffer[..bytes_read].to_vec(), ...).await?;
}

// Step 3: Clean up temp file
std::fs::remove_file(&zstd_file_path).ok();
```

## Note About Your Specific Error

### ACTUAL ROOT CAUSE (December 26, 2025)
The file `/home/mikko/.local/share/efm/files/manual/61392fe1-3cb6-45ed-9b94-b5d149c428c5.zst` is **NOT a zstd file**.

It contains an **S3/Backblaze B2 error response**:
```xml
<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Error>
    <Code>InvalidAccessKeyId</Code>
    <Message>Malformed Access Key Id</Message>
</Error>
```

**What happened:**
1. A cloud download was attempted with invalid credentials
2. The download "succeeded" but saved the XML error response as the file content
3. The file has a `.zst` extension but contains XML, not zstd data
4. When trying to decompress, zstd sees XML instead of the magic number `28 b5 2f fd`
5. Error: "Unknown frame descriptor"

**The real bug:** The cloud download code doesn't validate that downloaded content is actually zstd data before saving it with a `.zst` extension.

### Solutions
**Immediate fix for your file:**
```bash
# Delete the corrupted file
rm /home/mikko/.local/share/efm/files/manual/61392fe1-3cb6-45ed-9b94-b5d149c428c5.zst

# Re-download with valid credentials or copy the good version
cp /home/mikko/.local/share/efm/files/61392fe1-3cb6-45ed-9b94-b5d149c428c5.zst \
   /home/mikko/.local/share/efm/files/manual/61392fe1-3cb6-45ed-9b94-b5d149c428c5.zst
```

**Code fixes needed:**
1. Cloud download must validate HTTP response status codes
2. Must not save error responses as data files
3. Should verify zstd magic number after download
4. Should fail loudly when credentials are invalid

---

## Bug #3: Files Saved to Wrong Directory

### Problem
Files are being saved to the wrong directory path. 

### Evidence
- File EXISTS at: `/home/mikko/.local/share/efm/files/61392fe1-3cb6-45ed-9b94-b5d149c428c5.zst`
- File SHOULD be at: `/home/mikko/.local/share/efm/files/manual/61392fe1-3cb6-45ed-9b94-b5d149c428c5.zst`

The correct file is in the root `files/` directory when it should be in the `files/manual/` subdirectory based on the file type/category.

### Impact
- File organization is broken
- Files not in expected locations
- May cause duplicate files (one in correct location, one in wrong location)
- Makes file management and cleanup difficult

### Investigation Needed

**FOUND THE BUG!**

**Location:** `service/src/file_import/service.rs`, line 96

**Current buggy code:**
```rust
let output_dir = self.settings.collection_root_dir.clone();
```

**Should be:**
```rust
let output_dir = self.settings.get_file_type_path(&file_type);
```

**Explanation:**
- Files are being imported to the root collection directory (`/home/mikko/.local/share/efm/files/`)
- They should be imported to file-type-specific subdirectories (e.g., `/home/mikko/.local/share/efm/files/manual/`)
- The `Settings` struct already has a helper method `get_file_type_path()` that does this correctly (line 67-69 in `service/src/view_models.rs`)
- The export code uses this correctly via `resolve_file_type_path()` (line 124 in `service/src/export_service.rs`)
- But import code doesn't use it!

**File Type to Directory Mapping** (from `core_types/src/lib.rs` lines 102-116):
- `Manual` → `manual/`
- `Rom` → `rom/`
- `Screenshot` → `screenshot/`
- `DiskImage` → `disk_image/`
- etc.

### Impact
- All imported files go to wrong location
- Export expects files in subdirectories, import puts them in root
- Creates mismatches between database paths and actual file locations

---

## Bug #4: Credentials Not Persisted Between Sessions

### Problem
Cloud storage credentials (S3/Backblaze B2 access keys) are only stored for the current session. When the application is restarted, credentials are lost and must be re-entered.

### Evidence
- After entering valid credentials, cloud operations work correctly
- After restarting the application, cloud downloads fail with `InvalidAccessKeyId: Malformed Access Key Id`
- This causes Bug #1 to trigger (error responses saved as files)

### Impact
- Poor user experience - credentials must be re-entered every time
- Silent failures when credentials expire
- Downloads fail and save error responses instead of proper error handling
- Users may not realize credentials are missing until operations fail

### Root Cause (Unknown - Investigation Needed)
Two possible causes:
1. **Application Bug:** Credentials are not being saved to the credential store
2. **Credential Store Issue:** Local credential store (e.g., keyring, secret service) is not working properly

### Investigation Needed
1. Check if credentials storage is implemented at all
2. Verify credential store integration (likely using `keyring` crate or similar)
3. Check if credentials are being loaded on application startup
4. Verify credential store is accessible on the system

**NOTE:** There's a `credentials_storage` module that uses the `keyring` crate with `sync-secret-service` feature. A previous keyring backend issue was documented and supposedly fixed (see `credentials_storage/KEYRING_BACKEND_ISSUE.md` - fixed October 29, 2025). However, credentials are still not persisting between sessions as of December 26, 2025.

**Possible causes:**
1. Credentials are stored but not loaded on application startup
2. Credentials are stored in a different location than where they're loaded from
3. The loading code has a bug (silent failure, wrong error handling)
4. Service/username parameters don't match between store and load
5. Keyring backend regressed or system keyring not working

### Related Components
- `credentials_storage/` - Credential storage module using keyring
- `credentials_storage/src/lib.rs` - Store/load implementation
- `credentials_storage/KEYRING_BACKEND_ISSUE.md` - Previous fix (Oct 29, 2025)
- `service/src/settings_service.rs` - Service layer using credentials
- Look for initialization code that should load credentials on startup

---

## Recommended Actions

### Immediate
1. **Fix Bug #1:** Add HTTP response validation to cloud download code
2. **Fix Bug #2:** Fix CloudFileWriter double-compression
3. **Fix Bug #3:** Fix file path construction during import (one-line fix at `service/src/file_import/service.rs:96`)
4. **Fix Bug #4:** Investigate and fix credential persistence

### Validation
4. **Add integration tests** that:
   - Upload AND download files to catch compression issues
   - Verify files are in correct directories
   - Validate downloaded content before saving
   - Test with invalid credentials to ensure proper error handling

### Cleanup
5. **Identify all corrupted files** saved with XML error responses
6. **Identify all double-compressed files** uploaded with buggy CloudFileWriter
7. **Identify all misplaced files** in wrong directories
8. **Re-import/re-download** all affected files

## Related Files
- `file_import/src/file_writer.rs` - Bug #2: CloudFileWriter double-compression (lines 78-175)
- `file_export/src/lib.rs` - Where decompression fails (line 165)
- `service/src/file_import/service.rs` - Bug #3: Wrong directory (line 96)
- Cloud download code (location TBD) - Bug #1: Error response saving
- Credential storage code (location TBD) - Bug #4: Credentials not persisted
