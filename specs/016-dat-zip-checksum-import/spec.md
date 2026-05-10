# Spec 016: DAT ZIP Checksum Import

## Status
<!-- Planning | In Progress | Complete | Abandoned -->
In Progress

## Affected Crates
- `service` — preserve canonical file names through the file-import model while selecting files solely by SHA1
- `file_import` — import ZIP members by selected SHA1 only and return the canonical service-provided file name as the stored original filename

## Problem

ZIP import currently mixes two selection concepts:

1. `service` already tracks selected files by SHA1 checksum.
2. `file_import` still contains ZIP-member filename filtering logic.

That split makes the import contract more complicated than it needs to be and is especially
problematic for DAT-driven imports, where the checksum is already the authoritative identity.
When the canonical file name from `service` differs from the ZIP member name, the current design
forces `file_import` to care about an incidental archive detail instead of the selected checksum.

The import boundary should be simpler: `service` decides which SHA1 checksums are selected and
which file name should be stored for each one, while `file_import` only finds matching ZIP members
by SHA1 and persists the file under the provided canonical name.

## Proposed Solution

Make ZIP extraction SHA1-driven for all imports.

`service` will continue building `FileSetImportModel` from import metadata, but the import model
passed to `file_import` will include one selected entry per chosen SHA1. That selection metadata is
generic to all imports and captures only:

- the selected SHA1 checksum
- the canonical file name to persist for that checksum

`file_import` will consume that selection metadata through a single ZIP import flow with no
filename-based filtering.

When importing ZIP archives:

1. Iterate ZIP members as today.
2. Extract each ZIP member to a temp/staging location first instead of writing directly to the collection output directory.
3. While staging the member, compute its SHA1 checksum.
4. Match the staged member against the selected import metadata by SHA1 checksum only.
5. When a member is accepted, create the final imported output from the staged file and set `ImportedFile.original_file_name` to the canonical stored file name from the selected metadata, not the archive member name.
6. When a member is not accepted, delete the staged file and do not create collection output for it.
7. Return only matched selected files; do not import unrelated ZIP members.

This keeps the architecture boundary intact:

- `service` remains responsible for business rules and for deciding the canonical filename to store.
- `file_import` remains responsible for archive reading, checksum calculation, and file output,
  with no database awareness.

No database migration is required. The existing `file_set_file_info.file_name` column already
stores `ImportedFile.original_file_name`, so persisting the DAT ROM name only requires changing
the value passed into the existing service/database flow.

## Key Decisions
| Decision | Rationale |
|---|---|
| Use SHA1 as the only ZIP member selection key | `service` already selects files by checksum, and keeping a second filename filter adds complexity without adding correctness |
| Store the canonical service-provided file name as `original_file_name` when a ZIP member is accepted | The service layer owns naming decisions; `file_import` should persist the chosen name without deriving one from the archive |
| Use a typed selection model instead of filename-filter APIs | Keeps ZIP import logic in one place and lets `service` pass generic metadata without DAT-specific coupling |
| Stage ZIP members in temp before final persistence | Avoids writing unconfirmed files into the collection directory and keeps ZIP selection semantics consistent |
| Fail at the import stage when no ZIP member matches the selected SHA1 entries | Surfaces the real mismatch earlier instead of deferring to the generic `No files in file set` pipeline failure |

## Acceptance Criteria

1. A DAT-driven mass import succeeds when a ZIP archive contains a selected ROM under a different
   archive member name, as long as the member's SHA1 matches the selected DAT checksum.
2. For such imports, the created file set stores the canonical service-provided file name in the
   file set, not the ZIP member name.
3. ZIP members that do not match any selected import metadata by SHA1 are not persisted to the
   collection output.
4. ZIP imports outside the DAT flow also select members by SHA1 only through the same import model.
5. When an import selects a ZIP archive but none of its members match the selected SHA1 entries,
   the failure is reported from the import stage with an error that identifies the selection
   mismatch instead of only failing later with `No files in file set`.
6. Regression tests cover:
   - checksum-based success with mismatched archive member name
   - persisted canonical filename through the service/database flow
   - import-stage failure when no ZIP member matches the selected SHA1 entries

## As Implemented
<!-- Filled in at the end of Phase 3. Document any deviations from Proposed Solution. -->
<!-- Change Status to Complete once all code tasks are done and this section is filled. -->
_(Pending)_
