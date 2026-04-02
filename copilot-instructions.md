# Agent Behavior Guidelines

Constitutional principles for working on the Emulation File Manager project.

## Core Principles

### Always Explain Your Reasoning
- Explain which layer code belongs to and why
- Describe trade-offs between different approaches
- Reference the 4-layer architecture when making decisions
- Call out any deviations from established patterns

### Start with Design, Then Implementation
For non-trivial features:
1. Use the **architect agent** for design and layer placement
2. Use the **database agent** for schema and repositories
3. Use the **gui agent** for UI components

### Prefer Small, Focused Changes
- Make surgical changes to address specific issues
- Avoid large refactors unless explicitly requested
- Don't fix unrelated bugs or style issues
- One logical change per commit/PR

## The 4-Layer Architecture (Non-Negotiable)

**Layer Order:** Core → Database → Service → GUI

**Rules:**
- Upper layers can depend on lower layers, never reverse
- Core crates have NO dependencies on other project crates
- Business logic lives in service layer
- Database crate owns all data access

**Quick Decision Guide:**
- Domain type? → Core crate
- Data access? → Database crate  
- Business logic? → Service crate
- User interface? → GUI crate

## Critical Non-Negotiables

### SQLx Offline Mode
**ALWAYS** regenerate after query or schema changes:
```bash
cargo sqlx prepare --workspace -- --all-targets
```
CI will fail without up-to-date `.sqlx/` metadata.

### Layer Boundaries
**NEVER:**
- Add SQLx queries in GUI code
- Add business logic in database repositories
- Skip service layer (GUI → Database directly)
- Add project dependencies to core crates

### Error Handling
- All public APIs return `Result<T, Error>`
- Propagate errors, don't panic in production
- `unwrap()` only acceptable in tests

## Task-Specific Guidelines

**IMPORTANT:** Before working on these tasks, read the corresponding instruction file:

- **Architecture & Design**: Read `.github/copilot-instructions-architect.md`
- **Database Work** (schemas, queries, migrations): Read `.github/copilot-instructions-database.md`
- **GUI Development** (relm4 components): Read `.github/copilot-instructions-gui.md`
- **Testing** (writing tests, mocks): Read `.github/copilot-instructions-test.md`

These files contain essential patterns and requirements for each domain.

## Technical Requirements

### Async Runtime
Use async-std as the async runtime throughout the application.
- All async code uses async-std, not tokio
- Database: SQLx with "runtime-async-std" feature
- Tests: Use #[async_std::test] attribute

### Code Conventions
- Edition: Rust 2024
- No unwrap() in production code (tests OK)
- Prefer ? operator over unwrap/expect
- Pipeline steps in service layer is an exception to use expect when the value is tested in should_execute
- Use descriptive variable names
- Do **not** use `// ---` or similar comment dividers to split code into sections — split into separate files instead
- Public functions, structs, traits, and methods must have doc comments (`///`)
- **Single responsibility, no side effects**: Each function should do one thing and return its result. A function that computes a value must return it — never write computed results to shared state as a side effect inside the function. Callers are responsible for recording, sending, or persisting results.
- **DRY (Don't Repeat Yourself)**: Do not duplicate logic or code. Extract shared logic into the appropriate abstraction for the layer — a shared function, a reusable component, or a helper module. Place the abstraction at the nearest common ancestor so both consumers can access it without increasing visibility unnecessarily.

### Dependencies
When adding crates, prefer:
- async-std over tokio (consistency)
- Well-maintained crates with active development
- Minimal dependency trees

### Model Selection

When using the `task` tool or `/model` command, choose based on complexity:

- **Claude Haiku 4.5** (fast/cheap): Simple edits, searching, straightforward grep/view operations, file reading
- **Claude Sonnet 4.5** (default, standard): Most coding work — refactoring, bug fixes, straightforward implementations, tests
- **Claude Opus 4.6** (premium): Complex architecture decisions, design trade-offs, intricate debugging, multi-layer feature planning, sophisticated analysis

Default assumption is Sonnet — only override when the task clearly requires more (Opus) or less (Haiku) reasoning.

## Validation Checklist

After making changes:
- [ ] `cargo test` passes
- [ ] `cargo check` succeeds (use `cargo check` instead of `cargo build` to verify compilation)
- [ ] Regenerated `.sqlx/` if queries changed
- [ ] Ran `tbls doc` if schema changed
- [ ] No layer boundary violations
- [ ] Changes committed in small increments to keep diffs focused and reviewable

## When in Doubt

1. Check existing patterns in similar code
2. Ask clarifying questions
3. Prefer explicit over clever
4. Consult the appropriate specialized agent

---

**Keep this file lean.** Detailed technical patterns belong in `docs/patterns/`.
