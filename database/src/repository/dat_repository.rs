use std::sync::Arc;

use sqlx::{Pool, Sqlite};

use crate::{
    database_error::Error,
    helper::{AddDatFileParams, AddDatGameParams, AddDatRomParams},
    models::{DatFile, DatGame, DatRom},
};

#[derive(Debug)]
pub struct DatRepository {
    pool: Arc<Pool<Sqlite>>,
}

impl DatRepository {
    pub fn new(pool: Arc<Pool<Sqlite>>) -> Self {
        Self { pool }
    }
    pub async fn add_dat_file(&self, params: AddDatFileParams<'_>) -> Result<i64, Error> {
        let mut transaction = self.pool.begin().await?;
        let id = self.add_dat_file_with_tx(params, &mut transaction).await?;
        transaction.commit().await?;
        Ok(id)
    }

    pub async fn add_dat_file_with_tx(
        &self,
        params: AddDatFileParams<'_>,
        tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    ) -> Result<i64, Error> {
        println!("Inserting DAT file with system: {}", params.system_id);
        let result = sqlx::query!(
            "INSERT INTO dat_file (
                dat_id, 
                name, 
                description, 
                version, 
                date, 
                author, 
                homepage, 
                url, 
                subset, 
                system_id
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            params.dat_id,
            params.name,
            params.description,
            params.version,
            params.date,
            params.author,
            params.homepage,
            params.url,
            params.subset,
            params.system_id
        )
        .execute(&mut **tx)
        .await?;
        Ok(result.last_insert_rowid())
    }

    pub async fn add_dat_game(&self, params: AddDatGameParams<'_>) -> Result<i64, Error> {
        let mut transaction = self.pool.begin().await?;
        let id = self.add_dat_game_with_tx(params, &mut transaction).await?;
        transaction.commit().await?;
        Ok(id)
    }

