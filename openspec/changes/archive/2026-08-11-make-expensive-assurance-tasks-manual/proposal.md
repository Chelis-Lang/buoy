## Why

LLVM coverage and full mutation campaigns take much longer than normal repository checks. Maintainers need explicit control over these expensive executions.

The repository still needs fast, blocking checks for stale coverage and mutation evidence. Manual execution must not weaken evidence freshness or exactness rules.

## What Changes

- Detach `buoy:coverage` from `devenv test` and all phase dependencies.
- Keep `buoy:mutation-campaign` detached from `devenv test` and document its manual status.
- Keep deterministic mutation replay and evidence checks in the automatic graph.
- Keep recorded coverage freshness and exactness checks blocking in the automatic graph.
- Update the task graph policy, phase contracts, assurance registry, and operator documentation.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `devenv-workflow`: Define explicitly selected tasks outside the automatic acceptance graph.
- `llvm-coverage-testing`: Replace automatic LLVM instrumentation with manual execution and blocking recorded-evidence checks.
- `adversarial-verification`: Clarify that the full mutation campaign requires explicit selection while replay remains automatic.

## Impact

`devenv test` will not start LLVM instrumentation or `cargo-mutants`. Maintainers will run `devenv tasks run buoy:coverage` and `devenv tasks run buoy:mutation-campaign` explicitly.

Automatic acceptance will still fail when committed coverage or mutation evidence is absent, stale, malformed, or below its required threshold.
