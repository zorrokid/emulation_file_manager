use std::sync::Arc;

use dat_file_parser::DatFile as ParserDatFile;
use database::{
    helper::{AddDatFileParams, AddDatGameParams, AddDatRomParams},
    repository_manager::RepositoryManager,
};
use domain::naming_conventions::no_intro::{DatFile, DatGame, DatHeader, DatRom};

use crate::error::Error;

pub struct DatFileService {
    repository_manager: Arc<RepositoryManager>,
}

impl DatFileService {
    pub fn new(repository_manager: Arc<RepositoryManager>) -> Self {
        DatFileService { repository_manager }
    }

    pub async fn store_dat_file(
        &self,
        dat_file: &ParserDatFile,
        system_id: i64,
    ) -> Result<i64, Error> {
        let mut transaction = self
            .repository_manager
            .begin_transaction()
            .await
            .map_err(|e| Error::DbError(format!("{:?}", e)))?;

        let repo = self.repository_manager.get_dat_repository();

        let add_dat_file_params = AddDatFileParams {
            dat_id: dat_file.header.id,
            name: &dat_file.header.name,
            description: &dat_file.header.description,
            version: &dat_file.header.version,
            date: dat_file.header.date.as_deref(),
            author: &dat_file.header.author,
            homepage: dat_file.header.homepage.as_deref(),
            url: dat_file.header.url.as_deref(),
            subset: dat_file.header.subset.as_deref(),
            system_id,
        };

        let dat_file_id = repo
            .add_dat_file_with_tx(add_dat_file_params, &mut transaction)
            .await
            .map_err(|e| Error::DbError(format!("{:?}", e)))?;

        for game in &dat_file.games {
            let game_params = AddDatGameParams {
                dat_file_id,
                name: &game.name,
                game_id: game.id.as_deref(),
                description: &game.description,
                cloneof: game.cloneof.as_deref(),
                cloneofid: game.cloneofid.as_deref(),
            };
            let game_id = repo
                .add_dat_game_with_tx(game_params, &mut transaction)
                .await
                .map_err(|e| Error::DbError(format!("{:?}", e)))?;
            for rom in &game.roms {
                let rom_params = AddDatRomParams {
                    dat_game_id: game_id,
                    name: &rom.name,
                    size: rom.size as i64,
                    crc: &rom.crc,
                    md5: &rom.md5,
                    sha1: &rom.sha1,
                    sha256: rom.sha256.as_deref(),
                    status: rom.status.as_deref(),
                    serial: rom.serial.as_deref(),
                    header: rom.header.as_deref(),
                };
                repo.add_dat_rom_with_tx(rom_params, &mut transaction)
                    .await
                    .map_err(|e| Error::DbError(format!("{:?}", e)))?;
            }
        }

        transaction
            .commit()
            .await
            .map_err(|e| Error::DbError(format!("{:?}", e)))?;

        Ok(dat_file_id)
    }

    pub async fn fetch_dat_file(&self, dat_file_id: i64) -> Result<DatFile, Error> {
        let dat_file = self
            .repository_manager
            .get_dat_repository()
            .get_dat_file(dat_file_id)
            .await
            .map_err(|e| Error::DbError(format!("{:?}", e)))?;

        let dat_header = DatHeader {
            id: dat_file.id as i32,
            name: dat_file.name,
            description: dat_file.description,
            version: dat_file.version,
            date: dat_file.date,
            author: dat_file.author,
            homepage: dat_file.homepage,
            url: dat_file.url,
            subset: dat_file.subset,
        };

        let games = self
            .repository_manager
            .get_dat_repository()
            .get_games_in_dat_file(dat_file_id)
            .await
            .map_err(|e| Error::DbError(format!("{:?}", e)))?;

        let mut games_out: Vec<DatGame> = Vec::new();
        for game in &games {
            let roms = self
                .repository_manager
                .get_dat_repository()
                .get_roms_in_game(game.id)
                .await
                .map_err(|e| Error::DbError(format!("{:?}", e)))?
                .into_iter()
                .map(|rom| DatRom {
                    name: rom.name,
                    size: rom.size as u64,
                    crc: rom.crc,
                    md5: rom.md5,
                    sha1: rom.sha1,
                    sha256: rom.sha256,
                    status: rom.status,
                    serial: rom.serial,
                    header: rom.header,
                })
                .collect();
            let game = DatGame {
                name: game.name.clone(),
                id: game.game_id.clone(),
                description: game.description.clone(),
                cloneof: game.cloneof.clone(),
                cloneofid: game.cloneofid.clone(),
                categories: vec![], // Categories are not stored in the current implementation
                roms,
                releases: vec![], // Releases are not stored in the current implementation
            };
            games_out.push(game);
        }
        Ok(DatFile {
            header: dat_header,
            games: games_out,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dat_file_parser::{
        DatFile as ParserDatFile, DatGame as ParserDatGame, DatHeader as ParserDatHeader,
        DatRom as ParserDatRom,
    };
    use database::setup_test_db;

    #[async_std::test]
    async fn test_store_dat_file() {
        let pool = Arc::new(setup_test_db().await);
        let repository_manager = Arc::new(RepositoryManager::new(pool));
        let system_id = repository_manager
            .get_system_repository()
            .add_system("Test System")
            .await
            .unwrap();

        let dat_file = ParserDatFile {
            header: ParserDatHeader {
                id: 1,
                name: "Test DAT".to_string(),
                description: "Test DAT file".to_string(),
                version: "1.0".to_string(),
                date: Some("2026-01-01".to_string()),
                author: "Test Author".to_string(),
                homepage: Some("https://test.com".to_string()),
                url: Some("https://test.com/dat".to_string()),
                subset: None,
            },
            games: vec![ParserDatGame {
                name: "Test Game".to_string(),
                id: Some("game1".to_string()),
                description: "Test game description".to_string(),
                cloneof: None,
                cloneofid: None,
                categories: vec![],
                roms: vec![ParserDatRom {
                    name: "test.rom".to_string(),
                    size: 1024,
                    crc: "ABCD1234".to_string(),
                    md5: "d41d8cd98f00b204e9800998ecf8427e".to_string(),
                    sha1: "da39a3ee5e6b4b0d3255bfef95601890afd80709".to_string(),
                    sha256: Some(
                        "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
                            .to_string(),
                    ),
                    status: Some("verified".to_string()),
                    serial: None,
                    header: None,
                }],
                releases: vec![],
            }],
        };

        let service = DatFileService::new(repository_manager.clone());
        let dat_file_id = service.store_dat_file(&dat_file, system_id).await.unwrap();

        assert!(dat_file_id > 0);

        let stored_dat = repository_manager
            .get_dat_repository()
            .get_dat_file(dat_file_id)
            .await
            .unwrap();
        assert_eq!(stored_dat.name, "Test DAT");
        assert_eq!(stored_dat.version, "1.0");

        let games = repository_manager
            .get_dat_repository()
            .get_games_in_dat_file(dat_file_id)
            .await
            .unwrap();
        assert_eq!(games.len(), 1);
        assert_eq!(games[0].name, "Test Game");

        let dat_file = service.fetch_dat_file(dat_file_id).await.unwrap();
        assert_eq!(dat_file.header.name, "Test DAT");
        assert_eq!(dat_file.header.version, "1.0");
        assert_eq!(dat_file.header.author, "Test Author");
        assert_eq!(
            dat_file.header.homepage,
            Some("https://test.com".to_string())
        );
        assert_eq!(dat_file.games.len(), 1);
        assert_eq!(dat_file.games[0].name, "Test Game");
    }
}
