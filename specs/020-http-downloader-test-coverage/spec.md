# 020 — HTTP Downloader Test Coverage

## Status
Complete

## Affected Crates
- `http_downloader` — add unit and integration-style test coverage for filename parsing, error handling, progress reporting, and cancellation behavior

## Problem

`http_downloader` now contains production logic for streamed reqwest downloads, filename extraction from
`Content-Disposition`, filename normalization, partial-file cleanup, and progress event reporting, but it
has only one small unit test and several empty test placeholders.

That leaves the crate exposed to regressions in the riskiest paths:

1. successful downloads may stop emitting the expected event sequence
2. cancellation and error paths may leave partial files behind
3. malformed or missing filename metadata may produce the wrong file name or the wrong error
4. HTTP status handling may regress and save non-success responses as files

Because this crate is a reusable library used by higher layers, these behaviors should be verified at the
crate boundary instead of relying on GUI or service tests to catch failures indirectly.

## Proposed Solution

Add a focused downloader test suite with two layers of coverage:

1. **Pure helper unit tests** inside `http_downloader/src/lib.rs`
   - cover `extract_filename_from_url`
   - cover `normalize_filename`
   - cover `extract_filename_from_headers_reqwest` with valid, invalid, UTF-8, ASCII, and path-like inputs

2. **Integration-style async tests** under `http_downloader/tests/`
   - start an in-process Axum server on an ephemeral localhost port
   - exercise the public `download_file` API against real streamed HTTP responses
   - verify emitted `HttpDownloadEvent` values, resulting files, cleanup behavior, and public errors

Test infrastructure should live in the `http_downloader` crate itself and run under normal `cargo test`
in CI. No external services or repository-wide test harness should be required.

The integration server should support:

- static success responses with and without `Content-Disposition`
- malformed `Content-Disposition` values
- non-success status responses
- multi-chunk responses to verify progress and cancellation

## Key Decisions

| Decision | Rationale |
|---|---|
| Use Axum as an in-process test server | The downloader needs real streamed response behavior, not only stubbed request matching |
| Keep parsing/normalization tests separate from HTTP tests | Pure helper logic is easier to verify exhaustively without network setup |
| Test the public `download_file` API instead of mocking `reqwest::Client` | This verifies the real reqwest integration and event behavior at the crate boundary |
| Run async integration tests with Tokio | The downloader implementation already uses reqwest and Tokio I/O, and Axum integrates naturally with Tokio in test code |
| Keep all test infrastructure local to `http_downloader` | CI can run `cargo test -p http_downloader` without extra services or setup steps |

## Acceptance Criteria

- [x] `http_downloader` has unit tests covering URL filename extraction, filename normalization, and `Content-Disposition` parsing.
- [x] `http_downloader` has integration-style tests covering a successful download that emits `Started`, at least one `Progress`, and `Completed`.
- [x] A missing `Content-Disposition` header falls back to the URL filename in tests.
- [x] An invalid `Content-Disposition` header returns a filename-related error in tests.
- [x] A non-success HTTP response returns a request error and does not leave a downloaded file behind.
- [x] A cancelled download returns `DownloadError::Cancelled` and removes the partial file.
- [x] Tests run with `cargo test -p http_downloader` in CI without external services.
- [x] Added test code follows the project naming convention `test_<unit>_<scenario>_<expected_outcome>`.

## As Implemented
Added unit tests in `http_downloader/src/lib.rs` for URL filename extraction, filename
normalization, and `Content-Disposition` parsing against real `reqwest::Response` values.

Added Axum-backed integration tests in `http_downloader/tests/download_file.rs` covering successful
downloads, URL fallback when `Content-Disposition` is missing, malformed header failures, non-2xx
HTTP failures, and cancellation cleanup. Test infrastructure runs entirely in-process with
`cargo test -p http_downloader`.
