use std::{collections::HashMap, path::PathBuf};

use core_types::{Sha1Checksum, sha1_bytes_to_hex_string, sha1_from_hex_string};
use domain::naming_conventions::no_intro::{DatFile, DatGame};

use crate::{
    dat_game_status_service::DatGameFileSetStatus,
    file_import::model::{
        CreateReleaseParams, FileImportSource, ImportFileContent, UpdateFileSetModel,
    },
    mass_import::{
        models::{FileSetImportResult, FileSetImportStatus, MassImportSyncEvent},
        with_dat::context::DatFileMassImportContext,
    },
    pipeline::pipeline_step::{PipelineStep, StepAction},
};

/// Routes each DAT game status to the appropriate handler function.
///
/// Replaces the old `ImportFileSetsStep` + `HandleExistingFileSetsStep` pair with a single
/// explicit `match` over `DatGameFileSetStatus` variants. Every routing branch is visible in
/// one place, making it easy to understand and extend the handling for each case.
pub struct RouteAndProcessFileSetsStep;

#[async_trait::async_trait]
impl PipelineStep<DatFileMassImportContext> for RouteAndProcessFileSetsStep {
    fn name(&self) -> &'static str {
        "route_and_process_file_sets_step"
    }

    fn should_execute(&self, context: &DatFileMassImportContext) -> bool {
        !context.state.dat_game_statuses.is_empty()
            && context.state.dat_file.is_some()
            && context.state.dat_file_id.is_some()
    }

    async fn execute(&self, context: &mut DatFileMassImportContext) -> StepAction {
        // Move statuses out of context so we can pass `&mut context` into handlers
        // inside the loop without holding an immutable borrow. mem::take leaves an
        // empty Vec in place — dat_game_statuses is not read again after this step.
        let statuses = std::mem::take(&mut context.state.dat_game_statuses);
        let dat_file = context
            .state
            .dat_file
            .clone()
            .expect("DAT file should be present in state after should_execute guard");
        let sha1_map = context.scanned_files_by_sha1();

        for status in &statuses {
            let game_name = status.game().name.clone();
            let (file_set_id, import_status) = match status {
                DatGameFileSetStatus::NonExisting(game) => {
                    handle_new_file_set(game, &dat_file, &sha1_map, context).await
                }
                DatGameFileSetStatus::ExistingWithoutReleaseAndWithoutLinkToDat {
                    file_set_id,
                    game,
                    missing_files,
                } => {
                    handle_link_existing_to_dat(
                        *file_set_id,
                        game,
                        missing_files,
                        &sha1_map,
                        context,
                    )
                    .await
                }
                // File set is fully imported and linked — nothing to do.
                DatGameFileSetStatus::ExistingWithReleaseAndLinkedToDat {
                    file_set_id,
                    game,
                    missing_files,
                } if missing_files.is_empty() => handle_already_complete(*file_set_id, game),
                // File set is linked but has files that were missing at last import; check if
                // any are now locally available so we can complete the import.
                DatGameFileSetStatus::ExistingWithReleaseAndLinkedToDat {
                    file_set_id,
                    game,
                    missing_files,
                } => {
                    handle_existing_with_missing_files(
                        *file_set_id,
                        game,
                        missing_files,
                        &sha1_map,
                        context,
                    )
                    .await
                }
            };
            record_and_send(file_set_id, &game_name, import_status, context);
        }

        StepAction::Continue
    }
}

