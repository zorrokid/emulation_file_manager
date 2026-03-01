use crate::{
    dat_file_service::DatFileService,
    dat_game_status_service::{DatGameFileSetStatus, DatGameStatusService},
    error::Error,
    file_import::model::CreateReleaseParams,
    mass_import::with_dat::context::DatFileMassImportContext,
    pipeline::pipeline_step::{PipelineStep, StepAction},
};

pub struct ImportDatFileStep;

#[async_trait::async_trait]
impl PipelineStep<DatFileMassImportContext> for ImportDatFileStep {
    fn name(&self) -> &'static str {
        "import_dat_file_step"
    }

    fn should_execute(&self, context: &DatFileMassImportContext) -> bool {
        context.input.dat_file_path.is_some()
    }

    async fn execute(&self, context: &mut DatFileMassImportContext) -> StepAction {
        let dat_path = context
            .input
            .dat_file_path
            .as_ref()
            .expect("Dat file path should be present");

        let parse_res = context.ops.dat_file_parser_ops.parse_dat_file(dat_path);
        match parse_res {
            Ok(dat_file) => {
                println!("Successfully parsed DAT file: {:?}", dat_file);
                context.state.dat_file = Some(dat_file.into());
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

pub struct CheckExistingDatFileStep;
#[async_trait::async_trait]
impl PipelineStep<DatFileMassImportContext> for CheckExistingDatFileStep {
    fn name(&self) -> &'static str {
        "check_existing_dat_file_step"
    }
    fn should_execute(&self, context: &DatFileMassImportContext) -> bool {
        context.state.dat_file.is_some()
    }

    async fn execute(&self, context: &mut DatFileMassImportContext) -> StepAction {
        let dat_file = context
            .state
            .dat_file
            .as_ref()
            .expect("DAT file should be present in state");
        println!(
            "Checking if DAT file already exists in the database: name='{}', version='{}, system_id={}'",
            dat_file.header.name, dat_file.header.version, context.input.system_id
        );
        let is_existing_dat_res = context
            .deps
            .repository_manager
            .get_dat_repository()
            .check_dat_file_exists(
                dat_file.header.version.as_str(),
                dat_file.header.name.as_str(), // TODO: use dat file type instead?
                context.input.system_id,
            )
            .await;

        match is_existing_dat_res {
            Ok(id_res) => {
                println!(
                    "Check existing DAT file result for '{}': {:?}",
                    dat_file.header.name, id_res
                );
                if let Some(id) = id_res {
                    tracing::info!(
                        system_id = context.input.system_id,
                        dat_name = %dat_file.header.name,
                        dat_version = %dat_file.header.version,
                        "DAT file already exists in the database",
                    );
                    context.state.dat_file_id = Some(id);
                } else {
                    tracing::info!(
                        system_id = context.input.system_id,
                        dat_name = %dat_file.header.name,
                        dat_version = %dat_file.header.version,
                        "DAT file does not exist in the database, proceeding to store it",
                    );
                }
                StepAction::Continue
            }
            Err(err) => {
                tracing::error!(
                    system_id = context.input.system_id,
                    dat_name = %dat_file.header.name,
                    dat_version = %dat_file.header.version,
                    error = ?err,
                    "Error while checking if DAT file exists in the database",
                );
                StepAction::Abort(Error::DbError(format!(
                    "Error while checking if DAT file exists in the database: {}",
                    err
                )))
            }
        }
    }
}

pub struct StoreDatFileStep;
#[async_trait::async_trait]
impl PipelineStep<DatFileMassImportContext> for StoreDatFileStep {
    fn name(&self) -> &'static str {
        "store_dat_file_step"
    }
    fn should_execute(&self, context: &DatFileMassImportContext) -> bool {
        context.state.dat_file.is_some() && context.state.dat_file_id.is_none()
    }

    async fn execute(&self, context: &mut DatFileMassImportContext) -> StepAction {
        let dat_file = context
            .state
            .dat_file
            .as_ref()
            .expect("DAT file should be present in state");

        let dat_service = DatFileService::new(context.deps.repository_manager.clone());
        match dat_service
            .store_dat_file(dat_file, context.input.system_id)
            .await
        {
            Ok(dat_file_id) => {
                println!("Successfully stored DAT file with ID: {}", dat_file_id);
                context.state.dat_file_id = Some(dat_file_id);
            }
            Err(e) => {
                println!("Failed to store DAT file: {}", e);
                return StepAction::Abort(Error::DbError(format!(
                    "Failed to store DAT file: {}",
                    e
                )));
            }
        }

        StepAction::Continue
    }
}

/// This step will filter out file sets that already exist in the database based on their metadata.
///
/// There can be following cases:
///
/// 1. New file set, new software title, new release, not linked to dat file:
///
/// This is the basic case when importing dat file.
///
/// There is no existing file set with the same signature. We can proceed with importing it as a
/// new file set and link it to dat file. We will also create a new software title and release for
/// it. We don't currenctly check duplicates for software titles and releases in this case. The
/// possible duplicates should be merged manually by the user after the import. We will provide a
/// functionality to merge software titles and releases in the future.
///
/// 2. Existing file set, existing software title, existing release, linked to dat file:
///
/// This is basic case when user tries to import the same dat file twice. We could just check if
/// dat file already exists and abort the import but we will have a separate functionality for
/// adding dat files without import. So dat file may exists because of that.
///
/// There is an existing file set that is already linked to current dat file. We can consider it
/// as a duplicate and skip it.
///
/// 3. Existing file set, existing software title, existing release, not linked to dat file:
///
/// There is an existing file set with exactly the same signature but it's not linked to this
/// file set (e.g. because it was imported with a different DAT file or without a DAT file). In
/// this case we can link the existing file set to dat file.
///
/// 4. Existing file set, existing or non existing software title, existing or non existing release, not linked to dat file:
///
/// This case could happen when the same file set was imported with a different DAT file or by
/// adding as single file set software title and release may differ because of that.
///
/// Currently we treat this case as an existing file set and create a releaes and software title
/// for it and link it to dat file. Possible duplicates should be merged manually by the user after
/// the import. We will provide a functionality to merge software titles and releases in the
/// future.
///
pub struct FilterExistingFileSetsStep;

#[async_trait::async_trait]
impl PipelineStep<DatFileMassImportContext> for FilterExistingFileSetsStep {
    fn name(&self) -> &'static str {
        "filter_existing_file_sets_step"
    }
    fn should_execute(&self, context: &DatFileMassImportContext) -> bool {
        // we need file metadata to check for existing file sets
        !context.state.file_metadata.is_empty()
            // dat file has to be parsed for this step
            && context.state.dat_file.is_some()
            // dat file has to be inserted for this step
            && context.state.dat_file_id.is_some()
    }

    async fn execute(&self, context: &mut DatFileMassImportContext) -> StepAction {
        // TODO: add to context if needs injection for mocking in tests
        // now it's fine since we use in mem test db anyway in tests
        let dat_game_status_service =
            DatGameStatusService::new(context.deps.repository_manager.clone());
        let dat_file = context
            .state
            .dat_file
            .as_ref()
            .expect("DAT file should be present in state");
        let dat_file_id = context
            .state
            .dat_file_id
            .expect("DAT file ID should be present in state");
        for game in &dat_file.games {
            let status = dat_game_status_service
                .get_status(game, context.input.file_type, &dat_file.header, dat_file_id)
                .await;
            match status {
                Ok(status) => {
                    tracing::info!(
                        game = %game.name,
                        dat_file_id = dat_file_id,
                        "Got existing file set status for game",
                    );
                    context.state.statuses.push(status);
                }
                Err(e) => {
                    tracing::error!(
                        error = ?e,
                        game = %game.name,
                        dat_file_id = dat_file_id,
                        "Failed to get existing file set status for game",
                    );
                    // Let's still abort at this phase
                    return StepAction::Abort(Error::DbError(format!(
                        "Failed to get existing file set status for game '{}': {}",
                        game.name, e
                    )));
                }
            }
        }
        StepAction::Continue
    }
}

