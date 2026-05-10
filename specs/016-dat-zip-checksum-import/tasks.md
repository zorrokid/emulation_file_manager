## Phase 3 — Implementation

### Service
- [x] T1 [service] — Preserve DAT ROM metadata through file import models
  **File:** `service/src/file_import/model.rs`
  Extend the import model passed to `file_import` so selected entries carry only the selected SHA1 checksum and the canonical file name needed for persistence.

### File Import
- [x] T2 [file_import] — Add staged, checksum-aware ZIP member selection
  **File:** `file_import/src/lib.rs`
  Stage ZIP members to temp first, compute SHA1 while staging, and accept members only when the staged member's SHA1 matches selected import metadata.

- [x] T3 [file_import] — Preserve DAT ROM name for checksum-matched ZIP imports
  **File:** `file_import/src/lib.rs`
  When a ZIP member is accepted through checksum matching, store the canonical service-provided name as `ImportedFile.original_file_name` instead of the archive member name.

- [x] T4 [file_import] — Fail early when a DAT-selected ZIP imports no matching members
  **File:** `file_import/src/lib.rs`
  Return an import-stage error that identifies the ZIP-selection mismatch instead of only failing later with `No files in file set`, and ensure unmatched staged members do not produce collection output.

## Phase 5 — Tests

- [x] T5 [file_import] — Add checksum-based ZIP import regression test
  **File:** `file_import/src/lib.rs`
  Verify a DAT-selected ROM imports successfully when the ZIP member name differs from the DAT ROM name but the SHA1 checksum matches.

- [x] T6 [service] — Verify canonical file name is persisted through the service/database flow
  **File:** `service/src/file_import/`
  Add service-level coverage that confirms `FileImportService` persists the canonical service-provided file name to the created file set / `file_info` records, not just that `file_import` returned it.

- [x] T7 [file_import] — Verify ZIP selection mismatch fails at import stage
  **File:** `file_import/src/lib.rs`
  Confirm a selected ZIP with no members matching the selected SHA1 entries returns the earlier, specific import-stage error.

## Manual Verification Checklist

- [ ] Import a DAT-selected ZIP whose member name differs from the DAT ROM name but whose checksum matches.
- [ ] Confirm the resulting file set stores and shows the DAT ROM name.
- [ ] Confirm unrelated ZIP members are not imported.
