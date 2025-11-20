use super::pipeline_step::{PipelineStep, StepAction};
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
            if !step.should_execute(context) {
                tracing::info!("Step {} will be skipped based on context", step.name());
                continue;
            }

            tracing::info!("Executing step: {}", step.name());

            match step.execute(context).await {
                StepAction::Continue => {
                    // Proceed to next step
                    continue;
                }
                StepAction::Skip => {
                    tracing::info!("Step {} requested skip - stopping pipeline", step.name());
                    return Ok(());
                }
                StepAction::Abort(error) => {
                    tracing::error!("Step {} aborted the pipeline: {}", step.name(), error);
                    return Err(error);
                }
            }
        }

        Ok(())
    }
}
