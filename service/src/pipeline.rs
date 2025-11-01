use crate::error::Error;

/// A generic pipeline that executes a series of steps in sequence.
///
/// The pipeline pattern provides a structured way to organize complex operations
/// into discrete, testable steps. Each step can decide whether to continue, skip
/// remaining steps, or abort the entire pipeline.
///
/// # Type Parameters
///
/// * `T` - The context type that will be passed through all steps. The context
///   typically contains shared state, dependencies, and results that steps need
///   to read or modify.
///
/// # Example
///
/// ```ignore
/// // Define your context
/// struct MyContext {
///     data: String,
///     results: Vec<String>,
/// }
///
/// // Implement steps
/// struct Step1;
/// #[async_trait::async_trait]
/// impl SyncStep<MyContext> for Step1 {
///     fn name(&self) -> &'static str { "step_1" }
///     async fn execute(&self, context: &mut MyContext) -> StepAction {
///         context.results.push("step1".to_string());
///         StepAction::Continue
///     }
/// }
///
/// // Create and execute pipeline
/// let pipeline = Pipeline::<MyContext> {
///     steps: vec![Box::new(Step1)],
/// };
/// let mut context = MyContext { data: String::new(), results: Vec::new() };
/// pipeline.execute(&mut context).await?;
/// ```
pub struct Pipeline<T> {
    pub steps: Vec<Box<dyn PipelineStep<T>>>,
}

impl<T> Pipeline<T> {
    /// Create a pipeline with the given steps.
    pub fn with_steps(steps: Vec<Box<dyn PipelineStep<T>>>) -> Self {
        Self { steps }
    }

    /// Execute all steps in the pipeline in sequence.
    ///
    /// Steps are executed in order, with each step's `should_execute()` check
    /// determining if it runs. The pipeline continues until all steps complete,
    /// a step returns `Skip`, or a step returns `Abort`.
    ///
    /// # Arguments
    ///
    /// * `context` - Mutable reference to the context that will be passed to all steps
    ///
    /// # Returns
    ///
    /// `Ok(())` if all steps complete successfully or a step returns `Skip`,
    /// `Err(error)` if a step returns `Abort(error)`
    pub async fn execute(&self, context: &mut T) -> Result<(), Error> {
        for step in &self.steps {
            // Check if step should execute
            if !step.should_execute(context) {
                eprintln!("Skipping step: {}", step.name());
                continue;
            }

            eprintln!("Executing step: {}", step.name());

            match step.execute(context).await {
                StepAction::Continue => {
                    // Proceed to next step
                    continue;
                }
                StepAction::Skip => {
                    // Early successful exit
                    eprintln!("Step {} requested skip - stopping pipeline", step.name());
                    return Ok(());
                }
                StepAction::Abort(error) => {
                    // Error exit
                    eprintln!("Step {} requested abort - stopping pipeline", step.name());
                    return Err(error);
                }
            }
        }

        Ok(())
    }
}

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
