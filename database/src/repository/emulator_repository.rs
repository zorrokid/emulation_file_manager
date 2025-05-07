use std::sync::Arc;

use sqlx::{Pool, Sqlite};

use crate::{
    database_error::{DatabaseError, Error},
    models::{Emulator, EmulatorSystem},
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
            "SELECT id, name, executable, extract_files
             FROM emulator",
        )
        .fetch_all(&*self.pool)
        .await?;
        Ok(emulators)
    }

    pub async fn get_emulator_with_systems(
        &self,
        id: i64,
    ) -> Result<(Emulator, Vec<EmulatorSystem>), DatabaseError> {
        let emulator = sqlx::query_as::<_, Emulator>(
            "SELECT id, name, executable, extract_files 
             FROM emulator WHERE id = ?",
        )
        .bind(id)
        .fetch_one(&*self.pool)
        .await?;

        let emulator_systems = sqlx::query_as::<_, EmulatorSystem>(
            "SELECT 
                s.id AS system_id, 
                s.name AS system_name, 
                es.arguments 
             FROM emulator_system es 
             JOIN system s ON es.system_id = s.id 
             WHERE es.emulator_id = ?",
        )
        .bind(id)
        .fetch_all(&*self.pool)
        .await?;

        Ok((emulator, emulator_systems))
    }

    pub async fn add_emulator(
        &self,
        name: String,
        executable: String,
        extract_files: bool,
    ) -> Result<i64, Error> {
        let result = sqlx::query!(
            "INSERT INTO emulator (
                name, 
                executable, 
                extract_files 
            ) VALUES (?, ?, ?)",
            name,
            executable,
            extract_files,
        )
        .execute(&*self.pool)
        .await?;
        Ok(result.last_insert_rowid())
    }

    pub async fn delete_emulator(&self, id: i64) -> Result<(), DatabaseError> {
        sqlx::query!("DELETE FROM emulator WHERE id = ?", id)
            .execute(&*self.pool)
            .await?;
        Ok(())
    }

    pub async fn update_emulator(&self, emulator: &Emulator) -> Result<i64, DatabaseError> {
        let result = sqlx::query!(
            "UPDATE emulator SET 
             name = ?, 
             executable = ?, 
             extract_files = ?
             WHERE id = ?",
            emulator.name,
            emulator.executable,
            emulator.extract_files,
            emulator.id
        )
        .execute(&*self.pool)
        .await?;
        Ok(result.last_insert_rowid())
    }

    pub async fn add_emulator_system(
        &self,
        emulator_id: i64,
        system_id: i64,
        arguments: String,
    ) -> Result<(), DatabaseError> {
        sqlx::query!(
            "INSERT INTO emulator_system (
                emulator_id, 
                system_id, 
                arguments
            ) VALUES (?, ?, ?)",
            emulator_id,
            system_id,
            arguments,
        )
        .execute(&*self.pool)
        .await?;
        Ok(())
    }

    pub async fn remove_emulator_system(
        &self,
        emulator_id: i64,
        system_id: i64,
    ) -> Result<(), DatabaseError> {
        sqlx::query!(
            "DELETE FROM emulator_system WHERE emulator_id = ? AND system_id = ?",
            emulator_id,
            system_id
        )
        .execute(&*self.pool)
        .await?;
        Ok(())
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

        let emulator_id = repo
            .add_emulator(
                "Test Emulator".to_string(),
                "test_executable".to_string(),
                true,
            )
            .await
            .unwrap();

        let system_repo = SystemRepository::new(pool.clone());

        let system_id = system_repo
            .add_system("Test System".to_string())
            .await
            .unwrap();

        // Test add_emulator_system
        repo.add_emulator_system(emulator_id, system_id, "args".to_string())
            .await
            .unwrap();

        // Test get_emulator
        let (mut emulator, emulator_systems) =
            repo.get_emulator_with_systems(emulator_id).await.unwrap();
        assert_eq!(emulator.name, "Test Emulator");
        assert_eq!(emulator.executable, "test_executable");
        assert_eq!(emulator_systems.len(), 1);
        assert_eq!(emulator_systems[0].system_id, system_id);
        assert_eq!(emulator_systems[0].system_name, "Test System");
        assert_eq!(emulator_systems[0].arguments, "args");

        // Test get_emulators
        let emulators = repo.get_emulators().await.unwrap();
        assert_eq!(emulators.len(), 1);

        // Test update_emulator
        emulator.name = "Updated Emulator".to_string();
        repo.update_emulator(&emulator).await.unwrap();
        let (updated_emulator, _) = repo.get_emulator_with_systems(emulator_id).await.unwrap();
        assert_eq!(updated_emulator.name, "Updated Emulator");

        // try deleting the emulator before removing the system relation
        let result = repo.delete_emulator(emulator_id).await;
        assert!(result.is_err());

        repo.remove_emulator_system(emulator_id, system_id)
            .await
            .unwrap();

        let (_, emulator_systems) = repo.get_emulator_with_systems(emulator_id).await.unwrap();

        assert_eq!(emulator_systems.len(), 0);

        // Test delete_emulator
        repo.delete_emulator(emulator_id).await.unwrap();
        let emulators = repo.get_emulators().await.unwrap();
        assert_eq!(emulators.len(), 0);
    }
}
