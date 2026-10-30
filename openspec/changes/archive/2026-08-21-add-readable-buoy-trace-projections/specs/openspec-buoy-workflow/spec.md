## MODIFIED Requirements

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

## ADDED Requirements

### Requirement: OBW-008 Historical completion snapshot

The completed traceability artifact MUST record one historical snapshot for each implemented link action. Each row MUST identify the requirement, canonical record locator, consumer commit, and final static result.

The snapshot MUST state that Buoy remains the current-state authority. It MUST NOT copy the immutable record body or provider status.

Before any implemented link action exists, the completion table MUST remain empty and MUST NOT contain placeholder identity values.

#### Scenario: Positive - completed binding records its pointer

- **GIVEN** one successful binding operation and current static result
- **WHEN** the change completes its traceability artifact
- **THEN** the completion row records the exact object locator, consumer commit, and current result as historical data

#### Scenario: Positive - no completed relation keeps an empty snapshot

- **GIVEN** one change before any implementation relation is complete
- **WHEN** the completion snapshot is written
- **THEN** its table remains empty and contains no placeholder identity

#### Scenario: Negative - archived snapshot claims current authority

- **GIVEN** one archived change whose recorded binding later becomes stale
- **WHEN** a reader uses the completion snapshot without a current Buoy query
- **THEN** the artifact states its historical scope and MUST NOT represent the old result as current
