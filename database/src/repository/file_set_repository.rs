use std::{collections::HashSet, sync::Arc};

use core_types::{FileType, ImportedFile};
use sqlx::{sqlite::SqliteRow, FromRow, Pool, Row, Sqlite};

use crate::{
    database_error::{DatabaseError, Error},
    models::{FileSet, FileSetFileInfo},
};

#[derive(Debug)]
pub struct FileSetRepository {
    pool: Arc<Pool<Sqlite>>,
}

impl FileSetRepository {
    pub fn new(pool: Arc<Pool<Sqlite>>) -> Self {
        Self { pool }
    }
}

impl FromRow<'_, SqliteRow> for FileSet {
    fn from_row(row: &SqliteRow) -> Result<Self, sqlx::Error> {
        let file_type: FileType = FileType::from_db_int(row.try_get::<u8, _>("file_type")?)
            .expect("Invalid file type in DB");
        Ok(Self {
            id: row.try_get("id")?,
            file_name: row.try_get("file_name")?,
            file_type,
            name: row.try_get("name")?,
            source: row.try_get("source")?,
        })
    }
}

impl FileSetRepository {
    pub async fn get_file_sets_for_release(
        &self,
        release_id: i64,
    ) -> Result<Vec<FileSet>, DatabaseError> {
        let file_sets = sqlx::query_as(
            "SELECT c.id, c.file_name, c.file_type, c.name, c.source
             FROM file_set c 
             INNER JOIN release_file_set rcf
             ON c.id = rcf.file_set_id
             WHERE rcf.release_id = ?",
        )
        .bind(release_id)
        .fetch_all(&*self.pool)
        .await?;
        Ok(file_sets)
    }

    pub async fn get_file_sets_by_file_info(
        &self,
        file_info_id: i64,
    ) -> Result<Vec<FileSet>, DatabaseError> {
        let file_sets = sqlx::query_as(
            "SELECT fs.id, fs.file_name, fs.file_type, fs.name, fs.source
             FROM file_set fs
             INNER JOIN file_set_file_info fsfi ON fs.id = fsfi.file_set_id
             WHERE fsfi.file_info_id = ?",
        )
        .bind(file_info_id)
        .fetch_all(&*self.pool)
        .await?;
        Ok(file_sets)
    }

    pub async fn is_file_set_in_release(&self, file_set_id: i64) -> Result<bool, DatabaseError> {
        let count = sqlx::query_scalar!(
            "SELECT COUNT(*) 
             FROM release_file_set
             WHERE file_set_id = ?",
            file_set_id
        )
        .fetch_one(&*self.pool)
        .await?;
        Ok(count > 0)
    }

    pub async fn get_file_sets(&self, ids: Vec<i64>) -> Result<Vec<FileSet>, DatabaseError> {
        let placeholders = ids.iter().map(|_| "?").collect::<Vec<&str>>().join(",");
        let query = format!(
            "SELECT id, file_name, file_type, name, source 
             FROM file_set
             WHERE id IN ({})",
            placeholders
        );

        let mut query_builder = sqlx::query_as::<Sqlite, FileSet>(&query);
        for id in ids {
            query_builder = query_builder.bind(id);
        }

        let file_sets = query_builder.fetch_all(&*self.pool).await?;
        Ok(file_sets)
    }

    pub async fn get_file_set(&self, id: i64) -> Result<FileSet, DatabaseError> {
        let file_set = sqlx::query_as(
            "SELECT id, file_name, file_type, name, source 
             FROM file_set
             WHERE id = ?",
        )
        .bind(id)
        .fetch_one(&*self.pool)
        .await?;
        Ok(file_set)
    }

    pub async fn get_file_sets_by_release(
        &self,
        release_id: i64,
    ) -> Result<Vec<FileSet>, DatabaseError> {
        let file_sets = sqlx::query_as(
            "SELECT c.id, c.file_name, c.file_type, c.name, c.source
             FROM file_set c 
             INNER JOIN release_file_set rcf
             ON c.id = rcf.file_set_id
             WHERE rcf.release_id = ?",
        )
        .bind(release_id)
        .fetch_all(&*self.pool)
        .await?;
        Ok(file_sets)
    }

