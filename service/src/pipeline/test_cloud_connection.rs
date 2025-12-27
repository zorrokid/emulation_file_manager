use std::sync::Arc;

use cloud_storage::CloudStorageOps;

use crate::{
    error::Error,
    pipeline::pipeline_step::{PipelineStep, StepAction},
};

pub trait TestCloudConnectionContext {
    fn should_connect(&self) -> bool;
    fn cloud_ops(&self) -> Option<Arc<dyn CloudStorageOps>>;
}

pub struct TestConnectToCloudStep<T: TestCloudConnectionContext> {
    _phantom: std::marker::PhantomData<T>,
}

impl<T: TestCloudConnectionContext> Default for TestConnectToCloudStep<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: TestCloudConnectionContext> TestConnectToCloudStep<T> {
    pub fn new() -> Self {
        Self {
            _phantom: std::marker::PhantomData,
        }
    }
}

#[async_trait::async_trait]
impl<T: TestCloudConnectionContext + Send + Sync> PipelineStep<T> for TestConnectToCloudStep<T> {
    fn name(&self) -> &'static str {
        "test_connect_to_cloud"
    }

    fn should_execute(&self, context: &T) -> bool {
        context.should_connect() && context.cloud_ops().is_some()
    }

    async fn execute(&self, context: &mut T) -> StepAction {
        let res = context.cloud_ops().unwrap().test_connection().await;

        match res {
            Ok(_) => StepAction::Continue,
            Err(e) => StepAction::Abort(Error::CloudSyncError(format!(
                "Failed to connect to cloud storage: {}",
                e
            ))),
        }
    }
}
