use std::sync::Arc;

use core_types::FileType;
use sqlx::{query_as, Pool, Sqlite};

use crate::{
    database_error::{DatabaseError, Error},
    models::{Release, ReleaseExtended},
};

#[derive(Debug)]
pub struct ReleaseRepository {
    pool: Arc<Pool<Sqlite>>,
}

#[derive(sqlx::FromRow, Debug)]
struct ReleaseExtendedRaw {
    id: i64,
    name: String,
    system_names: Option<String>,
    software_title_names: Option<String>,
    file_types: Option<String>,
}

impl ReleaseRepository {
    pub fn new(pool: Arc<Pool<Sqlite>>) -> Self {
        Self { pool }
    }

    pub async fn get_release(&self, id: i64) -> Result<Release, DatabaseError> {
        let release = sqlx::query_as!(
            Release,
            "SELECT id, name 
             FROM release WHERE id = ?",
            id
        )
        .fetch_one(&*self.pool)
        .await?;

        Ok(release)
    }

    pub async fn get_releases(
        &self,
        system_id: Option<i64>,
        software_title_id: Option<i64>,
        file_set_id: Option<i64>,
    ) -> Result<Vec<ReleaseExtended>, Error> {
        let query = r#"
            SELECT
                r.id as id,
                r.name as name,
                GROUP_CONCAT(DISTINCT s.name) as system_names,
                GROUP_CONCAT(DISTINCT st.name) as software_title_names,
                GROUP_CONCAT(DISTINCT fs.file_type) as file_types
             FROM
                release r
             INNER JOIN
                release_software_title rst ON r.id = rst.release_id
             INNER JOIN
                software_title st ON rst.software_title_id = st.id
             INNER JOIN
                release_system rs ON r.id = rs.release_id
             INNER JOIN
                system s ON rs.system_id = s.id
             INNER JOIN
                release_file_set rfs ON r.id = rfs.release_id
             INNER JOIN
                file_set fs ON rfs.file_set_id = fs.id
            WHERE
                (? IS NULL OR s.id = ?)
                AND (? IS NULL OR st.id = ?)
                AND (? IS NULL OR fs.id = ?)
             GROUP BY
                r.id, r.name;
        "#;

        let raw_releases: Vec<ReleaseExtendedRaw> = query_as(query)
            .bind(system_id)
            .bind(system_id)
            .bind(software_title_id)
            .bind(software_title_id)
            .bind(file_set_id)
            .bind(file_set_id)
            .fetch_all(&*self.pool)
            .await?;

        let mut releases: Vec<ReleaseExtended> = Vec::new();

        for raw in raw_releases {
            dbg!("Raw release: {}", &raw);
            let system_names = raw
                .system_names
                .unwrap_or_default()
                .split(',')
                .map(String::from)
                .collect();
            let software_title_names = raw
                .software_title_names
                .unwrap_or_default()
                .split(',')
                .map(String::from)
                .collect();
            let file_types: Vec<FileType> = raw
                .file_types
                .unwrap_or_default()
                .split(',')
                .map(|ft| {
                    let int_ft: u8 = ft.parse().expect("Failed to parse file type as u8");
                    FileType::from_db_int(int_ft).expect("Invalid file type")
                })
                .collect();
            releases.push(ReleaseExtended {
                id: raw.id,
                name: raw.name,
                system_names,
                software_title_names,
                file_types,
            });
        }

        Ok(releases)
    }

    pub async fn get_releases_with_software_title(
        &self,
        software_title_id: i64,
    ) -> Result<Vec<Release>, DatabaseError> {
        let releases = sqlx::query_as!(
            Release,
            "SELECT r.id as id, r.name as name 
             FROM release r
             INNER JOIN release_software_title rst 
             ON r.id = rst.release_id
             WHERE rst.software_title_id = ?",
            software_title_id
        )
        .fetch_all(&*self.pool)
        .await?;

        Ok(releases)
    }

    pub async fn add_release(&self, release_name: &str) -> Result<i64, DatabaseError> {
        let result = sqlx::query!("INSERT INTO release (name) VALUES (?)", release_name)
            .execute(&*self.pool)
            .await?;
        Ok(result.last_insert_rowid())
    }