pub struct LinkExistingFileSetsStep;

#[async_trait::async_trait]
impl PipelineStep<DatFileMassImportContext> for LinkExistingFileSetsStep {
    fn name(&self) -> &'static str {
        "link_existing_file_sets_step"
    }

    fn should_execute(&self, context: &DatFileMassImportContext) -> bool {
        context.state.dat_file_id.is_some()
            && context.state.statuses.iter().any(|status| {
                matches!(
                    status,
                    DatGameFileSetStatus::ExistingWithoutReleaseAndWithoutLinkToDat { .. }
                )
            })
    }

    async fn execute(&self, context: &mut DatFileMassImportContext) -> StepAction {
        let dat_file_id = context
            .state
            .dat_file_id
            .expect("DAT file ID should be present in state");

        let statuses = context
            .state
            .statuses
            .iter()
            .filter(|status| {
                matches!(
                    status,
                    DatGameFileSetStatus::ExistingWithoutReleaseAndWithoutLinkToDat { .. }
                )
            })
            .collect::<Vec<_>>();

        for status in statuses {
            if let DatGameFileSetStatus::ExistingWithoutReleaseAndWithoutLinkToDat {
                game,
                file_set_id,
            } = status
            {
                tracing::info!(
                    game = %game.name,
                    file_set_id = file_set_id,
                    "Linking existing file set to dat file",
                );
                let file_set_id = *file_set_id;
                let res = context
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
                match res {
                    Ok(id) => {
                        tracing::info!(
                            game = %game.name,
                            file_set_id = file_set_id,
                            release_id = id,
                            dat_file_id = dat_file_id,
                            "Successfully linked existing file set to dat file and created a release",
                        );
                    }
                    Err(e) => {
                        tracing::error!(
                            error = ?e,
                            game = %game.name,
                            file_set_id = file_set_id,
                            dat_file_id = dat_file_id,
                            "Failed to link existing file set to dat file and create a release",
                        );
                        // Not aborting any more
                        // TODO: collect failed links and show them in the end of import process
                    }
                }
            }
        }
        StepAction::Continue
    }
}

