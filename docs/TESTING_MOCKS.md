# Testing Mocks

This document defines the repository-wide guidelines for mocks used in tests.

## What to Mock vs. Keep Real

### Keep Real

- `RepositoryManager` and repositories — use `database::setup_test_repository_manager().await`
- domain logic and pure functions

### Mock

- service traits (`FileSetServiceOps`, `CloudStorageOps`, `FileSystemOps`, `FileImportOps`, etc.)
- external I/O (file system, HTTP, S3)
- executable runners and other process-launching boundaries

## Core Mock Pattern

For shared mutable async mocks in this repository, default to a single
`Arc<Mutex<MockState>>`. Do not create one `Arc<Mutex<_>>` per field.

```rust
#[derive(Default)]
struct MockState {
    next_entity_id: i64,
    created_entities: HashMap<i64, CreateEntityParams>,
    lookup_results: HashMap<BTreeSet<Sha1Checksum>, EntityId>,
    fail_create_for: Vec<String>,
    fail_find_for: Vec<BTreeSet<Sha1Checksum>>,
}

#[derive(Clone)]
pub struct MockSomething {
    state: Arc<Mutex<MockState>>,
}
```

## Naming

- Use domain-specific field names in `MockState`
- Name tracking collections after the operation they represent
- Name failure collections after the operation they fail
- Avoid placeholder names like `data`, `items`, or `results` when a domain-specific name is possible

## Trait Implementation Rules

- Use the crate's real error enum in mocks
- Do not invent mock-only error types or variants
- Return existing generic error variants such as `DatabaseError(String)` or `Other(String)` when needed

```rust
#[async_trait]
impl SomethingOps for MockSomething {
    async fn create_entity(&self, params: CreateEntityParams) -> Result<EntityId, SomethingError> {
        let mut state = self.state.lock().unwrap();

        if state.fail_create_for.contains(&params.name) {
            return Err(SomethingError::DatabaseError(format!(
                "mock: forced failure for '{}'",
                params.name
            )));
        }

        let id = state.next_entity_id;
        state.next_entity_id += 1;
        state.created_entities.insert(id, params);

        Ok(id)
    }
}
```

## Async Safety

- Never hold a `MutexGuard` across `.await`
- Keep lock scopes short: read or update state, drop the guard, then await
- If a mock method needs both async work and shared state, split it into small lock/unlock phases

## Collection Keys

Use `BTreeSet` for set-like collections used as `HashMap` keys.

```rust
configured_results: HashMap<BTreeSet<Sha1Checksum>, Vec<FileInfo>>
```

Do not use `Vec` when order should not matter.

For membership-only tracking collections such as failure configuration, prefer `HashSet` or
`BTreeSet` over `Vec` unless insertion order is actually needed.

## Configuration, Verification, Reset

Mocks should usually provide:

- configuration helpers (`add_lookup_result`, `fail_create_for`, ...)
- verification helpers (`created_count`, `was_created`, ...)
- a `clear()` method that resets the whole state
- deterministic behavior unless the mock is explicitly simulating nondeterminism

```rust
pub fn clear(&self) {
    *self.state.lock().unwrap() = MockState {
        next_entity_id: 1,
        ..Default::default()
    };
}
```

### Prefer Interior Mutability for Rich Service Mocks

For more stateful service-boundary mocks, prefer exposing a shared
`Arc<Mutex<MockState>>` through a constructor such as `with_state(...)`.

This lets tests:

- configure the mock by mutating shared state directly
- inspect recorded calls and configured outcomes directly
- share one mock state between the test and the mock instance without adding many narrow helper methods

Use this when the mock has enough behavior that direct state setup and assertion is clearer
than adding many one-off configuration methods.

Guidelines:

- keep all shared mutable mock state in a single `MockState`
- keep lock scopes short
- never hold a `MutexGuard` across `.await`
- keep convenience helpers like `with_outcome`, `total_calls`, or `clear` when they still improve readability
- prefer this pattern for richer service mocks where tests need both direct configuration and direct assertions against mock state