    pub async fn add_release_full(
        &self,
        release_name: String,
        software_title_ids: Vec<i64>,
        file_set_ids: Vec<i64>,
        system_ids: Vec<i64>,
    ) -> Result<i64, Error> {
        let mut transaction = self.pool.begin().await?;

        let result = sqlx::query!("INSERT INTO release (name) VALUES (?)", release_name)
            .execute(&mut *transaction)
            .await?;

        let release_id = result.last_insert_rowid();

        for software_title_id in software_title_ids {
            sqlx::query!(
                "INSERT INTO release_software_title (release_id, software_title_id) VALUES (?, ?)",
                release_id,
                software_title_id
            )
            .execute(&mut *transaction)
            .await?;
        }

        for file_id in file_set_ids {
            sqlx::query!(
                "INSERT INTO release_file_set (release_id, file_set_id) VALUES (?, ?)",
                release_id,
                file_id
            )
            .execute(&mut *transaction)
            .await?;
        }

        for system_id in system_ids {
            sqlx::query!(
                "INSERT INTO release_system (release_id, system_id) VALUES (?, ?)",
                release_id,
                system_id
            )
            .execute(&mut *transaction)
            .await?;
        }

        transaction.commit().await?;

        Ok(release_id)
    }

    pub async fn update_release_full(
        &self,
        release_id: i64,
        release_name: String,
        software_title_ids: Vec<i64>,
        file_set_ids: Vec<i64>,
        system_ids: Vec<i64>,
    ) -> Result<i64, Error> {
        let mut transaction = self.pool.begin().await?;

        sqlx::query!("UPDATE release SET name = ?", release_name)
            .execute(&mut *transaction)
            .await?;

        // update software titles

        // get existing software titles
        let existing_software_titles = sqlx::query!(
            "SELECT software_title_id FROM release_software_title WHERE release_id = ?",
            release_id
        )
        .fetch_all(&mut *transaction)
        .await?
        .into_iter()
        .map(|row| row.software_title_id)
        .collect::<Vec<i64>>();

        let entries_to_be_deleted = existing_software_titles
            .iter()
            .filter(|&&id| !software_title_ids.contains(&id))
            .cloned()
            .collect::<Vec<i64>>();

        for id in &entries_to_be_deleted {
            sqlx::query!(
                "DELETE FROM release_software_title WHERE release_id = ? AND software_title_id = ?",
                release_id,
                id,
            )
            .execute(&mut *transaction)
            .await?;
        }

        let new_entries = software_title_ids
            .iter()
            .filter(|&&id| !existing_software_titles.contains(&id))
            .cloned()
            .collect::<Vec<i64>>();

        for id in new_entries {
            sqlx::query!(
                "INSERT INTO release_software_title (release_id, software_title_id) VALUES (?, ?)",
                release_id,
                id
            )
            .execute(&mut *transaction)
            .await?;
        }

        // NOTE: no need to update the existing ones

        // update systems titles
        // get existing systems
        let existing_systems = sqlx::query!(
            "SELECT system_id FROM release_system WHERE release_id = ?",
            release_id
        )
        .fetch_all(&mut *transaction)
        .await?
        .into_iter()
        .map(|row| row.system_id)
        .collect::<Vec<i64>>();

        let entries_to_be_deleted = existing_systems
            .iter()
            .filter(|&&id| !system_ids.contains(&id))
            .cloned()
            .collect::<Vec<i64>>();

        for id in &entries_to_be_deleted {
            sqlx::query!(
                "DELETE FROM release_system WHERE release_id = ? AND system_id = ?",
                release_id,
                id,
            )
            .execute(&mut *transaction)
            .await?;
        }

        let new_entries = system_ids
            .iter()
            .filter(|&&id| !existing_systems.contains(&id))
            .cloned()
            .collect::<Vec<i64>>();

        for id in new_entries {
            sqlx::query!(
                "INSERT INTO release_system (release_id, system_id) VALUES (?, ?)",
                release_id,
                id
            )
            .execute(&mut *transaction)
            .await?;
        }

        // NOTE: no need to update the existing ones

        // update file sets
        // get existing file sets
        let existing_file_sets = sqlx::query!(
            "SELECT file_set_id FROM release_file_set WHERE release_id = ?",
            release_id
        )
        .fetch_all(&mut *transaction)
        .await?
        .into_iter()
        .map(|row| row.file_set_id)
        .collect::<Vec<i64>>();

        let entries_to_be_deleted = existing_file_sets
            .iter()
            .filter(|&&id| !file_set_ids.contains(&id))
            .cloned()
            .collect::<Vec<i64>>();

        for id in &entries_to_be_deleted {
            sqlx::query!(
                "DELETE FROM release_file_set WHERE release_id = ? AND file_set_id = ?",
                release_id,
                id,
            )
            .execute(&mut *transaction)
            .await?;
        }

        let new_entries = file_set_ids
            .iter()
            .filter(|&&id| !existing_file_sets.contains(&id))
            .cloned()
            .collect::<Vec<i64>>();
        for id in new_entries {
            sqlx::query!(
                "INSERT INTO release_file_set (release_id, file_set_id) VALUES (?, ?)",
                release_id,
                id
            )
            .execute(&mut *transaction)
            .await?;
        }
        // NOTE: no need to update the existing ones

        transaction.commit().await?;
        Ok(release_id)
    }

