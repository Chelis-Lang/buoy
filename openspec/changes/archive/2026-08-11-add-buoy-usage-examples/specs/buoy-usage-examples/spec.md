## ADDED Requirements

### Requirement: BUE-001 Example index and boundaries

The repository MUST provide one example index that identifies each example, prerequisite, command, expected result, and assurance limit.

#### Scenario: Positive - user selects an example
- **GIVEN** a user needs a Rust, command-line, or OpenSpec example
- **WHEN** the user reads the example index
- **THEN** the index identifies a focused example and its exact verification command

#### Scenario: Negative - example implies excess assurance
- **GIVEN** an example demonstrates a successful test or trace link
- **WHEN** the index describes the demonstrated assurance
- **THEN** the index MUST NOT describe the result as mathematical proof or external provider verification

### Requirement: BUE-002 Rust core graph example

The repository MUST provide a compiled Rust example that constructs typed atoms, builds a graph, and reports deterministic graph diagnostics.

#### Scenario: Positive - valid graph
- **GIVEN** valid stable IDs, lifecycle values, scopes, and normative statements
- **WHEN** the Rust core graph example runs
- **THEN** the example MUST construct the graph and report no graph diagnostics

#### Scenario: Negative - stale or invalid graph relation
- **GIVEN** an adjacent negative control with an invalid graph relation
- **WHEN** the focused example test runs
- **THEN** the test MUST fail closed with the expected stable diagnostic code

### Requirement: BUE-003 Rust repository check example

The repository MUST provide a compiled Rust example that parses configuration, snapshots selected files, runs a static check, and renders deterministic output.

#### Scenario: Positive - saved repository input
- **GIVEN** a valid repository root and a valid `spec-provenance-config/v2` configuration
- **WHEN** the Rust repository example runs twice
- **THEN** both runs MUST produce byte-identical machine output

#### Scenario: Negative - invalid configuration
- **GIVEN** an invalid or incomplete repository configuration
- **WHEN** the configuration enters the Rust repository example
- **THEN** parsing MUST reject the input at the configuration boundary with the expected stable error code

### Requirement: BUE-004 OpenSpec authority example

The repository MUST provide a complete OpenSpec example with configuration, a standing specification, reserved IDs, and focused Buoy static commands.

#### Scenario: Positive - reserved requirement materializes
- **GIVEN** a requirement under `openspec/specs/<capability>/spec.md` with a reserved stable ID
- **WHEN** `buoy static` reads the example repository
- **THEN** the command MUST materialize the requirement and scenarios without diagnostics

#### Scenario: Negative - active change path is selected
- **GIVEN** equal requirement bytes under an active OpenSpec change path
- **WHEN** the OpenSpec adapter reads that path
- **THEN** the adapter MUST reject it with `SPG-OPENSPEC-STANDING-PATH`

### Requirement: BUE-005 External trace link example

The repository MUST provide OpenSpec-related examples for each closed external trace relation and one stale-link control.

#### Scenario: Positive - closed relations parse
- **GIVEN** links with `tracks`, `discusses`, `documents`, and `reviews`
- **WHEN** Buoy parses the example trace blocks
- **THEN** each link MUST remain navigation data without assurance or provider state

#### Scenario: Negative - requirement revision invalidates a link
- **GIVEN** a trace link bound to an earlier atom revision
- **WHEN** the related requirement changes without a link revision
- **THEN** graph validation MUST fail closed with `SPG-REF-DANGLING-OR-STALE`

### Requirement: BUE-006 Provider-neutral change example

The repository MUST provide base and head roots plus a materialized change manifest for a focused `buoy change` example.

#### Scenario: Positive - complete disposition closure
- **GIVEN** exact base and head graph digests with complete impact dispositions
- **WHEN** `buoy change` evaluates the example
- **THEN** the command MUST return success with deterministic machine output

#### Scenario: Negative - incomplete disposition closure
- **GIVEN** an impacted object without a required disposition
- **WHEN** `buoy change` evaluates the negative control
- **THEN** the command MUST return exit class `3` with deterministic diagnostics

### Requirement: BUE-007 Example verification and aggregation

Each example MUST have positive and negative controls before the example enters the maintained public index.

#### Scenario: Positive - all focused checks pass
- **GIVEN** all example sources, configurations, and expected outputs are current
- **WHEN** the focused example gate runs
- **THEN** the gate MUST compile Rust examples and verify all deterministic expected results

#### Scenario: Negative - one example drifts
- **GIVEN** one example source, command, diagnostic, or expected output differs from its reviewed value
- **WHEN** the focused example gate aggregates results
- **THEN** the gate MUST fail closed and identify the affected example deterministically
