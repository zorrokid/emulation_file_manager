# Import Flow

This document describes the domain logic of the mass import system — how files are scanned,
matched to DAT game definitions, stored, and tracked across multiple import runs. It is intended
as a reference for writing tests and for extending the import logic with new features.

> **Note:** The diagram in `mass_import_flow.dot` / `mass_import_flow.svg` reflects an older
> version of the pipeline and should be updated to match the current implementation.

---

## Two Import Modes

The system supports two modes, selectable by whether a DAT file path is provided:

| Mode | Entry point | Pipeline |
|---|---|---|
| **DAT-guided** | `MassImportService::import_with_dat` | 7-step `Pipeline<DatFileMassImportContext>` |
| **Files-only** | `MassImportService::import_files_only` | 4-step `Pipeline<FilesOnlyMassImportContext>` |

Files-only mode is simpler: scan → read metadata → filter already-existing → import new sets.
The rest of this document focuses on DAT-guided mode, which is significantly more complex.

---

## Core Concepts

### Identity: SHA1-based matching

A file set is identified by the set of SHA1 checksums of its files. Two file sets are considered
the same if they have the same SHA1s for the same file type. The DAT file provides the expected
SHA1s; the local scan provides what is actually present on disk.

### `file_info` is shared across file sets

`file_info` records are not owned by a single file set. Multiple file sets can reference the same
physical file (same SHA1 + file type). When a new file set is imported:
- If a `file_info` record already exists for that SHA1, it is reused — no duplicate is created.
- The `file_set_file_info` junction table links file sets to file info records.

This means a file physically imported for one game is automatically available for another game
that shares the same ROM file.

### `is_available` flag

Each `file_info` record has an `is_available` flag. This is set to `false` when a game is first
imported but some of its files are not yet present locally. When a re-run later finds those files,
the existing record is updated in-place (`UPDATE is_available = 1`) — a new record is never
created.

### DAT extras: recording missing files

When a new file set is imported with some ROMs missing, those missing ROMs are stored as
`DatImportExtras.missing_files` (as `Sha1Checksum` + file name + size). On subsequent import
runs, these stored SHA1s are cross-referenced against the current local scan to detect newly
available files.

---

## DAT-Guided Pipeline

### Overview of the 7 steps

```
ImportDatFileStep
  → CheckExistingDatFileStep
    → StoreDatFileStep
      → ReadFilesStep
        → ReadFileMetadataStep
          → CategorizeFileSetsForImportStep
            → RouteAndProcessFileSetsStep
```

### Step 1: ImportDatFileStep

Parses the DAT file at `input.dat_file_path` using the DAT parser and stores the result in
context state. Aborts the pipeline on parse failure.

**Skip condition:** No DAT file path provided.

### Step 2: CheckExistingDatFileStep

Checks whether the parsed DAT file already exists in the database, matched by name, version, and
system. If found, stores the existing `dat_file_id` in context state.

**Purpose:** Prevents inserting duplicate DAT file records on repeated imports.

### Step 3: StoreDatFileStep

Inserts the DAT file into the database and stores the new ID in context state.

**Skip condition:** DAT file already exists in the database (ID already set by step 2).

### Step 4: ReadFilesStep

Scans `input.source_path` recursively and collects all readable file paths. Files that fail to
open are tracked separately (`read_failed_files`). Directory scan errors are also collected but
do not abort the pipeline.

### Step 5: ReadFileMetadataStep

For each file collected in step 4, computes the SHA1 checksum and file size. Archives (zip) are
handled transparently — metadata is extracted for the files *inside* the archive, not the archive
itself. The result is a `HashMap<PathBuf, Vec<ReadFile>>` mapping file paths to their contents.

**Skip condition:** No readable files found in step 4.

### Step 6: CategorizeFileSetsForImportStep

For each game defined in the DAT, queries the database to determine its current status. The
resulting `Vec<DatGameFileSetStatus>` is stored in context state and drives the routing in step 7.

**Three possible statuses (see below for details):**
- `NonExisting` — no matching file set in the database
- `ExistingWithReleaseAndLinkedToDat` — file set exists and is already linked to this DAT
- `ExistingWithoutReleaseAndWithoutLinkToDat` — file set exists but is not yet linked to this DAT

**Skip condition:** No file metadata available, or DAT not parsed, or no DAT file ID.

### Step 7: RouteAndProcessFileSetsStep

Iterates over the statuses from step 6 and routes each game to the appropriate handler. A
progress event is sent per game after handling. Each handler returns `(Option<i64>,
FileSetImportStatus)` — the file set ID (if known) and the outcome.