    pub async fn update_release(&self, release: &Release) -> Result<u64, DatabaseError> {
        let result = sqlx::query!(
            "UPDATE release SET name = ? WHERE id = ?",
            release.name,
            release.id
        )
        .execute(&*self.pool)
        .await?;
        Ok(result.rows_affected())
    }

    pub async fn delete_release(&self, id: i64) -> Result<i64, DatabaseError> {
        let mut transaction = self.pool.begin().await?;
        sqlx::query!("DELETE FROM release WHERE id = ?", id)
            .execute(&mut *transaction)
            .await?;
        transaction.commit().await?;
        Ok(id)
    }

    pub async fn add_software_title_to_release(
        &self,
        release_id: i64,
        software_title_id: i64,
    ) -> Result<(), DatabaseError> {
        sqlx::query!(
            "INSERT INTO release_software_title (release_id, software_title_id) VALUES (?, ?)",
            release_id,
            software_title_id
        )
        .execute(&*self.pool)
        .await?;
        Ok(())
    }

    pub async fn remove_software_title_from_release(
        &self,
        release_id: i64,
        software_title_id: i64,
    ) -> Result<(), DatabaseError> {
        sqlx::query!(
            "DELETE FROM release_software_title WHERE release_id = ? AND software_title_id = ?",
            release_id,
            software_title_id
        )
        .execute(&*self.pool)
        .await?;
        Ok(())
    }

    pub async fn has_release_files(&self, release_id: i64) -> Result<bool, DatabaseError> {
        let count = sqlx::query_scalar!(
            "SELECT COUNT(*) FROM release_file_set WHERE release_id = ?",
            release_id
        )
        .fetch_one(&*self.pool)
        .await?;
        Ok(count > 0)
    }
}

#[cfg(test)]
mod tests {
    use core_types::ImportedFile;

    use super::*;
    use crate::{
        repository::{
            file_set_repository::FileSetRepository,
            software_title_repository::SoftwareTitleRepository,
            system_repository::SystemRepository,
        },
        setup_test_db,
    };

    #[async_std::test]
    async fn test_release_repository() {
        let pool = setup_test_db().await;
        let pool = Arc::new(pool);
        let release_repository = ReleaseRepository::new(pool.clone());

        let software_title_repository = SoftwareTitleRepository::new(pool.clone());

        let software_title_id = software_title_repository
            .add_software_title(&"Test Software Title".to_string(), None)
            .await
            .unwrap();

        let release_id = release_repository
            .add_release("Test Release")
            .await
            .unwrap();

        release_repository
            .add_software_title_to_release(release_id, software_title_id)
            .await
            .unwrap();

        let release = release_repository
            .get_releases_with_software_title(software_title_id)
            .await
            .unwrap();
        assert_eq!(release.len(), 1);
        assert_eq!(release[0].name, "Test Release");

        // Update the release
        let updated_release = Release {
            id: release_id,
            name: "Updated Release".to_string(),
        };
        release_repository
            .update_release(&updated_release)
            .await
            .unwrap();

        let updated_release = release_repository.get_release(release_id).await.unwrap();
        assert_eq!(updated_release.name, "Updated Release");

        // Delete the release - with CASCADE DELETE, this should succeed and remove relationships
        release_repository.delete_release(release_id).await.unwrap();

        // Verify that the software title relationship was cascaded away by checking junction table
        let relationship_count = sqlx::query_scalar!(
            "SELECT COUNT(*) FROM release_software_title WHERE release_id = ? AND software_title_id = ?",
            release_id,
            software_title_id
        )
        .fetch_one(pool.as_ref())
        .await
        .unwrap();
        assert_eq!(relationship_count, 0);

        // Verify that the software title itself still exists (only relationship was deleted)
        let software_title_still_exists = software_title_repository
            .get_software_title(software_title_id)
            .await
            .unwrap();
        assert_eq!(software_title_still_exists.name, "Test Software Title");

        // Verify deletion
        let result = release_repository.get_release(release_id).await;
        assert!(result.is_err());
    }

