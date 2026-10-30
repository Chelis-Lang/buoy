# spec-graph Specification

## Purpose

Define explicit atom authority, stable identities and lifecycle, syntax-attached evidence, closed references, source spans, complete governed surfaces, derivations, and total adapters.

## Requirements

### Requirement: SPG-001 Explicit atom authority
The system MUST recognize a live requirement atom only when one syntactically explicit declaration is its authority. References, summaries, examples, carriers, reports, comments, strings, and heading proximity MUST NOT create or redefine authority.

#### Scenario: Positive - one explicit authority
- **GIVEN** a valid specification with one explicit authority declaration for a live atom ID
- **WHEN** the repository graph is built
- **THEN** the graph contains that atom with the declared authority location

#### Scenario: Negative - duplicate authority
- **GIVEN** two explicit authority declarations for the same live atom ID
- **WHEN** the repository graph is built
- **THEN** construction fails with a deterministic duplicate-authority diagnostic and neither declaration wins

### Requirement: SPG-002 Stable identity and semantic revision
The system MUST use a stable atom ID for identity and a versioned full digest for semantic revision. The atom revision digest MUST cover canonical normative content and semantic metadata and MUST exclude the authority source location.

#### Scenario: Positive - unchanged atom moves
- **GIVEN** an atom whose normative content and semantic metadata are unchanged
- **WHEN** its sole authority is moved to another valid repository-relative location
- **THEN** its atom ID and revision digest remain unchanged while source-map and complete graph identities change

#### Scenario: Negative - semantic edit retains old digest
- **GIVEN** an atom whose normative content or semantic metadata changes
- **WHEN** a carrier presents the prior revision digest
- **THEN** the carrier is stale and MUST NOT satisfy current coverage

### Requirement: SPG-003 Closed lifecycle and tombstones
The system MUST permit only `proposed -> ratified`, `proposed -> withdrawn`, `ratified -> superseded`, and `ratified -> withdrawn` transitions. Superseded and withdrawn IDs MUST remain permanently reserved by tombstones containing the final revision and terminal state.

#### Scenario: Positive - ratified atom is superseded
- **GIVEN** a ratified atom and valid replacement atom references
- **WHEN** the atom transitions to superseded
- **THEN** a tombstone reserves the retired ID and records the final revision and replacements

#### Scenario: Negative - retired ID is reused
- **GIVEN** a tombstone for a withdrawn or superseded atom ID
- **WHEN** a new live declaration attempts to reuse that ID
- **THEN** graph validation fails with a deterministic retired-ID diagnostic

### Requirement: SPG-004 Syntax-attached evidence carriers
The system MUST count an evidence carrier only when valid metadata is attached by a supported parser to an eligible executable test, finite-model, structural-check, or verifier item.

#### Scenario: Positive - metadata is attached to a test item
- **GIVEN** valid carrier metadata attached to an eligible parsed test item
- **WHEN** the syntax adapter extracts carriers
- **THEN** the carrier is registered with its item identity, role, oracle, atom ID, and expected atom digest

#### Scenario: Negative - atom text appears without attachment
- **GIVEN** an atom ID in a comment, string, prose paragraph, code fence, file-level annotation, or unattached attribute
- **WHEN** the syntax adapter extracts carriers
- **THEN** no evidence carrier is registered for that occurrence

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

### Requirement: SPG-006 Implementation links are navigational
The system MUST keep implementation links separate from evidence carriers. An implementation link MUST support navigation and impact analysis but MUST NOT satisfy behavioral coverage or proof obligations.

#### Scenario: Positive - implementation link is reported
- **GIVEN** a valid implementation link from an atom to a parsed source item
- **WHEN** the graph report is rendered
- **THEN** the source item is available for navigation without changing evidence coverage

#### Scenario: Negative - implementation link is the only link
- **GIVEN** a ratified atom with only an implementation link and no required evidence carrier
- **WHEN** coverage is evaluated
- **THEN** the atom remains behaviorally uncovered

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

