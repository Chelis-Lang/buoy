## ADDED Requirements

### Requirement: DEV-010 Advisory downstream publication tasks

A downstream task introduced by this capability MUST remain explicit and advisory.

This rule applies to downstream publication checks, publication generation, evidence rendering, and consumer-package realization.

Such a task MUST have no direct or transitive dependency path to `devenv test`, a phase task, or another automatic task.

Buoy repository acceptance MUST test the owned implementation through its declared local test graph. It MUST NOT treat a downstream task result as execution evidence.

#### Scenario: Positive - maintainer selects publication check

- **GIVEN** one downstream environment with an explicit Buoy publication-check task
- **WHEN** a maintainer selects that task
- **THEN** the task runs without changing the consumer acceptance graph

#### Scenario: Negative - consumer test depends on publication check

- **GIVEN** one downstream `devenv test` graph with a path to a Buoy publication task
- **WHEN** task-graph policy validation runs
- **THEN** validation fails with a deterministic automatic-publication-task diagnostic
