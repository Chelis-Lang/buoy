## ADDED Requirements

### Requirement: OSA-001 Standing-spec authority boundary
The adapter MUST accept authority only from selected `openspec/specs/**/*.md` files. It MUST reject change artifacts as atom authority.

#### Scenario: Positive - standing requirement becomes an atom
- **GIVEN** a selected `openspec/specs/auth/spec.md` file with one valid requirement
- **WHEN** the OpenSpec adapter materializes the saved file
- **THEN** the graph contains one atom with an exact requirement source mapping

#### Scenario: Negative - active delta is selected
- **GIVEN** a selected path under `openspec/changes/add-auth/specs/`
- **WHEN** the OpenSpec adapter processes the path
- **THEN** materialization fails with a deterministic forbidden-OpenSpec-path diagnostic

### Requirement: OSA-002 Stable requirement IDs and allocation
Each accepted heading MUST use `### Requirement: <stable-id> <title>`. The append-only Buoy registry MUST reserve each stable ID exactly once.

#### Scenario: Positive - reserved ID appears once
- **GIVEN** one requirement heading with a valid ID that the registry reserves
- **WHEN** all selected standing specs are materialized
- **THEN** the adapter creates one atom with that stable ID

#### Scenario: Negative - ID is missing, duplicate, or unreserved
- **GIVEN** a requirement heading with no valid ID, a duplicate ID, or an unreserved ID
- **WHEN** the adapter materializes all selected specs
- **THEN** graph construction fails with a stable ID-specific diagnostic and no duplicate wins

### Requirement: OSA-003 Parser-backed requirement structure and spans
The adapter MUST parse requirement and scenario headings through a pinned CommonMark syntax tree. Each atom span MUST cover its complete requirement section.

#### Scenario: Positive - multibyte text precedes a requirement
- **GIVEN** a valid spec with multibyte Unicode before one requirement heading
- **WHEN** the adapter reports the requirement span
- **THEN** the byte offsets identify the exact heading, prose, scenarios, and final section boundary

#### Scenario: Negative - malformed scenario heading appears
- **GIVEN** a requirement with a scenario heading that uses three hashes instead of four
- **WHEN** the adapter parses the requirement section
- **THEN** materialization fails with a deterministic malformed-scenario diagnostic

### Requirement: OSA-004 Complete requirement revision
The atom revision MUST bind the title, normative prose, scenario names, and scenario bodies. Schema-declared Markdown layout trivia MUST remain nonsemantic.

#### Scenario: Positive - blank-line layout changes alone
- **GIVEN** two requirement sections with equal parsed content and different allowed blank-line layout
- **WHEN** the adapter creates both atom revisions
- **THEN** both revisions have the same digest while their raw source bindings differ

#### Scenario: Negative - scenario result changes
- **GIVEN** a current requirement with one positive scenario
- **WHEN** the scenario result changes by one semantic byte
- **THEN** the atom revision changes and every old code, evidence, and trace link becomes stale

### Requirement: OSA-005 Explicit authority metadata configuration
The adapter configuration MUST supply each atom kind, scope, lifecycle state, registry binding, and required approval. The adapter MUST NOT infer these fields.

#### Scenario: Positive - capability configuration is complete
- **GIVEN** a selected capability with explicit scope, default kind, lifecycle entries, and registry binding
- **WHEN** the adapter constructs its atoms
- **THEN** each atom receives only the configured metadata for its stable ID

#### Scenario: Negative - scope configuration is absent
- **GIVEN** a valid requirement under a capability with no configured scope
- **WHEN** the adapter tries to create the atom
- **THEN** materialization fails with a deterministic missing-scope diagnostic

### Requirement: OSA-006 Explicit lifecycle and approval
The default atom state MUST be `proposed`. A `ratified` atom MUST have an exact repository-owned approval binding in adapter configuration.

#### Scenario: Positive - configured ratification is accepted
- **GIVEN** a requirement with `ratified` state and a valid approval record path, digest, and authority ID
- **WHEN** the adapter validates atom authority
- **THEN** the atom is ratified with that exact approval binding

#### Scenario: Negative - archive state implies ratification
- **GIVEN** an archived OpenSpec change and no repository-owned approval binding for its standing requirement
- **WHEN** the adapter materializes the standing requirement
- **THEN** the atom remains proposed and MUST NOT gain review-only assurance

### Requirement: OSA-007 Total OpenSpec disposition
Every selected syntax node MUST become a semantic object, a schema-defined nonnormative disposition, or a deterministic diagnostic. Unsupported syntax MUST fail closed.

#### Scenario: Positive - purpose text is nonnormative
- **GIVEN** a selected OpenSpec file with purpose text and one valid requirement
- **WHEN** the adapter processes the file
- **THEN** the purpose text is nonnormative and the requirement creates one atom

#### Scenario: Negative - unsupported format version appears
- **GIVEN** a selected file that declares an unsupported OpenSpec format version
- **WHEN** the adapter processes the file
- **THEN** graph construction fails with a deterministic unsupported-format diagnostic

### Requirement: OSA-008 Deterministic offline materialization
Byte-identical saved specs and configuration MUST produce byte-identical adapter output. Root, locale, traversal order, provider state, and network state MUST not affect output.

#### Scenario: Positive - separate roots have equal output
- **GIVEN** equal saved specs and configuration under two different absolute roots
- **WHEN** both adapter runs use different traversal orders and locales
- **THEN** both runs produce byte-identical graph fragments, source maps, dispositions, and diagnostics

#### Scenario: Negative - issue state changes during materialization
- **GIVEN** unchanged saved inputs and different live issue or pull-request state
- **WHEN** the OpenSpec adapter runs offline
- **THEN** the adapter output remains byte-identical and contains no fetched provider facts

### Requirement: OSA-009 Requirement-level traceability scope
The first adapter version MUST link code and evidence to requirement atom revisions. It MUST NOT report separate scenario coverage.

#### Scenario: Positive - test links to the requirement revision
- **GIVEN** a test carrier that binds the current revision of an OpenSpec requirement atom
- **WHEN** registration and freshness are validated
- **THEN** the report shows a fresh requirement-level test link within its exact declared scope

#### Scenario: Negative - one test name implies all scenario coverage
- **GIVEN** a test whose name matches one OpenSpec scenario but whose carrier binds only the requirement
- **WHEN** assurance is aggregated
- **THEN** the report MUST NOT claim separate or complete scenario-level coverage
