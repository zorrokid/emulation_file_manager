use std::{collections::HashMap, path::PathBuf, sync::Arc};

use core_types::SettingName;
use credentials_storage::{CloudCredentials, CredentialsError};
use database::repository_manager::RepositoryManager;

use crate::{error::Error, view_models::Settings};

pub struct SettingsSaveModel {
    pub endpoint: String,
    pub region: String,
    pub bucket: String,
    pub sync_enabled: bool,
    pub access_key_id: String,
    pub secret_access_key: String,
    pub collection_root_dir: Option<PathBuf>,
}

/// Service for managing application settings including settings stored to database and secure credentials stored in system keyring.
///
/// This service provides a unified interface for:
/// - Saving/loading settings from the database
/// - Storing/retrieving S3 credentials securely in the system keyring
#[derive(Debug)]
pub struct SettingsService {
    repository_manager: Arc<RepositoryManager>,
}

impl SettingsService {
    pub fn new(repository_manager: Arc<RepositoryManager>) -> Self {
        Self { repository_manager }
    }

    /// Save S3 settings to the database and optionally store credentials in the keyring.
    ///
    /// This method handles both database settings and secure credential storage:
    /// - Database settings (endpoint, region, bucket, sync enabled) are always saved to the database
    /// - If credentials are provided (both access_key_id and secret_access_key are non-empty),
    ///   they are stored securely in the system keyring
    /// - If credentials are empty, they are left unchanged in the keyring (use `delete_credentials()` to remove them)
    ///
    /// # Arguments
    ///
    /// * `settings` - Settings to save (includes both database settings and optional credentials)
    ///
    /// # Errors
    ///
    /// Returns an error if database operations fail. Credential storage errors are logged but
    /// don't cause the overall operation to fail, as the database settings can still be used
    /// with environment variable fallback for credentials.
    pub async fn save_settings(&self, settings: SettingsSaveModel) -> Result<(), Error> {
        let mut settings_map = HashMap::from([
            (SettingName::S3Bucket, settings.bucket),
            (SettingName::S3EndPoint, settings.endpoint),
            (SettingName::S3Region, settings.region),
            (
                SettingName::S3FileSyncEnabled,
                if settings.sync_enabled {
                    "true".to_string()
                } else {
                    "false".to_string()
                },
            ),
        ]);

        if let Some(collection_root_dir) = settings.collection_root_dir {
            settings_map.insert(
                SettingName::CollectionRootDir,
                // TODO: maybe consider some other option to store path instead of lossy string
                // (e.g. base64 encoded bytes)
                collection_root_dir.to_string_lossy().to_string(),
            );
        }

        // Save database settings first
        self.repository_manager
            .get_settings_repository()
            .add_or_update_settings(&settings_map)
            .await
            .map_err(|e| Error::DbError(format!("Failed to save settings: {}", e)))?;

        // Store credentials only if both are provided and non-empty
        if !settings.access_key_id.is_empty() && !settings.secret_access_key.is_empty() {
            let creds = CloudCredentials {
                access_key_id: settings.access_key_id.clone(),
                secret_access_key: settings.secret_access_key.clone(),
            };

            if let Err(e) = credentials_storage::store_credentials(&creds) {
                // Log error but don't fail - credentials can be provided via env vars
                eprintln!("Warning: Failed to store credentials in keyring: {}", e);
            }
        }
        // If credentials are empty, we leave existing keyring credentials unchanged
        // Use delete_credentials() to explicitly remove them

        Ok(())
    }

    /// Load settings from database.
    ///
    /// # Returns
    ///
    /// Returns a `Settings` object with application settings populated from the database.
    ///
    /// # Errors
    ///
    /// Returns an error if database operations fail.
    pub async fn load_settings(&self) -> Result<Settings, Error> {
        let settings_map = self
            .repository_manager
            .get_settings_repository()
            .get_settings()
            .await
            .map_err(|e| Error::DbError(format!("Failed to load settings: {}", e)))?;

        Ok(Settings::from(settings_map))
    }

