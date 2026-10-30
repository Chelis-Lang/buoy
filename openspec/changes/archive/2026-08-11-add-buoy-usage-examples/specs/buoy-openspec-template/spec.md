## ADDED Requirements

### Requirement: BOT-001 Reusable schema boundary

The example project MUST provide a project-local OpenSpec schema named `buoy-traceability` without replacement of the internal `buoy-bridge` schema.

#### Scenario: Positive - downstream project selects the schema
- **GIVEN** a downstream project copies the example schema and selects `schema: buoy-traceability`
- **WHEN** OpenSpec resolves the project-local schema
- **THEN** OpenSpec MUST select the copied schema and expose its complete artifact order

#### Scenario: Negative - internal rules leak into the schema
- **GIVEN** the reusable schema templates and configuration
- **WHEN** the schema controls inspect their text
- **THEN** the controls MUST reject required Buoy-internal tools, repository paths, or acceptance commands

### Requirement: BOT-002 Traceability artifact order

The schema MUST require proposal, specification, design, traceability, and task artifacts in that dependency order.

#### Scenario: Positive - traceability plan exists
- **GIVEN** completed proposal, specification, and design artifacts
- **WHEN** OpenSpec reports the next artifact
- **THEN** `traceability.md` MUST become ready before `tasks.md`

#### Scenario: Negative - tasks precede traceability
- **GIVEN** no completed traceability artifact
- **WHEN** OpenSpec evaluates task readiness
- **THEN** the schema MUST keep `tasks.md` blocked

### Requirement: BOT-003 Buoy-compatible specification template

The specification template MUST require stable IDs, RFC 2119 statements, and explicit positive and negative Given/When/Then scenarios.

#### Scenario: Positive - archived specification is compatible
- **GIVEN** a completed requirement that follows the template
- **WHEN** OpenSpec synchronizes it into `openspec/specs/<capability>/spec.md`
- **THEN** the resulting requirement MUST use Buoy-compatible requirement and scenario headings

#### Scenario: Negative - requirement ID is absent
- **GIVEN** a requirement heading without a stable ID
- **WHEN** the template control evaluates the specification
- **THEN** the control MUST fail with a deterministic message before the example passes

### Requirement: BOT-004 Complete traceability plan template

The traceability template MUST record each requirement ID, target class, action, owner, focused Buoy command, current-link disposition, and external target text.

#### Scenario: Positive - each affected requirement has actions
- **GIVEN** an OpenSpec change with added, modified, or removed requirements
- **WHEN** an author completes `traceability.md`
- **THEN** every affected requirement MUST have each planned code, evidence, or external-link action

#### Scenario: Negative - provider state enters the plan
- **GIVEN** a proposed traceability row with provider status, fetched title, user, or timestamp
- **WHEN** the template control evaluates the row
- **THEN** the control MUST reject the provider-state field deterministically

### Requirement: BOT-005 Authority and lifecycle separation

The templates MUST state that Buoy repository configuration owns capability authority, lifecycle, reserved IDs, and approval bindings.

#### Scenario: Positive - configuration owns authority
- **GIVEN** a proposed OpenSpec change and a separate Buoy adapter configuration
- **WHEN** the workflow describes authority
- **THEN** the workflow MUST direct authority edits to reviewed repository configuration

#### Scenario: Negative - template metadata claims authority
- **GIVEN** lifecycle or approval text only in an OpenSpec planning artifact
- **WHEN** Buoy performs a static check
- **THEN** Buoy MUST NOT treat that planning text as canonical authority

### Requirement: BOT-006 Active-change materialization boundary

The example workflow MUST explain that active OpenSpec change artifacts are plans, not Buoy standing-spec authority.

#### Scenario: Positive - candidate head contains synchronized specs
- **GIVEN** a base root and a candidate head root with synchronized standing specifications
- **WHEN** `buoy change` compares the roots
- **THEN** Buoy MUST evaluate only the materialized repository bytes and explicit change manifest

#### Scenario: Negative - active delta is passed as authority
- **GIVEN** a delta specification under `openspec/changes/<change>/`
- **WHEN** a user attempts direct OpenSpec authority extraction
- **THEN** the example MUST identify the unsupported path and expected fail-closed diagnostic

### Requirement: BOT-007 Reusable schema verification

The example schema MUST have structural controls and strict OpenSpec validation under the pinned toolchain.

#### Scenario: Positive - schema and artifact flow are valid
- **GIVEN** the complete reusable example project
- **WHEN** its focused verification command runs
- **THEN** the command MUST validate the schema and verify the required artifact dependency flow

#### Scenario: Negative - template or dependency drift occurs
- **GIVEN** an absent required field or a changed artifact dependency
- **WHEN** the focused verification command runs
- **THEN** the command MUST fail closed with a deterministic identification of the changed contract