    pub async fn get_all_file_sets(&self) -> Result<Vec<FileSet>, DatabaseError> {
        let file_sets = sqlx::query_as(
            "SELECT id, file_name, file_type, name, source 
             FROM file_set",
        )
        .fetch_all(&*self.pool)
        .await?;
        Ok(file_sets)
    }

    pub async fn get_file_sets_by_file_type_and_systems(
        &self,
        file_type: FileType,
        system_ids: &[i64],
    ) -> Result<Vec<FileSet>, DatabaseError> {
        println!(
            "Getting file sets for file type: {:?} and systems: {:?}",
            file_type, system_ids
        );
        let placeholders = system_ids
            .iter()
            .map(|_| "?")
            .collect::<Vec<_>>()
            .join(", ");

        let file_sets_query = format!(
            "SELECT DISTINCT fs.id, fs.file_name, fs.file_type, fs.name, fs.source 
             FROM file_set fs
             INNER JOIN file_set_file_info fsfi ON fs.id = fsfi.file_set_id
             INNER JOIN file_info_system fis ON fsfi.file_info_id = fis.file_info_id
             WHERE fs.file_type = ? AND fis.system_id IN ({})",
            placeholders
        );
        let mut query_builder =
            sqlx::query_as::<Sqlite, FileSet>(&file_sets_query).bind(file_type as i64);
        for system_id in system_ids {
            query_builder = query_builder.bind(system_id);
        }
        let file_sets = query_builder.fetch_all(&*self.pool).await?;
        Ok(file_sets)
    }

    // TODO: update file set
    pub async fn add_file_set(
        &self,
        file_set_name: &str,
        file_set_file_name: &str,
        file_type: &FileType,
        source: &str,
        files_in_fileset: &[ImportedFile],
        system_ids: &[i64],
    ) -> Result<i64, Error> {
        println!(
            "Adding file set: {}, {} file type: {:?}, files: {:?}, systems: {:?}",
            file_set_name, file_set_file_name, file_type, files_in_fileset, system_ids
        );
        let file_type = file_type.to_db_int();

        let mut transaction = self.pool.begin().await?;

        let result = sqlx::query!(
            "INSERT INTO file_set(
                file_name, 
                file_type,
                name,
                source) 
             VALUES (?, ?, ?, ?)",
            file_set_file_name,
            file_type,
            file_set_name,
            source,
        )
        .execute(&mut *transaction)
        .await?;
        let file_set_id = result.last_insert_rowid();
        println!("File set created with ID: {}", file_set_id);

        for file in files_in_fileset {
            let checksum = file.sha1_checksum.to_vec();
            // if file_info exists, use its id, otherwise insert new file_info
            let existing_file_info = sqlx::query_scalar!(
                "SELECT id 
                 FROM file_info 
                 WHERE sha1_checksum = ? and file_type = ?",
                checksum,
                file_type
            )
            .fetch_optional(&mut *transaction)
            .await?;

            println!(
                "Existing file info for checksum {:?}: {:?}",
                checksum, existing_file_info
            );

            let archive_file_name = &file.archive_file_name;

            let file_info_id = match existing_file_info {
                Some(id) => id,
                None => {
                    let file_size = file.file_size as i64;
                    let file_info_result = sqlx::query!(
                        "INSERT INTO file_info (
                            sha1_checksum, 
                            file_size, 
                            archive_file_name,
                            file_type
                        ) VALUES (?, ?, ?, ?)",
                        checksum,
                        file_size,
                        archive_file_name,
                        file_type
                    )
                    .execute(&mut *transaction)
                    .await?;

                    file_info_result.last_insert_rowid()
                }
            };

            println!(
                "File info ID for file {}: {}",
                file.original_file_name, file_info_id
            );

            // insert new systems for file_info

