pub mod context;
pub mod pipeline;
pub mod route_and_process_step;
mod steps;

use std::{collections::HashMap, path::PathBuf};

use core_types::{Sha1Checksum, sha1_from_hex_string};
use domain::naming_conventions::no_intro::{DatGame, DatHeader};

use crate::{
    error::Error,
    file_import::model::{
        CreateReleaseParams, DatImportExtras, FileImportSource, FileSetImportModel, ImportFileContent,
    },
};

use self::context::DatFileMassImportContext;

/// Builds a `FileSetImportModel` for a DAT game, matching each ROM to a local file via SHA1.
/// ROMs without a local match are recorded in `dat_extras.missing_files` for future re-runs.
///
/// Returns `Err` if any ROM in the DAT game contains an invalid SHA1 hex string.
fn build_file_set_import_model(
    game: &DatGame,
    header: &DatHeader,
    sha1_to_file_map: &HashMap<Sha1Checksum, PathBuf>,
    context: &DatFileMassImportContext,
) -> Result<FileSetImportModel, Error> {
    let mut import_files_map: HashMap<PathBuf, Vec<ImportFileContent>> = HashMap::new();
    let mut selected_files: Vec<Sha1Checksum> = vec![];
    let mut missing_files: Vec<ImportFileContent> = vec![];

    for rom in &game.roms {
        let sha1: Sha1Checksum = sha1_from_hex_string(&rom.sha1)
            .map_err(|e| Error::ParseError(format!("Invalid SHA1 '{}' in DAT game '{}': {}", rom.sha1, game.name, e)))?;
        if let Some(path) = sha1_to_file_map.get(&sha1) {
            selected_files.push(sha1);
            import_files_map
                .entry(path.clone())
                .or_default()
                .push(ImportFileContent {
                    file_name: rom.name.clone(),
                    sha1_checksum: sha1,
                    file_size: rom.size,
                });
        } else {
            missing_files.push(ImportFileContent {
                file_name: rom.name.clone(),
                sha1_checksum: sha1,
                file_size: rom.size,
            });
        }
    }

    Ok(FileSetImportModel {
        import_files: import_files_map
            .into_iter()
            .map(|(path, contents)| FileImportSource {
                path,
                content: contents.into_iter().map(|c| (c.sha1_checksum, c)).collect(),
            })
            .collect(),
        selected_files,
        system_ids: vec![context.input.system_id],
        file_type: context.input.file_type,
        source: header.get_source(),
        file_set_name: game.name.clone(),
        file_set_file_name: game.name.clone(),
        item_ids: vec![],
        item_types: context
            .input
            .item_type
            .map_or_else(Vec::new, |item_type| vec![item_type]),
        create_release: Some(CreateReleaseParams {
            release_name: game.get_release_name(),
            software_title_name: game.get_software_title_name(),
        }),
        dat_extras: Some(DatImportExtras {
            missing_files,
            dat_file_id: context.state.dat_file_id,
        }),
    })
}