---

## DAT Game Status Classification

`DatGameStatusService` classifies each DAT game by querying the database with
`FileSetEqualitySpecs` (built from the game's ROM SHA1s and file type). The classification logic:

1. **Find file set by SHA1s.** If no match → `NonExisting`.
2. **If a match is found**, check whether it is linked to the current DAT file.
   - Linked → `ExistingWithReleaseAndLinkedToDat` (with the list of missing SHA1s, if any)
   - Not linked → `ExistingWithoutReleaseAndWithoutLinkToDat` (with missing SHA1s)

A file set is considered a match even if some of its files are marked `is_available = false` —
the match is based on the declared SHA1s, not on physical availability.

---

## Routing: Four Cases

### Case 1: New file set (`NonExisting`)

The game has never been imported. A full import is performed:

1. Build `FileSetImportModel` from the DAT game:
   - ROMs present in the local scan → added to `import_files` and `selected_files`
   - ROMs absent from the local scan → recorded in `dat_extras.missing_files`
2. Call `FileImportService::create_file_set` which runs the **add-file-set pipeline**.
3. Return `Success` if all ROMs were present, `SuccessWithWarnings` if some were missing.

**Edge cases:**
- All ROMs present → `Success`
- Some ROMs missing → file set is created with those files marked `is_available = false`;
  `SuccessWithWarnings` lists the missing file names
- All ROMs missing → file set is still created (as a placeholder), all files marked unavailable;
  `SuccessWithWarnings` with all files listed as missing
- Import pipeline step failures → `SuccessWithWarnings` includes both step errors and missing
  file warnings <= TODO is this clear enough what it does, what failed? Is it really always success?

### Case 2: Already complete (`ExistingWithReleaseAndLinkedToDat`, no missing files)

The file set is fully imported and already linked to the current DAT. Nothing to do.

Returns `AlreadyExists`. No database writes occur.

### Case 3: Linked but incomplete (`ExistingWithReleaseAndLinkedToDat`, with missing files)

The file set is linked to this DAT but was previously imported with some files missing. This
case handles re-runs where more local files may have become available.

Delegates to `complete_missing_files` (see below).

### Case 4: Existing but not linked (`ExistingWithoutReleaseAndWithoutLinkToDat`)

A file set matching the DAT game exists in the database (possibly imported without a DAT, or
from a different DAT), but it is not yet linked to the current DAT file.

1. Call `FileSetService::create_release_for_file_set` to link the file set to the current DAT
   and create a release + software title if needed.
2. If the file set also has missing files → delegate to `complete_missing_files`.

**Edge case — link fails:** If `create_release_for_file_set` returns an error (e.g., database
constraint), the game is recorded as `Failed` and processing continues with the next game.

---

## `complete_missing_files`: Re-run Completion Logic

Called by cases 3 and 4 when a file set has previously-recorded missing files.

1. **Partition** the stored `missing_files` SHA1s against the current local scan (`sha1_map`):
   - `newly_available` — SHA1s now present on disk
   - `still_missing` — SHA1s still absent

2. **If nothing is newly available** → return `StillMissingFiles` with a list of the still-absent
   file names. No database writes occur.

3. **If some are newly available** → build `UpdateFileSetModel` from the `newly_available` SHA1s
   and call `FileImportService::update_file_set` which runs the **update-file-set pipeline**.

4. Return status:
   - All files now present → `Success`
   - Some files still missing → `SuccessWithWarnings` listing the remaining missing files
   - Update pipeline failed → `Failed`

**Edge case — SHA1 no longer in DAT game:** If a newly-available SHA1 cannot be matched back to
a ROM in `game.roms` (can happen if the DAT file changed between the initial import and the
re-run), that SHA1 is skipped with a warning log. Only matched SHA1s are included in the update
model. <= TODO: DAT file shouldn't change between impots, since the DAT files are versioned?

---

## Sub-Pipeline: Add File Set

Called by `FileImportService::create_file_set`. Runs once per new file set.

| Step | Purpose | Skip condition |
|---|---|---|
| CheckExistingFilesStep | Query DB for `file_info` records matching selected SHA1s (excluding SHA1s already linked to this file set) | No selected SHA1s |
| CheckExistingFileSetStep | Check if a file set with this name already exists | — |
| ImportFilesStep | Copy physical files to the collection directory | No new files to import (`needs_file_info_upsert` is false) |
| CreateFileSetToDatabaseStep | Insert `file_set`, `file_info`, `release`, `software_title` records | — |
| AddFileSetItemTypesStep | Associate `item_type` records with the file set | No item types specified |

**`file_info` deduplication:** `file_set_repository.save` checks for an existing `file_info`
record by `(sha1_checksum, file_type)` before inserting. If a record already exists:
- If `file.is_available = true`: update `is_available = 1` on the existing record
- Either way: reuse the existing record's ID — no duplicate is inserted

---

## Sub-Pipeline: Update File Set

Called by `FileImportService::update_file_set`. Runs when newly-available files are added to an
existing file set.

| Step | Purpose | Skip condition |
|---|---|---|
| FetchFileSetStep | Load current file set from DB | — |
| FetchFilesInFileSetStep | Load all `file_info` records currently linked to this file set | — |
| UpdateFileInfoToDatabaseStep | Insert or restore `file_info` records | `needs_file_info_upsert` is false, or no imported files, or file set not loaded |
| UpdateFileSetFilesStep | Link new `file_info` records to the file set | No new file_info IDs to link |
| CollectDeletionCandidatesStep | Identify files removed from the selected set | No removed files |
| UnlinkFilesFromFileSetStep | Remove unlinked files from `file_set_file_info` | No removed files |

### `UpdateFileInfoToDatabaseStep` — insert vs. update logic

For each imported file, the step checks whether a `file_info` record for that SHA1 is already
linked to this file set (in `files_in_file_set`) with `is_available = false`:

- **Already linked, unavailable** → call `update_is_available(id)` to restore it. No new record
  or new link is created.
- **Not yet in this file set** → insert a new `file_info` record. The new ID will be linked in
  `UpdateFileSetFilesStep`.

### `UpdateFileSetFilesStep` — linking new records

Only file_info IDs that are not already linked to this file set are passed to
`add_files_to_file_set`. Already-linked IDs are excluded in `get_file_info_ids_with_file_names`
by checking against `files_in_file_set` SHA1s before the lookup — this prevents a primary key
violation on `file_set_file_info(file_set_id, file_info_id)`.

---

## Import Status Values

| Status | Meaning | Triggered by |
|---|---|---|
| `Success` | All files imported successfully | New import with all ROMs present; re-run that completes a previously partial import |
| `SuccessWithWarnings` | Import succeeded but some files are missing or some pipeline steps failed | New import with missing ROMs; re-run that imports some but not all missing ROMs |
| `StillMissingFiles` | Re-run attempted but no new files were found locally | Re-run where `newly_available` partition is empty |
| `AlreadyExists` | File set is complete and already linked — nothing to do | Case 2 (fully complete, linked) |
| `Failed` | A critical error prevented import or linking | DAT link failure; `create_file_set` error |

---

## Progress Reporting

Import progress is reported in real time via a `flume` channel (`progress_tx`). After each game
is handled by `RouteAndProcessFileSetsStep`, a `MassImportSyncEvent` is sent containing the game
name and its `FileSetImportStatus`. The UI component (`ImportForm`) receives these events and
updates a live progress list.

Results are also accumulated in `context.state.common_state.import_results` and returned as
`DatFileMassImportResult` when the pipeline completes.

---

## Edge Case Summary

| Edge case | Where handled | Outcome |
|---|---|---|
| DAT file already in DB | `CheckExistingDatFileStep` | Reuse existing ID; `StoreDatFileStep` is skipped |
| File fails to read from disk | `ReadFilesStep` | Tracked in `read_failed_files`; pipeline continues |
| ROM in DAT but not on disk (first import) | `handle_new_file_set` | File set created with `is_available = false`; SHA1 stored in `dat_extras.missing_files` |
| ROM on disk but not yet imported (re-run) | `complete_missing_files` | SHA1 partitioned into `newly_available`; `update_file_set` called |
| ROM still not on disk (re-run) | `complete_missing_files` | Stays in `still_missing`; `StillMissingFiles` status |
| ROM in DAT no longer matches any `game.roms` (DAT changed) | `build_update_model` | SHA1 skipped with warning log; only matched SHA1s used |
| File set exists but not linked to this DAT | `handle_link_existing_to_dat` | Linked via `create_release_for_file_set` |
| Linking to DAT fails | `handle_link_existing_to_dat` | `Failed` status; next game continues |
| Re-inserting already-linked `file_info` (PK violation risk) | `get_file_info_ids_with_file_names` | Skipped by SHA1 check against `files_in_file_set` before id lookup |
| `file_info` shared by multiple file sets | `file_set_repository.save` | Existing record reused; `is_available` updated if needed |
| All ROMs missing on first import | `handle_new_file_set` | File set created as placeholder; `SuccessWithWarnings` |