/// Imports a brand-new file set for a DAT game.
///
/// Each ROM in the game is looked up by SHA1 in the local file scan:
/// - ROMs with a local match are added to `import_files` and `selected_files`.
/// - ROMs without a match are recorded as `missing_files` in the DAT extras so the
///   import can be completed on a later re-run.
///
/// Returns `Success` when all ROMs are present, `SuccessWithWarnings` when some are missing.
async fn handle_new_file_set(
    game: &DatGame,
    dat_file: &DatFile,
    sha1_map: &HashMap<Sha1Checksum, PathBuf>,
    context: &DatFileMassImportContext,
) -> (Option<i64>, FileSetImportStatus) {
    tracing::info!(
        game = game.name.as_str(),
        "Importing new file set from DAT game"
    );

    let model = match super::build_file_set_import_model(game, &dat_file.header, sha1_map, context)
    {
        Ok(m) => m,
        Err(e) => {
            tracing::error!(game = game.name.as_str(), error = %e, "Invalid SHA1 in DAT game — skipping");
            return (None, FileSetImportStatus::Failed(e.to_string()));
        }
    };

    let missing_file_warnings: Vec<String> = model
        .dat_extras
        .as_ref()
        .map(|e| {
            e.missing_files
                .iter()
                .map(|f| format!("Missing file: {}", f.file_name))
                .collect()
        })
        .unwrap_or_default();

    let result = context
        .ops
        .file_import_service_ops
        .create_file_set(model)
        .await;

    match result {
        Ok(import_result) if import_result.failed_steps.is_empty() => {
            tracing::info!(
                game = game.name.as_str(),
                file_set_id = import_result.file_set_id,
                "Successfully imported new file set",
            );
            let status = if missing_file_warnings.is_empty() {
                FileSetImportStatus::Success
            } else {
                FileSetImportStatus::SuccessWithWarnings(missing_file_warnings)
            };
            (Some(import_result.file_set_id), status)
        }
        Ok(import_result) => {
            tracing::warn!(
                game = game.name.as_str(),
                file_set_id = import_result.file_set_id,
                "File set imported but some pipeline steps failed",
            );
            let step_errors: Vec<String> = import_result
                .failed_steps
                .iter()
                .map(|(step, error)| format!("{}: {}", step, error))
                .collect();
            let messages = [step_errors, missing_file_warnings].concat();
            (
                Some(import_result.file_set_id),
                FileSetImportStatus::SuccessWithWarnings(messages),
            )
        }
        Err(e) => {
            tracing::error!(error = ?e, game = game.name.as_str(), "Failed to import new file set");
            (None, FileSetImportStatus::Failed(format!("{}", e)))
        }
    }
}

/// Records `AlreadyExists` for a file set that is fully linked to the current DAT — no I/O needed.
fn handle_already_complete(file_set_id: i64, game: &DatGame) -> (Option<i64>, FileSetImportStatus) {
    tracing::info!(
        game = game.name.as_str(),
        file_set_id,
        "File set already exists with release and linked to DAT, skipping",
    );
    (Some(file_set_id), FileSetImportStatus::AlreadyExists)
}

/// Links an existing (unlinked) file set to the current DAT file and creates a release for it.
///
/// After linking, if there were missing files recorded for this file set, checks whether any
/// are now locally available and calls `update_file_set` to import them if so.
async fn handle_link_existing_to_dat(
    file_set_id: i64,
    game: &DatGame,
    missing_files: &[Sha1Checksum],
    sha1_map: &HashMap<Sha1Checksum, PathBuf>,
    context: &DatFileMassImportContext,
) -> (Option<i64>, FileSetImportStatus) {
    let dat_file_id = context
        .state
        .dat_file_id
        .expect("DAT file ID should be present in state after should_execute guard");

    tracing::info!(
        game = game.name.as_str(),
        file_set_id,
        dat_file_id,
        "Linking existing file set to DAT file and creating release",
    );

    let link_res = context
        .ops
        .file_set_service_ops
        .create_release_for_file_set(
            &[file_set_id],
            CreateReleaseParams {
                release_name: game.get_release_name(),
                software_title_name: game.get_software_title_name(),
            },
            &[context.input.system_id],
            Some(dat_file_id),
        )
        .await;

    if let Err(e) = link_res {
        tracing::error!(
            error = ?e,
            game = game.name.as_str(),
            file_set_id,
            dat_file_id,
            "Failed to link existing file set to DAT file",
        );
        return (
            Some(file_set_id),
            FileSetImportStatus::Failed(format!(
                "Failed to link existing file set to DAT file and create a release: {}",
                e
            )),
        );
    }

    tracing::info!(
        game = game.name.as_str(),
        file_set_id,
        dat_file_id,
        "Successfully linked existing file set to DAT file",
    );

    if missing_files.is_empty() {
        return (Some(file_set_id), FileSetImportStatus::Success);
    }

    let status = complete_missing_files(file_set_id, game, missing_files, sha1_map, context).await;
    (Some(file_set_id), status)
}