    #[async_std::test]
    async fn test_add_and_update_release() {
        let pool = setup_test_db().await;
        let pool = Arc::new(pool);
        let release_repository = ReleaseRepository::new(pool.clone());

        // let's insert first some systems, software_titles and file sets
        let software_title_repository = SoftwareTitleRepository::new(pool.clone());

        let software_title_1_id = software_title_repository
            .add_software_title(&"Software 1".to_string(), None)
            .await
            .unwrap();

        let software_title_2_id = software_title_repository
            .add_software_title(&"Software 1".to_string(), None)
            .await
            .unwrap();

        let software_title_3_id = software_title_repository
            .add_software_title(&"Software 3".to_string(), None)
            .await
            .unwrap();

        let system_repository = SystemRepository::new(pool.clone());
        let system_1_id = system_repository
            .add_system(&"System 1".to_string())
            .await
            .unwrap();
        let system_2_id = system_repository
            .add_system(&"System 2".to_string())
            .await
            .unwrap();
        let system_3_id = system_repository
            .add_system(&"System 3".to_string())
            .await
            .unwrap();

        let file_set_repository = FileSetRepository::new(pool.clone());
        let file_set_1_id = file_set_repository
            .add_file_set(
                &"Test file set".to_string(),
                &"File Set 1".to_string(),
                &FileType::Rom,
                "",
                &[ImportedFile {
                    original_file_name: "File1.bin".to_string(),
                    archive_file_name: "File1.zst".to_string(),
                    file_size: 1024,
                    sha1_checksum: [0; 20],
                }],
                &[system_1_id],
            )
            .await
            .unwrap();

        let file_set_2_id = file_set_repository
            .add_file_set(
                &"Test file set 2".to_string(),
                &"File Set 2".to_string(),
                &FileType::Rom,
                "",
                &[ImportedFile {
                    original_file_name: "File2.bin".to_string(),
                    archive_file_name: "File1.zst".to_string(),
                    file_size: 1024,
                    sha1_checksum: [1; 20],
                }],
                &[system_2_id],
            )
            .await
            .unwrap();

        let file_set_3_id = file_set_repository
            .add_file_set(
                &"Test file set 3".to_string(),
                &"File Set 3".to_string(),
                &FileType::Rom,
                "",
                &[ImportedFile {
                    original_file_name: "File3.bin".to_string(),
                    archive_file_name: "File1.zst".to_string(),
                    file_size: 1024,
                    sha1_checksum: [2; 20],
                }],
                &[system_3_id],
            )
            .await
            .unwrap();

        // Add a release
        let release_id = release_repository
            .add_release_full(
                "Test Release".to_string(),
                vec![software_title_1_id],
                vec![file_set_1_id],
                vec![system_1_id],
            )
            .await
            .unwrap();

        // Verify the release was added
        let release = release_repository.get_release(release_id).await.unwrap();
        assert_eq!(release.name, "Test Release");

        // Verify the software title was added
        let releases = release_repository
            .get_releases_with_software_title(software_title_1_id)
            .await
            .unwrap();
        assert_eq!(releases.len(), 1);
        assert_eq!(releases[0].name, "Test Release");

        // Verify the file set was added
        let file_sets = sqlx::query!(
            "SELECT file_set_id FROM release_file_set WHERE release_id = ?",
            release_id
        )
        .fetch_all(&*pool)
        .await
        .unwrap();
        assert_eq!(file_sets.len(), 1);
        assert_eq!(file_sets[0].file_set_id, file_set_1_id);

        // Verify the system was added
        let systems = sqlx::query!(
            "SELECT system_id FROM release_system WHERE release_id = ?",
            release_id
        )
        .fetch_all(&*pool)
        .await
        .unwrap();

        assert_eq!(systems.len(), 1);
        assert_eq!(systems[0].system_id, system_1_id);

        // Update the release
        let updated_release_id = release_repository
            .update_release_full(
                release_id,
                "Updated Release".to_string(),
                vec![software_title_2_id, software_title_3_id],
                vec![file_set_2_id, file_set_3_id],
                vec![system_2_id, system_3_id],
            )
            .await
            .unwrap();

        // Verify the release was updated
        let updated_release = release_repository
            .get_release(updated_release_id)
            .await
            .unwrap();
        assert_eq!(updated_release.name, "Updated Release");
        assert_eq!(updated_release.id, release_id);

        // Verify the software titles were updated
        let releases = release_repository
            .get_releases_with_software_title(software_title_2_id)
            .await
            .unwrap();
        assert_eq!(releases.len(), 1);
        assert_eq!(releases[0].name, "Updated Release");
        let releases = release_repository
            .get_releases_with_software_title(software_title_3_id)
            .await
            .unwrap();
        assert_eq!(releases.len(), 1);
        assert_eq!(releases[0].name, "Updated Release");
        let releases = release_repository
            .get_releases_with_software_title(software_title_1_id)
            .await
            .unwrap();
        assert_eq!(releases.len(), 0);

        // Verify the file sets were updated
        let file_sets = sqlx::query!(
            "SELECT file_set_id FROM release_file_set WHERE release_id = ?",
            updated_release_id
        )
        .fetch_all(&*pool)
        .await
        .unwrap();

        assert_eq!(file_sets.len(), 2);
        assert_eq!(file_sets[0].file_set_id, file_set_2_id);
        assert_eq!(file_sets[1].file_set_id, file_set_3_id);

        // Verify the systems were updated
        let systems = sqlx::query!(
            "SELECT system_id FROM release_system WHERE release_id = ?",
            updated_release_id
        )
        .fetch_all(&*pool)
        .await
        .unwrap();
        assert_eq!(systems.len(), 2);
        assert_eq!(systems[0].system_id, system_2_id);
        assert_eq!(systems[1].system_id, system_3_id);
    }

