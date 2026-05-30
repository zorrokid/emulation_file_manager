# 020 — HTTP Downloader Test Coverage: Task Breakdown

## Phase 3 — Implementation

### Test Harness

- [x] T1 [http_downloader] — Add downloader integration-test dependencies and server harness
  **File:** `http_downloader/Cargo.toml`, `http_downloader/tests/download_file.rs`
  Add test-only dependencies needed for an in-process Axum server and temporary directories, then
  create a reusable test server helper that binds to an ephemeral localhost port and serves
  deterministic downloader fixtures.

- [x] T2 [http_downloader] — Add streamed-response fixtures for downloader behavior
  **File:** `http_downloader/tests/download_file.rs`
  Implement handlers/fixtures for successful downloads, missing-header downloads, malformed-header
  downloads, non-success responses, and multi-chunk responses suitable for cancellation and progress
  assertions.

### Unit Tests

- [x] T3 [http_downloader] — Expand URL filename extraction coverage
  **File:** `http_downloader/src/lib.rs`
  Add unit tests covering plain filenames, nested paths, query strings, trailing slashes, and URLs
  without a usable filename segment.

- [x] T4 [http_downloader] — Add filename normalization coverage
  **File:** `http_downloader/src/lib.rs`
  Add unit tests covering valid names, path-like header values, whitespace trimming, empty names,
  `.` / `..`, and names that sanitize to an unusable result.

- [x] T5 [http_downloader] — Add `Content-Disposition` parsing coverage
  **File:** `http_downloader/src/lib.rs`
  Add unit tests for ASCII `filename=`, UTF-8 `filename*=`, precedence of `filename*` over
  `filename`, malformed header content, invalid UTF decoding, and headers with no usable filename.

## Phase 5 — Tests

- [x] T6 [http_downloader] — Add successful download event-flow coverage
  **File:** `http_downloader/tests/download_file.rs`
  Verify `download_file` stores the response body, chooses the header filename when present, and
  emits `Started`, at least one `Progress`, and `Completed` in order.

- [x] T7 [http_downloader] — Add URL-fallback coverage for missing header
  **File:** `http_downloader/tests/download_file.rs`
  Verify `download_file` derives the target filename from the URL when no `Content-Disposition`
  header is present.

- [x] T8 [http_downloader] — Add malformed-header failure coverage
  **File:** `http_downloader/tests/download_file.rs`
  Verify `download_file` returns `DownloadError::FilenameError(...)` when the response includes an
  invalid `Content-Disposition` value.

- [x] T9 [http_downloader] — Add non-success response coverage
  **File:** `http_downloader/tests/download_file.rs`
  Verify `download_file` returns a request failure for non-2xx responses and does not leave a file
  in the target directory.

- [x] T10 [http_downloader] — Add cancellation and partial-file cleanup coverage
  **File:** `http_downloader/tests/download_file.rs`
  Verify cancelling a multi-chunk download returns `DownloadError::Cancelled`, leaves no partial
  file behind, and does not emit `Completed`.

## Manual Verification Checklist

- [x] Run `cargo test -p http_downloader`.
- [x] Confirm the new integration tests do not require any external HTTP server or CI service setup.
- [x] Confirm test names follow `test_<unit>_<scenario>_<expected_outcome>`.
- [x] Confirm temporary files and test servers are cleaned up automatically by the test process.
