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

### Mock Implementation Pattern
When creating mocks for traits (for testing):

**Structure:**
```rust
#[derive(Clone, Default)]
pub struct MockSomethingService {
    // Counters for auto-incrementing IDs
    next_id: Arc<Mutex<i64>>,
    
    // State tracking (what operations were performed)
    performed_operations: Arc<Mutex<HashMap<K, V>>>,
    
    // Pre-configured results (for lookups)
    // Use BTreeSet for order-independent sets
    configured_results: Arc<Mutex<HashMap<BTreeSet<Key>, Value>>>,
    
    // Failure simulation
    fail_for: Arc<Mutex<Vec<Condition>>>,
}
```

**Required Methods:**
- `new()` - Create with sensible defaults
- `add_*()` / `set_*()` - Configure behavior
- `fail_*_for()` - Simulate failures
- `was_*()` / `get_*()` / `*_count()` - Verify operations
- `clear()` - Reset state between tests

**Best Practices:**
- Use `BTreeSet` for unordered collections (not `Vec`)
- Use `Arc<Mutex<>>` for shared mutable state
- Implement `Clone` and `Default`
- Include comprehensive tests (9+ test cases)
- Document capabilities in doc comments
- Check failure conditions first in methods

**Examples:** `MockCloudStorage`, `MockFileSetService`

## When to Use Which Agent

| Need | Agent | Why |
|------|-------|-----|
| Where should X live? | architect | Understands layers & boundaries |
| Add table or query | database | Knows SQLx patterns & migrations |
| Create UI component | gui | Knows relm4 patterns & gotchas |
| Review design | architect | Evaluates trade-offs |

**Detailed implementation patterns are in the specialized agent profiles—not here.**

## Validation Checklist

After making changes:
- [ ] `cargo test` passes
- [ ] `cargo build` succeeds
- [ ] Regenerated `.sqlx/` if queries changed
- [ ] Ran `tbls doc` if schema changed
- [ ] No layer boundary violations

## When in Doubt

1. Check existing patterns in similar code
2. Ask clarifying questions
3. Prefer explicit over clever
4. Consult the appropriate specialized agent

---

**Keep this file lean.** Detailed technical patterns belong in specialized agent profiles (`.github/agents/`).
