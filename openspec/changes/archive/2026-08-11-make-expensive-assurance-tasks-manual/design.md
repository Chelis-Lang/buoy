## Context

The Devenv graph currently includes `buoy:coverage` in P0. This edge makes each `devenv test` run the complete LLVM instrumentation lane.

The full mutation campaign already has no acceptance edge. The automatic graph runs only its deterministic replay task.

Both controls write repository evidence that binds current sources, tools, and configuration. Existing ledger checks can reject stale source digests without a new campaign.

## Goals / Non-Goals

**Goals:**
- Require explicit selection for LLVM coverage and full mutation campaigns.
- Keep stale or invalid recorded evidence blocking.
- Keep mutation replay automatic.
- Reject future graph edges that make either expensive task automatic.
- Preserve exact coverage and mutation thresholds.

**Non-Goals:**
- Reduce exact coverage requirements.
- Reduce mutation thresholds or named tripwires.
- Change the functional core or its `no_std` boundary.
- Make generated evidence self-certifying.
- Run either expensive task in CI.

## Decisions

### 1. Declare manual tasks in the workflow policy

Add a closed `manual` task list to the workflow policy. The typed graph checker will require each task and reject reachability from `devenv:enterTest`.

The list will contain both canonical tasks and their compatibility aliases. Stable diagnostics will identify missing manual tasks and automatic dependency paths.

**Alternatives considered:**
- Depend only on absent phase membership. Rejected because another automatic task can add a transitive edge later.
- Use `status` or `execIfModified`. Rejected because these fields create conditional automatic execution rather than explicit selection.

### 2. Replace coverage execution with a fast evidence check

P0 will retain schema, reference, fixture, property, and finite-model execution. It will not depend on `buoy:coverage`.

P0 will depend on `buoy:coverage-evidence`. This task checks result structure, current source digests, configuration, inventory, and exact metrics without LLVM instrumentation.

P3 will still validate the complete ledger. The earlier check gives a focused failure before later assurance phases.

**Alternatives considered:**
- Check coverage only during ledger closure. Rejected because stale coverage must fail before later assurance phases.

### 3. Keep mutation replay in P1

`buoy:mutation-replay` will remain in P1. It checks named tripwires, campaign bindings, policy, and the recorded artifact without `cargo-mutants`.

`buoy:mutation-campaign` will remain a separate task with no dependencies. Its description will state that it requires explicit selection.

### 4. Preserve evidence boundaries

Manual tasks produce effectful execution verdicts. Automatic checks only validate recorded facts against current repository inputs.

A current record does not prove behavior, completeness, compiler correctness, or defect absence. Verus and AeneasVerif evidence remains separate and unchanged.

### 5. Use one automatic acceptance oracle

`devenv test` remains the automatic acceptance oracle. It succeeds only when all automatic phases pass and recorded expensive evidence remains current.

Manual control commands remain:
- `devenv tasks run buoy:coverage`
- `devenv tasks run buoy:mutation-campaign`

## Risks / Trade-offs

- A source change can make `devenv test` fail until a maintainer selects coverage.
- Normal acceptance becomes faster when expensive evidence remains current.
- Detached campaigns can run less often. Blocking freshness checks limit this risk.
- The workflow checker gains policy complexity. Closed parsing and negative fixtures limit configuration drift.
