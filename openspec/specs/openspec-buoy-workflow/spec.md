# OpenSpec Buoy Workflow

## Purpose

Define the project workflow for OpenSpec planning, traceability, and separate Buoy enforcement.

## Requirements

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
The traceability artifact MUST list each affected requirement ID and each planned code, evidence, issue, pull-request, document, or implementation-trace action.

Each planned implementation row MUST record the stable relation ID, consumer repository, logical target selector, action, owner, and focused Buoy command. Planned rows MUST omit unresolved current digests.

A change without a planned compact relation MUST use an empty implementation-relation table. It MUST NOT use placeholder IDs, selectors, or digests.

#### Scenario: Positive - affected requirement has complete actions
- **GIVEN** a change that adds one requirement, code link, test carrier, and issue link
- **WHEN** the traceability artifact is created
- **THEN** it records the stable IDs, target classes, actions, owners, target selectors, and required Buoy commands

#### Scenario: Positive - implementation relation is planned before binding
- **GIVEN** one reserved requirement ID and one reviewed implementation target
- **WHEN** the traceability artifact is completed before implementation
- **THEN** it records the stable relation ID and target selector without a fabricated record digest

#### Scenario: Positive - change has no compact relation
- **GIVEN** one change with code actions and no planned compact implementation relation
- **WHEN** the traceability artifact is completed
- **THEN** its implementation-relation table is empty and contains no placeholder identity

#### Scenario: Negative - requirement change omits link review
- **GIVEN** a modified requirement with current links and no traceability action for those links
- **WHEN** the Buoy workflow control validates the artifact
- **THEN** validation fails with a deterministic missing-link-review diagnostic

#### Scenario: Negative - planned row fabricates current identity
- **GIVEN** one planned link row with a record, requirement, subject, or atom digest that no completed binding produced
- **WHEN** the Buoy workflow control validates the artifact
- **THEN** validation fails with a deterministic premature-completion diagnostic

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

### Requirement: OBW-008 Historical completion snapshot

The completed traceability artifact MUST include one marker-delimited `buoy/trace-publication-evidence/v1` Markdown block.

Generated evidence MUST identify the requirement, canonical record locator, logical consumer target, final graphs, external link, page digest, and final static result.

A separate human completion block MUST retain the consumer commit, owner, review decision, and operation scope. Generated evidence MUST NOT fabricate those facts.

Workflow control MUST parse both blocks strictly. It MUST retain read-only compatibility with archived legacy four-column completion tables.

The snapshot MUST state that Buoy remains the current-state authority. It MUST NOT copy the immutable record body or provider status.

Before any implemented link action exists, the completion section MUST remain empty and MUST NOT contain placeholder identity values.

#### Scenario: Positive - completed binding receives generated evidence

- **GIVEN** one successful binding, current publication plan, current page, and current external link
- **WHEN** Buoy renders completion evidence
- **THEN** the marker-delimited block records exact identities and the separate human block retains human completion facts

#### Scenario: Positive - no completed relation keeps an empty snapshot

- **GIVEN** one change before any implementation relation is complete
- **WHEN** the completion snapshot is written
- **THEN** its generated evidence section remains empty and contains no placeholder identity

#### Scenario: Negative - human transcribes a conflicting digest

- **GIVEN** one completion table whose manual digest differs from Buoy-generated evidence
- **WHEN** workflow conformance runs
- **THEN** validation fails with a deterministic completion-evidence-drift diagnostic

#### Scenario: Negative - archived snapshot claims current authority

- **GIVEN** one archived change whose recorded publication later becomes stale
- **WHEN** a reader uses its completion snapshot without a current Buoy query
- **THEN** the artifact states its historical scope and MUST NOT represent the old result as current

### Requirement: OBW-009 Stable publication policy excludes volatile identities

A planned publication row MUST record its stable publication ID, typed selector, relation policy, owner repository, logical outputs, and focused Buoy commands.

Planning data MUST express current-revision and current-record invariants. It MUST NOT copy a volatile atom, node, graph, page, or record digest before completion.

Exact volatile identities MUST enter only generated completion evidence after the corresponding current objects exist.

#### Scenario: Positive - plan declares a current atom invariant

- **GIVEN** one planned external link before its requirement edit is final
- **WHEN** the traceability plan is written
- **THEN** it requires the current semantic atom revision without fabricating its future digest

#### Scenario: Negative - planning row pins an ungenerated atom digest

- **GIVEN** one planned publication row with a proposed atom or graph digest
- **WHEN** workflow conformance runs
- **THEN** validation fails with a deterministic premature-publication-identity diagnostic

### Requirement: OBW-010 Buoy owns mechanical publication operations

OpenSpec guidance MUST direct page, external-link, conformance, and evidence operations to the Buoy publication command family.

A consumer repository MUST NOT need a repository-specific parser, page writer, link synchronizer, report checker, or Buoy source-fetch wrapper.

OpenSpec retains normative prose, task completion, owners, review decisions, and approvals. Buoy generation MUST NOT supply those human facts.

#### Scenario: Positive - downstream task invokes pinned Buoy package

- **GIVEN** one consumer with a publication plan and pinned Buoy package
- **WHEN** its focused Devenv task runs
- **THEN** the task passes roots and plan bytes to Buoy without consumer-owned publication logic

#### Scenario: Negative - consumer shell validates JSON fragments

- **GIVEN** one consumer publication check that searches trace JSON as untyped text
- **WHEN** workflow ownership conformance runs
- **THEN** the check fails the ownership policy and cannot satisfy publication conformance
