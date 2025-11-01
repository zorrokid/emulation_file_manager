use std::sync::Arc;

use cloud_storage::{CloudStorageOps, S3CloudStorage};

use crate::{error::Error, pipeline::{PipelineStep, StepAction}, settings_service::SettingsService, view_models::Settings};

/// A trait for contexts that support cloud connection.
/// 
/// This trait should be implemented by any pipeline context that needs to
/// connect to cloud storage. It provides access to the settings, credentials
/// service, and a place to store the cloud operations instance.
pub trait CloudConnectionContext {
    /// Get the settings containing S3 configuration
    fn settings(&self) -> &Arc<Settings>;
    
    /// Get the settings service for loading credentials
    fn settings_service(&self) -> &Arc<SettingsService>;
    
    /// Get a mutable reference to the cloud_ops field
    fn cloud_ops_mut(&mut self) -> &mut Option<Arc<dyn CloudStorageOps>>;
    
    /// Check if cloud connection should be established
    /// 
    /// Override this method to provide context-specific logic for when
    /// to establish the cloud connection.
    fn should_connect(&self) -> bool {
        true
    }
}

/// A generic step for connecting to cloud storage.
/// 
/// This step can be used in any pipeline where the context implements
/// `CloudConnectionContext`. It handles:
/// - Loading S3 settings from the context
/// - Loading credentials from the settings service
/// - Establishing the connection
/// - Storing the cloud operations instance in the context
/// 
/// # Type Parameters
/// 
/// * `T` - The context type that implements `CloudConnectionContext`
pub struct ConnectToCloudStep<T: CloudConnectionContext> {
    _phantom: std::marker::PhantomData<T>,
}

impl<T: CloudConnectionContext> ConnectToCloudStep<T> {
    pub fn new() -> Self {
        Self {
            _phantom: std::marker::PhantomData,
        }
    }
}

impl<T: CloudConnectionContext> Default for ConnectToCloudStep<T> {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl<T: CloudConnectionContext + Send + Sync> PipelineStep<T> for ConnectToCloudStep<T> {
    fn name(&self) -> &'static str {
        "connect_to_cloud"
    }

    fn should_execute(&self, context: &T) -> bool {
        context.should_connect()
    }

    async fn execute(&self, context: &mut T) -> StepAction {
        let s3_settings = match context.settings().s3_settings.clone() {
            Some(settings) => settings,
            None => {
                eprintln!("S3 settings are not configured.");
                return StepAction::Abort(Error::SettingsError("S3 settings missing".to_string()));
            }
        };
        
        let credentials = match context.settings_service().load_credentials().await {
            Ok(Some(creds)) => creds,
            Ok(None) => {
                eprintln!("No S3 credentials found in keyring or environment.");
                return StepAction::Abort(Error::SettingsError(
                    "S3 credentials not found".to_string(),
                ));
            }
            Err(e) => {
                eprintln!("Error retrieving S3 credentials: {}", e);
                return StepAction::Abort(Error::SettingsError(format!(
                    "Failed to get S3 credentials: {}",
                    e
                )));
            }
        };
        
        let cloud_ops_res = S3CloudStorage::connect(
            s3_settings.endpoint.as_str(),
            s3_settings.region.as_str(),
            s3_settings.bucket.as_str(),
            credentials.access_key_id.as_str(),
            credentials.secret_access_key.as_str(),
        )
        .await;

        match cloud_ops_res {
            Ok(cloud_ops) => {
                *context.cloud_ops_mut() = Some(Arc::new(cloud_ops));
                StepAction::Continue
            }
            Err(e) => {
                eprintln!("Error connecting to S3: {}", e);
                StepAction::Abort(Error::CloudSyncError(format!(
                    "Failed to connect to S3: {}",
                    e
                )))
            }
        }
    }
}