    #[async_std::test]
    async fn test_cascade_delete_release_system_when_release_deleted() {
        let pool = Arc::new(setup_test_db().await);

        // Create release and system
        let release_id = insert_test_release(&pool).await;
        let system_id = insert_test_system(&pool).await;

        // Link them in release_system
        sqlx::query!(
            "INSERT INTO release_system (release_id, system_id) VALUES (?, ?)",
            release_id,
            system_id
        )
        .execute(&*pool)
        .await
        .unwrap();

        // Verify relationship exists
        let count_before = sqlx::query_scalar!(
            "SELECT COUNT(*) FROM release_system WHERE release_id = ? AND system_id = ?",
            release_id,
            system_id
        )
        .fetch_one(&*pool)
        .await
        .unwrap();
        assert_eq!(count_before, 1);

        // Delete the release
        sqlx::query!("DELETE FROM release WHERE id = ?", release_id)
            .execute(&*pool)
            .await
            .unwrap();

        // Verify CASCADE DELETE removed the relationship
        let count_after = sqlx::query_scalar!(
            "SELECT COUNT(*) FROM release_system WHERE release_id = ? AND system_id = ?",
            release_id,
            system_id
        )
        .fetch_one(&*pool)
        .await
        .unwrap();
        assert_eq!(count_after, 0);

        // Verify system still exists
        let system_exists =
            sqlx::query_scalar!("SELECT COUNT(*) FROM system WHERE id = ?", system_id)
                .fetch_one(&*pool)
                .await
                .unwrap();
        assert_eq!(system_exists, 1);
    }

