use std::sync::Arc;

use core_types::{FileSetEqualitySpecs, FileSetFileEqualitySpecs, FileType, sha1_from_hex_string};
use database::repository_manager::RepositoryManager;
use domain::naming_conventions::no_intro::{DatGame, DatHeader};

use crate::error::Error;

#[derive(Debug, PartialEq, Eq)]
pub enum DatGameFileSetStatus {
    NonExisting(DatGame),
    /// Existing with release means that both Software Title and Release matches
    ExistingWithReleaseAndLinkedToDat {
        file_set_id: i64,
        game: DatGame,
    },
    // Create Release and Software Title for this and link to Dat
    ExistingWithoutReleaseAndWithoutLinkToDat {
        file_set_id: i64,
        game: DatGame,
    },
}

pub struct DatGameStatusService {
    repository_manager: Arc<RepositoryManager>,
}

impl DatGameStatusService {
    pub fn new(repository_manager: Arc<RepositoryManager>) -> Self {
        Self { repository_manager }
    }

    pub async fn get_status(
        &self,
        game: &DatGame,
        file_type: FileType,
        header: &DatHeader,
        dat_file_id: i64,
    ) -> Result<DatGameFileSetStatus, Error> {
        let mut file_set_file_info: Vec<FileSetFileEqualitySpecs> = Vec::new();
        for rom in &game.roms {
            let sha1_checksum = match sha1_from_hex_string(&rom.sha1) {
                Ok(checksum) => checksum,
                Err(e) => {
                    tracing::error!(
                        error = ?e,
                        rom_sha1 = %rom.sha1,
                        rom_name = %rom.name,
                        "Failed to parse SHA1 checksum from hex string",
                    );
                    return Err(Error::ParseError(format!(
                        "Failed to parse SHA1 checksum for ROM '{}': {}",
                        rom.name, e
                    )));
                }
            };

            file_set_file_info.push(FileSetFileEqualitySpecs {
                file_name: rom.name.clone(),
                file_type,
                sha1_checksum,
            });
        }

        let file_set_equality_specs = FileSetEqualitySpecs {
            file_set_name: game.name.clone(),
            file_set_file_name: game.name.clone(),
            file_type,
            source: header.get_source(),
            file_set_file_info,
        };

        let existing_file_set_res = self
            .repository_manager
            .get_file_set_repository()
            .find_file_set(&file_set_equality_specs)
            .await;

        match existing_file_set_res {
            Ok(Some(existing_file_set_id)) => {
                tracing::info!(
                    file_set_name = %game.name,
                    file_set_id = existing_file_set_id,
                    "Found existing file set matching the game in DAT file",
                );
                // Let's check if this file set is already linked to the dat game
                let file_set_dat = self
                    .repository_manager
                    .get_file_set_repository()
                    .get_dat_files_for_file_set(existing_file_set_id)
                    .await;
                match file_set_dat {
                    Ok(dat_file_ids) => {
                        // TODO: currently we just assume that when file set is linked to a dat
                        // file it's created properly by creating also a related release and
                        // software title. We should probably check that as well in the future.
                        if dat_file_ids.iter().any(|dat_file| *dat_file == dat_file_id) {
                            tracing::info!(
                                file_set_name = %game.name,
                                file_set_id = existing_file_set_id,
                                "Existing file set is already linked to the DAT game",
                            );
                            return Ok(DatGameFileSetStatus::ExistingWithReleaseAndLinkedToDat {
                                file_set_id: existing_file_set_id,
                                game: game.clone(),
                            });
                        } else {
                            return Ok(
                                DatGameFileSetStatus::ExistingWithoutReleaseAndWithoutLinkToDat {
                                    file_set_id: existing_file_set_id,
                                    game: game.clone(),
                                },
                            );
                        }
                    }
                    Err(e) => {
                        tracing::error!(
                            error = ?e,
                            file_set_name = %game.name,
                            file_set_id = existing_file_set_id,
                            "Failed to check if existing file set is linked to the DAT game",
                        );
                        return Err(Error::DbError(format!(
                            "Failed to check if existing file set with id {} is linked to the DAT game '{}': {}",
                            existing_file_set_id, game.name, e
                        )));
                    }
                }
            }
            Ok(None) => {
                tracing::info!(
                    file_set_name = %game.name,
                    "No existing file set found matching the game in DAT file, it will be imported as new",
                );
                return Ok(DatGameFileSetStatus::NonExisting(game.clone()));
            }
            Err(e) => {
                tracing::error!(
                    error = ?e,
                    file_set_name = %game.name,
                    "Failed to check for existing file set matching the game in DAT file",
                );
                return Err(Error::DbError(format!(
                    "Failed to check for existing file set matching the game '{}': {}",
                    game.name, e
                )));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use core_types::ImportedFile;
    use database::{helper::AddDatFileParams, setup_test_repository_manager};
    use domain::naming_conventions::no_intro::DatRom;

    use super::*;

    #[async_std::test]
    async fn test_get_status_non_existing_file_set() {
        // Arrange
        let repository_manager = setup_test_repository_manager().await;
        let service = DatGameStatusService::new(repository_manager);
        let game = DatGame {
            name: "Test Game".to_string(),
            roms: vec![DatRom {
                name: "test_rom.bin".to_string(),
                size: 1024,
                sha1: "1234567890abcdef1234567890abcdef12345678".to_string(),
                ..Default::default()
            }],
            ..Default::default()
        };
        let header = DatHeader {
            name: "Test Dat".to_string(),
            version: "1.0".to_string(),
            ..Default::default()
        };

        let file_type = FileType::Rom;
        let dat_file_id = 1;

        // Act
        let result = service
            .get_status(&game, file_type, &header, dat_file_id)
            .await;

        // Assert
        assert!(
            result.is_ok(),
            "Expected get_status to succeed, but got error: {:?}",
            result.err()
        );
        let status = result.unwrap();
        assert_eq!(
            status,
            DatGameFileSetStatus::NonExisting(game.clone()),
            "Expected status to be NonExisting, but got: {:?}",
            status
        );
    }

    #[async_std::test]
    async fn test_get_status_existing_file_set_not_linked_to_dat() {
        // Arrange
        let repository_manager = setup_test_repository_manager().await;

        let service = DatGameStatusService::new(repository_manager);
        let game = DatGame {
            name: "Test Game".to_string(),
            roms: vec![DatRom {
                name: "test_rom.bin".to_string(),
                size: 1024,
                sha1: "1234567890abcdef1234567890abcdef12345678".to_string(),
                ..Default::default()
            }],
            ..Default::default()
        };
        let header = DatHeader {
            name: "Test Dat".to_string(),
            version: "1.0".to_string(),
            ..Default::default()
        };

        let source = header.get_source();

        let file_type = FileType::Rom;
        let dat_file_id = 1;

        let system_id = service
            .repository_manager
            .get_system_repository()
            .add_system("Test System")
            .await
            .expect("Failed to add system to repository");

        let file_set_id = service
            .repository_manager
            .get_file_set_repository()
            .add_file_set(
                "Test Game",
                "Test Game",
                &file_type,
                &source,
                &[ImportedFile {
                    original_file_name: "test_rom.bin".to_string(),
                    archive_file_name: "test_rom.bin".to_string(),
                    file_size: 1024,
                    sha1_checksum: sha1_from_hex_string(&game.roms[0].sha1).unwrap(),
                }],
                &[system_id],
            )
            .await
            .expect("Failed to add file set to repository");

        // Act
        let result = service
            .get_status(&game, file_type, &header, dat_file_id)
            .await;

        // Assert
        assert!(
            result.is_ok(),
            "Expected get_status to succeed, but got error: {:?}",
            result.err()
        );
        let status = result.unwrap();
        assert_eq!(
            status,
            DatGameFileSetStatus::ExistingWithoutReleaseAndWithoutLinkToDat {
                file_set_id,
                game: game.clone()
            },
            "Expected status to be NonExisting, but got: {:?}",
            status
        );
    }

    #[async_std::test]
    async fn test_get_status_existing_file_set_linked_to_dat() {
        // Arrange
        let repository_manager = setup_test_repository_manager().await;

        let service = DatGameStatusService::new(repository_manager.clone());
        let game = DatGame {
            name: "Test Game".to_string(),
            roms: vec![DatRom {
                name: "test_rom.bin".to_string(),
                size: 1024,
                sha1: "1234567890abcdef1234567890abcdef12345678".to_string(),
                ..Default::default()
            }],
            ..Default::default()
        };
        let header = DatHeader {
            name: "Test Dat".to_string(),
            version: "1.0".to_string(),
            ..Default::default()
        };

        let source = header.get_source();

        let file_type = FileType::Rom;

        let system_id = service
            .repository_manager
            .get_system_repository()
            .add_system("Test System")
            .await
            .expect("Failed to add system to repository");

        let add_dat_file_params = AddDatFileParams {
            dat_id: 1,
            name: &header.name,
            description: &header.description,
            version: &header.version,
            date: header.date.as_deref(),
            author: &header.author,
            homepage: header.homepage.as_deref(),
            url: header.url.as_deref(),
            subset: header.subset.as_deref(),
            system_id,
        };

        let dat_file_id = service
            .repository_manager
            .get_dat_repository()
            .add_dat_file(add_dat_file_params)
            .await
            .expect("Failed to add dat file to repository");

        let file_set_id = service
            .repository_manager
            .get_file_set_repository()
            .add_file_set(
                "Test Game",
                "Test Game",
                &file_type,
                &source,
                &[ImportedFile {
                    original_file_name: "test_rom.bin".to_string(),
                    archive_file_name: "test_rom.bin".to_string(),
                    file_size: 1024,
                    sha1_checksum: sha1_from_hex_string(&game.roms[0].sha1).unwrap(),
                }],
                &[system_id],
            )
            .await
            .expect("Failed to add file set to repository");

        repository_manager
            .get_file_set_repository()
            .link_file_set_to_dat_file(file_set_id, dat_file_id)
            .await
            .expect("Failed to link file set to dat file");

        // Act
        let result = service
            .get_status(&game, file_type, &header, dat_file_id)
            .await;

        // Assert
        assert!(
            result.is_ok(),
            "Expected get_status to succeed, but got error: {:?}",
            result.err()
        );
        let status = result.unwrap();
        assert_eq!(
            status,
            DatGameFileSetStatus::ExistingWithReleaseAndLinkedToDat {
                file_set_id,
                game: game.clone()
            },
            "Expected status to be NonExisting, but got: {:?}",
            status
        );
    }
}
