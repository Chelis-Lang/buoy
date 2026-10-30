## ADDED Requirements

### Requirement: PID-009 Optional OpenSpec standing-spec adapter boundary
Schema v3 MUST support an optional shell adapter for saved standing specs under `openspec/specs/`.

The adapter MUST NOT parse OpenSpec change state as product authority. `buoy-core` MUST remain independent from OpenSpec, parsers, processes, and providers.

#### Scenario: Positive - v3 adapter reads standing specs
- **GIVEN** schema v3, an explicit OpenSpec selector, saved standing specs, and complete adapter configuration
- **WHEN** the shell materializes the selected requirements
- **THEN** the graph contains their atoms while `buoy-core` receives only complete parsed values

#### Scenario: Positive - v3 cutover is complete
- **GIVEN** a repository that enables schema v3
- **WHEN** all selected adapters run
- **THEN** every parsed item uses a schema-v3 source item kind

#### Scenario: Negative - archive action grants authority
- **GIVEN** an OpenSpec archive operation without an explicit Buoy lifecycle or approval update
- **WHEN** schema v3 materializes the resulting standing requirement
- **THEN** the operation MUST NOT ratify the atom or create review-only assurance

#### Scenario: Negative - core imports OpenSpec behavior
- **GIVEN** a change that adds OpenSpec parsing, lifecycle state, filesystem access, or process access to `buoy-core`
- **WHEN** architecture validation runs
- **THEN** validation fails with a stable dependency-boundary diagnostic
