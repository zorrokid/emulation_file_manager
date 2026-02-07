# Understanding Trait Objects and Dynamic Dispatch

## The Problem: Storing Different Types Together

In the `DeletionPipeline`, we have multiple steps that all implement `DeletionStep<F>`:

```rust
struct ValidateNotInUseStep;
struct FetchFileInfosStep;
struct DeleteFileSetStep;
struct FilterDeletableFilesStep;
struct MarkForCloudDeletionStep;
struct DeleteLocalFilesStep;
```

Each is a **different concrete type**. You can't just put them in a `Vec` together:

```rust
// ❌ This doesn't work!
let steps = vec![
    ValidateNotInUseStep,      // Type: ValidateNotInUseStep
    FetchFileInfosStep,        // Type: FetchFileInfosStep
    DeleteFileSetStep,         // Type: DeleteFileSetStep
];
// Error: Vec needs all elements to be the same type!
```

## Solution: Trait Objects with `Box<dyn Trait>`

```rust
pub struct DeletionPipeline<F: FileSystemOps> {
    steps: Vec<Box<dyn DeletionStep<F>>>,
    //          ^^^  ^^^              
    //          |    |                
    //          |    Any type implementing DeletionStep<F>
    //          Heap-allocated pointer (fixed size)
}
```

### Breaking Down the Components

#### 1. `dyn DeletionStep<F>` - Dynamic Dispatch

- `dyn` means "dynamic dispatch" - the actual concrete type is determined at runtime
- `DeletionStep<F>` is the trait
- Together: `dyn DeletionStep<F>` means "any type that implements `DeletionStep<F>`"
- This is called a **trait object**

#### 2. `Box<...>` - Heap Allocation

- Trait objects like `dyn DeletionStep<F>` have **unknown size** at compile time
- Rust needs to know the size of things stored in a `Vec`
- `Box` puts the data on the heap and stores a pointer (which has known size)
- `Box<dyn DeletionStep<F>>` is a pointer to heap-allocated data implementing `DeletionStep<F>`
- On 64-bit systems, a `Box` pointer is always 8 bytes

#### 3. `Vec<Box<dyn ...>>` - Storing Different Types

Now you can store different step types in the same vector:

```rust
impl<F: FileSystemOps> DeletionPipeline<F> {
    pub fn new() -> Self {
        Self {
            steps: vec![
                Box::new(ValidateNotInUseStep),      // Box<ValidateNotInUseStep>
                Box::new(FetchFileInfosStep),        // Box<FetchFileInfosStep>
                Box::new(DeleteFileSetStep),         // Box<DeleteFileSetStep>
                Box::new(FilterDeletableFilesStep),  // Box<FilterDeletableFilesStep>
                Box::new(MarkForCloudDeletionStep),  // Box<MarkForCloudDeletionStep>
                Box::new(DeleteLocalFilesStep),      // Box<DeleteLocalFilesStep>
                // All stored as Box<dyn DeletionStep<F>> ✓
            ],
        }
    }
}
```

## Visualizing the Memory Layout

```
Stack (Pipeline):
┌─────────────────────┐
│ DeletionPipeline    │
│ ┌─────────────────┐ │
│ │ steps: Vec      │ │
│ │  ┌──┐ ┌──┐ ┌──┐│ │
│ │  │ptr│ │ptr│ │ptr││ │  <- Pointers (fixed size: 8 bytes each on 64-bit)
│ │  └─┬┘ └─┬┘ └─┬┘│ │
│ └────┼────┼────┼──┘ │
└──────┼────┼────┼────┘
       │    │    │
       ▼    ▼    ▼
Heap:
┌─────────────────────┐  ┌─────────────────────┐  ┌─────────────────────┐
│ ValidateNotInUseStep│  │ FetchFileInfosStep  │  │ DeleteFileSetStep   │
│ + vtable pointer    │  │ + vtable pointer    │  │ + vtable pointer    │
│ (actual step data)  │  │ (actual step data)  │  │ (actual step data)  │
└─────────────────────┘  └─────────────────────┘  └─────────────────────┘
```