#[cfg(test)]
mod tests {
    use std::{collections::HashMap, path::PathBuf, sync::Arc};

    use dat_file_parser::{DatFileParserError, DatFileParserOps, MockDatParser};
    use database::helper::AddDatFileParams;
    use domain::naming_conventions::no_intro::{DatFile, DatGame, DatHeader, DatRom};
    use file_metadata::SendReaderFactoryFn;

    use crate::{
        file_import::file_import_service_ops::{FileImportServiceOps, MockFileImportServiceOps},
        file_set::mock_file_set_service::MockFileSetService,
        file_system_ops::{FileSystemOps, mock::MockFileSystemOps},
        mass_import::{
            common_steps::context::MassImportDeps, models::MassImportInput,
            test_utils::create_mock_reader_factory, with_dat::context::DatFileMassImportOps,
        },
    };

    use super::*;

    fn get_ops(
        dat_file_parser_ops: Option<Arc<dyn DatFileParserOps>>,
        fs_ops: Option<Arc<dyn FileSystemOps>>,
        reader_factory_fn: Option<Arc<SendReaderFactoryFn>>,
        file_import_ops: Option<Arc<dyn FileImportServiceOps>>,
    ) -> DatFileMassImportOps {
        let file_import_service_ops =
            file_import_ops.unwrap_or_else(|| Arc::new(MockFileImportServiceOps::new()));
        let parse_result: Result<dat_file_parser::DatFile, DatFileParserError> =
            Ok(dat_file_parser::DatFile {
                header: dat_file_parser::DatHeader::default(),
                games: vec![],
            });
        let dat_file_parser_ops =
            dat_file_parser_ops.unwrap_or(Arc::new(MockDatParser::new(parse_result)));
        let fs_ops = fs_ops.unwrap_or(Arc::new(MockFileSystemOps::new()));
        let reader_factory_fn = reader_factory_fn
            .unwrap_or(Arc::new(create_mock_reader_factory(HashMap::new(), vec![])));
        let file_set_service_ops = Arc::new(MockFileSetService::new());
        DatFileMassImportOps {
            fs_ops,
            file_import_service_ops,
            reader_factory_fn,
            dat_file_parser_ops,
            file_set_service_ops,
        }
    }

