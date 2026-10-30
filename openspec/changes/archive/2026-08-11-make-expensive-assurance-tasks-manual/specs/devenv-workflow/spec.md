## ADDED Requirements

### Requirement: DEV-009 Explicit selection for expensive assurance tasks

The resolved Devenv graph MUST expose `buoy:coverage` and `buoy:mutation-campaign` as focused tasks. These tasks and their compatibility aliases MUST have no dependency path to `devenv test`, any phase task, or another automatic task.

Automatic acceptance MUST retain blocking deterministic checks for recorded coverage evidence, mutation evidence, and named mutation tripwires. These checks MUST NOT start LLVM instrumentation or `cargo-mutants`.

The task graph checker MUST reject an automatic dependency on either expensive task. It MUST also reject removal of the required replay and evidence checks.

#### Scenario: Positive - maintainer selects an expensive task
- **GIVEN** the Devenv environment exposes both expensive focused tasks
- **WHEN** a maintainer explicitly selects one task
- **THEN** only that task starts its declared LLVM coverage or mutation campaign process

#### Scenario: Positive - automatic checks use recorded evidence
- **GIVEN** current coverage and mutation artifacts exist
- **WHEN** `devenv test` runs
- **THEN** deterministic replay and freshness checks pass without either expensive process

#### Scenario: Negative - a phase depends on an expensive task
- **GIVEN** one phase has a direct or transitive dependency on `buoy:coverage` or `buoy:mutation-campaign`
- **WHEN** the task graph checker validates the resolved graph
- **THEN** it fails with one stable manual-task dependency diagnostic

#### Scenario: Negative - manual status removes blocking replay
- **GIVEN** both expensive tasks remain detached but a required replay or evidence check is absent
- **WHEN** automatic acceptance is validated
- **THEN** the graph or assurance check fails closed for the missing blocking check