/// Handles a file set that is linked to the DAT but has files recorded as missing.
///
/// Cross-references the missing SHA1s against the current local file scan and imports any
/// newly available files via `update_file_set`.
async fn handle_existing_with_missing_files(
    file_set_id: i64,
    game: &DatGame,
    missing_files: &[Sha1Checksum],
    sha1_map: &HashMap<Sha1Checksum, PathBuf>,
    context: &DatFileMassImportContext,
) -> (Option<i64>, FileSetImportStatus) {
    tracing::info!(
        game = game.name.as_str(),
        file_set_id,
        missing_file_count = missing_files.len(),
        "File set has missing files — checking for newly available local files",
    );
    let status = complete_missing_files(file_set_id, game, missing_files, sha1_map, context).await;
    (Some(file_set_id), status)
}

/// Cross-references `missing_files` SHA1s against the local file scan.
///
/// - If none are newly available, returns `SuccessWithWarnings` listing the still-missing files.
/// - If some are newly available, calls `update_file_set` to import them.
///   Returns `Success` when all files are now present, `SuccessWithWarnings` if some remain.
async fn complete_missing_files(
    file_set_id: i64,
    game: &DatGame,
    missing_files: &[Sha1Checksum],
    sha1_map: &HashMap<Sha1Checksum, PathBuf>,
    context: &DatFileMassImportContext,
) -> FileSetImportStatus {
    let (newly_available, still_missing) = partition_missing_files(missing_files, sha1_map);

    if newly_available.is_empty() {
        tracing::info!(
            game = game.name.as_str(),
            file_set_id,
            still_missing_count = still_missing.len(),
            "No newly available files found for file set",
        );
        return FileSetImportStatus::StillMissingFiles(sha1s_to_warning_messages(
            &still_missing,
            game,
        ));
    }

    tracing::info!(
        game = game.name.as_str(),
        file_set_id,
        newly_available_count = newly_available.len(),
        still_missing_count = still_missing.len(),
        "Found newly available files for file set — calling update_file_set",
    );

    let model = build_update_model(file_set_id, game, &newly_available, sha1_map, context);
    match context
        .ops
        .file_import_service_ops
        .update_file_set(model)
        .await
    {
        Ok(_) if still_missing.is_empty() => {
            tracing::info!(
                game = game.name.as_str(),
                file_set_id,
                "File set fully completed after update",
            );
            FileSetImportStatus::Success
        }
        Ok(_) => {
            tracing::info!(
                game = game.name.as_str(),
                file_set_id,
                still_missing_count = still_missing.len(),
                "File set partially completed — some files still missing after update",
            );
            FileSetImportStatus::SuccessWithWarnings(sha1s_to_warning_messages(
                &still_missing,
                game,
            ))
        }
        Err(e) => {
            tracing::error!(
                error = ?e,
                game = game.name.as_str(),
                file_set_id,
                "Failed to update file set with newly available files",
            );
            FileSetImportStatus::Failed(format!("{}", e))
        }
    }
}

/// Splits `missing_files` into files now present in the local scan (`newly_available`)
/// and files that are still absent (`still_missing`).
fn partition_missing_files(
    missing_files: &[Sha1Checksum],
    sha1_map: &HashMap<Sha1Checksum, PathBuf>,
) -> (Vec<Sha1Checksum>, Vec<Sha1Checksum>) {
    missing_files
        .iter()
        .cloned()
        .partition(|sha1| sha1_map.contains_key(sha1))
}

/// Maps SHA1 checksums back to human-readable ROM file names for warning messages.
/// Falls back to the raw SHA1 hex string for checksums not found in `game.roms`.
fn sha1s_to_warning_messages(sha1s: &[Sha1Checksum], game: &DatGame) -> Vec<String> {
    sha1s
        .iter()
        .map(|sha1| {
            game.roms
                .iter()
                .find(|r| sha1_from_hex_string(&r.sha1).ok().as_ref() == Some(sha1))
                .map_or_else(
                    || format!("Missing file: {}", sha1_bytes_to_hex_string(sha1)),
                    |r| format!("Missing file: {}", r.name),
                )
        })
        .collect()
}

