use std::{collections::HashMap, sync::Arc};

use core_types::SettingName;
use credentials_storage::{CloudCredentials, CredentialsError};
use database::repository_manager::RepositoryManager;

use crate::{error::Error, view_models::Settings};

/// Service for managing application settings including database settings and secure credentials.
///
/// This service provides a unified interface for:
/// - Saving/loading settings from the database
/// - Storing/retrieving S3 credentials securely in the system keyring
/// - Loading complete settings with credentials for use by other services
pub struct SettingsService {
    repository_manager: Arc<RepositoryManager>,
}

impl SettingsService {
    pub fn new(repository_manager: Arc<RepositoryManager>) -> Self {
        Self {
            repository_manager,
        }
    }

    /// Save S3 settings to the database and optionally store credentials in the keyring.
    ///
    /// This method handles both database settings and secure credential storage:
    /// - Database settings (endpoint, region, bucket, sync enabled) are saved to the database
    /// - If credentials are provided (both access_key_id and secret_access_key are non-empty),
    ///   they are stored securely in the system keyring
    /// - If both credential fields are empty, any existing credentials are deleted from the keyring
    ///
    /// # Arguments
    ///
    /// * `endpoint` - S3 endpoint URL (e.g., "s3.eu-central-003.backblazeb2.com")
    /// * `region` - S3 region (e.g., "eu-central-003")
    /// * `bucket` - S3 bucket name
    /// * `sync_enabled` - Whether S3 sync is enabled
    /// * `access_key_id` - Optional S3 access key ID (empty string to delete credentials)
    /// * `secret_access_key` - Optional S3 secret access key (empty string to delete credentials)
    ///
    /// # Errors
    ///
    /// Returns an error if database operations fail. Credential storage errors are logged but
    /// don't cause the overall operation to fail, as the database settings can still be used
    /// with environment variable fallback for credentials.
    pub async fn save_s3_settings(
        &self,
        endpoint: String,
        region: String,
        bucket: String,
        sync_enabled: bool,
        access_key_id: String,
        secret_access_key: String,
    ) -> Result<(), Error> {
        let settings_map = HashMap::from([
            (SettingName::S3Bucket, bucket),
            (SettingName::S3EndPoint, endpoint),
            (SettingName::S3Region, region),
            (
                SettingName::S3FileSyncEnabled,
                if sync_enabled {
                    "true".to_string()
                } else {
                    "false".to_string()
                },
            ),
        ]);

        // Save database settings first
        self.repository_manager
            .get_settings_repository()
            .add_or_update_settings(&settings_map)
            .await
            .map_err(|e| Error::DbError(format!("Failed to save settings: {}", e)))?;

        // Handle credentials storage
        if !access_key_id.is_empty() && !secret_access_key.is_empty() {
            // Store credentials if both are provided
            let creds = CloudCredentials {
                access_key_id,
                secret_access_key,
            };

            if let Err(e) = credentials_storage::store_credentials(&creds) {
                // Log error but don't fail - credentials can be provided via env vars
                eprintln!("Warning: Failed to store credentials in keyring: {}", e);
            }
        } else if access_key_id.is_empty() && secret_access_key.is_empty() {
            // Delete credentials if both are empty
            if let Err(e) = credentials_storage::delete_credentials() {
                eprintln!("Warning: Failed to delete credentials from keyring: {}", e);
            }
        }

        Ok(())
    }

    /// Load complete settings including credentials from keyring or environment variables.
    ///
    /// This method loads settings from the database and attempts to load credentials
    /// from the system keyring. If no credentials are found in the keyring, it falls
    /// back to AWS environment variables (AWS_ACCESS_KEY_ID, AWS_SECRET_ACCESS_KEY).
    ///
    /// # Returns
    ///
    /// Returns a `Settings` object with S3 settings populated from the database.
    /// Note: Credentials are not included in the Settings object - they should be
    /// loaded separately using `load_credentials_for_sync()` when needed.
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
    /// This is a convenience method for loading credentials when needed for sync operations.
    /// It tries the keyring first, then falls back to environment variables.
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
    pub async fn load_credentials_for_sync(&self) -> Result<Option<CloudCredentials>, Error> {
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
        self.load_credentials_for_sync().await.unwrap_or(None).is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use database::setup_test_db;

    #[async_std::test]
    async fn test_save_and_load_settings() {
        let pool = Arc::new(setup_test_db().await);
        let repo_manager = Arc::new(RepositoryManager::new(pool));
        let service = SettingsService::new(repo_manager);

        // Save settings
        let result = service
            .save_s3_settings(
                "s3.example.com".to_string(),
                "us-east-1".to_string(),
                "my-bucket".to_string(),
                true,
                String::new(), // Empty credentials to avoid keyring in tests
                String::new(),
            )
            .await;

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