## How Dynamic Dispatch Works

When you call a method on a trait object:

```rust
for step in &self.steps {
    step.execute(context).await;  // Which execute()?
}
```

Rust uses a **vtable (virtual function table)** to determine which implementation to call:

```
step (Box<dyn DeletionStep<F>>)
  │
  ├─> Data pointer ──────────> [Actual step data on heap]
  │
  └─> Vtable pointer ────────> [Function pointers]
                                 ├─> execute()
                                 ├─> name()
                                 └─> should_execute()
```

**At runtime:**
1. Dereference the vtable pointer
2. Look up the function pointer for `execute()`
3. Call that function with the data pointer

This is slightly slower than static dispatch (direct function call), but enables runtime polymorphism.

## Alternative: Enum (Static Dispatch)

You *could* use an enum instead:

```rust
enum Step<F: FileSystemOps> {
    ValidateNotInUse(ValidateNotInUseStep),
    FetchFileInfos(FetchFileInfosStep),
    DeleteFileSet(DeleteFileSetStep),
    FilterDeletableFiles(FilterDeletableFilesStep),
    MarkForCloudDeletion(MarkForCloudDeletionStep),
    DeleteLocalFiles(DeleteLocalFilesStep),
}

pub struct DeletionPipeline<F: FileSystemOps> {
    steps: Vec<Step<F>>,  // No Box/dyn needed
}

impl<F: FileSystemOps> DeletionPipeline<F> {
    pub async fn execute(&self, context: &mut DeletionContext<F>) -> Result<(), Error> {
        for step in &self.steps {
            match step {
                Step::ValidateNotInUse(s) => s.execute(context).await?,
                Step::FetchFileInfos(s) => s.execute(context).await?,
                Step::DeleteFileSet(s) => s.execute(context).await?,
                Step::FilterDeletableFiles(s) => s.execute(context).await?,
                Step::MarkForCloudDeletion(s) => s.execute(context).await?,
                Step::DeleteLocalFiles(s) => s.execute(context).await?,
            }
        }
        Ok(())
    }
}
```

### Comparison: Enum vs Trait Objects

| Aspect | Enum (Static) | Trait Objects (Dynamic) |
|--------|--------------|------------------------|
| **Performance** | Faster (no vtable lookup) | Slightly slower (vtable lookup) |
| **Extensibility** | Must update enum for new steps | Just `Box::new(NewStep)` |
| **Boilerplate** | Match statement needed everywhere | Clean trait method calls |
| **Compile-time safety** | All cases checked at compile time | Type checked but dispatch at runtime |
| **Binary size** | Larger (code duplication from monomorphization) | Smaller (one implementation) |
| **Flexibility** | Can't add steps at runtime | Could potentially load steps dynamically |

**For our pipeline, trait objects win because:**
- ✅ Steps are coarse-grained (do lots of work) - slight overhead doesn't matter
- ✅ Clean, extensible design - easy to add new steps
- ✅ Less boilerplate - no giant match statements
- ✅ The flexibility is worth the tiny performance cost

## Why `Send + Sync` in the Trait?

```rust
pub trait DeletionStep<F: FileSystemOps>: Send + Sync {
    //                                     ^^^^^^^^^^^
    //                                     Required for trait objects
    fn name(&self) -> &'static str;
    fn should_execute(&self, context: &DeletionContext<F>) -> bool { true }
    async fn execute(&self, context: &mut DeletionContext<F>) -> StepAction;
}
```

These trait bounds are required because:

### `Send` - Can be moved between threads

- `Box<dyn DeletionStep<F>>` might need to move across thread boundaries
- Without `Send`, you couldn't do:
  ```rust
  let pipeline = Arc::new(DeletionPipeline::new());
  tokio::spawn(async move {
      pipeline.execute(&mut context).await  // Moved to another thread
  });
  ```

### `Sync` - Can be shared between threads

