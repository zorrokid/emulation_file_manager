use std::sync::Arc;

use sqlx::{Pool, Sqlite};

use crate::{
    database_error::{DatabaseError, Error},
    models::{Emulator, EmulatorSystem, EmulatorSystemUpdateModel},
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

    pub async fn get_emulators_for_systems(
        &self,
        system_ids: &[i64],
    ) -> Result<Vec<Emulator>, DatabaseError> {
        if system_ids.is_empty() {
            return Ok(vec![]);
        }

        let mut query_builder = sqlx::QueryBuilder::<Sqlite>::new(
            "SELECT DISTINCT e.id, e.name, e.executable, e.extract_files 
             FROM emulator e 
             JOIN emulator_system es ON e.id = es.emulator_id 
             WHERE es.system_id IN (",
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
                es.id,
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

    pub async fn add_emulator_with_systems(
        &self,
        name: String,
        executable: String,
        extract_files: bool,
        systems: Vec<EmulatorSystemUpdateModel>,
    ) -> Result<i64, Error> {
        let mut transaction = self.pool.begin().await?;

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
        .execute(&mut *transaction)
        .await?;

        let emulator_id = result.last_insert_rowid();

        for system in systems {
            sqlx::query!(
                "INSERT INTO emulator_system (
                    emulator_id, 
                    system_id, 
                    arguments
                ) VALUES (?, ?, ?)",
                emulator_id,
                system.system_id,
                system.arguments,
            )
            .execute(&mut *transaction)
            .await?;
        }

        transaction.commit().await?;
        Ok(emulator_id)
    }

    pub async fn update_emulator_with_systems(
        &self,
        emulator_id: i64,
        name: String,
        executable: String,
        extract_files: bool,
        systems: Vec<EmulatorSystemUpdateModel>,
    ) -> Result<i64, Error> {
        let mut transaction = self.pool.begin().await?;
        dbg!("Updating emulator with id: {}", emulator_id);

        // update first the emulator
        sqlx::query!(
            "UPDATE emulator 
             SET 
                name = ?, 
                executable = ?, 
                extract_files = ? 
                WHERE id = ?",
            name,
            executable,
            extract_files,
            emulator_id,
        )
        .execute(&mut *transaction)
        .await?;

        // split emulator system to ones that are new and ones that are existing
        let (systems_to_update, systems_to_insert): (Vec<_>, Vec<_>) =
            systems.iter().partition(|s| s.id.is_some());

        let systems_to_update_ids = systems_to_update
            .iter()
            .filter_map(|s| s.id)
            .collect::<Vec<_>>();

        // delete obsolete emulator systems
        // get the ids of emulator systems that should be deleted
        let existing_system_ids: Vec<i64> = sqlx::query!(
            "SELECT id 
             FROM emulator_system 
             WHERE emulator_id = ?",
            emulator_id,
        )
        .fetch_all(&mut *transaction)
        .await?
        .iter()
        .map(|system| system.id)
        .collect();

        let removable_system_ids = existing_system_ids
            .into_iter()
            .filter(|id| !systems_to_update_ids.contains(id))
            .collect::<Vec<i64>>();

        println!("removable_system_ids: {:?}", removable_system_ids);

        for id in &removable_system_ids {
            sqlx::query!(
                "DELETE FROM emulator_system 
                 WHERE emulator_id = ? AND id = ?",
                emulator_id,
                id
            )
            .execute(&mut *transaction)
            .await?;
        }

        // insert new emulator systems

        for system in systems_to_insert {
            sqlx::query!(
                "INSERT INTO emulator_system (
                    emulator_id, 
                    system_id, 
                    arguments
                ) VALUES (?, ?, ?)",
                emulator_id,
                system.system_id,
                system.arguments,
            )
            .execute(&mut *transaction)
            .await?;
        }

        // update existing emulator systems
        for system in systems_to_update {
            sqlx::query!(
                "UPDATE emulator_system 
                 SET 
                    arguments = ? 
                 WHERE id = ?",
                system.arguments,
                system.id,
            )
            .execute(&mut *transaction)
            .await?;
        }

        transaction.commit().await?;
        Ok(emulator_id)
    }

    pub async fn delete_emulator(&self, id: i64) -> Result<i64, Error> {
        sqlx::query!("DELETE FROM emulator WHERE id = ?", id)
            .execute(&*self.pool)
            .await?;
        Ok(id)
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
        let system_repo = SystemRepository::new(pool.clone());
        let system_1_id = system_repo
            .add_system(&"Test System 1".to_string())
            .await
            .unwrap();

        let system_2_id = system_repo
            .add_system(&"Test System 2".to_string())
            .await
            .unwrap();

        let system_3_id = system_repo
            .add_system(&"Test System 3".to_string())
            .await
            .unwrap();

        let emulator_systems = vec![
            EmulatorSystemUpdateModel {
                id: None,
                system_id: system_1_id,
                arguments: "args".to_string(),
            },
            EmulatorSystemUpdateModel {
                id: None,
                system_id: system_2_id,
                arguments: "args".to_string(),
            },
        ];

        let emulator_id = repo
            .add_emulator_with_systems(
                "Test Emulator".to_string(),
                "test_executable".to_string(),
                true,
                emulator_systems,
            )
            .await
            .unwrap();

        // Test get_emulator
        let (mut emulator, emulator_systems) =
            repo.get_emulator_with_systems(emulator_id).await.unwrap();
        assert_eq!(emulator.name, "Test Emulator");
        assert_eq!(emulator.executable, "test_executable");
        assert_eq!(emulator_systems.len(), 2);

        let emulator_system_1 = &emulator_systems
            .iter()
            .find(|s| s.system_id == system_1_id)
            .unwrap();

        let emulator_system_2 = &emulator_systems
            .iter()
            .find(|s| s.system_id == system_2_id)
            .unwrap();

        assert_eq!(emulator_system_1.system_name, "Test System 1");
        assert_eq!(emulator_system_2.system_name, "Test System 2");

        // Test get_emulators
        let emulators = repo.get_emulators().await.unwrap();
        assert_eq!(emulators.len(), 1);

        // Test update_emulator
        emulator.name = "Updated Emulator".to_string();
        repo.update_emulator(&emulator).await.unwrap();
        let (updated_emulator, _) = repo.get_emulator_with_systems(emulator_id).await.unwrap();
        assert_eq!(updated_emulator.name, "Updated Emulator");

        // Test update_emulator_with_systems
        emulator.name = "Updated Emulator".to_string();
        let updated_emulator_systems = vec![
            // update system 1
            EmulatorSystemUpdateModel {
                id: Some(emulator_systems[0].id),
                system_id: system_1_id,
                arguments: "new_args".to_string(),
            },
            // add system 3
            EmulatorSystemUpdateModel {
                id: None,
                system_id: system_3_id,
                arguments: "another_system_args".to_string(),
            },
            // remove system 2 since it's not in collection
        ];
        repo.update_emulator_with_systems(
            emulator.id,
            emulator.name.clone(),
            emulator.executable.clone(),
            emulator.extract_files,
            updated_emulator_systems,
        )
        .await
        .unwrap();

        let (updated_emulator, updated_emulator_systems) =
            repo.get_emulator_with_systems(emulator_id).await.unwrap();
        assert_eq!(updated_emulator.name, "Updated Emulator");
        assert_eq!(updated_emulator_systems.len(), 2);
        let updated_emulator_system_1 = &updated_emulator_systems
            .iter()
            .find(|s| s.system_id == system_1_id)
            .unwrap();

        let updated_emulator_system_3 = &updated_emulator_systems
            .iter()
            .find(|s| s.system_id == system_3_id)
            .unwrap();

        assert_eq!(updated_emulator_system_1.system_name, "Test System 1");
        assert_eq!(updated_emulator_system_1.arguments, "new_args");
        assert_eq!(updated_emulator_system_3.system_name, "Test System 3");
        assert_eq!(updated_emulator_system_3.arguments, "another_system_args");

        let result = repo.delete_emulator(emulator_id).await;
        assert!(result.is_ok());

        // try get emulator, should go to an error
        let result = repo.get_emulator_with_systems(emulator_id).await;
        assert!(result.is_err());
        // try get emulator system 1, should go to an error
        let result = sqlx::query!(
            "SELECT id 
             FROM emulator_system 
             WHERE emulator_id = ? AND system_id = ?",
            emulator_id,
            system_1_id
        )
        .fetch_one(&*pool)
        .await;

        assert!(result.is_err());
    }
}