    async fn get_deps() -> MassImportDeps {
        MassImportDeps {
            repository_manager: database::setup_test_repository_manager().await,
        }
    }

    #[async_std::test]
    async fn test_import_dat_file_step() {
        let parse_result: Result<dat_file_parser::DatFile, DatFileParserError> =
            Ok(dat_file_parser::DatFile {
                header: dat_file_parser::DatHeader::default(),
                games: vec![],
            });
        let dat_file_parser_ops = Arc::new(MockDatParser::new(parse_result));

        let mut context = DatFileMassImportContext::new(
            get_deps().await,
            MassImportInput {
                source_path: PathBuf::from("/path/to/source"),
                dat_file_path: Some(PathBuf::from("/path/to/datfile.dat")),
                file_type: core_types::FileType::Rom,
                item_type: None,
                system_id: 1,
            },
            get_ops(Some(dat_file_parser_ops), None, None, None),
            None,
        );

        let step = ImportDatFileStep;
        let result = step.execute(&mut context).await;
        assert!(matches!(result, StepAction::Continue));
        assert!(context.state.dat_file.is_some());
    }

    #[async_std::test]
    async fn test_check_existing_dat_file_step_with_existing_dat_file() {
        // Arrange

        // Prepare a dat file and add it to the repository to simulate existing dat fil
        let deps = get_deps().await;
        let system_repo = deps.repository_manager.get_system_repository();
        let system_id = system_repo
            .add_system("Test System")
            .await
            .expect("Failed to add test system");
        let dat_repo = deps.repository_manager.get_dat_repository();

        let dat_file = DatFile {
            header: DatHeader {
                name: "Test DAT".to_string(),
                version: "1.0".to_string(),
                ..Default::default()
            },
            games: vec![],
        };

        let add_dat_file_params = AddDatFileParams {
            dat_id: dat_file.header.id,
            name: dat_file.header.name.as_str(),
            description: dat_file.header.description.as_str(),
            version: dat_file.header.version.as_str(),
            date: dat_file.header.date.as_deref(),
            author: dat_file.header.author.as_str(),
            homepage: dat_file.header.homepage.as_deref(),
            url: dat_file.header.url.as_deref(),
            subset: dat_file.header.subset.as_deref(),
            system_id,
        };
        let dat_id = dat_repo.add_dat_file(add_dat_file_params).await.unwrap();

        let dat_file_parser_ops = Arc::new(MockDatParser::new(Ok(dat_file.clone().into())));
        let mut context = DatFileMassImportContext::new(
            deps,
            MassImportInput {
                source_path: PathBuf::from("/path/to/source"),
                dat_file_path: Some(PathBuf::from("/path/to/datfile.dat")),
                file_type: core_types::FileType::Rom,
                item_type: None,
                system_id,
            },
            get_ops(Some(dat_file_parser_ops), None, None, None),
            None,
        );

        // Pre-populate state with a dat file to trigger the step
        context.state.dat_file = Some(dat_file);

        // Act
        let result = CheckExistingDatFileStep.execute(&mut context).await;

        // Assert
        assert!(matches!(result, StepAction::Continue));
        assert_eq!(context.state.dat_file_id, Some(dat_id));
    }

