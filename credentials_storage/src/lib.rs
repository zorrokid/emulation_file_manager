use keyring::Entry;
use serde::{Deserialize, Serialize};

#[cfg(not(test))]
const SERVICE_NAME: &str = "efm-cloud-sync";
const USERNAME: &str = "s3-credentials"; // Fixed username for all credentials

// Test-specific service name to avoid polluting production keyring
#[cfg(test)]
const TEST_SERVICE_NAME: &str = "efm-cloud-sync-test";

/// Get the appropriate service name based on build configuration
fn get_service_name() -> &'static str {
    #[cfg(test)]
    {
        TEST_SERVICE_NAME
    }
    #[cfg(not(test))]
    {
        SERVICE_NAME
    }
}

/// Cloud storage credentials (S3-compatible)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CloudCredentials {
    pub access_key_id: String,
    pub secret_access_key: String,
}

/// Errors that can occur when working with credentials
#[derive(Debug, thiserror::Error)]
pub enum CredentialsError {
    #[error("Keyring error: {0}")]
    Keyring(#[from] keyring::Error),

    #[error("No credentials stored")]
    NoCredentials,

    #[error("Failed to serialize/deserialize credentials: {0}")]
    Serialization(#[from] serde_json::Error),
}

/// Store cloud sync credentials securely in the system keyring.
///
/// The credentials are stored as JSON in the system's native credential store
///
/// # Arguments
///
/// * `credentials` - The credentials to store
///
/// # Errors
///
/// Returns an error if the keyring is not accessible or the credentials cannot be serialized.
///
/// # Example
///
/// ```ignore
/// let creds = CloudCredentials {
///     access_key_id: "my-key-id".to_string(),
///     secret_access_key: "my-secret".to_string(),
/// };
/// store_credentials(&creds)?;
/// ```
pub fn store_credentials(credentials: &CloudCredentials) -> Result<(), CredentialsError> {
    let entry = Entry::new(get_service_name(), USERNAME)?;
    let json = serde_json::to_string(credentials)?;
    entry.set_password(&json)?;
    Ok(())
}

/// Load cloud sync credentials from the system keyring.
///
/// # Errors
///
/// Returns `CredentialsError::NoCredentials` if no credentials are stored.
/// Returns other errors if the keyring is not accessible or credentials are corrupted.
///
/// # Example
///
/// ```ignore
/// match load_credentials() {
///     Ok(creds) => println!("Loaded credentials for: {}", creds.access_key_id),
///     Err(CredentialsError::NoCredentials) => println!("No credentials stored"),
///     Err(e) => eprintln!("Error: {}", e),
/// }
/// ```
pub fn load_credentials() -> Result<CloudCredentials, CredentialsError> {
    let entry = Entry::new(get_service_name(), USERNAME)?;
    match entry.get_password() {
        Ok(json) => {
            let credentials = serde_json::from_str(&json)?;
            Ok(credentials)
        }
        Err(keyring::Error::NoEntry) => Err(CredentialsError::NoCredentials),
        Err(e) => Err(CredentialsError::Keyring(e)),
    }
}

/// Load credentials from keyring, falling back to environment variables if not found.
///
/// This method first tries to load credentials from the system keyring. If no credentials
/// are found there, it falls back to the AWS standard environment variables:
/// - `AWS_ACCESS_KEY_ID`
/// - `AWS_SECRET_ACCESS_KEY`
///
/// This provides backward compatibility and allows for different deployment scenarios
/// (e.g., containerized environments where environment variables are preferred).
///
/// # Errors
///
/// Returns `CredentialsError::NoCredentials` if credentials are not found in either
/// the keyring or environment variables.
///
/// # Example
///
/// ```ignore
/// // Will use keyring if available, otherwise environment variables
/// let creds = load_credentials_with_fallback()?;
/// ```
pub fn load_credentials_with_fallback() -> Result<CloudCredentials, CredentialsError> {
    // Try keyring first
    match load_credentials() {
        Ok(creds) => Ok(creds),
        Err(CredentialsError::NoCredentials) => {
            // Fall back to environment variables
            let access_key = std::env::var("AWS_ACCESS_KEY_ID").ok();
            let secret_key = std::env::var("AWS_SECRET_ACCESS_KEY").ok();

            match (access_key, secret_key) {
                (Some(access_key_id), Some(secret_access_key)) => Ok(CloudCredentials {
                    access_key_id,
                    secret_access_key,
                }),
                _ => Err(CredentialsError::NoCredentials),
            }
        }
        Err(e) => Err(e),
    }
}

/// Delete cloud sync credentials from the system keyring.
///
/// This operation is idempotent - deleting already-deleted credentials succeeds.
///
/// # Errors
///
/// Returns an error if the keyring is not accessible (but not if credentials don't exist).
///
/// # Example
///
/// ```ignore
/// delete_credentials()?;
/// println!("Credentials removed");
/// ```
pub fn delete_credentials() -> Result<(), CredentialsError> {
    let entry = Entry::new(get_service_name(), USERNAME)?;
    match entry.delete_credential() {
        Ok(_) => Ok(()),
        Err(keyring::Error::NoEntry) => Ok(()), // Already deleted
        Err(e) => Err(CredentialsError::Keyring(e)),
    }
}

/// Check if credentials are currently stored in the keyring.
///
/// Note: This does not check environment variables. Use `load_credentials_with_fallback()`
/// if you want to check all sources.
///
/// # Errors
///
/// Returns an error if the keyring is not accessible.
///
/// # Example
///
/// ```ignore
/// if has_credentials()? {
///     println!("Credentials are stored");
/// } else {
///     println!("No credentials found");
/// }
/// ```
pub fn has_credentials() -> Result<bool, CredentialsError> {
    match load_credentials() {
        Ok(_) => Ok(true),
        Err(CredentialsError::NoCredentials) => Ok(false),
        Err(e) => Err(e),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    // Tests that access the keyring must run sequentially because:
    // 1. All tests share the same test keyring entry ("efm-cloud-sync-test")
    // 2. Running in parallel causes race conditions where tests interfere with each other
    // 3. The #[serial] attribute ensures these tests run one at a time
    //
    // Note: test_credentials_serialization doesn't need #[serial] because it doesn't
    // access the keyring - it only tests JSON serialization.

    // Helper to clean up test credentials after each test
    fn cleanup_test_credentials() {
        delete_credentials().ok(); // Ignore errors - might not exist
    }

    // Helper to check if keyring service is available
    fn is_keyring_available() -> bool {
        let entry = match Entry::new(get_service_name(), USERNAME) {
            Ok(e) => e,
            Err(_) => return false,
        };
        
        // Try a simple operation - if it fails with platform/service error, keyring is unavailable
        match entry.get_password() {
            Ok(_) => true, // Password exists, service is available
            Err(keyring::Error::NoEntry) => true, // No password stored but service is available
            Err(keyring::Error::PlatformFailure(_)) => false, // Service unavailable
            Err(_) => true, // Other errors mean service is available but operation failed
        }
    }

    #[test]
    fn test_credentials_serialization() {
        let creds = CloudCredentials {
            access_key_id: "test-key-id".to_string(),
            secret_access_key: "test-secret".to_string(),
        };

        let json = serde_json::to_string(&creds).unwrap();
        let deserialized: CloudCredentials = serde_json::from_str(&json).unwrap();

        assert_eq!(creds, deserialized);
    }

    #[test]
    #[serial]
    fn test_store_and_load() {
        if !is_keyring_available() {
            eprintln!("Skipping test_store_and_load: keyring service not available");
            return;
        }

        cleanup_test_credentials();

        let creds = CloudCredentials {
            access_key_id: "test-store-key".to_string(),
            secret_access_key: "test-store-secret".to_string(),
        };

        // Store
        store_credentials(&creds).expect("Failed to store credentials");

        // Load
        let loaded = load_credentials().expect("Failed to load credentials");
        assert_eq!(creds, loaded);

        cleanup_test_credentials();
    }

    #[test]
    #[serial]
    fn test_delete() {
        if !is_keyring_available() {
            eprintln!("Skipping test_delete: keyring service not available");
            return;
        }

        cleanup_test_credentials();

        let creds = CloudCredentials {
            access_key_id: "test-delete-key".to_string(),
            secret_access_key: "test-delete-secret".to_string(),
        };

        store_credentials(&creds).unwrap();
        assert!(has_credentials().unwrap());

        delete_credentials().unwrap();
        assert!(!has_credentials().unwrap());

        cleanup_test_credentials();
    }
}