    pub async fn add_dat_game_with_tx(
        &self,
        params: AddDatGameParams<'_>,
        tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    ) -> Result<i64, Error> {
        let result = sqlx::query!(
            "INSERT INTO dat_game (
                dat_file_id, 
                name, 
                game_id, 
                description, 
                cloneof, 
                cloneofid
            ) VALUES (?, ?, ?, ?, ?, ?)",
            params.dat_file_id,
            params.name,
            params.game_id,
            params.description,
            params.cloneof,
            params.cloneofid
        )
        .execute(&mut **tx)
        .await?;
        Ok(result.last_insert_rowid())
    }

    pub async fn add_dat_rom(&self, params: AddDatRomParams<'_>) -> Result<i64, Error> {
        let mut transaction = self.pool.begin().await?;
        let id = self.add_dat_rom_with_tx(params, &mut transaction).await?;
        transaction.commit().await?;
        Ok(id)
    }

    pub async fn add_dat_rom_with_tx(
        &self,
        params: AddDatRomParams<'_>,
        tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    ) -> Result<i64, Error> {
        let result = sqlx::query!(
            "INSERT INTO dat_rom (
                dat_game_id, 
                name, 
                size, 
                crc, 
                md5, 
                sha1, 
                sha256, 
                status, 
                serial, 
                header
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            params.dat_game_id,
            params.name,
            params.size,
            params.crc,
            params.md5,
            params.sha1,
            params.sha256,
            params.status,
            params.serial,
            params.header
        )
        .execute(&mut **tx)
        .await?;
        Ok(result.last_insert_rowid())
    }

    pub async fn get_dat_file(&self, id: i64) -> Result<DatFile, Error> {
        let dat_file = sqlx::query_as!(
            DatFile,
            "SELECT id, dat_id, name, description, version, date, author, homepage, url, subset, system_id, imported_at
             FROM dat_file
             WHERE id = ?",
            id
        )
        .fetch_one(&*self.pool)
        .await?;
        Ok(dat_file)
    }

    pub async fn get_games_in_dat_file(&self, dat_file_id: i64) -> Result<Vec<DatGame>, Error> {
        let games = sqlx::query_as!(
            DatGame,
            "SELECT id, dat_file_id, name, game_id, description, cloneof, cloneofid
             FROM dat_game
             WHERE dat_file_id = ?
             ORDER BY name",
            dat_file_id
        )
        .fetch_all(&*self.pool)
        .await?;
        Ok(games)
    }

    pub async fn get_roms_in_game(&self, dat_game_id: i64) -> Result<Vec<DatRom>, Error> {
        let roms = sqlx::query_as!(
            DatRom,
            "SELECT id, dat_game_id, name, size, crc, md5, sha1, sha256, status, serial, header
             FROM dat_rom
             WHERE dat_game_id = ?
             ORDER BY name",
            dat_game_id
        )
        .fetch_all(&*self.pool)
        .await?;
        Ok(roms)
    }

    pub async fn get_game_by_rom_sha1(&self, sha1: &str) -> Result<Option<DatGame>, Error> {
        let game = sqlx::query_as!(
            DatGame,
            "SELECT g.id, g.dat_file_id, g.name, g.game_id, g.description, g.cloneof, g.cloneofid
             FROM dat_game g
             INNER JOIN dat_rom r ON g.id = r.dat_game_id
             WHERE r.sha1 = ?
             LIMIT 1",
            sha1
        )
        .fetch_optional(&*self.pool)
        .await?;
        Ok(game)
    }

    pub async fn check_dat_file_exists(
        &self,
        version: &str,
        name: &str,
        system_id: i64,
    ) -> Result<Option<i64>, Error> {
        let id = sqlx::query_scalar!(
            "SELECT id FROM dat_file WHERE version = ? AND name = ? AND system_id = ?",
            version,
            name,
            system_id
        )
        .fetch_optional(&*self.pool)
        .await?;
        Ok(id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{repository::system_repository::SystemRepository, setup_test_db};

    fn create_dat_file_params<'a>(system_id: i64) -> AddDatFileParams<'a> {
        AddDatFileParams {
            dat_id: 1,
            name: "Test DAT",
            description: "A test DAT file",
            version: "1.0",
            date: Some("2025-01-01"),
            author: "Test Author",
            homepage: Some("https://example.com"),
            url: Some("https://example.com/dat"),
            subset: None,
            system_id,
        }
    }

    #[async_std::test]
    async fn test_add_dat_file() {
        let pool = Arc::new(setup_test_db().await);
        let system_repo = SystemRepository::new(pool.clone());
        let dat_repo = DatRepository::new(pool.clone());

        // Create a system first
        let system_id = system_repo.add_system("Nintendo 64").await.unwrap();

        let params = AddDatFileParams {
            dat_id: 1,
            name: "Nintendo 64",
            description: "Nintendo 64 ROMs",
            version: "20250118",
            date: Some("2025-01-18"),
            author: "No-Intro",
            homepage: Some("https://no-intro.org"),
            url: Some("https://datomatic.no-intro.org"),
            subset: None,
            system_id,
        };

        // Add a DAT file
        let dat_file_id = dat_repo.add_dat_file(params).await.unwrap();

        assert!(dat_file_id > 0);

        // Verify it was inserted
        let result = sqlx::query!("SELECT * FROM dat_file WHERE id = ?", dat_file_id)
            .fetch_one(&*pool)
            .await
            .unwrap();

        assert_eq!(result.dat_id, 1);
        assert_eq!(result.name, "Nintendo 64");
        assert_eq!(result.description, "Nintendo 64 ROMs");
        assert_eq!(result.version, "20250118");
        assert_eq!(result.date, Some("2025-01-18".to_string()));
        assert_eq!(result.author, "No-Intro");
        assert_eq!(result.homepage, Some("https://no-intro.org".to_string()));
        assert_eq!(
            result.url,
            Some("https://datomatic.no-intro.org".to_string())
        );
        assert_eq!(result.subset, None);
        assert_eq!(result.system_id, system_id);
    }

    #[async_std::test]
    async fn test_add_dat_game() {
        let pool = Arc::new(setup_test_db().await);
        let system_repo = SystemRepository::new(pool.clone());
        let dat_repo = DatRepository::new(pool.clone());

        // Create system and DAT file
        let system_id = system_repo.add_system("SNES").await.unwrap();
        let dat_file_params = create_dat_file_params(system_id);
        let dat_file_id = dat_repo.add_dat_file(dat_file_params).await.unwrap();

        let game_params = AddDatGameParams {
            dat_file_id,
            name: "Super Mario World",
            game_id: Some("0001"),
            description: "Super Mario World (USA)",
            cloneof: None,
            cloneofid: None,
        };
        // Add a game
        let game_id = dat_repo.add_dat_game(game_params).await.unwrap();

        assert!(game_id > 0);

        // Verify it was inserted
        let result = sqlx::query!("SELECT * FROM dat_game WHERE id = ?", game_id)
            .fetch_one(&*pool)
            .await
            .unwrap();

        assert_eq!(result.dat_file_id, dat_file_id);
        assert_eq!(result.name, "Super Mario World");
        assert_eq!(result.game_id, Some("0001".to_string()));
        assert_eq!(result.description, "Super Mario World (USA)");
        assert_eq!(result.cloneof, None);
        assert_eq!(result.cloneofid, None);
    }

    #[async_std::test]
    async fn test_add_dat_rom() {
        let pool = Arc::new(setup_test_db().await);
        let system_repo = SystemRepository::new(pool.clone());
        let dat_repo = DatRepository::new(pool.clone());

        // Create system, DAT file, and game
        let system_id = system_repo.add_system("Game Boy").await.unwrap();
        let dat_file_params = create_dat_file_params(system_id);
        let dat_file_id = dat_repo.add_dat_file(dat_file_params).await.unwrap();

        let game_params = AddDatGameParams {
            dat_file_id,
            name: "Tetris",
            game_id: Some("0100"),
            description: "Tetris (World)",
            cloneof: None,
            cloneofid: None,
        };
        let game_id = dat_repo.add_dat_game(game_params).await.unwrap();

        // Add a ROM
        let rom_params = AddDatRomParams {
            dat_game_id: game_id,
            name: "Tetris (World).gb",
            size: 32768,
            crc: "46df91ad",
            md5: "5a6e7a59d8f95d4c0f6c2f1e3e8c4a98",
            sha1: "b58ce7e7c37e5e8e3a7ff5bb3e0ad6b48c8c8d6e",
            sha256: Some("abc123def456"),
            status: Some("verified"),
            serial: Some("DMG-TR-USA"),
            header: None,
        };
        let rom_id = dat_repo.add_dat_rom(rom_params).await.unwrap();

        assert!(rom_id > 0);

        // Verify it was inserted
        let result = sqlx::query!("SELECT * FROM dat_rom WHERE id = ?", rom_id)
            .fetch_one(&*pool)
            .await
            .unwrap();

        assert_eq!(result.dat_game_id, game_id);
        assert_eq!(result.name, "Tetris (World).gb");
        assert_eq!(result.size, 32768);
        assert_eq!(result.crc, "46df91ad");
        assert_eq!(result.md5, "5a6e7a59d8f95d4c0f6c2f1e3e8c4a98");
        assert_eq!(result.sha1, "b58ce7e7c37e5e8e3a7ff5bb3e0ad6b48c8c8d6e");
        assert_eq!(result.sha256, Some("abc123def456".to_string()));
        assert_eq!(result.status, Some("verified".to_string()));
        assert_eq!(result.serial, Some("DMG-TR-USA".to_string()));
        assert_eq!(result.header, None);
    }

    #[async_std::test]
    async fn test_cascade_delete() {
        let pool = Arc::new(setup_test_db().await);
        let system_repo = SystemRepository::new(pool.clone());
        let dat_repo = DatRepository::new(pool.clone());

        // Create full hierarchy
        let system_id = system_repo.add_system("NES").await.unwrap();
        let dat_file_params = create_dat_file_params(system_id);
        let dat_file_id = dat_repo.add_dat_file(dat_file_params).await.unwrap();
        let game_params = AddDatGameParams {
            dat_file_id,
            name: "Super Mario Bros.",
            game_id: Some("0050"),
            description: "Super Mario Bros. (USA)",
            cloneof: None,
            cloneofid: None,
        };
        let game_id = dat_repo.add_dat_game(game_params).await.unwrap();
        let rom_params = AddDatRomParams {
            dat_game_id: game_id,
            name: "Super Mario Bros. (USA).nes",
            size: 40976,
            crc: "3337ec46",
            md5: "811b027eaf99c2def7b933c5208636de",
            sha1: "ea343f4e445a9050d4b4fbac2c77d0693b1d0922",
            sha256: None,
            status: None,
            serial: None,
            header: None,
        };
        let rom_id = dat_repo.add_dat_rom(rom_params).await.unwrap();

        // Delete the DAT file - should cascade to game and ROM
        sqlx::query!("DELETE FROM dat_file WHERE id = ?", dat_file_id)
            .execute(&*pool)
            .await
            .unwrap();

        // Verify game was deleted
        let game_count = sqlx::query_scalar!("SELECT COUNT(*) FROM dat_game WHERE id = ?", game_id)
            .fetch_one(&*pool)
            .await
            .unwrap();
        assert_eq!(game_count, 0);

        // Verify ROM was deleted
        let rom_count = sqlx::query_scalar!("SELECT COUNT(*) FROM dat_rom WHERE id = ?", rom_id)
            .fetch_one(&*pool)
            .await
            .unwrap();
        assert_eq!(rom_count, 0);
    }

    #[async_std::test]
    async fn test_get_dat_file() {
        let pool = Arc::new(setup_test_db().await);
        let system_repo = SystemRepository::new(pool.clone());
        let dat_repo = DatRepository::new(pool.clone());

        let system_id = system_repo.add_system("Genesis").await.unwrap();
        let dat_file_params = AddDatFileParams {
            dat_id: 5,
            name: "Sega Genesis",
            description: "Genesis ROMs",
            version: "20250118-1900",
            date: Some("2025-01-18"),
            author: "No-Intro Team",
            homepage: Some("https://no-intro.org"),
            url: Some("https://datomatic.no-intro.org"),
            subset: Some("USA"),
            system_id,
        };
        let dat_file_id = dat_repo.add_dat_file(dat_file_params).await.unwrap();

        let dat_file = dat_repo.get_dat_file(dat_file_id).await.unwrap();

        assert_eq!(dat_file.id, dat_file_id);
        assert_eq!(dat_file.dat_id, 5);
        assert_eq!(dat_file.name, "Sega Genesis");
        assert_eq!(dat_file.description, "Genesis ROMs");
        assert_eq!(dat_file.version, "20250118-1900");
        assert_eq!(dat_file.date, Some("2025-01-18".to_string()));
        assert_eq!(dat_file.author, "No-Intro Team");
        assert_eq!(dat_file.system_id, system_id);
    }

    #[async_std::test]
    async fn test_get_games_in_dat_file() {
        let pool = Arc::new(setup_test_db().await);
        let system_repo = SystemRepository::new(pool.clone());
        let dat_repo = DatRepository::new(pool.clone());

        let system_id = system_repo.add_system("Atari 2600").await.unwrap();
        let dat_file_params = AddDatFileParams {
            dat_id: 6,
            name: "Atari 2600",
            description: "Atari 2600 ROMs",
            version: "20250118",
            date: None,
            author: "No-Intro",
            homepage: None,
            url: None,
            subset: None,
            system_id,
        };
        let dat_file_id = dat_repo.add_dat_file(dat_file_params).await.unwrap();

        // Add multiple games
        let game_1_params = AddDatGameParams {
            dat_file_id,
            name: "Pac-Man",
            game_id: Some("0100"),
            description: "",
            cloneof: None,
            cloneofid: None,
        };
        let game_2_params = AddDatGameParams {
            dat_file_id,
            name: "Adventure",
            game_id: Some("0050"),
            description: "",
            cloneof: None,
            cloneofid: None,
        };

        let game1_id = dat_repo.add_dat_game(game_1_params).await.unwrap();

        let game2_id = dat_repo.add_dat_game(game_2_params).await.unwrap();

        let games = dat_repo.get_games_in_dat_file(dat_file_id).await.unwrap();

        assert_eq!(games.len(), 2);
        // Should be sorted by name: Adventure comes before Pac-Man
        assert_eq!(games[0].name, "Adventure");
        assert_eq!(games[0].id, game2_id);
        assert_eq!(games[1].name, "Pac-Man");
        assert_eq!(games[1].id, game1_id);
    }

    #[async_std::test]
    async fn test_get_roms_in_game() {
        let pool = Arc::new(setup_test_db().await);
        let system_repo = SystemRepository::new(pool.clone());
        let dat_repo = DatRepository::new(pool.clone());

        let system_id = system_repo.add_system("Master System").await.unwrap();
        let dat_file_params = create_dat_file_params(system_id);
        let dat_file_id = dat_repo.add_dat_file(dat_file_params).await.unwrap();

        let game_params = AddDatGameParams {
            dat_file_id,
            name: "Sonic the Hedgehog",
            game_id: Some("0200"),
            description: "Sonic the Hedgehog (USA, Europe)",
            cloneof: None,
            cloneofid: None,
        };
        let game_id = dat_repo.add_dat_game(game_params).await.unwrap();

        // Add multiple ROMs
        let rom_1_params = AddDatRomParams {
            dat_game_id: game_id,
            name: "Sonic the Hedgehog (USA).sms",
            size: 131072,
            crc: "b04e1c0a",
            md5: "5c3c9fd2f44d8b9d2e6f5e3e1e0e9a5d",
            sha1: "bd507642be0d1b2c8f2ae0f0ee2f9c3b6f1e8e7e",
            sha256: Some("sha256hash1"),
            status: Some("verified"),
            serial: None,
            header: None,
        };

        let rom_2_params = AddDatRomParams {
            dat_game_id: game_id,
            name: "Sonic the Hedgehog (Europe).sms",
            size: 131072,
            crc: "a15c3f81",
            md5: "6d4d0fe3f55e9c0d3f7bf6f4f2f1f0b6e",
            sha1: "ce608753cf1e2c9d3f7ae1f1ff3f0d4c7f2f9f8f",
            sha256: None,
            status: None,
            serial: None,
            header: None,
        };

        let rom1_id = dat_repo.add_dat_rom(rom_1_params).await.unwrap();
        let rom2_id = dat_repo.add_dat_rom(rom_2_params).await.unwrap();

        let roms = dat_repo.get_roms_in_game(game_id).await.unwrap();

        assert_eq!(roms.len(), 2);
        // Should be sorted by name
        assert_eq!(roms[0].name, "Sonic the Hedgehog (Europe).sms");
        assert_eq!(roms[0].id, rom2_id);
        assert_eq!(roms[0].size, 131072);
        assert_eq!(roms[0].crc, "a15c3f81");
        assert_eq!(roms[1].name, "Sonic the Hedgehog (USA).sms");
        assert_eq!(roms[1].id, rom1_id);
        assert_eq!(roms[1].sha256, Some("sha256hash1".to_string()));
        assert_eq!(roms[1].status, Some("verified".to_string()));
    }

    #[async_std::test]
    async fn test_get_game_by_rom_sha1() {
        let pool = Arc::new(setup_test_db().await);
        let system_repo = SystemRepository::new(pool.clone());
        let dat_repo = DatRepository::new(pool.clone());

        let system_id = system_repo.add_system("PlayStation").await.unwrap();
        let dat_file_params = create_dat_file_params(system_id);
        let dat_file_id = dat_repo.add_dat_file(dat_file_params).await.unwrap();

        let game_params = AddDatGameParams {
            dat_file_id,
            name: "Final Fantasy VII",
            game_id: Some("0300"),
            description: "Final Fantasy VII (USA)",
            cloneof: None,
            cloneofid: None,
        };
        let game_id = dat_repo.add_dat_game(game_params).await.unwrap();

        let rom_params = AddDatRomParams {
            dat_game_id: game_id,
            name: "Final Fantasy VII (USA) (Disc 1).bin",
            size: 737280000,
            crc: "abc12345",
            md5: "def67890abc12345def67890abc12345",
            sha1: "a1b2c3d4e5f6a7b8c9d0e1f2a3b4c5d6e7f8a9b0",
            sha256: None,
            status: None,
            serial: None,
            header: None,
        };
        dat_repo.add_dat_rom(rom_params).await.unwrap();

        // Search by SHA1
        let found_game = dat_repo
            .get_game_by_rom_sha1("a1b2c3d4e5f6a7b8c9d0e1f2a3b4c5d6e7f8a9b0")
            .await
            .unwrap();

        assert!(found_game.is_some());
        let game = found_game.unwrap();
        assert_eq!(game.id, game_id);
        assert_eq!(game.name, "Final Fantasy VII");
        assert_eq!(game.description, "Final Fantasy VII (USA)");
        assert_eq!(game.game_id, Some("0300".to_string()));

        // Search with non-existent SHA1
        let not_found = dat_repo
            .get_game_by_rom_sha1("nonexistent1234567890abcdef1234567890abcd")
            .await
            .unwrap();

        assert!(not_found.is_none());
    }

    #[async_std::test]
    async fn test_check_dat_file_exists() {
        let pool = Arc::new(setup_test_db().await);
        let system_repo = SystemRepository::new(pool.clone());
        let system_id = system_repo.add_system("Commodore 64").await.unwrap();
        let dat_repo = DatRepository::new(pool.clone());
        let id = dat_repo
            .add_dat_file(AddDatFileParams {
                dat_id: 10,
                name: "Test DAT",
                description: "A test DAT file",
                version: "1.0",
                date: None,
                author: "Test Author",
                homepage: None,
                url: None,
                subset: None,
                system_id,
            })
            .await
            .unwrap();
        let exists = dat_repo
            .check_dat_file_exists("1.0", "Test DAT", system_id)
            .await
            .expect("Failed to check if DAT file exists");
        assert!(exists.is_some());
        let exists_id = exists.expect("Expected DAT file to exist but it does not");
        assert_eq!(exists_id, id);
    }
}