    #[async_std::test]
    async fn test_cascade_delete_release_system_when_system_deleted() {
        let pool = Arc::new(setup_test_db().await);

        // Create release and system
        let release_id = insert_test_release(&pool).await;
        let system_id = insert_test_system(&pool).await;

        // Link them in release_system
        sqlx::query!(
            "INSERT INTO release_system (release_id, system_id) VALUES (?, ?)",
            release_id,
            system_id
        )
        .execute(&*pool)
        .await
        .unwrap();

        // Delete the system
        sqlx::query!("DELETE FROM system WHERE id = ?", system_id)
            .execute(&*pool)
            .await
            .unwrap();

        // Verify CASCADE DELETE removed the relationship
        let count_after = sqlx::query_scalar!(
            "SELECT COUNT(*) FROM release_system WHERE release_id = ? AND system_id = ?",
            release_id,
            system_id
        )
        .fetch_one(&*pool)
        .await
        .unwrap();
        assert_eq!(count_after, 0);

        // Verify release still exists
        let release_exists =
            sqlx::query_scalar!("SELECT COUNT(*) FROM release WHERE id = ?", release_id)
                .fetch_one(&*pool)
                .await
                .unwrap();
        assert_eq!(release_exists, 1);
    }

    #[async_std::test]
    async fn test_cascade_delete_release_software_title_when_release_deleted() {
        let pool = Arc::new(setup_test_db().await);

        // Create release and software title
        let release_id = insert_test_release(&pool).await;
        let software_title_id = insert_test_software_title(&pool).await;

        // Link them in release_software_title
        sqlx::query!(
            "INSERT INTO release_software_title (release_id, software_title_id) VALUES (?, ?)",
            release_id,
            software_title_id
        )
        .execute(&*pool)
        .await
        .unwrap();

        // Delete the release
        sqlx::query!("DELETE FROM release WHERE id = ?", release_id)
            .execute(&*pool)
            .await
            .unwrap();

        // Verify CASCADE DELETE removed the relationship
        let count_after = sqlx::query_scalar!(
            "SELECT COUNT(*) FROM release_software_title WHERE release_id = ? AND software_title_id = ?",
            release_id,
            software_title_id
        )
        .fetch_one(&*pool)
        .await
        .unwrap();
        assert_eq!(count_after, 0);

        // Verify software title still exists
        let software_title_exists = sqlx::query_scalar!(
            "SELECT COUNT(*) FROM software_title WHERE id = ?",
            software_title_id
        )
        .fetch_one(&*pool)
        .await
        .unwrap();
        assert_eq!(software_title_exists, 1);
    }

    #[async_std::test]
    async fn test_cascade_delete_release_software_title_when_software_title_deleted() {
        let pool = Arc::new(setup_test_db().await);

        // Create release and software title
        let release_id = insert_test_release(&pool).await;
        let software_title_id = insert_test_software_title(&pool).await;

        // Link them in release_software_title
        sqlx::query!(
            "INSERT INTO release_software_title (release_id, software_title_id) VALUES (?, ?)",
            release_id,
            software_title_id
        )
        .execute(&*pool)
        .await
        .unwrap();

        // Delete the software title
        sqlx::query!("DELETE FROM software_title WHERE id = ?", software_title_id)
            .execute(&*pool)
            .await
            .unwrap();

        // Verify CASCADE DELETE removed the relationship
        let count_after = sqlx::query_scalar!(
            "SELECT COUNT(*) FROM release_software_title WHERE release_id = ? AND software_title_id = ?",
            release_id,
            software_title_id
        )
        .fetch_one(&*pool)
        .await
        .unwrap();
        assert_eq!(count_after, 0);

        // Verify release still exists
        let release_exists =
            sqlx::query_scalar!("SELECT COUNT(*) FROM release WHERE id = ?", release_id)
                .fetch_one(&*pool)
                .await
                .unwrap();
        assert_eq!(release_exists, 1);
    }

    async fn insert_test_release(pool: &Pool<Sqlite>) -> i64 {
        let result = sqlx::query!("INSERT INTO release (name) VALUES (?)", "Test Release")
            .execute(pool)
            .await
            .unwrap();
        result.last_insert_rowid()
    }

    async fn insert_test_system(pool: &Pool<Sqlite>) -> i64 {
        let result = sqlx::query!("INSERT INTO system (name) VALUES (?)", "Test System")
            .execute(pool)
            .await
            .unwrap();
        result.last_insert_rowid()
    }

    async fn insert_test_software_title(pool: &Pool<Sqlite>) -> i64 {
        let result = sqlx::query!(
            "INSERT INTO software_title (name) VALUES (?)",
            "Test Software Title"
        )
        .execute(pool)
        .await
        .unwrap();
        result.last_insert_rowid()
    }
}