## Reference Implementations

Use these as models when creating new mocks:

- `cloud_storage/src/mock.rs`
- `service/src/file_set/mock_file_set_service.rs`
- `service/src/file_system_ops.rs` (`service::file_system_ops::mock::MockFileSystemOps`)
- `service/src/file_set_download/download_service_ops.rs` (`MockDownloadServiceOps`) — example of a service-boundary mock with configured outcomes, progress events, call tracking, and shared interior-mutable state

## Making Mocking Easier in This Repository

The main way to reduce mocking friction in this codebase is not to add a mocking framework, but to make dependencies easier to substitute.

### Prefer Traits at Service Boundaries

If a service depends on another service as a capability, prefer a trait boundary over a concrete service type.

```rust
// Harder to test
pub struct MyService {
    download_service: Arc<DownloadService>,
}

// Easier to test
pub struct MyService {
    download_service: Arc<dyn DownloadServiceOps>,
}
```

Use this especially for:

- external I/O
- cross-service orchestration dependencies
- dependencies used inside pipeline contexts and steps

### Prefer `new_with_deps` / `new_with_ops` Constructors

Keep the normal production constructor simple, but add an injection-friendly constructor for tests.

```rust
pub fn new(...) -> Self
pub fn new_with_deps(...) -> Self
```

This is the default seam when a full trait abstraction is not yet worth introducing.

### Extract Pure Logic Before Introducing Mocks

Before adding a mock, first ask whether the logic can be moved into a pure helper and tested directly.

Good candidates:

- mapping logic
- selection logic
- parsing helpers
- small validation rules

Pure helpers are preferred over mocks when possible.

### Make Pipeline Contexts Store Capabilities, Not Concrete Services

Pipeline steps are easiest to test when their context dependencies are trait objects or other mockable abstractions.

```rust
pub struct MyContextDeps {
    pub file_ops: Arc<dyn FileSystemOps>,
}
```

Avoid storing concrete services in pipeline contexts unless the step truly depends on the full concrete implementation.

### Good Candidates for Future Cleanup

Across the repository, the highest-value mocking improvements are:

- converting concrete cross-service dependencies to `*Ops` traits where they act as capabilities
- adding `new_with_deps` constructors where only production constructors exist
- extracting pure helpers from service/event/wiring code before testing

## Checklist for Every Test Implementation

Go through this checklist each time you add or update tests:

- [ ] Am I keeping `RepositoryManager` and repositories real unless the test is specifically about a boundary that should be mocked?
- [ ] Before adding a mock, did I check whether the behavior can be tested through a pure helper instead?
- [ ] If the code is hard to test, should I first add or use an existing seam such as `Arc<dyn *Ops>`, `new_with_deps(...)`, or `new_with_ops(...)` instead of forcing a heavier mock setup?
- [ ] If this is service or pipeline code, are dependencies modeled as capabilities rather than concrete services where practical?
- [ ] If this is UI, input, mapping, or event translation code, did I extract the testable logic from wiring code before mocking anything?
- [ ] Am I mocking only external I/O, process-launching, cross-service capability boundaries, or other true side-effect boundaries?
- [ ] Does each mock use the crate's real error type instead of mock-only errors?
- [ ] Are mock state, helper names, and verification methods domain-specific and easy to read?
- [ ] Are mock behaviors deterministic unless the test is explicitly about nondeterminism?
- [ ] If the mock uses shared mutable state, are lock scopes short and never held across `.await`?
- [ ] Does the mock provide clear configuration, verification, and reset helpers when they add value?
- [ ] Am I covering the behavior that matters: happy path, expected failure path, and important boundary conditions?
- [ ] If I had to introduce extra seams or helpers for testability, did I choose the smallest change that also improves future tests in this area?

## Related Guidance

- `.github/copilot-instructions.md` — short project conventions
- `.github/skills/qa/SKILL.md` — expanded testing and coverage guidance
