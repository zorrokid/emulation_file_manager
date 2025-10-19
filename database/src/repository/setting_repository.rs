use std::{collections::HashMap, sync::Arc};

use core_types::SettingName;
use sqlx::{Pool, Sqlite};

use crate::database_error::DatabaseError;

#[derive(Debug)]
pub struct SettingRepository {
    pool: Arc<Pool<Sqlite>>,
}

impl SettingRepository {
    pub fn new(pool: Arc<Pool<Sqlite>>) -> Self {
        Self { pool }
    }

    pub async fn get_settings(&self) -> Result<HashMap<String, String>, DatabaseError> {
        let rows = sqlx::query!("SELECT key, value FROM setting")
            .fetch_all(&*self.pool)
            .await?;
        let settings = rows.into_iter().map(|row| (row.key, row.value)).collect();

        Ok(settings)
    }

    pub async fn get_setting(&self, key: &SettingName) -> Result<String, DatabaseError> {
        let key = key.as_str();
        let row = sqlx::query!("SELECT value FROM setting WHERE key = ?", key)
            .fetch_one(&*self.pool)
            .await?;
        Ok(row.value)
    }

    pub async fn add_setting(&self, key: &SettingName, value: &str) -> Result<(), DatabaseError> {
        let key = key.as_str();
        sqlx::query!(
            "INSERT INTO setting (key, value) 
             VALUES (?, ?)
             ",
            key,
            value
        )
        .execute(&*self.pool)
        .await?;
        Ok(())
    }

    pub async fn update_setting(
        &self,
        key: &SettingName,
        value: &str,
    ) -> Result<(), DatabaseError> {
        let key = key.as_str();
        sqlx::query!(
            "UPDATE setting SET value = ? 
             WHERE key = ?
            ",
            value,
            key
        )
        .execute(&*self.pool)
        .await?;
        Ok(())
    }

    pub async fn add_or_update_setting(
        &self,
        key: &SettingName,
        value: &str,
    ) -> Result<(), DatabaseError> {
        if self.get_setting(key).await.is_ok() {
            self.update_setting(key, value).await?;
        } else {
            self.add_setting(key, value).await?;
        }
        Ok(())
    }

    pub async fn add_or_update_settings(
        &self,
        settings: &HashMap<SettingName, String>,
    ) -> Result<(), DatabaseError> {
        for (key, value) in settings {
            self.add_or_update_setting(key, value).await?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use core_types::SettingName;

    use crate::setup_test_db;

    use super::SettingRepository;

    #[async_std::test]
    async fn test_get_settings() {
        let pool = Arc::new(setup_test_db().await);
        let repository = SettingRepository::new(pool.clone());
        repository
            .add_setting(&SettingName::S3Region, "test_value")
            .await
            .unwrap();

        let settings = repository.get_settings().await.unwrap();
        assert_eq!(
            settings.get(SettingName::S3Region.as_str()).unwrap(),
            "test_value"
        );

        repository
            .update_setting(&SettingName::S3Region, "updated_value")
            .await
            .unwrap();

        let setting = repository
            .get_setting(&SettingName::S3Region)
            .await
            .unwrap();
        assert_eq!(setting, "updated_value");

        repository
            .add_or_update_setting(&SettingName::S3Region, "new_value")
            .await
            .unwrap();
        let setting = repository
            .get_setting(&SettingName::S3Region)
            .await
            .unwrap();
        assert_eq!(setting, "new_value");
    }
}
