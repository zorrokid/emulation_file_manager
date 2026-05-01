# Cloud Download Validation Bug

## Summary

This document tracks the remaining cloud download bug where an S3-compatible error response can be saved to disk as if it were a valid `.zst` file.

The earlier issues that were originally bundled into this document are no longer active and are kept here only as brief historical notes.

## Current Status

- **Active:** cloud download can save error response bodies as file data
- **Resolved:** dead `CloudFileWriter` double-compression path
- **Resolved:** imported files now use file-type-specific output directories
- **Resolved:** test credentials no longer overwrite production keyring entries

## Active Bug: Error Responses Saved as `.zst` Files

### Problem

`cloud_storage/src/lib.rs` currently downloads with `bucket.get_object_stream(key).await?`, creates the destination file immediately, and writes every returned chunk directly to disk.

If the server responds with an XML or other error body instead of actual zstd content, that response can be persisted as a `.zst` file. Later decompression then fails because the file does not contain valid zstd frames.

### Current Code Path

- `cloud_storage/src/lib.rs`
- `download_file()`

Current behavior:

1. open response stream
2. create local output file
3. write all returned bytes to disk
4. report progress
5. return success if the stream finishes without a transport-level error

### Why This Is Unsafe

The current implementation does **not** guarantee that the downloaded body is actually a valid file payload.

Missing safeguards:

- no validation that the response represents a successful file download before writing
- no validation of the first bytes before persisting the file as `.zst`
- no protection against error bodies being written as user data

## Required Behavior of the Fix

Any final fix should guarantee all of the following:

- failed downloads do **not** create or keep corrupted output files
- an authentication or authorization failure is surfaced as an error, not as a fake successful download
- invalid or unexpected response bodies are rejected before being treated as zstd files
- partial or invalid outputs are cleaned up on failure

Implementation details can vary, but the fix must be behaviorally equivalent to:

- validate response success before writing, or
- validate the first chunk before writing any file data, and
- remove any partially written file if validation fails

## Validation Checklist

After fixing the bug, verify at least these cases:

- valid download writes a readable `.zst` file
- invalid credentials return an error and do not leave a corrupted local file
- missing object / other failure response does not leave a corrupted local file
- non-zstd response body is rejected before being accepted as a successful download
- progress events are still emitted correctly for successful downloads

## Cleanup for Already-Corrupted Files

This code fix alone does not repair files already written incorrectly.

Repository/users may still need cleanup for:

- files whose `.zst` content is actually XML or another error body
- files that need to be re-downloaded after credentials or connectivity issues are fixed

Operational follow-up may include:

- identifying suspicious `.zst` files that do not start with the zstd magic bytes
- deleting corrupted local copies
- re-downloading known-good versions

## Historical Notes on Resolved Items

### Resolved: Dead Double-Compression Path

The previously documented `CloudFileWriter` double-compression concern is no longer active. That path was dead code and has been removed. The current import compression path is `file_import/src/file_outputter.rs`.

### Resolved: Wrong Import Output Directory

The import flow now uses file-type-specific output directories through:

- `service/src/file_import/service.rs`
- `Settings::get_file_type_path(...)`

### Resolved: Test Credentials Polluting Production Keyring

Credential storage now separates test and production service names in:

- `credentials_storage/src/lib.rs`

This prevents tests from overwriting production credentials.