            let file_info_systems = sqlx::query!(
                "SELECT system_id FROM file_info_system 
                 WHERE file_info_id = ?",
                file_info_id
            )
            .fetch_all(&mut *transaction)
            .await?
            .into_iter()
            .map(|row| row.system_id)
            .collect::<HashSet<_>>();

            println!(
                "Existing systems for file info ID {}: {:?}",
                file_info_id, file_info_systems
            );

            let system_ids: HashSet<i64> = system_ids.iter().copied().collect();

            let new_system_ids = system_ids.difference(&file_info_systems);

            println!(
                "New systems to add for file info ID {}: {:?}",
                file_info_id, new_system_ids
            );

            for system_id in new_system_ids {
                sqlx::query!(
                    "INSERT INTO file_info_system (file_info_id, system_id) 
                     VALUES (?, ?)",
                    file_info_id,
                    system_id
                )
                .execute(&mut *transaction)
                .await?;
            }

            // insert file_set_file_info

            sqlx::query!(
                "INSERT INTO file_set_file_info (
                    file_set_id, 
                    file_info_id, 
                    file_name
                 ) VALUES (?, ?, ?)",
                file_set_id,
                file_info_id,
                file.original_file_name
            )
            .execute(&mut *transaction)
            .await?;
        }

        transaction.commit().await?;

        println!("File set with ID {} added successfully", file_set_id);

        Ok(file_set_id)
    }

    pub async fn update_file_set(
        &self,
        id: i64,
        file_set_file_name: &str,
        file_set_name: &str,
        source: &str,
        file_type: &FileType,
    ) -> Result<i64, DatabaseError> {
        let file_type = *file_type as i64;
        sqlx::query!(
            "UPDATE file_set 
             SET 
                file_name = ?, 
                name = ?, 
                source = ?, 
                file_type = ? 
             WHERE id = ?",
            file_set_file_name,
            file_set_name,
            source,
            file_type,
            id
        )
        .execute(&*self.pool)
        .await?;
        Ok(id)
    }

    pub async fn is_in_use(&self, id: i64) -> Result<bool, DatabaseError> {
        let count = sqlx::query_scalar!(
            "SELECT COUNT(*) 
             FROM release_file_set
             WHERE file_set_id = ?",
            id
        )
        .fetch_one(&*self.pool)
        .await?;
        Ok(count > 0)
    }

    pub async fn delete_file_set(&self, id: i64) -> Result<i64, DatabaseError> {
        if self.is_in_use(id).await? {
            return Err(DatabaseError::InUse);
        }

        let mut transaction = self.pool.begin().await?;

        // NOTE: we don't delete file_info, because it can be used in other file sets
        // TODO: maybe check if file_info is used in other file sets and delete it if not?
        // NOTE: file info is dependent on physical file, so we delete it only in those case when
        // the actual file is deleted
        sqlx::query!("DELETE FROM file_set_file_info WHERE file_set_id = ?", id)
            .execute(&mut *transaction)
            .await?;

        sqlx::query!("DELETE FROM file_set WHERE id = ?", id)
            .execute(&mut *transaction)
            .await?;

        transaction.commit().await?;
        Ok(id)
    }

    pub async fn get_file_set_file_info(
        &self,
        file_set_id: i64,
    ) -> Result<Vec<FileSetFileInfo>, DatabaseError> {
        let test = sqlx::query!(
            "SELECT * 
             FROM file_set_file_info 
             WHERE file_set_id = ?",
            file_set_id
        );
        let test = test.fetch_all(&*self.pool).await;
        println!("Query result: {:?}", test);
        let file_infos = sqlx::query_as!(
            FileSetFileInfo,
            "SELECT 
                fsfi.file_set_id, 
                fsfi.file_info_id, 
                fsfi.file_name, 
                fi.sha1_checksum, 
                fi.file_size, 
                fi.archive_file_name
             FROM file_set_file_info fsfi
             JOIN file_info fi ON fsfi.file_info_id = fi.id
             WHERE fsfi.file_set_id = ?",
            file_set_id
        )
        .fetch_all(&*self.pool)
        .await?;
        println!(
            "Retrieved {:?} file infos for file set ID {}",
            file_infos, file_set_id
        );
        Ok(file_infos)
    }
}

