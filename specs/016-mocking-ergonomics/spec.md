# 016: Mocking Ergonomics

## Status
<!-- Planning | In Progress | Complete | Abandoned -->
Planning

## Affected Crates
- `service` — trait boundaries, test-friendly constructors, and pipeline context dependencies
- `relm4-ui` — extraction of pure mapping/helpers where UI event translation is currently harder to test than necessary
- `cloud_storage` — reference example for mock conventions and capability-style abstractions
- `docs` — repository guidance for mock structure and testability seams

## Problem

The repository already has a workable mocking pattern, but mocking effort is still inconsistent across crates. Some logic is easy to test because it depends on traits or pure helpers, while other areas still depend on concrete services or mix mapping/wiring logic with behavior that should be unit-tested in isolation.

This creates avoidable friction:

- unit tests sometimes need heavier setup than the behavior under test really requires
- some service and pipeline code is harder to isolate because dependencies are concrete services rather than capabilities
- test seams vary by module, so contributors have to rediscover the preferred pattern repeatedly

The goal is not to introduce a mocking framework. The goal is to make the existing ecosystem easier to test by standardizing dependency inversion and pure-helper extraction where they provide clear value.

## Proposed Solution

Adopt a phased repository-wide cleanup that improves testability without over-abstracting everything.

### Phase 1 — Inventory and Prioritization

Identify service, pipeline, and UI modules where tests are currently harder than necessary because they depend on concrete collaborators or because logic that could be pure is embedded in event/service wiring.

Prioritize:

1. cross-service dependencies that behave like capabilities
2. pipeline contexts that store concrete services
3. mapping/validation logic that can become pure helpers

### Phase 2 — Standardize Test Seams

Use the following default rules:

- prefer trait boundaries for capability-style dependencies (`Arc<dyn *Ops>`)
- keep production constructors simple with `new(...)`
- add `new_with_deps(...)` or `new_with_ops(...)` constructors when test injection is useful and a trait is not yet justified
- extract pure helpers before introducing a new mock

### Phase 3 — Apply Incrementally

Refactor the highest-value modules first. Do not attempt a repository-wide rewrite in one pass. Each conversion should be narrowly scoped and should preserve existing production behavior.

### Phase 4 — Document and Reinforce

Keep `docs/TESTING_MOCKS.md` as the canonical repository guidance for mocks and test seams, and keep instructions/skills aligned with that document.

## Key Decisions

| Decision | Rationale |
|---|---|
| Do not introduce a new mocking framework | The repository already has an established mock pattern; the bigger problem is inconsistent dependency seams, not missing tooling. |
| Prefer traits only at capability boundaries | This improves testability without flooding the codebase with unnecessary abstraction. |
| Use `new_with_deps` / `new_with_ops` as the default fallback seam | This provides a low-cost testing seam where a trait would be premature. |
| Prefer pure helper extraction over mocks when possible | Pure logic is cheaper to test, easier to reason about, and avoids unnecessary dependency setup. |
| Apply changes incrementally | A phased rollout reduces churn and keeps refactors reviewable. |

## Acceptance Criteria

- The repository has a documented, explicit policy for when to use traits, pure helpers, and alternate constructors to improve testability.
- New or refactored service/pipeline code can usually be unit-tested without constructing unrelated concrete services.
- Pipeline contexts prefer capability-style dependencies over concrete service storage where practical.
- UI/input/event translation code extracts reusable pure helpers when that materially simplifies testing.
- The mock guidance in `docs/TESTING_MOCKS.md`, `.github/copilot-instructions.md`, and `.github/skills/qa/SKILL.md` remains aligned.

## As Implemented

_(Pending)_
