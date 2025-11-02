use crate::error::Error;

/// The action to take after a step completes.
///
/// Steps return this enum to control pipeline flow:
/// - `Continue`: Proceed to the next step normally
/// - `Skip`: Successfully exit early without running remaining steps
/// - `Abort`: Stop the pipeline with an error
#[derive(Debug, Clone, PartialEq)]
pub enum StepAction {
    /// Continue to the next step
    Continue,
    /// Skip all remaining steps (successful early exit)
    Skip,
    /// Abort the pipeline with an error
    Abort(Error),
}

/// A trait for defining pipeline steps.
///
/// Each step receives a mutable reference to the context and can:
/// - Read from the context to access shared state and dependencies
/// - Modify the context to store results or update state
/// - Return a `StepAction` to control pipeline execution flow
///
/// # Type Parameters
///
/// * `T` - The context type that contains all state and dependencies needed by steps
///
/// # Example
///
/// ```ignore
/// struct ValidateInputStep;
///
/// #[async_trait::async_trait]
/// impl SyncStep<MyContext> for ValidateInputStep {
///     fn name(&self) -> &'static str {
///         "validate_input"
///     }
///
///     fn should_execute(&self, context: &MyContext) -> bool {
///         !context.input.is_empty()
///     }
///
///     async fn execute(&self, context: &mut MyContext) -> StepAction {
///         if context.input.len() > 1000 {
///             return StepAction::Abort(Error::ValidationError("Input too large".into()));
///         }
///         StepAction::Continue
///     }
/// }
/// ```
#[async_trait::async_trait]
pub trait PipelineStep<T>: Send + Sync {
    /// Returns the name of this step for logging and debugging.
    fn name(&self) -> &'static str;

    /// Determines if this step should execute based on current context.
    ///
    /// This is called before `execute()` and allows steps to conditionally
    /// skip themselves based on the context state. Steps that return `false`
    /// will be skipped without affecting the pipeline flow.
    ///
    /// # Arguments
    ///
    /// * `context` - Read-only reference to the context
    ///
    /// # Returns
    ///
    /// `true` if the step should execute, `false` to skip this step
    ///
    /// # Default Implementation
    ///
    /// By default, always returns `true` (always execute).
    fn should_execute(&self, _context: &T) -> bool {
        true // By default, always execute
    }

    /// Execute the step, modifying the context and returning the next action.
    ///
    /// This is the core logic of the step. It can:
    /// - Read from and modify the context
    /// - Perform async operations (database queries, API calls, etc.)
    /// - Return `Continue` to proceed to the next step
    /// - Return `Skip` to successfully exit without running remaining steps
    /// - Return `Abort(error)` to stop the pipeline with an error
    ///
    /// # Arguments
    ///
    /// * `context` - Mutable reference to the context
    ///
    /// # Returns
    ///
    /// A `StepAction` indicating what the pipeline should do next
    async fn execute(&self, context: &mut T) -> StepAction;
}