    #[async_std::test]
    async fn test_check_existing_dat_file_step_with_non_existing_dat_file() {
        // Arrange
        let dat_file = DatFile {
            header: DatHeader {
                name: "Non-Existing DAT".to_string(),
                version: "1.0".to_string(),
                ..Default::default()
            },
            games: vec![],
        };
        let dat_file_parser_ops = Arc::new(MockDatParser::new(Ok(dat_file.clone().into())));
        let mut context = DatFileMassImportContext::new(
            get_deps().await,
            MassImportInput {
                source_path: PathBuf::from("/path/to/source"),
                dat_file_path: Some(PathBuf::from("/path/to/datfile.dat")),
                file_type: core_types::FileType::Rom,
                item_type: None,
                system_id: 1,
            },
            get_ops(Some(dat_file_parser_ops), None, None, None),
            None,
        );

        // Pre-populate state with a dat file to trigger the step
        context.state.dat_file = Some(dat_file);

        // Act
        let result = CheckExistingDatFileStep.execute(&mut context).await;

        // Assert
        assert!(matches!(result, StepAction::Continue));
        assert_eq!(context.state.dat_file_id, None);
    }

    #[async_std::test]
    async fn test_store_dat_file_step() {
        // Arrange
        let dat_file = DatFile {
            header: DatHeader {
                name: "Test DAT".to_string(),
                version: "1.0".to_string(),
                ..Default::default()
            },
            games: vec![],
        };
        let deps = get_deps().await;
        let system_repo = deps.repository_manager.get_system_repository();
        let system_id = system_repo
            .add_system("Test System")
            .await
            .expect("Failed to add test system");
        let dat_file_parser_ops = Arc::new(MockDatParser::new(Ok(dat_file.clone().into())));
        let mut context = DatFileMassImportContext::new(
            deps,
            MassImportInput {
                source_path: PathBuf::from("/path/to/source"),
                dat_file_path: Some(PathBuf::from("/path/to/datfile.dat")),
                file_type: core_types::FileType::Rom,
                item_type: None,
                system_id,
            },
            get_ops(Some(dat_file_parser_ops), None, None, None),
            None,
        );

        // Pre-populate state with a dat file to trigger the step
        context.state.dat_file = Some(dat_file);
        context.state.dat_file_id = None; // Ensure dat file ID is None to trigger storage

        // Act
        let step = StoreDatFileStep;

        assert!(step.should_execute(&context));
        let result = step.execute(&mut context).await;

        // Assert
        assert!(matches!(result, StepAction::Continue));
        assert!(context.state.dat_file_id.is_some());
    }

    #[async_std::test]
    async fn test_store_dat_file_step_dat_id_set() {
        // Arrange
        let dat_file = DatFile {
            header: DatHeader {
                name: "Test DAT".to_string(),
                version: "1.0".to_string(),
                ..Default::default()
            },
            games: vec![],
        };
        let dat_file_parser_ops = Arc::new(MockDatParser::new(Ok(dat_file.clone().into())));
        let mut context = DatFileMassImportContext::new(
            get_deps().await,
            MassImportInput {
                source_path: PathBuf::from("/path/to/source"),
                dat_file_path: Some(PathBuf::from("/path/to/datfile.dat")),
                file_type: core_types::FileType::Rom,
                item_type: None,
                system_id: 1,
            },
            get_ops(Some(dat_file_parser_ops), None, None, None),
            None,
        );

        // Pre-populate state with a dat file and a dat file ID to ensure step should not execute
        context.state.dat_file = Some(dat_file);
        context.state.dat_file_id = Some(1); // Simulate existing dat file ID

        // Act
        let step = StoreDatFileStep;
        assert!(!step.should_execute(&context));
    }

