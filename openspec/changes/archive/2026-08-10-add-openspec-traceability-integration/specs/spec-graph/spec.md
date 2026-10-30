## MODIFIED Requirements

### Requirement: SPG-005 Closed graph references
The system MUST reject each dangling internal reference. This rule covers atoms, source items, external trace links, carriers, implementations, and all governed objects.

A versioned schema MUST define each external reference class. A trace target remains informational and MUST NOT become an internal object reference.

#### Scenario: Positive - all references resolve
- **GIVEN** a graph in which every internal reference resolves to one canonical object of the expected kind
- **WHEN** graph closure validation runs
- **THEN** closure validation succeeds

#### Scenario: Negative - carrier references an unknown oracle
- **GIVEN** a carrier whose oracle ID does not resolve
- **WHEN** graph closure validation runs
- **THEN** validation fails with a deterministic dangling-reference diagnostic

#### Scenario: Negative - trace link references an unknown atom revision
- **GIVEN** an external trace link whose atom ID or revision does not resolve
- **WHEN** graph closure validation runs
- **THEN** validation fails and MUST NOT keep the link as unbound navigation data

### Requirement: SPG-007 Parser-backed source spans
The system MUST derive semantic object boundaries from supported syntax trees. Each source mapping MUST bind a declared source-item kind and normalized original-byte span.

Spans MUST remain correct for multibyte Unicode, fenced Markdown, nested constructs, Rust items, and OpenSpec requirement sections.

#### Scenario: Positive - Unicode span is precise
- **GIVEN** valid source containing multibyte Unicode before an atom declaration
- **WHEN** the declaration is extracted
- **THEN** its normalized diagnostic span identifies the exact declaration under the schema offset convention

#### Scenario: Positive - OpenSpec requirement span is complete
- **GIVEN** a valid OpenSpec requirement with normative prose and two scenarios
- **WHEN** the OpenSpec adapter extracts the atom
- **THEN** its source span covers the requirement heading, prose, both scenarios, and no next requirement

#### Scenario: Negative - fence contains atom-like syntax
- **GIVEN** a Markdown code fence containing text that resembles an atom authority
- **WHEN** the Markdown adapter extracts normative atoms
- **THEN** the fenced text does not become authority unless the schema marks that fence as normative

#### Scenario: Negative - source item uses an undeclared kind
- **GIVEN** a parser-backed source span with a kind absent from the adapter descriptor
- **WHEN** source-map validation runs
- **THEN** validation fails with a deterministic unsupported-kind diagnostic

### Requirement: SPG-011 Total adapter disposition
Each adapter MUST totally classify every selected path and supported syntax class. Each semantic source item MUST use a kind from the adapter descriptor.

Each selected syntax node MUST produce a semantic object, an explicit nonnormative disposition, or a deterministic diagnostic. Unsupported or malformed authority syntax MUST fail closed.

An authority-required root with no authorities MUST fail. An adapter MUST NOT report an empty successful graph for that root.

#### Scenario: Positive - ordinary prose is explicitly nonnormative
- **GIVEN** a selected Markdown document containing ordinary prose outside authoritative syntax
- **WHEN** the configured adapter processes the document
- **THEN** the prose receives the schema-defined nonnormative disposition and creates no atom

#### Scenario: Positive - OpenSpec purpose and requirement are both classified
- **GIVEN** a selected standing spec with purpose text and one valid requirement
- **WHEN** the OpenSpec adapter processes the document
- **THEN** the purpose is nonnormative and the requirement creates one semantic atom

#### Scenario: Negative - unsupported syntax version is skipped
- **GIVEN** a selected input that declares an unsupported authority syntax version
- **WHEN** the configured adapter processes the input
- **THEN** graph construction fails with a deterministic unsupported-syntax-version diagnostic

#### Scenario: Negative - expected authority root is empty
- **GIVEN** a policy that selects a root that must contain normative authorities but extraction produces none
- **WHEN** graph validation runs
- **THEN** validation fails and MUST NOT report vacuous success

#### Scenario: Negative - OpenSpec change path is selected as authority
- **GIVEN** a selected OpenSpec proposal, design, task, active delta, or archived change path
- **WHEN** the OpenSpec adapter processes the path
- **THEN** materialization fails with a deterministic forbidden-OpenSpec-path diagnostic
