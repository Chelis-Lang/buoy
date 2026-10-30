## ADDED Requirements

### Requirement: OBW-001 Project-level `buoy-bridge` schema
The repository MUST provide a versioned `buoy-bridge` workflow under `openspec/schemas/`. The workflow MUST retain proposal, specs, design, tasks, apply, and archive behavior.

#### Scenario: Positive - project schema resolves locally
- **GIVEN** the repository contains a valid `openspec/schemas/buoy-bridge/schema.yaml`
- **WHEN** OpenSpec resolves the `buoy-bridge` schema
- **THEN** it selects the version-controlled project schema and all required templates

#### Scenario: Negative - global schema shadows the project schema
- **GIVEN** project and user schemas with the same `buoy-bridge` name
- **WHEN** OpenSpec resolves the schema for this repository
- **THEN** the project schema wins and schema identity remains explicit

### Requirement: OBW-002 Traceability artifact dependency
The workflow MUST add one `traceability` artifact after specs and design. Tasks MUST require the completed traceability artifact.

#### Scenario: Positive - traceability is ready after planning
- **GIVEN** complete proposal, specs, and design artifacts
- **WHEN** OpenSpec calculates the artifact graph
- **THEN** the traceability artifact is ready and tasks remain blocked until it exists

#### Scenario: Negative - tasks exist without traceability
- **GIVEN** a change with specs and design but no traceability artifact
- **WHEN** apply readiness is calculated
- **THEN** the change is not apply-ready and reports the missing dependency

### Requirement: OBW-003 Complete planned link inventory
The traceability artifact MUST list each affected requirement ID and each planned code, evidence, issue, pull-request, or document link action.

#### Scenario: Positive - affected requirement has complete actions
- **GIVEN** a change that adds one requirement, code link, test carrier, and issue link
- **WHEN** the traceability artifact is created
- **THEN** it records the stable ID, target class, action, owner, and required Buoy command

#### Scenario: Negative - requirement change omits link review
- **GIVEN** a modified requirement with current links and no traceability action for those links
- **WHEN** the traceability artifact is validated by the Buoy workflow control
- **THEN** validation fails with a deterministic missing-link-review diagnostic

### Requirement: OBW-004 Stable-ID and scenario author rules
Project rules MUST require one stable ID per requirement and explicit positive and negative scenarios. Prompt rules MUST NOT allocate IDs automatically.

#### Scenario: Positive - generated spec obeys project rules
- **GIVEN** a new change under the `buoy-bridge` schema and one reserved stable ID
- **WHEN** the agent creates the delta spec
- **THEN** the requirement heading uses that ID and includes positive and negative scenarios

#### Scenario: Negative - agent invents an unreserved ID
- **GIVEN** a generated delta spec with an ID absent from the append-only registry
- **WHEN** Buoy validates the standing spec after sync
- **THEN** validation fails and MUST NOT allocate the ID from source order

### Requirement: OBW-005 Apply and archive guidance
Apply guidance MUST describe repository link updates and focused Buoy commands. Archive guidance MUST describe strict OpenSpec validation and the Buoy change gate.

#### Scenario: Positive - apply reads current guidance
- **GIVEN** current project guidance for link updates and focused commands
- **WHEN** the apply workflow starts
- **THEN** it receives the current guidance as separate operation input

#### Scenario: Negative - guidance overrides a built-in safety rule
- **GIVEN** operation guidance that conflicts with a built-in path or archive rule
- **WHEN** the workflow evaluates the conflict
- **THEN** it rejects the conflicting guidance and preserves the built-in rule

### Requirement: OBW-006 Guidance is not enforcement
OpenSpec rules, templates, artifact existence, and operation guidance MUST remain advisory. Buoy and CI MUST provide all technical link enforcement.

#### Scenario: Positive - valid artifacts and valid links
- **GIVEN** complete OpenSpec artifacts and current repository-owned Buoy links
- **WHEN** strict OpenSpec validation and the Buoy gate run
- **THEN** both controls pass within their separate scopes

#### Scenario: Negative - traceability artifact claims a false link
- **GIVEN** a complete traceability artifact that names a missing code or external link
- **WHEN** OpenSpec validation and the Buoy gate run
- **THEN** OpenSpec can pass while Buoy fails with the exact missing-link diagnostic

### Requirement: OBW-007 Workflow schema validation and drift control
The project MUST validate the custom schema, templates, configuration, and generated workflow files. Drift MUST fail before archive acceptance.

#### Scenario: Positive - schema and generated files agree
- **GIVEN** a valid project schema with current generated workflow files
- **WHEN** the repository workflow control runs
- **THEN** schema validation and generated-file drift validation pass

#### Scenario: Negative - template changes without workflow refresh
- **GIVEN** a changed `buoy-bridge` template and stale generated workflow files
- **WHEN** the repository workflow control runs
- **THEN** validation fails with a deterministic workflow-drift diagnostic