/// Builds an `UpdateFileSetModel` for the `newly_available` SHA1s.
///
/// File names and sizes come from `game.roms` (matched by SHA1); file paths come from `sha1_map`.
fn build_update_model(
    file_set_id: i64,
    game: &DatGame,
    newly_available: &[Sha1Checksum],
    sha1_map: &HashMap<Sha1Checksum, PathBuf>,
    context: &DatFileMassImportContext,
) -> UpdateFileSetModel {
    let mut import_files_map: HashMap<PathBuf, Vec<ImportFileContent>> = HashMap::new();
    let mut matched_sha1s: Vec<Sha1Checksum> = Vec::new();
    for sha1 in newly_available {
        let path = sha1_map
            .get(sha1)
            .expect("newly_available SHA1 must be present in sha1_map — partition guarantees this");
        if let Some(rom) = game
            .roms
            .iter()
            .find(|r| sha1_from_hex_string(&r.sha1).ok().as_ref() == Some(sha1))
        {
            import_files_map
                .entry(path.clone())
                .or_default()
                .push(ImportFileContent {
                    file_name: rom.name.clone(),
                    sha1_checksum: *sha1,
                    file_size: rom.size,
                });
            matched_sha1s.push(*sha1);
        } else {
            tracing::warn!(
                game = game.name.as_str(),
                sha1 = sha1_bytes_to_hex_string(sha1).as_str(),
                "Newly available SHA1 has no matching ROM in DAT game — skipping (DAT may have changed since initial import)",
            );
        }
    }

    UpdateFileSetModel {
        import_files: import_files_map
            .into_iter()
            .map(|(path, contents)| FileImportSource {
                path,
                content: contents.into_iter().map(|c| (c.sha1_checksum, c)).collect(),
            })
            .collect(),
        selected_files: matched_sha1s,
        source: context
            .state
            .dat_file
            .as_ref()
            .map(|f| f.header.get_source())
            .unwrap_or_default(),
        file_set_id,
        file_set_name: game.name.clone(),
        file_set_file_name: game.name.clone(),
        file_type: context.input.file_type,
        item_ids: vec![],
        item_types: context.input.item_type.map_or_else(Vec::new, |it| vec![it]),
    }
}