#[cfg(test)]
mod tests {
    use crate::{repository::system_repository::SystemRepository, setup_test_db};

    use super::*;
    use sqlx::{query, query_scalar};

    #[async_std::test]
    async fn test_get_file_sets_for_release() {
        let pool = setup_test_db().await;
        let file_set = FileSet {
            id: 1,
            file_name: "test.zip".to_string(),
            file_type: FileType::Rom,
            name: "Test file set".to_string(),
            source: "".to_string(),
        };
        let file_type = file_set.file_type as i64;

        let result = query!(
            "INSERT INTO file_set (
                file_name,
                file_type,
                name,
                source
            ) VALUES (?, ?, ?, ?)",
            file_set.file_name,
            file_type,
            file_set.name,
            file_set.source,
        )
        .execute(&pool)
        .await
        .unwrap();

        let release_id = insert_test_release(&pool).await;
        let file_set_id = result.last_insert_rowid();

        query!(
            "INSERT INTO release_file_set(release_id, file_set_id) VALUES (?, ?)",
            release_id,
            file_set_id
        )
        .execute(&pool)
        .await
        .unwrap();

        let result = FileSetRepository {
            pool: Arc::new(pool),
        }
        .get_file_sets_for_release(release_id)
        .await
        .unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].id, file_set_id);
    }

    #[async_std::test]
    async fn test_is_file_set_in_release() {
        let pool = setup_test_db().await;
        let result = query!(
            "INSERT INTO file_set (
                file_name,
                file_type,
                name,
                source
            ) VALUES (?, ?, ?, ?)",
            "test",
            FileType::Rom as i64,
            "Test File Set",
            ""
        )
        .execute(&pool)
        .await
        .unwrap();

        let release_id = insert_test_release(&pool).await;
        let file_set_id = result.last_insert_rowid();

        query!(
            "INSERT INTO release_file_set (release_id, file_set_id) VALUES (?, ?)",
            release_id,
            file_set_id
        )
        .execute(&pool)
        .await
        .unwrap();

        let result = FileSetRepository {
            pool: Arc::new(pool),
        }
        .is_file_set_in_release(release_id)
        .await
        .unwrap();
        assert!(result);
    }

    #[async_std::test]
    async fn test_add_file_set() {
        let pool = Arc::new(setup_test_db().await);
        let file_name = "test file".to_string();
        // create some guid for archive file name
        let archive_file_name_1 = "123e4567-e89b-12d3-a456-426614174001";
        let archive_file_name_2 = "123e4567-e89b-12d3-a456-426614174002";
        let file_type = FileType::Rom;
        let checksum_1: [u8; 20] = [0; 20];
        let checksum_2: [u8; 20] = [1; 20];
        let files = vec![
            ImportedFile {
                sha1_checksum: checksum_1,
                file_size: 123,
                original_file_name: "test".to_string(),
                archive_file_name: archive_file_name_1.to_string(),
            },
            ImportedFile {
                sha1_checksum: checksum_2,
                file_size: 123,
                original_file_name: "test2".to_string(),
                archive_file_name: archive_file_name_2.to_string(),
            },
        ];

        let system_id = SystemRepository::new(pool.clone())
            .add_system(&"Test System".to_string())
            .await
            .unwrap();

        let file_set_id = FileSetRepository { pool: pool.clone() }
            .add_file_set(
                "Test File Set",
                &file_name,
                &file_type,
                "",
                &files,
                &[system_id],
            )
            .await
            .unwrap();

        let files_for_file_set = query_scalar!(
            "SELECT COUNT(*) 
             FROM file_set_file_info 
             WHERE file_set_id = ?",
            file_set_id
        )
        .fetch_one(&*pool)
        .await
        .unwrap();

        assert_eq!(files_for_file_set, 2);
    }

    #[async_std::test]
    async fn test_add_file_sets_with_common_files() {
        let pool = Arc::new(setup_test_db().await);
        let file_set_1_name = "file set 1".to_string();
        let file_set_2_name = "file set 2".to_string();

        let file_type = FileType::Rom;

        let checksum_1: [u8; 20] = [0; 20];
        let checksum_2: [u8; 20] = [1; 20];
        let checksum_3: [u8; 20] = [2; 20];
        let all_files = [
            ImportedFile {
                sha1_checksum: checksum_1,
                file_size: 123,
                original_file_name: "file 1".to_string(),
                archive_file_name: "file_1.zip".to_string(),
            },
            ImportedFile {
                sha1_checksum: checksum_2,
                file_size: 123,
                original_file_name: "file 2".to_string(),
                archive_file_name: "file_2.zip".to_string(),
            },
            ImportedFile {
                sha1_checksum: checksum_3,
                file_size: 123,
                original_file_name: "file 3".to_string(),
                archive_file_name: "file_3.zip".to_string(),
            },
        ];

        let file_set_1_files = vec![all_files[0].clone(), all_files[1].clone()];
        let file_set_2_files = vec![all_files[1].clone(), all_files[2].clone()];

        let repo = FileSetRepository { pool: pool.clone() };

        let system_1_id = SystemRepository::new(pool.clone())
            .add_system(&"Test System 1".to_string())
            .await
            .unwrap();

        let _file_set_1_id = repo
            .add_file_set(
                &"Test File Set 1".to_string(),
                &file_set_1_name,
                &file_type,
                "",
                &file_set_1_files,
                &[system_1_id],
            )
            .await
            .unwrap();

        let _file_set_2_id = repo
            .add_file_set(
                &"Test File Set 2".to_string(),
                &file_set_2_name,
                &file_type,
                "",
                &file_set_2_files,
                &[system_1_id],
            )
            .await
            .unwrap();

        // In this case, expected behaviour is the file 2 is only added once
        // and file set 1 and file set 2 are linked to the same file info
        //

        let checksum_2_as_vec = checksum_2.to_vec();

        let file_2_instances = query_scalar!(
            "SELECT COUNT(*) 
             FROM file_info 
             WHERE sha1_checksum = ?",
            checksum_2_as_vec
        )
        .fetch_one(&*pool)
        .await
        .unwrap();
        assert_eq!(file_2_instances, 1);
    }

    #[async_std::test]
    async fn test_delete_file_set() {
        let pool = setup_test_db().await;
        let file_name = "test file".to_string();
        let file_type = FileType::Rom as i64;
        let file_set_name = "Test File Set".to_string();
        let result = query!(
            "INSERT INTO file_set (
                file_name,
                file_type,
                name,
                source
            ) VALUES (?, ?, ?, ?)",
            file_name,
            file_type,
            file_set_name,
            ""
        )
        .execute(&pool)
        .await
        .unwrap();

        let file_set_id = result.last_insert_rowid();

        let repository = FileSetRepository {
            pool: Arc::new(pool),
        };

        let result = repository.delete_file_set(file_set_id).await.unwrap();
        assert_eq!(result, file_set_id);
        let result = repository.get_file_sets(vec![file_set_id]).await.unwrap();
        assert_eq!(result.len(), 0);
    }

    #[async_std::test]
    async fn test_delete_file_set_in_use() {
        let pool = setup_test_db().await;
        let file_name = "test file".to_string();
        let file_type = FileType::Rom as i64;
        let filet_set_name = "Test File Set".to_string();
        let result = query!(
            "INSERT INTO file_set (
                file_name,
                file_type,
                name,
                source
            ) VALUES (?, ?, ?, ?)",
            file_name,
            file_type,
            filet_set_name,
            ""
        )
        .execute(&pool)
        .await
        .unwrap();

        let release_id = insert_test_release(&pool).await;
        let file_set_id = result.last_insert_rowid();

        query!(
            "INSERT INTO release_file_set (release_id, file_set_id) VALUES (?, ?)",
            release_id,
            file_set_id
        )
        .execute(&pool)
        .await
        .unwrap();

        let result = FileSetRepository {
            pool: Arc::new(pool),
        }
        .delete_file_set(file_set_id)
        .await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), DatabaseError::InUse);
    }

    #[async_std::test]
    async fn test_cascade_delete_when_release_deleted() {
        let pool = setup_test_db().await;

        // Create a release
        let release_id = insert_test_release(&pool).await;

        // Create a file set
        let file_set_result = query!(
            "INSERT INTO file_set (file_name, file_type, name, source) VALUES (?, ?, ?, ?)",
            "test.zip",
            FileType::Rom as i64,
            "Test File Set",
            ""
        )
        .execute(&pool)
        .await
        .unwrap();
        let file_set_id = file_set_result.last_insert_rowid();

        // Link them in release_file_set
        query!(
            "INSERT INTO release_file_set (release_id, file_set_id) VALUES (?, ?)",
            release_id,
            file_set_id
        )
        .execute(&pool)
        .await
        .unwrap();

        // Verify the relationship exists
        let count_before = query_scalar!(
            "SELECT COUNT(*) FROM release_file_set WHERE release_id = ? AND file_set_id = ?",
            release_id,
            file_set_id
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(count_before, 1);

        // Delete the release
        query!("DELETE FROM release WHERE id = ?", release_id)
            .execute(&pool)
            .await
            .unwrap();

        // Verify that the release_file_set entry was CASCADE deleted
        let count_after = query_scalar!(
            "SELECT COUNT(*) FROM release_file_set WHERE release_id = ? AND file_set_id = ?",
            release_id,
            file_set_id
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(count_after, 0);

        // Verify that the file_set still exists (should not be deleted)
        let file_set_exists =
            query_scalar!("SELECT COUNT(*) FROM file_set WHERE id = ?", file_set_id)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(file_set_exists, 1);
    }

    #[async_std::test]
    async fn test_cascade_delete_when_file_set_deleted() {
        let pool = setup_test_db().await;

        // Create a release
        let release_id = insert_test_release(&pool).await;

        // Create a file set
        let file_set_result = query!(
            "INSERT INTO file_set (file_name, file_type, name, source) VALUES (?, ?, ?, ?)",
            "test.zip",
            FileType::Rom as i64,
            "Test File Set",
            ""
        )
        .execute(&pool)
        .await
        .unwrap();
        let file_set_id = file_set_result.last_insert_rowid();

        // Link them in release_file_set
        query!(
            "INSERT INTO release_file_set (release_id, file_set_id) VALUES (?, ?)",
            release_id,
            file_set_id
        )
        .execute(&pool)
        .await
        .unwrap();

        // Verify the relationship exists
        let count_before = query_scalar!(
            "SELECT COUNT(*) FROM release_file_set WHERE release_id = ? AND file_set_id = ?",
            release_id,
            file_set_id
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(count_before, 1);

        // Delete the file_set
        query!("DELETE FROM file_set WHERE id = ?", file_set_id)
            .execute(&pool)
            .await
            .unwrap();

        // Verify that the release_file_set entry was CASCADE deleted
        let count_after = query_scalar!(
            "SELECT COUNT(*) FROM release_file_set WHERE release_id = ? AND file_set_id = ?",
            release_id,
            file_set_id
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(count_after, 0);

        // Verify that the release still exists (should not be deleted)
        let release_exists = query_scalar!("SELECT COUNT(*) FROM release WHERE id = ?", release_id)
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(release_exists, 1);
    }

    async fn insert_test_release(pool: &Pool<Sqlite>) -> i64 {
        let result = query!(
            "INSERT INTO release (
                name
            ) VALUES (?)",
            "test",
        )
        .execute(pool)
        .await
        .unwrap();
        result.last_insert_rowid()
    }
}
