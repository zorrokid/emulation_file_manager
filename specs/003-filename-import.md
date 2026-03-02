# Filename-Based Mass Import Feature Specification

**Branch**: `003-filename-import`
**Created**: 2026-02-23

## Overview

Add a second import mode to the mass import service that does not require a DAT file. Instead of matching files against a structured XML catalogue, the import derives SoftwareTitle and Release names directly from the filename of each scanned file. This allows users to quickly import a directory of files without a matching DAT.

## Goals

1. **DAT-free import**: Users can run mass import on any directory without requiring a matching DAT file
2. **Automatic naming**: SoftwareTitle and Release names are derived from filenames using existing domain normalisation logic
3. **Reuse existing infrastructure**: File scanning, SHA1 extraction, and file set creation steps are shared with the DAT-based pipeline
4. **No duplicates**: Files whose SHA1 already exists in the database are skipped, consistent with DAT import behaviour
5. **Same entry point**: `MassImportService::import()` selects the appropriate pipeline based on whether `dat_file_path` is provided

## How Filename Derivation Works

The `domain` crate already contains `get_software_title(release_name: &str) -> SoftwareTitle` in `domain/src/title_normalizer/normalizer.rs`:

```rust
pub struct SoftwareTitle {
    pub release_name: String,       // original input (filename without extension)
    pub software_title_name: String, // canonical form (parentheticals stripped, title-cased)
}
```

Examples (filename without extension → derived names):

| Filename (no ext) | `release_name` | `software_title_name` |
|---|---|---|
| `Donkey Kong (USA, Europe) (v1.1)` | `Donkey Kong (USA, Europe) (v1.1)` | `Donkey Kong` |
| `Frogger II - ThreeeDeep! (USA) (Beta)` | `Frogger II - ThreeeDeep! (USA) (Beta)` | `Frogger II - ThreeeDeep!` |
| `Activision Decathlon, The (USA)` | `Activision Decathlon, The (USA)` | `The Activision Decathlon` |
| `simple_game` | `simple_game` | `simple_game` |

**Pipeline for each file:**
1. Strip file extension (e.g. `Donkey Kong (USA).nes` → `Donkey Kong (USA)`)
2. Call `get_software_title(name_without_ext)` to obtain both names
3. Construct one `ImportItem` per file

## Scope

### In scope

- One `FileSet` per file (1:1 mapping, no multi-disk grouping)
- One `Release` per file, using `release_name` as the display name
- One `SoftwareTitle` per file, using `software_title_name` as the canonical name
- Skip files whose SHA1 checksum already exists in the database (existing FileSet detected)
- Progress reporting via the existing `MassImportSyncEvent` channel
- Same `MassImportInput` struct (no new user-facing fields needed; absence of `dat_file_path` triggers this mode)

### Out of scope (for this feature)

- Multi-disk grouping (e.g., grouping `Game (Disk 1).d64` and `Game (Disk 2).d64` into one FileSet)
- Merging with an existing SoftwareTitle or Release if one with the same name already exists — always create new
- UI changes (the existing import UI already sends `dat_file_path: None` when the field is blank)
- DAT linking (no `dat_file_id`, no `DatGameFileSetStatus` tracking)

## Architecture

### Context trait pattern

The two pipelines share a `MassImportContext` **trait** (not the existing concrete struct). Each pipeline gets its own concrete context type:

- **`DatImportContext`** — current concrete struct renamed; holds all DAT-specific state (`dat_file`, `dat_file_id`, `statuses`)
- **`FileNameImportContext`** — new; holds only shared state (file scan results, metadata, import items, results)

Shared steps become generic:

```rust
impl<C: MassImportContext> PipelineStep<C> for ReadFilesStep { ... }
impl<C: MassImportContext> PipelineStep<C> for ReadFileMetadataStep { ... }
impl<C: MassImportContext> PipelineStep<C> for ImportFileSetsStep { ... }
```

DAT-specific steps remain concretely typed against `DatImportContext`.

### `ImportItem` change

`dat_game: DatGame` becomes `dat_game: Option<DatGame>`. It is `None` in all filename-pipeline `ImportItem`s. `ImportFileSetsStep` must handle both cases.

### Pipeline definitions

**Existing `DatImportPipeline` (renamed, unchanged steps):**
```
ImportDatFileStep → CheckExistingDatFileStep → StoreDatFileStep
→ ReadFilesStep → ReadFileMetadataStep → FilterExistingFileSetsStep
→ ImportFileSetsStep → LinkExistingFileSetsStep
```

**New `FileNameImportPipeline`:**
```
ReadFilesStep → ReadFileMetadataStep → BuildImportItemsFromFileNamesStep → ImportFileSetsStep
```

### New step: `BuildImportItemsFromFileNamesStep`

Typed against `FileNameImportContext`. For each entry in `file_metadata`:

1. Extract the filename stem (no extension) using `domain::title_normalizer::rules::extension::strip_extension`
2. Call `domain::title_normalizer::normalizer::get_software_title(stem)` to obtain `release_name` and `software_title_name`
3. Check the database for an existing FileSet containing a file with the same SHA1 — skip if found
4. Construct `ImportItem { dat_game: None, dat_roms_available: vec![], dat_roms_missing: vec![], release_name, software_title_name, file_set: Some(...), status: Pending }`
5. Push to `context.import_items_mut()`

### Service entry point

```rust
// MassImportService::import()
if input.dat_file_path.is_some() {
    let mut ctx = DatImportContext::new(deps, input, ops, progress_tx);
    DatImportPipeline::new().execute(&mut ctx).await;
    ctx.into_result()
} else {
    let mut ctx = FileNameImportContext::new(deps, input, ops, progress_tx);
    FileNameImportPipeline::new().execute(&mut ctx).await;
    ctx.into_result()
}
```

## Domain functions to use

| Function | Location | Purpose |
|---|---|---|
| `get_software_title(name)` | `domain/src/title_normalizer/normalizer.rs` | Derives both names from filename stem |
| `strip_extension(name)` | `domain/src/title_normalizer/rules/extension.rs` | Strips file extension |

Both functions are currently dead code — this feature is their first production use.

## Success Criteria

- `MassImportService::import()` with `dat_file_path: None` runs the filename pipeline without error
- Each file in the source directory results in exactly one FileSet, Release, and SoftwareTitle in the database (unless already imported by SHA1)
- `release_name` matches the filename stem; `software_title_name` is the normalised canonical form
- All existing DAT-import tests continue to pass unchanged
- `cargo clippy --all-targets` produces no new warnings