    #[async_std::test]
    async fn test_link_existing_file_sets_step() {
        // Arrange
        let deps = get_deps().await;

        // Create a system and dat file
        let system_id = deps
            .repository_manager
            .get_system_repository()
            .add_system("Test System")
            .await
            .expect("Failed to add test system");

        let dat_file = DatFile {
            header: DatHeader {
                name: "Test DAT".to_string(),
                version: "1.0".to_string(),
                ..Default::default()
            },
            games: vec![DatGame {
                name: "Test Game".to_string(),
                description: "Test Description".to_string(),
                roms: vec![DatRom {
                    name: "test.bin".to_string(),
                    size: 1024,
                    sha1: "0000000000000000000000000000000000000001".to_string(),
                    ..Default::default()
                }],
                ..Default::default()
            }],
        };

        let dat_file_id = deps
            .repository_manager
            .get_dat_repository()
            .add_dat_file(AddDatFileParams {
                dat_id: dat_file.header.id,
                name: dat_file.header.name.as_str(),
                description: dat_file.header.description.as_str(),
                version: dat_file.header.version.as_str(),
                date: dat_file.header.date.as_deref(),
                author: dat_file.header.author.as_str(),
                homepage: dat_file.header.homepage.as_deref(),
                url: dat_file.header.url.as_deref(),
                subset: dat_file.header.subset.as_deref(),
                system_id,
            })
            .await
            .expect("Failed to add dat file");

        // Create an existing file set without a release (simulating a file set that was imported before)
        let existing_file_set_id = deps
            .repository_manager
            .get_file_set_repository()
            .add_file_set(
                &dat_file.games[0].name,
                &dat_file.games[0].name,
                &core_types::FileType::Rom,
                &dat_file.header.get_source(),
                &[],
                &[system_id],
            )
            .await
            .expect("Failed to create existing file set");

        // Use real FileSetService
        use crate::file_set::file_set_service::FileSetService;
        let file_set_service = Arc::new(FileSetService::new(deps.repository_manager.clone()));

        let ops = DatFileMassImportOps {
            dat_file_parser_ops: Arc::new(MockDatParser::new(Ok(dat_file.clone().into()))),
            fs_ops: Arc::new(MockFileSystemOps::new()),
            reader_factory_fn: Arc::new(create_mock_reader_factory(HashMap::new(), vec![])),
            file_set_service_ops: file_set_service,
            file_import_service_ops: Arc::new(MockFileImportServiceOps::new()),
        };

        let mut context = DatFileMassImportContext::new(
            MassImportDeps {
                repository_manager: deps.repository_manager.clone(),
            },
            MassImportInput {
                source_path: PathBuf::from("/path/to/source"),
                dat_file_path: Some(PathBuf::from("/path/to/datfile.dat")),
                file_type: core_types::FileType::Rom,
                item_type: None,
                system_id,
            },
            ops,
            None,
        );

        // Pre-populate state as if previous steps found an existing file set without release
        context.state.dat_file_id = Some(dat_file_id);
        context.state.statuses = vec![
            DatGameFileSetStatus::ExistingWithoutReleaseAndWithoutLinkToDat {
                game: dat_file.games[0].clone(),
                file_set_id: existing_file_set_id,
            },
        ];

        // Act
        let step = LinkExistingFileSetsStep;
        let result = step.execute(&mut context).await;

        // Assert
        assert!(matches!(result, StepAction::Continue));

        // Verify that the file set is now linked to a release
        let is_in_release = deps
            .repository_manager
            .get_file_set_repository()
            .is_file_set_in_release(existing_file_set_id)
            .await
            .expect("Failed to check if file set is in release");
        assert!(is_in_release, "File set should now be linked to a release");

        let release = deps
            .repository_manager
            .get_release_repository()
            .get_releases(None, vec![], Some(existing_file_set_id))
            .await
            .expect("Failed to get release by file set ID");

        assert_eq!(
            release.len(),
            1,
            "There should be exactly one release linked to the file set"
        );

        let release = &release[0];

        assert_eq!(
            release.name,
            dat_file.games[0].get_release_name(),
            "Release name should match the expected format"
        );

        let software_title = deps
            .repository_manager
            .get_software_title_repository()
            .get_software_titles_by_release(release.id)
            .await
            .expect("Failed to get software title by ID");

        assert_eq!(
            software_title.len(),
            1,
            "There should be exactly one software title linked to the release"
        );

        let software_title = &software_title[0];

        assert_eq!(
            software_title.name,
            dat_file.games[0].get_software_title_name(),
            "Software title name should match the expected format"
        );
    }

    // TODO: create a test case where re-importing the same dat file
    // - shouldn't create a new dat file in the database
    // - shouldn't create duplicate file sets
    // - should create file sets with they do not exist
    // - shouldn't create duplicate releases if file set and release already exist for the same dat
    // file and game combination
}
