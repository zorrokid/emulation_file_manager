use crate::error::Error;

pub struct Pipeline<T> {
    pub steps: Vec<Box<dyn SyncStep<T>>>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum StepAction {
    /// Continue to the next step
    Continue,
    /// Skip all remaining steps (successful early exit)
    Skip,
    /// Abort the pipeline with an error
    Abort(Error),
}

#[async_trait::async_trait]
pub trait SyncStep<T>: Send + Sync {
    fn name(&self) -> &'static str;

    /// Determines if this step should execute based on current context
    fn should_execute(&self, _context: &T) -> bool {
        true // By default, always execute
    }

    /// Execute the step, modifying the context and returning the next action
    async fn execute(&self, context: &mut T) -> StepAction;
}
