use std::{collections::HashMap, path::PathBuf};

use core_types::{Sha1Checksum, sha1_from_hex_string};

use crate::{
    error::Error,
    file_import::model::{FileImportSource, FileSetImportModel, ImportFileContent},
    mass_import::context::{ImportItem, MassImportContext},
    pipeline::pipeline_step::{PipelineStep, StepAction},
};

pub struct ImportDatFileStep;

#[async_trait::async_trait]
impl PipelineStep<MassImportContext> for ImportDatFileStep {
    fn name(&self) -> &'static str {
        "import_dat_file_step"
    }

    fn should_execute(&self, context: &MassImportContext) -> bool {
        context.dat_file_path.is_some()
    }

    async fn execute(&self, context: &mut MassImportContext) -> StepAction {
        let dat_path = context
            .dat_file_path
            .as_ref()
            .expect("Dat file path should be present");

        let parse_res = context.dat_file_parser_ops.parse_dat_file(dat_path);
        match parse_res {
            Ok(dat_file) => {
                println!("Successfully parsed DAT file: {:?}", dat_file);
                context.dat_file = Some(dat_file);
            }
            Err(e) => {
                // Abort since dat file was explicitly provided
                return StepAction::Abort(Error::ParseError(format!(
                    "Failed to parse DAT file {}: {}",
                    dat_path.display(),
                    e
                )));
            }
        }

        StepAction::Continue
    }
}

pub struct ReadFilesStep;
#[async_trait::async_trait]
impl PipelineStep<MassImportContext> for ReadFilesStep {
    fn name(&self) -> &'static str {
        "read_files_step"
    }
    async fn execute(&self, context: &mut MassImportContext) -> StepAction {
        let files_res = context.fs_ops.read_dir(context.source_path.as_path());
        let files = match files_res {
            Ok(files) => files,
            Err(e) => {
                return StepAction::Abort(Error::IoError(format!(
                    "Failed to read source path {}: {}",
                    context.source_path.display(),
                    e
                )));
            }
        };

        for file_res in files {
            match file_res {
                Ok(file) => {
                    tracing::info!("Found file: {}", file.path.display());
                    context.files.push(file.path.clone());
                }
                Err(e) => {
                    tracing::error!(
                        error = ?e,
                        path = %context.source_path.display(),
                        "Failed to read a file entry"
                    );
                    context.failed_files.push(context.source_path.clone());
                }
            }
        }

        // Implementation for reading files goes here
        StepAction::Continue
    }
}

pub struct ReadFileMetadataStep;

#[async_trait::async_trait]
impl PipelineStep<MassImportContext> for ReadFileMetadataStep {
    fn name(&self) -> &'static str {
        "read_file_metadata_step"
    }

    fn should_execute(&self, context: &MassImportContext) -> bool {
        !context.get_non_failed_files().is_empty()
    }

    async fn execute(&self, context: &mut MassImportContext) -> StepAction {
        tracing::info!(
            len = %context.get_non_failed_files().len(),
            "Reading metadata for files...",
        );
        for file in &mut context.get_non_failed_files() {
            tracing::info!("Creating metadata reader for file: {}", file.display());
            let reader_res = (context.reader_factory_fn)(file);
            match reader_res {
                Ok(reader) => {
                    tracing::info!(
                        file = %file.display(),
                        "Successfully created metadata reader",
                    );
                    let res = reader.read_metadata();
                    tracing::info!(
                        file = %file.display(),
                        "Successfully read metadata",
                    );
                    match res {
                        Ok(metadata_entries) => {
                            context.file_metadata.insert(file.clone(), metadata_entries);
                        }
                        Err(e) => {
                            tracing::error!(
                                error = ?e,
                                file = %file.display(),
                                "Failed to read metadata",
                            );
                            context.failed_files.push(file.clone());
                        }
                    }
                }
                Err(e) => {
                    tracing::error!(
                        error = ?e,
                        file = %file.display(),
                        "Failed to create metadata reader",
                    );
                    context.failed_files.push(file.clone());
                }
            }
        }
        StepAction::Continue
    }
}

pub struct MapDatEntriesToImportItemsStep;

#[async_trait::async_trait]
impl PipelineStep<MassImportContext> for MapDatEntriesToImportItemsStep {
    fn name(&self) -> &'static str {
        "map_dat_entries_to_import_items_step"
    }
    fn should_execute(&self, context: &MassImportContext) -> bool {
        context.dat_file.is_some() && !context.file_metadata.is_empty()
    }

    async fn execute(&self, context: &mut MassImportContext) -> StepAction {
        // Implementation for mapping DAT entries to import items goes here
        println!("Mapping DAT entries to import items...");
        let dat_file = context
            .dat_file
            .as_ref()
            .expect("DAT file should be present");
        let dat_games = dat_file.games.clone();

        let sha1_to_file_map = context.build_sha1_to_file_map();

        for game in &dat_games {
            println!("DAT Game: {:?}", game);

            let mut import_item = ImportItem::new(game.clone());
            let mut import_files: HashMap<PathBuf, Vec<ImportFileContent>> = HashMap::new();
            for rom in &game.roms {
                let sha1_bytes_res: Sha1Checksum =
                    sha1_from_hex_string(&rom.sha1).expect("Invalid SHA1 in DAT");

                if let Some(source_file) = sha1_to_file_map.get(&sha1_bytes_res) {
                    println!(
                        "Matched ROM SHA1 {} to source file {}",
                        rom.sha1,
                        source_file.display()
                    );
                    import_item.dat_roms_available.push(rom.clone());
                    import_files
                        .entry(source_file.clone())
                        .or_insert_with(Vec::new)
                        .push(ImportFileContent {
                            file_name: rom.name.clone(),
                            sha1_checksum: sha1_bytes_res,
                            file_size: rom.size,
                        });
                } else {
                    println!("No matching source file found for ROM SHA1 {}", rom.sha1);
                    import_item.dat_roms_missing.push(rom.clone());
                }
            }

            let item_types = context
                .item_type
                .map_or_else(Vec::new, |item_type| vec![item_type]);

            let import_files: Vec<FileImportSource> = import_files
                .into_iter()
                .map(|(path, contents)| FileImportSource {
                    path,
                    content: contents
                        .iter()
                        .map(|c| (c.sha1_checksum, c.clone()))
                        .collect(),
                })
                .collect();

            import_item.file_set = Some(FileSetImportModel {
                import_files,
                selected_files: vec![],

                system_ids: vec![context.system_id],
                file_type: context.file_type,

                source: format!("{} {}", dat_file.header.name, dat_file.header.version),
                file_set_name: game.name.clone(),
                file_set_file_name: game.name.clone(),

                item_ids: vec![],
                item_types,
            });
            context.import_items.push(import_item);
        }
        StepAction::Continue
    }
}