### Requirement: SPG-008 Governed decision-surface completeness
The system MUST represent a governed decision surface from a versioned finite member domain with a canonical domain digest and closed member-key and disposition schemas. Every expected member MUST occur exactly once, and missing, duplicate, or unknown members MUST fail closed.

#### Scenario: Positive - every declared member has one disposition
- **GIVEN** a decision surface whose canonical domain contains four members and whose declaration contains each member exactly once with a schema-valid disposition
- **WHEN** surface completeness is validated
- **THEN** completeness succeeds with the declared domain and surface digests

#### Scenario: Negative - expected member is absent
- **GIVEN** a decision surface declaration that omits one member from its canonical finite domain
- **WHEN** surface completeness is validated
- **THEN** validation fails with a deterministic missing-surface-member diagnostic

#### Scenario: Negative - undeclared member is substituted
- **GIVEN** a decision surface declaration containing an unknown member key in place of an expected member
- **WHEN** surface completeness is validated
- **THEN** validation reports both the unknown and missing members and MUST NOT treat the row count as completeness

### Requirement: SPG-009 Authority-bound surface dispositions
Every governed surface member MUST have exactly one schema-defined disposition and, when required by its coverage policy, MUST bind a live atom ID and exact revision digest. A disposition or implementation state MUST NOT be inferred from source behavior, an implementation link, or the absence of a member, and disposition authority MUST NOT count as behavioral evidence.

#### Scenario: Positive - rejection is explicitly authorized
- **GIVEN** a surface member with a schema-valid rejected disposition bound to the current revision of a live atom
- **WHEN** disposition authority is validated
- **THEN** the member is normatively dispositioned without gaining tested or implemented assurance

#### Scenario: Negative - implementation is the only authority
- **GIVEN** a surface member represented by implementation code but lacking the atom binding required by policy
- **WHEN** disposition authority is validated
- **THEN** the member remains normatively undecided and validation fails

### Requirement: SPG-010 Digest-bound derivation records
The system MUST represent an intentional generated or mirrored artifact relation with a `Derivation` that binds exact source object and byte digests, target artifact digests, generator or consistency-checker identity and configuration, and its oracle. Derivation freshness MUST remain separate from the oracle verdict and from behavioral assurance.

#### Scenario: Positive - generated target has current bindings
- **GIVEN** a derivation whose source, target, generator, configuration, and oracle bindings match the current graph
- **WHEN** derivation freshness is evaluated
- **THEN** the derivation is fresh while consistency execution remains independently reported

#### Scenario: Negative - mirrored target changes alone
- **GIVEN** a previously fresh derivation
- **WHEN** the target artifact changes without a corresponding current derivation binding
- **THEN** the derivation becomes stale and MUST NOT report the copies as structurally consistent

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

### Requirement: SPG-012 Content-addressed implementation binding nodes

The graph MUST represent each compact implementation binding as one immutable binding node identified by its complete digest. The node MUST retain exact source and authority references.

The binding node MUST remain navigational. It MUST NOT satisfy evidence, execution, or proof obligations.

#### Scenario: Positive - fresh binding supports navigation
- **GIVEN** one current binding node with closed source and atom references
- **WHEN** the graph report is rendered
- **THEN** the report exposes its traversal links without adding an assurance class

#### Scenario: Negative - binding is treated as evidence
- **GIVEN** one ratified atom with only a fresh content-addressed implementation binding
- **WHEN** coverage aggregation runs
- **THEN** the atom remains behaviorally uncovered

### Requirement: SPG-013 Binding impact propagation

Change analysis MUST report a changed implementation binding when either referenced source node or the atom revision changes. Impact traversal MUST follow the binding node to dependent governed objects.

#### Scenario: Positive - implementation node changes
- **GIVEN** base and head graphs with different implementation nodes for one stable relation
- **WHEN** change analysis runs
- **THEN** the implementation binding is directly changed and its dependents are transitively impacted

#### Scenario: Negative - source change is hidden by attribute reuse
- **GIVEN** a changed implementation item with its old compact attribute
- **WHEN** change and freshness validation run
- **THEN** validation reports the stale binding and MUST NOT report an unchanged relation