/// Appends a result to `import_results` and sends a progress event to the sync channel.
fn record_and_send(
    file_set_id: Option<i64>,
    file_set_name: &str,
    status: FileSetImportStatus,
    context: &mut DatFileMassImportContext,
) {
    context
        .state
        .common_state
        .import_results
        .push(FileSetImportResult {
            file_set_id,
            status: status.clone(),
            file_set_name: file_set_name.to_string(),
        });
    if let Some(tx) = &context.progress_tx {
        let event = MassImportSyncEvent {
            file_set_name: file_set_name.to_string(),
            status,
        };
        if let Err(e) = tx.send(event) {
            tracing::error!(error = ?e, "Failed to send progress event");
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{collections::HashMap, path::PathBuf, sync::Arc};

    use core_types::{FileType, sha1_from_hex_string};
    use database::setup_test_repository_manager;
    use domain::naming_conventions::no_intro::{DatFile, DatGame, DatHeader, DatRom};
    use file_metadata::SendReaderFactoryFn;

    use crate::{
        dat_game_status_service::DatGameFileSetStatus,
        file_import::file_import_service_ops::{CreateMockState, MockFileImportServiceOps},
        file_set::mock_file_set_service::MockFileSetService,
        file_system_ops::mock::MockFileSystemOps,
        mass_import::{
            common_steps::context::MassImportDeps,
            models::{DatMassImportInput, FileSetImportStatus},
            test_utils::create_mock_reader_factory,
            with_dat::context::{DatFileMassImportContext, DatFileMassImportOps},
        },
        pipeline::pipeline_step::{PipelineStep, StepAction},
    };

    use super::RouteAndProcessFileSetsStep;

    const SHA1_HEX: &str = "1234567890abcdef1234567890abcdef12345678";

    fn make_dat_file(game_name: &str, rom_name: &str) -> DatFile {
        DatFile {
            header: DatHeader {
                name: "Test DAT".to_string(),
                version: "1.0".to_string(),
                ..Default::default()
            },
            games: vec![DatGame {
                name: game_name.to_string(),
                description: String::new(),
                roms: vec![DatRom {
                    name: rom_name.to_string(),
                    size: 1024,
                    sha1: SHA1_HEX.to_string(),
                    ..Default::default()
                }],
                ..Default::default()
            }],
        }
    }

    fn make_ops(
        file_import_ops: Arc<dyn crate::file_import::file_import_service_ops::FileImportServiceOps>,
        reader_factory_fn: Arc<SendReaderFactoryFn>,
    ) -> DatFileMassImportOps {
        DatFileMassImportOps {
            dat_file_parser_ops: Arc::new(dat_file_parser::MockDatParser::new(Ok(
                dat_file_parser::DatFile {
                    header: dat_file_parser::DatHeader::default(),
                    games: vec![],
                },
            ))),
            fs_ops: Arc::new(MockFileSystemOps::new()),
            reader_factory_fn,
            file_import_service_ops: file_import_ops,
            file_set_service_ops: Arc::new(MockFileSetService::new()),
        }
    }

    async fn make_context(
        ops: DatFileMassImportOps,
        dat_file: DatFile,
        statuses: Vec<DatGameFileSetStatus>,
        reader_factory_fn: Arc<SendReaderFactoryFn>,
    ) -> DatFileMassImportContext {
        let deps = MassImportDeps {
            repository_manager: setup_test_repository_manager().await,
        };
        let mut context = DatFileMassImportContext::new(
            deps,
            DatMassImportInput {
                source_path: PathBuf::from("/roms"),
                dat_file_path: PathBuf::from("/dat/test.dat"),
                file_type: FileType::Rom,
                item_type: None,
                system_id: 1,
            },
            ops,
            None,
        );
        context.state.dat_file = Some(dat_file);
        context.state.dat_file_id = Some(1);
        context.state.common_state.file_metadata = {
            let sha1 = sha1_from_hex_string(SHA1_HEX).unwrap();
            let mut map = HashMap::new();
            map.insert(
                PathBuf::from("/roms/test.bin"),
                vec![core_types::ReadFile {
                    file_name: "test.bin".to_string(),
                    sha1_checksum: sha1,
                    file_size: 1024,
                }],
            );
            map
        };
        context.state.dat_game_statuses = statuses;
        context
    }

    #[async_std::test]
    async fn test_new_file_set_all_files_present_produces_success() {
        // Arrange: ROM is locally available (sha1_map will find it)
        let dat_file = make_dat_file("Test Game", "test.bin");
        let sha1 = sha1_from_hex_string(SHA1_HEX).unwrap();
        let reader_factory = Arc::new(create_mock_reader_factory(HashMap::new(), vec![]));
        let file_import_ops = Arc::new(MockFileImportServiceOps::with_create_mock(
            CreateMockState {
                file_set_id: 1,
                release_id: Some(1),
            },
        ));
        let ops = make_ops(file_import_ops.clone(), reader_factory.clone());
        let statuses = vec![DatGameFileSetStatus::NonExisting(dat_file.games[0].clone())];
        let mut context = make_context(ops, dat_file, statuses, reader_factory).await;
        // The scanned file metadata already points SHA1 → /roms/test.bin via make_context
        let _ = sha1; // used via make_context

        // Act
        let result = RouteAndProcessFileSetsStep.execute(&mut context).await;

        // Assert
        assert!(matches!(result, StepAction::Continue));
        let results = &context.state.common_state.import_results;
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].status, FileSetImportStatus::Success);
        assert_eq!(results[0].file_set_id, Some(1));
    }

    #[async_std::test]
    async fn test_new_file_set_missing_files_produces_success_with_warnings() {
        // Arrange: no local files scanned — ROM will be missing
        let dat_file = make_dat_file("Test Game", "test.bin");
        let reader_factory = Arc::new(create_mock_reader_factory(HashMap::new(), vec![]));
        let file_import_ops = Arc::new(MockFileImportServiceOps::with_create_mock(
            CreateMockState {
                file_set_id: 1,
                release_id: Some(1),
            },
        ));
        let ops = make_ops(file_import_ops, reader_factory.clone());
        let statuses = vec![DatGameFileSetStatus::NonExisting(dat_file.games[0].clone())];
        let mut context = make_context(ops, dat_file, statuses, reader_factory).await;
        // Clear the file metadata so no files are locally available
        context.state.common_state.file_metadata.clear();

        // Act
        let result = RouteAndProcessFileSetsStep.execute(&mut context).await;

        // Assert
        assert!(matches!(result, StepAction::Continue));
        let results = &context.state.common_state.import_results;
        assert_eq!(results.len(), 1);
        assert!(
            matches!(&results[0].status, FileSetImportStatus::SuccessWithWarnings(w) if !w.is_empty()),
            "Expected SuccessWithWarnings with a missing-file message"
        );
    }

    #[async_std::test]
    async fn test_handle_new_file_set_all_roms_missing_creates_placeholder() {
        const SHA1_A: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
        const SHA1_B: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
        const SHA1_C: &str = "cccccccccccccccccccccccccccccccccccccccc";

        // Arrange: DAT file with 3 ROMs, none locally available
        let dat_file = DatFile {
            header: DatHeader {
                name: "Multi-ROM DAT".to_string(),
                version: "1.0".to_string(),
                ..Default::default()
            },
            games: vec![DatGame {
                name: "Multi-ROM Game".to_string(),
                description: String::new(),
                roms: vec![
                    DatRom {
                        name: "rom_a.bin".to_string(),
                        size: 512,
                        sha1: SHA1_A.to_string(),
                        ..Default::default()
                    },
                    DatRom {
                        name: "rom_b.bin".to_string(),
                        size: 512,
                        sha1: SHA1_B.to_string(),
                        ..Default::default()
                    },
                    DatRom {
                        name: "rom_c.bin".to_string(),
                        size: 512,
                        sha1: SHA1_C.to_string(),
                        ..Default::default()
                    },
                ],
                ..Default::default()
            }],
        };
        let reader_factory = Arc::new(create_mock_reader_factory(HashMap::new(), vec![]));
        let file_import_ops = Arc::new(MockFileImportServiceOps::with_create_mock(
            CreateMockState {
                file_set_id: 7,
                release_id: Some(3),
            },
        ));
        let ops = make_ops(file_import_ops, reader_factory.clone());
        let statuses = vec![DatGameFileSetStatus::NonExisting(dat_file.games[0].clone())];
        let mut context = make_context(ops, dat_file, statuses, reader_factory).await;
        // No local files available — all 3 ROMs must be treated as missing
        context.state.common_state.file_metadata.clear();

        // Act
        let result = RouteAndProcessFileSetsStep.execute(&mut context).await;

        // Assert
        assert!(matches!(result, StepAction::Continue));
        let results = &context.state.common_state.import_results;
        assert_eq!(results.len(), 1);
        assert_eq!(
            results[0].file_set_id,
            Some(7),
            "file_set_id should match mock"
        );
        let warnings = match &results[0].status {
            FileSetImportStatus::SuccessWithWarnings(w) => w,
            other => panic!("Expected SuccessWithWarnings, got {:?}", other),
        };
        assert_eq!(warnings.len(), 3, "Expected one warning per missing ROM");
        assert!(
            warnings.iter().any(|w| w.contains("rom_a.bin")),
            "Missing warning for rom_a.bin"
        );
        assert!(
            warnings.iter().any(|w| w.contains("rom_b.bin")),
            "Missing warning for rom_b.bin"
        );
        assert!(
            warnings.iter().any(|w| w.contains("rom_c.bin")),
            "Missing warning for rom_c.bin"
        );
    }

    #[async_std::test]
    async fn test_handle_link_existing_to_dat_success() {
        // Arrange: file set exists but has no release/DAT link yet; no missing files
        let dat_file = make_dat_file("Test Game", "test.bin");
        let reader_factory = Arc::new(create_mock_reader_factory(HashMap::new(), vec![]));
        let ops = make_ops(
            Arc::new(MockFileImportServiceOps::new()),
            reader_factory.clone(),
        );
        let statuses = vec![
            DatGameFileSetStatus::ExistingWithoutReleaseAndWithoutLinkToDat {
                file_set_id: 42,
                game: dat_file.games[0].clone(),
                missing_files: vec![],
            },
        ];
        let mut context = make_context(ops, dat_file, statuses, reader_factory).await;

        // Act
        let result = RouteAndProcessFileSetsStep.execute(&mut context).await;

        // Assert
        assert!(matches!(result, StepAction::Continue));
        let results = &context.state.common_state.import_results;
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].status, FileSetImportStatus::Success);
        assert_eq!(results[0].file_set_id, Some(42));
    }

    #[async_std::test]
    async fn test_handle_link_existing_to_dat_with_failure() {
        // Arrange: file set exists but linking to DAT fails
        let dat_file = make_dat_file("Test Game", "test.bin");
        let reader_factory = Arc::new(create_mock_reader_factory(HashMap::new(), vec![]));
        let file_set_service = Arc::new(MockFileSetService::new());
        file_set_service.fail_create_release();
        let ops = DatFileMassImportOps {
            dat_file_parser_ops: Arc::new(dat_file_parser::MockDatParser::new(Ok(
                dat_file_parser::DatFile {
                    header: dat_file_parser::DatHeader::default(),
                    games: vec![],
                },
            ))),
            fs_ops: Arc::new(MockFileSystemOps::new()),
            reader_factory_fn: reader_factory.clone(),
            file_import_service_ops: Arc::new(MockFileImportServiceOps::new()),
            file_set_service_ops: file_set_service,
        };
        let statuses = vec![
            DatGameFileSetStatus::ExistingWithoutReleaseAndWithoutLinkToDat {
                file_set_id: 42,
                game: dat_file.games[0].clone(),
                missing_files: vec![],
            },
        ];
        let mut context = make_context(ops, dat_file, statuses, reader_factory).await;

        // Act
        let result = RouteAndProcessFileSetsStep.execute(&mut context).await;

        // Assert: pipeline continues (doesn't abort on link failure) but result is Failed
        assert!(matches!(result, StepAction::Continue));
        let results = &context.state.common_state.import_results;
        assert_eq!(results.len(), 1);
        assert!(
            matches!(&results[0].status, FileSetImportStatus::Failed(msg) if msg.contains("Failed to link")),
            "Expected Failed status with descriptive message, got {:?}",
            results[0].status
        );
        assert_eq!(
            results[0].file_set_id,
            Some(42),
            "file_set_id should be returned even on failure"
        );
    }

    #[async_std::test]
    async fn test_already_linked_complete_file_set_produces_already_exists() {
        // Arrange: file set is already linked and has no missing files
        let dat_file = make_dat_file("Test Game", "test.bin");
        let reader_factory = Arc::new(create_mock_reader_factory(HashMap::new(), vec![]));
        let ops = make_ops(
            Arc::new(MockFileImportServiceOps::new()),
            reader_factory.clone(),
        );
        let sha1 = sha1_from_hex_string(SHA1_HEX).unwrap();
        let statuses = vec![DatGameFileSetStatus::ExistingWithReleaseAndLinkedToDat {
            file_set_id: 42,
            game: dat_file.games[0].clone(),
            missing_files: vec![],
        }];
        let mut context = make_context(ops, dat_file, statuses, reader_factory).await;
        let _ = sha1;

        // Act
        let result = RouteAndProcessFileSetsStep.execute(&mut context).await;

        // Assert
        assert!(matches!(result, StepAction::Continue));
        let results = &context.state.common_state.import_results;
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].status, FileSetImportStatus::AlreadyExists);
        assert_eq!(results[0].file_set_id, Some(42));
    }

    #[async_std::test]
    async fn test_existing_linked_file_set_with_newly_available_file_completes_successfully() {
        // Arrange: file set is linked to DAT but had a missing file; ROM is now locally available
        let dat_file = make_dat_file("Test Game", "test.bin");
        let sha1 = sha1_from_hex_string(SHA1_HEX).unwrap();
        let reader_factory = Arc::new(create_mock_reader_factory(HashMap::new(), vec![]));
        let file_import_ops = Arc::new(MockFileImportServiceOps::with_update_mock(
            CreateMockState {
                file_set_id: 99,
                release_id: None,
            },
        ));
        let ops = make_ops(file_import_ops, reader_factory.clone());
        let statuses = vec![DatGameFileSetStatus::ExistingWithReleaseAndLinkedToDat {
            file_set_id: 99,
            game: dat_file.games[0].clone(),
            missing_files: vec![sha1],
        }];
        let mut context = make_context(ops, dat_file, statuses, reader_factory).await;
        // file_metadata already contains sha1 → /roms/test.bin (from make_context)

        // Act
        let result = RouteAndProcessFileSetsStep.execute(&mut context).await;

        // Assert
        assert!(matches!(result, StepAction::Continue));
        let results = &context.state.common_state.import_results;
        assert_eq!(results.len(), 1);
        assert_eq!(
            results[0].status,
            FileSetImportStatus::Success,
            "All missing files are now available — expected Success"
        );
        assert_eq!(results[0].file_set_id, Some(99));
    }

    #[async_std::test]
    async fn test_handle_existing_with_missing_files_link_fails() {
        const SHA1_B: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

        // Arrange: file set is linked to DAT with two missing files.
        // SHA1_HEX is locally available (from make_context) → newly available → triggers update_file_set.
        // SHA1_B is not locally available → still missing.
        // update_file_set is configured to fail.
        let dat_file = make_dat_file("Test Game", "test.bin");
        let sha1_a = sha1_from_hex_string(SHA1_HEX).unwrap();
        let sha1_b = sha1_from_hex_string(SHA1_B).unwrap();
        let reader_factory = Arc::new(create_mock_reader_factory(HashMap::new(), vec![]));
        let file_import_ops = Arc::new(MockFileImportServiceOps {
            should_fail: true,
            ..Default::default()
        });
        let ops = make_ops(file_import_ops, reader_factory.clone());
        let statuses = vec![DatGameFileSetStatus::ExistingWithReleaseAndLinkedToDat {
            file_set_id: 99,
            game: dat_file.games[0].clone(),
            missing_files: vec![sha1_a, sha1_b],
        }];
        // make_context puts SHA1_HEX into file_metadata → sha1_a is "newly available"
        let mut context = make_context(ops, dat_file, statuses, reader_factory).await;

        // Act
        let result = RouteAndProcessFileSetsStep.execute(&mut context).await;

        // Assert: pipeline continues but the import result is Failed
        assert!(matches!(result, StepAction::Continue));
        let results = &context.state.common_state.import_results;
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].file_set_id, Some(99));
        assert!(
            matches!(&results[0].status, FileSetImportStatus::Failed(_)),
            "Expected Failed status when update_file_set errors, got {:?}",
            results[0].status
        );
    }

    #[async_std::test]
    async fn test_existing_linked_file_set_with_no_newly_available_files_returns_still_missing() {
        // Arrange: the file set is linked to DAT and has a missing file that is NOT available
        // locally. make_context populates file_metadata with SHA1_HEX only; MISSING_SHA1 is absent.
        const MISSING_SHA1: &str = "cccccccccccccccccccccccccccccccccccccccc";
        let dat_file = make_dat_file("Test Game", "test.bin");
        let reader_factory = Arc::new(create_mock_reader_factory(HashMap::new(), vec![]));
        let ops = make_ops(
            Arc::new(MockFileImportServiceOps::new()),
            reader_factory.clone(),
        );
        let missing_sha1 = sha1_from_hex_string(MISSING_SHA1).unwrap();
        let statuses = vec![DatGameFileSetStatus::ExistingWithReleaseAndLinkedToDat {
            file_set_id: 77,
            game: dat_file.games[0].clone(),
            missing_files: vec![missing_sha1],
        }];
        let mut context = make_context(ops, dat_file, statuses, reader_factory).await;

        // Act
        let result = RouteAndProcessFileSetsStep.execute(&mut context).await;

        // Assert: pipeline continues and records StillMissingFiles
        assert!(matches!(result, StepAction::Continue));
        let results = &context.state.common_state.import_results;
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].file_set_id, Some(77));
        assert!(
            matches!(&results[0].status, FileSetImportStatus::StillMissingFiles(msgs) if !msgs.is_empty()),
            "Expected StillMissingFiles with warning messages, got {:?}",
            results[0].status
        );
    }
}
