# Spec 016: DAT ZIP Checksum Import

## Status
<!-- Planning | In Progress | Complete | Abandoned -->
Planning

## Affected Crates
- `service` — preserve DAT ROM names through the mass-import and file-import models for DAT-driven imports
- `file_import` — import ZIP members by selected SHA1, not only by archive member name, and return the DAT ROM name as the stored original filename

## Problem

DAT-driven mass import currently selects files in two different ways:

1. `service/src/mass_import/with_dat/` identifies candidate source archives by SHA1 checksum.
2. `file_import` then extracts ZIP members by exact filename match.

This split causes a false negative when the DAT ROM name and the ZIP member name differ even
though the file contents are identical. In that case the import model contains the correct
archive path and selected checksum, but ZIP extraction imports zero files and the add-file-set
pipeline fails later with `FileImportError("No files in file set.")`.

For DAT imports, the checksum is the authoritative identity of the ROM. The archive member name
is incidental and may contain typos or naming variations. When a ZIP member matches a selected
DAT checksum, the imported file set should store the DAT ROM name, not the archive member name.

## Proposed Solution

Make ZIP extraction checksum-aware for DAT-driven imports while preserving current behavior for
filename-driven imports.

`service` will continue building `FileSetImportModel` from DAT ROM metadata, but the import model
passed to `file_import` will include enough selected-file metadata to map:

- selected SHA1 checksum
- expected stored file name (the DAT ROM name)

`file_import` will use that metadata when importing ZIP archives:

1. Iterate ZIP members as today.
2. Accept a member when either:
   - its filename matches the selected filename filter, or
   - its computed SHA1 matches one of the selected checksums.
3. When a member is accepted via checksum match, set `ImportedFile.original_file_name` to the DAT
   ROM name from the selected metadata instead of the archive member name.
4. Return only matched selected files; do not import unrelated ZIP members.

This keeps the architecture boundary intact:

- `service` remains responsible for DAT-specific business rules and for deciding the canonical
  filename to store.
- `file_import` remains responsible for archive reading, checksum calculation, and file output,
  with no database awareness.

No database migration is required. The existing `file_set_file_info.file_name` column already
stores `ImportedFile.original_file_name`, so persisting the DAT ROM name only requires changing
the value passed into the existing service/database flow.

## Key Decisions
| Decision | Rationale |
|---|---|
| Use SHA1 as the source of truth for DAT ZIP member selection | DAT import already identifies ROMs by checksum; filename mismatches should not block valid imports |
| Store the DAT ROM name as `original_file_name` when a ZIP member is accepted for a DAT import | The DAT is the canonical catalog entry and should define the persisted file name shown in the file set |
| Keep checksum-aware fallback scoped to selected import metadata rather than adding DAT logic to `file_import` | Preserves crate boundaries by passing generic selection metadata instead of leaking DAT-specific types downward |
| Preserve filename-only behavior for non-DAT imports | Avoids changing unrelated import flows and keeps the fix surgical |
| Fail at the import stage when no ZIP member matches the selected DAT files | Surfaces the real mismatch earlier instead of deferring to the generic `No files in file set` pipeline failure |

## Acceptance Criteria

1. A DAT-driven mass import succeeds when a ZIP archive contains a selected ROM under a different
   archive member name, as long as the member's SHA1 matches the DAT ROM checksum.
2. For such imports, the created file set stores the DAT ROM name as the file name in the file
   set, not the ZIP member name.
3. ZIP members that do not match a selected filename or selected checksum are not imported.
4. Non-DAT imports continue to use the existing filename-driven behavior unchanged.
5. When a DAT-driven import selects a ZIP archive but none of its members match the selected DAT
   ROMs by filename or checksum, the failure is reported from the import stage with an error that
   identifies the selection mismatch instead of only failing later with `No files in file set`.
6. Regression tests cover:
   - checksum-based success with mismatched archive member name
   - persisted DAT filename for the imported file
   - non-DAT filename-driven ZIP import remaining unchanged

## As Implemented
<!-- Filled in at the end of Phase 3. Document any deviations from Proposed Solution. -->
<!-- Change Status to Complete once all code tasks are done and this section is filled. -->
_(Pending)_
