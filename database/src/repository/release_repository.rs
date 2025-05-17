use std::sync::Arc;

use sqlx::{query_as, Pool, Sqlite};

use crate::{
    database_error::{DatabaseError, Error},
    models::{FileType, Release, ReleaseExtended},
};

#[derive(Debug)]
pub struct ReleaseRepository {
    pool: Arc<Pool<Sqlite>>,
}

#[derive(sqlx::FromRow)]
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

    pub async fn get_releases(&self) -> Result<Vec<ReleaseExtended>, Error> {
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
             GROUP BY
                r.id, r.name;
        "#;
        let raw_releases: Vec<ReleaseExtendedRaw> = query_as(query).fetch_all(&*self.pool).await?;

        let mut releases: Vec<ReleaseExtended> = Vec::new();

        for raw in raw_releases {
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
            let file_types = raw
                .file_types
                .unwrap_or_default()
                .split(',')
                .map(|ft| {
                    ft.parse::<i64>()
                        .map_err(|e| {
                            Error::ParseError(format!("Failed to parse '{}' as i64: {}", ft, e))
                        })
                        .and_then(|ft| {
                            FileType::try_from(ft)
                                .map_err(|e| Error::ParseError(format!("Invalid file type {}", ft)))
                        })
                })
                .collect::<Result<Vec<FileType>, Error>>()?;
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
            .execute(&*self.pool)
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
        let count = sqlx::query_scalar!(
            "SELECT COUNT(*) FROM release_file_set WHERE release_id = ?",
            id
        )
        .fetch_one(&*self.pool)
        .await?;

        if count > 0 {
            return Err(DatabaseError::InUse);
        }
        sqlx::query!("DELETE FROM release WHERE id = ?", id)
            .execute(&*self.pool)
            .await?;
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
    use super::*;
    use crate::{repository::software_title_repository::SoftwareTitleRepository, setup_test_db};

    #[async_std::test]
    async fn test_release_repository() {
        let pool = setup_test_db().await;
        let pool = Arc::new(pool);
        let release_repository = ReleaseRepository::new(pool.clone());

        let software_title_repository = SoftwareTitleRepository::new(pool.clone());

        let software_title_id = software_title_repository
            .add_software_title("Test Software Title".to_string(), None)
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

        // try deleting the release before removing the software title relation
        let result = release_repository.delete_release(release_id).await;
        assert!(result.is_err());

        release_repository
            .remove_software_title_from_release(release_id, software_title_id)
            .await
            .unwrap();

        // Verify that the release is deleted
        release_repository.delete_release(release_id).await.unwrap();

        // Verify deletion
        let result = release_repository.get_release(release_id).await;
        assert!(result.is_err());
    }
}