- Multiple threads can hold references (`&`) to the same step
- Without `Sync`, you couldn't do:
  ```rust
  let pipeline = Arc::new(DeletionPipeline::new());  // Arc requires Sync
  let pipeline_clone = pipeline.clone();
  tokio::spawn(async move {
      pipeline_clone.execute(&mut context).await  // Shared across threads
  });
  ```

### Why It Matters for Async

In async Rust:
- Tasks can migrate between threads (on multi-threaded runtimes)
- `Box<dyn Trait>` without `Send + Sync` isn't `Send`, so can't cross await points in async functions
- The compiler enforces this to prevent data races

**Without `Send + Sync`:**
```rust
// ❌ This would fail to compile
async fn do_deletion() {
    let pipeline = DeletionPipeline::new();  // Not Send
    pipeline.execute(&mut context).await;    // ERROR: can't send across threads
}
```

**With `Send + Sync`:**
```rust
// ✅ This works
async fn do_deletion() {
    let pipeline = DeletionPipeline::new();  // Send + Sync
    pipeline.execute(&mut context).await;    // OK!
}
```

## Performance Considerations

### Dynamic Dispatch Cost

**Per method call:**
- Extra pointer dereference to access vtable
- Function pointer lookup in vtable
- Can't be inlined by compiler
- Estimated overhead: ~5-10 nanoseconds

**Is this significant?**

For our pipeline: **No!** Here's why:

```rust
// Each step does heavy work:
async fn execute(&self, context: &mut DeletionContext<F>) -> StepAction {
    // Database queries (milliseconds)
    let file_infos = context.repository_manager
        .get_file_info_repository()
        .get_file_infos_by_file_set(context.file_set_id)  // ~1-100ms
        .await?;
    
    // File I/O (milliseconds)
    context.fs_ops.remove_file(&path)?;  // ~1-50ms
    
    // Network operations (seconds)
    upload_to_cloud(&file).await?;  // ~100-10000ms
}
```

The vtable lookup (~0.00001ms) is **completely negligible** compared to the actual work!

### When Dynamic Dispatch Matters

Dynamic dispatch overhead is only significant for:
- Hot loops with millions of iterations
- Performance-critical numerical computations
- Real-time systems with strict latency requirements

For our use case (file operations, database queries, network calls), the clean design is far more valuable than the unmeasurable performance difference.

## Summary

| Concept | Purpose | Example |
|---------|---------|---------|
| `dyn Trait` | Runtime polymorphism | "Any type implementing this trait" |
| `Box<...>` | Heap allocation with fixed-size pointer | Store dynamically-sized data |
| `Box<dyn Trait>` | Trait object | Store different concrete types together |
| `Vec<Box<dyn Trait>>` | Collection of different types | Pipeline with heterogeneous steps |
| `Send` | Can move between threads | Required for async tasks |
| `Sync` | Can share between threads | Required for `Arc<T>` |

## Adding a New Step

With trait objects, adding a new step is trivial:

```rust
// 1. Define the step
struct MyNewStep;

// 2. Implement the trait
#[async_trait]
impl<F: FileSystemOps> DeletionStep<F> for MyNewStep {
    fn name(&self) -> &'static str {
        "my_new_step"
    }
    
    async fn execute(&self, context: &mut DeletionContext<F>) -> StepAction {
        // Implementation
        StepAction::Continue
    }
}

// 3. Add to pipeline
impl<F: FileSystemOps> DeletionPipeline<F> {
    pub fn new() -> Self {
        Self {
            steps: vec![
                Box::new(ValidateNotInUseStep),
                Box::new(FetchFileInfosStep),
                Box::new(MyNewStep),           // ← Just add it here!
                Box::new(DeleteFileSetStep),
                // ...
            ],
        }
    }
}
```

**Done!** No enums to update, no match statements to modify. That's the power of trait objects! 🎯

## Further Reading

- [The Rust Book - Trait Objects](https://doc.rust-lang.org/book/ch17-02-trait-objects.html)
- [Rust Reference - Trait Objects](https://doc.rust-lang.org/reference/types/trait-object.html)
- [Send and Sync](https://doc.rust-lang.org/nomicon/send-and-sync.html)
