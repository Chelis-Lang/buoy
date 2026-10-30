## MODIFIED Requirements

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

## ADDED Requirements

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