    /// Load S3 credentials from keyring with fallback to environment variables.
    ///
    /// This method tries the keyring first, then falls back to environment variables
    /// (AWS_ACCESS_KEY_ID, AWS_SECRET_ACCESS_KEY). It wraps the credentials_storage
    /// functions and provides a consistent error handling pattern for the service layer.
    ///
    /// # Returns
    ///
    /// Returns `Ok(Some(credentials))` if credentials are found in keyring or environment.
    /// Returns `Ok(None)` if no credentials are available.
    /// Returns `Err` only for unexpected keyring errors (not for missing credentials).
    ///
    /// # Example
    ///
    /// ```ignore
    /// match settings_service.load_credentials_for_sync().await? {
    ///     Some(creds) => {
    ///         // Use credentials for S3 connection
    ///         S3CloudStorage::connect_with_credentials(&creds, endpoint, region, bucket).await?
    ///     }
    ///     None => {
    ///         // No credentials available
    ///         eprintln!("No S3 credentials found");
    ///     }
    /// }
    /// ```
    pub async fn load_credentials(&self) -> Result<Option<CloudCredentials>, Error> {
        match credentials_storage::load_credentials_with_fallback() {
            Ok(creds) => Ok(Some(creds)),
            Err(CredentialsError::NoCredentials) => Ok(None),
            Err(e) => Err(Error::SettingsError(format!(
                "Failed to load credentials: {}",
                e
            ))),
        }
    }

    /// Check if S3 credentials are available in keyring or environment variables.
    ///
    /// This is useful for determining whether sync operations can proceed.
    ///
    /// # Returns
    ///
    /// Returns `true` if credentials are available, `false` otherwise.
    pub async fn has_credentials(&self) -> bool {
        self.load_credentials().await.unwrap_or(None).is_some()
    }

    /// Delete credentials from the system keyring.
    ///
    /// This removes any stored credentials from the keyring. After calling this,
    /// credentials will only be available via environment variables.
    ///
    /// # Errors
    ///
    /// Returns an error if the keyring is not accessible. Does not error if
    /// credentials don't exist (operation is idempotent).
    pub async fn delete_credentials(&self) -> Result<(), Error> {
        credentials_storage::delete_credentials()
            .map_err(|e| Error::SettingsError(format!("Failed to delete credentials: {}", e)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use database::setup_test_db;

    #[async_std::test]
    async fn test_save_and_load_settings() {
        // Clean up any test credentials before starting
        credentials_storage::delete_credentials().ok();

        let pool = Arc::new(setup_test_db().await);
        let repo_manager = Arc::new(RepositoryManager::new(pool));
        let service = SettingsService::new(repo_manager);

        let save_model = SettingsSaveModel {
            endpoint: "s3.example.com".to_string(),
            region: "us-east-1".to_string(),
            bucket: "my-bucket".to_string(),
            sync_enabled: true,
            access_key_id: "test-access-key".to_string(),
            secret_access_key: "test-secret-key".to_string(),
            collection_root_dir: Some(PathBuf::from("/data/collections")),
        };

        // Save settings
        let result = service.save_settings(save_model).await;

        assert!(result.is_ok());

        // Load settings
        let settings = service.load_settings().await.unwrap();
        assert!(settings.s3_sync_enabled);
        assert_eq!(
            settings.s3_settings.as_ref().unwrap().endpoint,
            "s3.example.com"
        );
        assert_eq!(settings.s3_settings.as_ref().unwrap().region, "us-east-1");
        assert_eq!(settings.s3_settings.as_ref().unwrap().bucket, "my-bucket");
        assert_eq!(
            settings.collection_root_dir.to_string_lossy(),
            "/data/collections"
        );

        // Clean up test credentials after test
        credentials_storage::delete_credentials().ok();
    }

    #[async_std::test]
    async fn test_load_settings_empty() {
        let pool = Arc::new(setup_test_db().await);
        let repo_manager = Arc::new(RepositoryManager::new(pool));
        let service = SettingsService::new(repo_manager);

        // Should work even with no settings
        let settings = service.load_settings().await.unwrap();
        assert!(!settings.s3_sync_enabled);
        assert!(settings.s3_settings.is_none());
    }
}
