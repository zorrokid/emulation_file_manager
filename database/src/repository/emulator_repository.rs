use std::sync::Arc;

use sqlx::{Pool, Sqlite};

use crate::{
    database_error::{DatabaseError, Error},
    models::Emulator,
};

#[derive(Debug)]
pub struct EmulatorRepository {
    pool: Arc<Pool<Sqlite>>,
}

impl EmulatorRepository {
    pub fn new(pool: Arc<Pool<Sqlite>>) -> Self {
        Self { pool }
    }

    pub async fn get_emulators(&self) -> Result<Vec<Emulator>, DatabaseError> {
        let emulators = sqlx::query_as::<_, Emulator>(
            "SELECT id, name, executable, extract_files, system_id, arguments
             FROM emulator",
        )
        .fetch_all(&*self.pool)
        .await?;
        Ok(emulators)
    }

    pub async fn get_emulators_for_systems(
        &self,
        system_ids: &[i64],
    ) -> Result<Vec<Emulator>, DatabaseError> {
        if system_ids.is_empty() {
            return Ok(vec![]);
        }

        let mut query_builder = sqlx::QueryBuilder::<Sqlite>::new(
            "SELECT DISTINCT id, name, executable, extract_files, system_id, arguments
             FROM emulator 
             WHERE system_id IN (",
        );
        let mut separated = query_builder.separated(", ");
        for id in system_ids {
            separated.push_bind(*id);
        }
        separated.push_unseparated(")");

        let query = query_builder.build_query_as::<Emulator>();
        let emulators = query.fetch_all(&*self.pool).await?;
        Ok(emulators)
    }

    pub async fn get_emulator(&self, id: i64) -> Result<Emulator, DatabaseError> {
        let emulator = sqlx::query_as::<_, Emulator>(
            "SELECT id, name, executable, extract_files, arguments, system_id
             FROM emulator WHERE id = ?",
        )
        .bind(id)
        .fetch_one(&*self.pool)
        .await?;
        Ok(emulator)
    }

    pub async fn add_emulator(
        &self,
        name: String,
        executable: String,
        extract_files: bool,
        arguments: String,
        system_id: i64,
    ) -> Result<i64, DatabaseError> {
        let result = sqlx::query!(
            "INSERT INTO emulator (
                name, 
                executable, 
                extract_files, 
                arguments,
                system_id
            ) VALUES (?, ?, ?, ?, ?)",
            name,
            executable,
            extract_files,
            arguments,
            system_id,
        )
        .execute(&*self.pool)
        .await?;

        Ok(result.last_insert_rowid())
    }

    pub async fn delete_emulator(&self, id: i64) -> Result<i64, Error> {
        sqlx::query!("DELETE FROM emulator WHERE id = ?", id)
            .execute(&*self.pool)
            .await?;
        Ok(id)
    }

    pub async fn update_emulator(&self, emulator: &Emulator) -> Result<i64, DatabaseError> {
        println!("Updating emulator: {:?}", emulator);
        let result = sqlx::query!(
            "UPDATE emulator SET
             name = ?, 
             executable = ?, 
             extract_files = ?,
             arguments = ?,
             system_id = ?
             WHERE id = ?",
            emulator.name,
            emulator.executable,
            emulator.extract_files,
            emulator.arguments,
            emulator.system_id,
            emulator.id
        )
        .execute(&*self.pool)
        .await?;
        Ok(result.last_insert_rowid())
    }
}

#[cfg(test)]
mod tests {
    use crate::{repository::system_repository::SystemRepository, setup_test_db};

    use super::*;

    #[async_std::test]
    async fn test_emulator_repository() {
        let pool = setup_test_db().await;
        let pool = Arc::new(pool);
        let repo = EmulatorRepository::new(pool.clone());
        let system_repo = SystemRepository::new(pool.clone());
        let system_id = system_repo
            .add_system(&"Test System 1".to_string())
            .await
            .unwrap();

        let system = system_repo.get_system(system_id).await.unwrap();
        assert_eq!(system.name, "Test System 1");
        assert_eq!(system.id, system_id);

        let emulator_id = repo
            .add_emulator(
                "Test Emulator".to_string(),
                "test_executable".to_string(),
                true,
                "".to_string(),
                system_id,
            )
            .await
            .unwrap();

        // Test get_emulator
        let mut emulator = repo.get_emulator(emulator_id).await.unwrap();
        assert_eq!(emulator.name, "Test Emulator");
        assert_eq!(emulator.executable, "test_executable");
        assert_eq!(emulator.system_id, system_id);

        // Test get_emulators
        let emulators = repo.get_emulators().await.unwrap();
        assert_eq!(emulators.len(), 1);

        // Test update_emulator
        emulator.name = "Updated Emulator".to_string();
        repo.update_emulator(&emulator).await.unwrap();
        let updated_emulator = repo.get_emulator(emulator_id).await.unwrap();
        assert_eq!(updated_emulator.name, "Updated Emulator");

        let result = repo.delete_emulator(emulator_id).await;
        assert!(result.is_ok());

        // try get emulator, should go to an error
        let result = repo.get_emulator(emulator_id).await;
        assert!(result.is_err());
    }
}
