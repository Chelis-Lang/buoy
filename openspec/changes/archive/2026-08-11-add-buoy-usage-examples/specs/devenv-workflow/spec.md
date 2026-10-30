## MODIFIED Requirements

### Requirement: DEV-009 Explicit selection for coverage and mutation checks

The resolved Devenv graph MUST expose these focused tasks:

- `buoy:coverage`
- `buoy:coverage-evidence`
- `buoy:mutation-campaign`
- `buoy:mutation-replay`

These tasks and their compatibility aliases MUST have no dependency path to `devenv test`, a phase task, or another automatic task.

Automatic acceptance MUST NOT start LLVM instrumentation, `cargo-mutants`, the recorded coverage check, or the named mutation replay.

Coverage-bearing source rows MUST retain manual coverage dispositions. The focused coverage task MUST continue to select these rows.

Automatic ledger validation MUST classify manual coverage rows as not applicable. Stale manual coverage evidence MUST NOT block `devenv test`.

The task graph checker MUST reject an automatic dependency on any of these focused tasks.

#### Scenario: Positive - maintainer selects a focused task

- **GIVEN** the Devenv environment exposes the four focused tasks
- **WHEN** a maintainer explicitly selects one task
- **THEN** only that task starts its declared check or campaign

#### Scenario: Positive - automatic acceptance excludes focused checks

- **GIVEN** the four focused tasks exist without automatic dependencies
- **WHEN** `devenv test` runs
- **THEN** it MUST complete without those tasks

#### Scenario: Positive - stale manual coverage remains outside acceptance

- **GIVEN** a manual coverage row has stale recorded evidence
- **WHEN** automatic ledger validation runs
- **THEN** it MUST classify that row as not applicable
- **AND** the explicit coverage task MUST still select that row

#### Scenario: Negative - a phase depends on a focused task

- **GIVEN** one phase has a direct or transitive dependency on a listed focused task
- **WHEN** the task graph checker validates the resolved graph
- **THEN** it MUST fail with one stable manual-task dependency diagnostic

#### Scenario: Negative - a focused task becomes an automatic prerequisite

- **GIVEN** an automatic task depends on `buoy:coverage-evidence` or `buoy:mutation-replay`
- **WHEN** automatic acceptance is validated
- **THEN** the graph check MUST reject the dependency
