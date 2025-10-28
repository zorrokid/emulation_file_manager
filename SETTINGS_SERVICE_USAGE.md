# SettingsService Usage Guide

The `SettingsService` provides a unified interface for managing application settings and S3 credentials.

## Integration in UI (relm4-ui/src/settings_form.rs)

### 1. Import the service

```rust
use service::settings_service::SettingsService;
```

### 2. Add service to the component model

```rust
pub struct SettingsForm {
    pub repository_manager: Arc<RepositoryManager>,
    pub settings: Arc<Settings>,
    pub settings_service: SettingsService,  // Add this
    
    // ... other fields
}
```

### 3. Initialize the service

```rust
fn init(
    init: Self::Init,
    root: Self::Root,
    sender: ComponentSender<Self>,
) -> ComponentParts<Self> {
    let s3_settings = init.settings.s3_settings.clone().unwrap_or_default();
    
    // Create settings service
    let settings_service = SettingsService::new(init.repository_manager.clone());
    
    let model = Self {
        s3_bucket_name: s3_settings.bucket.clone(),
        s3_endpoint: s3_settings.endpoint.clone(),
        s3_region: s3_settings.region.clone(),
        s3_access_key: String::new(),  // Don't load credentials in UI for security
        s3_secret_key: String::new(),
        s3_sync_enabled: init.settings.s3_sync_enabled,
        repository_manager: init.repository_manager,
        settings: init.settings,
        settings_service,  // Add this
    };
    
    let widgets = view_output!();
    ComponentParts { model, widgets }
}
```

### 4. Save settings using the service

```rust
SettingsFormMsg::Submit => {
    let settings_service = self.settings_service.clone();
    let endpoint = self.s3_endpoint.clone();
    let region = self.s3_region.clone();
    let bucket = self.s3_bucket_name.clone();
    let sync_enabled = self.s3_sync_enabled;
    let access_key = self.s3_access_key.clone();
    let secret_key = self.s3_secret_key.clone();
    
    sender.oneshot_command(async move {
        // Save everything in one call
        if let Err(e) = settings_service
            .save_s3_settings(
                endpoint,
                region,
                bucket,
                sync_enabled,
                access_key,
                secret_key,
            )
            .await
        {
            eprintln!("Error saving settings: {}", e);
        }
        
        SettingsFormCommandMsg::SettingsSaved
    });
}
```

## Using Credentials in Cloud Sync Service

In `cloud_sync/service.rs` or wherever you need credentials:

```rust
use service::settings_service::SettingsService;

pub struct CloudSyncService {
    repository_manager: Arc<RepositoryManager>,
    settings: Arc<Settings>,
    settings_service: SettingsService,
}

impl CloudSyncService {
    pub fn new(
        repository_manager: Arc<RepositoryManager>,
        settings: Arc<Settings>,
    ) -> Self {
        let settings_service = SettingsService::new(repository_manager.clone());
        Self {
            repository_manager,
            settings,
            settings_service,
        }
    }
    
    pub async fn sync(&self) -> Result<(), Error> {
        // Load credentials (tries keyring first, falls back to env vars)
        let credentials = match self.settings_service.load_credentials_for_sync().await? {
            Some(creds) => creds,
            None => {
                return Err(Error::SettingsError(
                    "No S3 credentials available".to_string()
                ));
            }
        };
        
        // Use credentials for S3 connection
        // Note: You might need to add a method to S3CloudStorage to accept credentials directly
        // For now, credentials are set via environment variables in load_credentials_with_fallback
        
        // ... rest of sync logic
        Ok(())
    }
}
```

## Benefits of this approach

1. **Separation of concerns**: UI doesn't know about keyring implementation
2. **Centralized logic**: All settings operations in one place
3. **Secure by default**: Credentials stored in system keyring
4. **Fallback support**: Automatically falls back to environment variables
5. **Easy testing**: Service can be tested independently
6. **Clean API**: Simple methods for common operations

## Security Notes

- Credentials are stored in the system keyring (Keychain on macOS, Credential Manager on Windows, Secret Service on Linux)
- UI form fields for credentials should use `gtk::PasswordEntry` or similar for password masking
- Credentials are never logged or displayed in plain text
- Empty credential fields on save will delete stored credentials
- The service gracefully handles missing credentials (returns None instead of error)
