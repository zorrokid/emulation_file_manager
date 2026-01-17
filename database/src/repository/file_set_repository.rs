use std::{collections::HashSet, sync::Arc};

use core_types::{FileType, ImportedFile, Sha1Checksum, item_type::ItemType};
use sqlx::{FromRow, Pool, Row, Sqlite, sqlite::SqliteRow};

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

    /// Validates that file_type of FileInfo matches FileSet.
    /// This ensures consistency between the file_type stored in both tables.
    async fn validate_file_type_consistency(
        &self,
        transaction: &mut sqlx::Transaction<'_, Sqlite>,
        file_set_id: i64,
        file_info_id: i64,
    ) -> Result<(), DatabaseError> {
        let result = sqlx::query!(
            "SELECT 
                fs.file_type as file_set_type,
                fi.file_type as file_info_type
             FROM file_set fs, file_info fi
             WHERE fs.id = ? AND fi.id = ?",
            file_set_id,
            file_info_id
        )
        .fetch_optional(&mut **transaction)
        .await?;

        if let Some(row) = result {
            let file_info_type = row.file_info_type.ok_or_else(|| {
                DatabaseError::ValidationError(format!(
                    "FileInfo (id={}) has NULL file_type, which is not allowed",
                    file_info_id
                ))
            })?;

            if row.file_set_type != file_info_type {
                return Err(DatabaseError::ValidationError(format!(
                    "FileInfo (id={}, file_type={}) cannot be added to FileSet (id={}, file_type={}) - file types must match",
                    file_info_id, file_info_type, file_set_id, row.file_set_type
                )));
            }
        }

        Ok(())
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

impl FromRow<'_, SqliteRow> for FileSetFileInfo {
    fn from_row(row: &SqliteRow) -> Result<Self, sqlx::Error> {
        let file_type_int: u8 = row.try_get("file_type")?;
        let sha1_checksum: Vec<u8> = row.try_get("sha1_checksum")?;
        let sha1_checksum: Sha1Checksum = sha1_checksum
            .try_into()
            .expect("Invalid SHA1 checksum length in DB");
        let file_type: FileType =
            FileType::from_db_int(file_type_int).expect("Invalid file type in DB");
        Ok(Self {
            file_set_id: row.try_get("file_set_id")?,
            file_info_id: row.try_get("file_info_id")?,
            file_name: row.try_get("file_name")?,
            file_type,
            sha1_checksum,
            file_size: row.try_get("file_size")?,
            archive_file_name: row.try_get("archive_file_name")?,
            sort_order: row.try_get("sort_order")?,
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

    /// Adds a new file set along with its associated files and system links.
    /// Checks for existing file info entries to avoid duplicates.
    /// Returns the ID of the newly created file set.
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

        // First create the file set

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

            // for newly inserted file_info, there are no systems yet,
            // but for existing file_info can be alrady to linked some system.
            // new system_id(s) can be something different from the old one(s).
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

            // Validate that file_type matches between FileSet and FileInfo
            self.validate_file_type_consistency(&mut transaction, file_set_id, file_info_id)
                .await
                .map_err(|e| Error::DbError(e.to_string()))?;

            sqlx::query!(
                "INSERT INTO file_set_file_info (
                    file_set_id, 
                    file_info_id, 
                    file_name,
                    sort_order
                 ) VALUES (?, ?, ?, ?)",
                file_set_id,
                file_info_id,
                file.original_file_name,
                0 // TODO: get sort order from UI
            )
            .execute(&mut *transaction)
            .await?;
        }

        transaction.commit().await?;

        println!("File set with ID {} added successfully", file_set_id);

        Ok(file_set_id)
    }

    pub async fn add_files_to_file_set(
        &self,
        file_set_id: i64,
        file_info_ids_and_names: &[(i64, String)],
    ) -> Result<(), Error> {
        let mut transaction = self.pool.begin().await?;

        // get systems linked to file set
        let system_ids = sqlx::query!(
            "SELECT DISTINCT fis.system_id
             FROM file_set_file_info fsfi
             JOIN file_info_system fis ON fsfi.file_info_id = fis.file_info_id
             WHERE fsfi.file_set_id = ?",
            file_set_id
        )
        .fetch_all(&mut *transaction)
        .await?;

        for (file_info_id, file_name) in file_info_ids_and_names {
            // Validate that file_type matches between FileSet and FileInfo
            self.validate_file_type_consistency(&mut transaction, file_set_id, *file_info_id)
                .await
                .map_err(|e| Error::DbError(e.to_string()))?;

            sqlx::query!(
                "INSERT INTO file_set_file_info (
                    file_set_id, 
                    file_info_id, 
                    file_name,
                    sort_order
                 ) VALUES (?, ?, ?, ?)",
                file_set_id,
                file_info_id,
                file_name,
                0 // TODO: get sort order from UI
            )
            .execute(&mut *transaction)
            .await?;

            for system in &system_ids {
                sqlx::query!(
                    "INSERT OR IGNORE INTO file_info_system (
                        file_info_id, 
                        system_id
                    ) VALUES (?, ?)",
                    file_info_id,
                    system.system_id
                )
                .execute(&mut *transaction)
                .await?;
            }
        }

        transaction.commit().await?;
        Ok(())
    }

    pub async fn remove_files_from_file_set(
        &self,
        file_set_id: i64,
        file_info_ids: &[i64],
    ) -> Result<(), DatabaseError> {
        let mut transaction = self.pool.begin().await?;

        for file_info_id in file_info_ids {
            sqlx::query!(
                "DELETE FROM file_set_file_info 
                 WHERE file_set_id = ? AND file_info_id = ?",
                file_set_id,
                file_info_id
            )
            .execute(&mut *transaction)
            .await?;
        }

        transaction.commit().await?;
        Ok(())
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

    pub async fn update_file_type(
        &self,
        id: &i64,
        new_file_type: &FileType,
    ) -> Result<(), DatabaseError> {
        let new_file_type = new_file_type.to_db_int();
        sqlx::query!(
            "UPDATE file_set 
             SET 
                file_type = ? 
             WHERE id = ?",
            new_file_type,
            id
        )
        .execute(&*self.pool)
        .await?;
        Ok(())
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

        // NOTE: we don't delete file_info, because it can be used in other file sets and
        // file info is dependent on physical file, so we delete it only in those case when
        // the actual file is deleted.
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
        let query = sqlx::query_as::<_, FileSetFileInfo>(
            "SELECT 
                fsfi.file_set_id, 
                fsfi.file_info_id, 
                fsfi.file_name, 
                fi.sha1_checksum, 
                fi.file_size, 
                fi.archive_file_name,
                fi.file_type,
                fsfi.sort_order
             FROM file_set_file_info fsfi
             JOIN file_info fi ON fsfi.file_info_id = fi.id
             WHERE fsfi.file_set_id = ?
             ORDER BY fsfi.sort_order ASC
             ",
        )
        .bind(file_set_id);

        let file_set_file_infos = query.fetch_all(&*self.pool).await?;
        Ok(file_set_file_infos)
    }

    // TODO: is this needed? maybe the sort order will be updated with file set update
    pub async fn update_file_set_file_info_sort_order(
        &self,
        file_set_id: i64,
        file_info_id: i64,
        sort_order: i64,
    ) -> Result<(), DatabaseError> {
        sqlx::query!(
            "UPDATE file_set_file_info 
             SET sort_order = ? 
             WHERE file_set_id = ? AND file_info_id = ?",
            sort_order,
            file_set_id,
            file_info_id
        )
        .execute(&*self.pool)
        .await?;
        Ok(())
    }

    pub async fn update_file_set_file_infos_sort_order(
        &self,
        file_set_id: i64,
        file_info_sort_orders: &[(i64, i64)],
    ) -> Result<(), DatabaseError> {
        let mut transaction = self.pool.begin().await?;

        for (file_info_id, sort_order) in file_info_sort_orders {
            sqlx::query!(
                "UPDATE file_set_file_info 
                 SET sort_order = ? 
                 WHERE file_set_id = ? AND file_info_id = ?",
                sort_order,
                file_set_id,
                file_info_id
            )
            .execute(&mut *transaction)
            .await?;
        }

        transaction.commit().await?;
        Ok(())
    }

    pub async fn add_item_type_to_file_set(
        &self,
        file_set_id: &i64,
        item_type: &ItemType,
    ) -> Result<(), DatabaseError> {
        let item_type = item_type.to_db_int();
        sqlx::query!(
            "INSERT INTO file_set_item_type (file_set_id, item_type) 
                 VALUES (?, ?)",
            file_set_id,
            item_type
        )
        .execute(&*self.pool)
        .await?;

        Ok(())
    }

    pub async fn get_item_types_for_file_set(
        &self,
        file_set_id: i64,
    ) -> Result<Vec<ItemType>, DatabaseError> {
        let rows = sqlx::query!(
            "SELECT item_type 
             FROM file_set_item_type 
             WHERE file_set_id = ?",
            file_set_id
        )
        .fetch_all(&*self.pool)
        .await?;

        let item_types = rows
            .into_iter()
            .filter_map(|row| ItemType::from_db_int(row.item_type as u8).ok())
            .collect();

        Ok(item_types)
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        repository::{
            file_info_repository::FileInfoRepository, system_repository::SystemRepository,
        },
        setup_test_db,
    };

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

    #[async_std::test]
    async fn test_add_files_to_file_set() {
        let pool = Arc::new(setup_test_db().await);
        let file_set_file_name = "test file set".to_string();
        let file_type = FileType::Rom;

        let files = vec![ImportedFile {
            sha1_checksum: [0; 20],
            file_size: 123,
            original_file_name: "test.rom".to_string(),
            archive_file_name: "archive_file_name_1".to_string(),
        }];

        let system_id = SystemRepository::new(pool.clone())
            .add_system("Test System")
            .await
            .unwrap();

        let file_set_repository = FileSetRepository { pool: pool.clone() };

        let file_set_id = file_set_repository
            .add_file_set(
                "Test File Set",
                &file_set_file_name,
                &file_type,
                "",
                &files,
                &[system_id],
            )
            .await
            .unwrap();

        let new_file_info = ImportedFile {
            sha1_checksum: [2; 20],
            file_size: 456,
            original_file_name: "test2.rom".to_string(),
            archive_file_name: "archive_file_name_2".to_string(),
        };

        let file_info_repo = FileInfoRepository::new(pool.clone());
        let new_file_info_id = file_info_repo
            .add_file_info(
                &new_file_info.sha1_checksum,
                new_file_info.file_size as i64,
                &new_file_info.archive_file_name,
                file_type,
            )
            .await
            .unwrap();

        file_set_repository
            .add_files_to_file_set(
                file_set_id,
                &[(new_file_info_id, new_file_info.original_file_name.clone())],
            )
            .await
            .unwrap();

        // assert that the file_set_file_info entry exists

        let count = query_scalar!(
            "SELECT COUNT(*) 
             FROM file_set_file_info 
             WHERE file_set_id = ? AND file_info_id = ?",
            file_set_id,
            new_file_info_id
        )
        .fetch_one(&*pool)
        .await
        .unwrap();

        assert_eq!(count, 1);

        // assert that the name inside file set is correct
        let result = query!(
            "SELECT file_name 
             FROM file_set_file_info 
             WHERE file_set_id = ? AND file_info_id = ?",
            file_set_id,
            new_file_info_id
        )
        .fetch_one(&*pool)
        .await
        .unwrap();
        assert_eq!(result.file_name, new_file_info.original_file_name);

        // assert that the file_info_system entry exists
        let count = query_scalar!(
            "SELECT COUNT(*) 
             FROM file_info_system 
             WHERE file_info_id = ? AND system_id = ?",
            new_file_info_id,
            system_id
        )
        .fetch_one(&*pool)
        .await
        .unwrap();

        assert_eq!(count, 1);
    }

    #[async_std::test]
    async fn test_add_files_to_file_set_file_info_already_linked_with_system() {
        let pool = Arc::new(setup_test_db().await);
        let file_set_file_name = "test file set".to_string();
        let file_type = FileType::Rom;

        let files = vec![ImportedFile {
            sha1_checksum: [0; 20],
            file_size: 123,
            original_file_name: "test.rom".to_string(),
            archive_file_name: "archive_file_name_1".to_string(),
        }];

        let system_id = SystemRepository::new(pool.clone())
            .add_system("Test System")
            .await
            .unwrap();

        let file_set_repository = FileSetRepository { pool: pool.clone() };

        let file_set_id = file_set_repository
            .add_file_set(
                "Test File Set",
                &file_set_file_name,
                &file_type,
                "",
                &files,
                &[system_id],
            )
            .await
            .unwrap();

        let new_file_info = ImportedFile {
            sha1_checksum: [2; 20],
            file_size: 456,
            original_file_name: "test2.rom".to_string(),
            archive_file_name: "archive_file_name_2".to_string(),
        };

        let file_info_repo = FileInfoRepository::new(pool.clone());
        let new_file_info_id = file_info_repo
            .add_file_info(
                &new_file_info.sha1_checksum,
                new_file_info.file_size as i64,
                &new_file_info.archive_file_name,
                file_type,
            )
            .await
            .unwrap();

        // Manually link the new file info with the systems
        query!(
            "INSERT INTO file_info_system (file_info_id, system_id) VALUES (?, ?)",
            new_file_info_id,
            system_id
        )
        .execute(&*pool)
        .await
        .unwrap();

        // Assert previous
        let count = query_scalar!(
            "SELECT COUNT(*) 
             FROM file_info_system 
             WHERE file_info_id = ? AND system_id = ?",
            new_file_info_id,
            system_id
        )
        .fetch_one(&*pool)
        .await
        .unwrap();

        assert_eq!(count, 1);

        // add file to file set, already existing link with system should be ignored and no error
        // should occur

        file_set_repository
            .add_files_to_file_set(
                file_set_id,
                &[(new_file_info_id, new_file_info.original_file_name.clone())],
            )
            .await
            .unwrap();

        // assert that the file_set_file_info entry exists

        let count = query_scalar!(
            "SELECT COUNT(*) 
             FROM file_set_file_info 
             WHERE file_set_id = ? AND file_info_id = ?",
            file_set_id,
            new_file_info_id
        )
        .fetch_one(&*pool)
        .await
        .unwrap();

        assert_eq!(count, 1);

        // assert that the name inside file set is correct
        let result = query!(
            "SELECT file_name 
             FROM file_set_file_info 
             WHERE file_set_id = ? AND file_info_id = ?",
            file_set_id,
            new_file_info_id
        )
        .fetch_one(&*pool)
        .await
        .unwrap();
        assert_eq!(result.file_name, new_file_info.original_file_name);

        // assert that single file_info_system entry exists
        let count = query_scalar!(
            "SELECT COUNT(*) 
             FROM file_info_system 
             WHERE file_info_id = ? AND system_id = ?",
            new_file_info_id,
            system_id
        )
        .fetch_one(&*pool)
        .await
        .unwrap();

        assert_eq!(count, 1);
    }

    #[async_std::test]
    async fn test_add_files_to_non_existing_file_set() {
        let pool = Arc::new(setup_test_db().await);

        let file_set_repository = FileSetRepository { pool: pool.clone() };

        let new_file_info = ImportedFile {
            sha1_checksum: [2; 20],
            file_size: 456,
            original_file_name: "test2.rom".to_string(),
            archive_file_name: "archive_file_name_2".to_string(),
        };

        let file_info_repo = FileInfoRepository::new(pool.clone());
        let new_file_info_id = file_info_repo
            .add_file_info(
                &new_file_info.sha1_checksum,
                new_file_info.file_size as i64,
                &new_file_info.archive_file_name,
                FileType::Rom,
            )
            .await
            .unwrap();

        let res = file_set_repository
            .add_files_to_file_set(
                9999,
                &[(new_file_info_id, new_file_info.original_file_name.clone())],
            )
            .await;

        assert!(res.is_err());
    }

    #[async_std::test]
    async fn test_update_file_set_file_infos_sort_order() {
        let pool = Arc::new(setup_test_db().await);
        let file_set_file_name = "test file set".to_string();
        let file_type = FileType::Rom;

        let files = vec![
            ImportedFile {
                sha1_checksum: [0; 20],
                file_size: 123,
                original_file_name: "test1.rom".to_string(),
                archive_file_name: "archive_file_name_1".to_string(),
            },
            ImportedFile {
                sha1_checksum: [1; 20],
                file_size: 456,
                original_file_name: "test2.rom".to_string(),
                archive_file_name: "archive_file_name_2".to_string(),
            },
        ];

        let system_id = SystemRepository::new(pool.clone())
            .add_system("Test System")
            .await
            .unwrap();

        let file_set_repository = FileSetRepository { pool: pool.clone() };

        let file_set_id = file_set_repository
            .add_file_set(
                "Test File Set",
                &file_set_file_name,
                &file_type,
                "",
                &files,
                &[system_id],
            )
            .await
            .unwrap();

        let file_infos = file_set_repository
            .get_file_set_file_info(file_set_id)
            .await
            .unwrap();

        let file_info_id_1 = file_infos[0].file_info_id;
        let file_info_id_2 = file_infos[1].file_info_id;

        // Update sort order
        let sort_orders = vec![(file_info_id_2, 1), (file_info_id_1, 2)];

        file_set_repository
            .update_file_set_file_infos_sort_order(file_set_id, &sort_orders)
            .await
            .unwrap();

        // Verify sort order
        let result = file_set_repository
            .get_file_set_file_info(file_set_id)
            .await
            .unwrap();

        assert_eq!(result[0].file_info_id, file_info_id_2);
        assert_eq!(result[1].file_info_id, file_info_id_1);
    }

    #[async_std::test]
    async fn test_file_type_validation_prevents_mismatched_types() {
        let pool = setup_test_db().await;
        let pool = Arc::new(pool);

        // Create a FileSet with Rom type
        let rom_file_set_id = {
            let file_type = FileType::Rom as i64;
            let result = query!(
                "INSERT INTO file_set (
                    file_name,
                    file_type,
                    name,
                    source
                ) VALUES (?, ?, ?, ?)",
                "rom_set.zip",
                file_type,
                "ROM File Set",
                ""
            )
            .execute(&*pool)
            .await
            .unwrap();
            result.last_insert_rowid()
        };

        // Create a FileInfo with Document type (different from FileSet)
        let doc_file_info_id = {
            let file_type = FileType::Document as i64;
            let checksum = vec![1u8; 20];
            let result = query!(
                "INSERT INTO file_info (
                    sha1_checksum,
                    file_size,
                    archive_file_name,
                    file_type
                ) VALUES (?, ?, ?, ?)",
                checksum,
                1000,
                "document.pdf",
                file_type
            )
            .execute(&*pool)
            .await
            .unwrap();
            result.last_insert_rowid()
        };

        // Try to add Document FileInfo to Rom FileSet - should fail
        let repository = FileSetRepository { pool: pool.clone() };
        let result = repository
            .add_files_to_file_set(
                rom_file_set_id,
                &[(doc_file_info_id, "doc.pdf".to_string())],
            )
            .await;

        // Verify that validation error occurs
        assert!(result.is_err());
        let error_msg = result.unwrap_err().to_string();
        assert!(
            error_msg.contains("file types must match") || error_msg.contains("file_type"),
            "Expected validation error message, got: {}",
            error_msg
        );
    }

    #[async_std::test]
    async fn test_file_type_validation_allows_matching_types() {
        let pool = setup_test_db().await;
        let pool = Arc::new(pool);

        // Create a FileSet with Rom type
        let rom_file_set_id = {
            let file_type = FileType::Rom as i64;
            let result = query!(
                "INSERT INTO file_set (
                    file_name,
                    file_type,
                    name,
                    source
                ) VALUES (?, ?, ?, ?)",
                "rom_set.zip",
                file_type,
                "ROM File Set",
                ""
            )
            .execute(&*pool)
            .await
            .unwrap();
            result.last_insert_rowid()
        };

        // Create a FileInfo with Rom type (matching FileSet)
        let rom_file_info_id = {
            let file_type = FileType::Rom as i64;
            let checksum = vec![2u8; 20];
            let result = query!(
                "INSERT INTO file_info (
                    sha1_checksum,
                    file_size,
                    archive_file_name,
                    file_type
                ) VALUES (?, ?, ?, ?)",
                checksum,
                2000,
                "game.rom",
                file_type
            )
            .execute(&*pool)
            .await
            .unwrap();
            result.last_insert_rowid()
        };

        // Try to add Rom FileInfo to Rom FileSet - should succeed
        let repository = FileSetRepository { pool: pool.clone() };
        let result = repository
            .add_files_to_file_set(
                rom_file_set_id,
                &[(rom_file_info_id, "game.rom".to_string())],
            )
            .await;

        // Verify success
        assert!(result.is_ok());

        // Verify the association was created
        let file_infos = repository
            .get_file_set_file_info(rom_file_set_id)
            .await
            .unwrap();
        assert_eq!(file_infos.len(), 1);
        assert_eq!(file_infos[0].file_info_id, rom_file_info_id);
        assert_eq!(file_infos[0].file_type, FileType::Rom);
    }

    #[async_std::test]
    async fn test_add_item_type_to_file_set() {
        let pool = setup_test_db().await;
        let pool = Arc::new(pool);
        let file_set_repository = FileSetRepository { pool: pool.clone() };
        let file_set_id = file_set_repository
            .add_file_set("Test File Set", "test.zip", &FileType::Rom, "", &[], &[])
            .await
            .unwrap();

        file_set_repository
            .add_item_type_to_file_set(&file_set_id, &ItemType::Box)
            .await
            .unwrap();

        file_set_repository
            .add_item_type_to_file_set(&file_set_id, &ItemType::Manual)
            .await
            .unwrap();

        let item_types = file_set_repository
            .get_item_types_for_file_set(file_set_id)
            .await
            .unwrap();

        assert_eq!(item_types.len(), 2);
        assert!(item_types.contains(&ItemType::Box));
        assert!(item_types.contains(&ItemType::Manual));
    }
}
