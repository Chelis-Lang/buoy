## ADDED Requirements

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
The system MUST reject dangling atom, surface, disposition, carrier, implementation, derivation, coverage-policy, debt, waiver, oracle, model, property, assumption, proof, and replacement references unless a versioned schema explicitly defines that external reference class.

#### Scenario: Positive - all references resolve
- **GIVEN** a graph in which every internal reference resolves to one canonical object of the expected kind
- **WHEN** graph closure validation runs
- **THEN** closure validation succeeds

#### Scenario: Negative - carrier references an unknown oracle
- **GIVEN** a carrier whose oracle ID does not resolve
- **WHEN** graph closure validation runs
- **THEN** validation fails with a deterministic dangling-reference diagnostic

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
The system MUST derive atom and carrier boundaries from supported syntax trees and MUST report normalized source spans that remain correct for multibyte Unicode and fenced or nested Markdown constructs.

#### Scenario: Positive - Unicode span is precise
- **GIVEN** valid source containing multibyte Unicode before an atom declaration
- **WHEN** the declaration is extracted
- **THEN** its normalized diagnostic span identifies the exact declaration under the schema's offset convention

#### Scenario: Negative - fence contains atom-like syntax
- **GIVEN** a Markdown code fence containing text that resembles an atom authority
- **WHEN** the Markdown adapter extracts normative atoms
- **THEN** the fenced text does not become an authority unless the schema explicitly marks that fence as normative

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
For every configured selected path and supported syntax class, an adapter MUST produce supported semantic objects, an explicit schema-defined nonnormative disposition, or a deterministic malformed or unsupported diagnostic. Unsupported schema versions, malformed authority-like constructs, and an expected authority scope that yields no authorities MUST NOT be silently treated as an empty successful graph.

#### Scenario: Positive - ordinary prose is explicitly nonnormative
- **GIVEN** a selected Markdown document containing ordinary prose outside authoritative syntax
- **WHEN** the configured adapter processes the document
- **THEN** the prose receives the schema-defined nonnormative disposition and creates no atom

#### Scenario: Negative - unsupported syntax version is skipped
- **GIVEN** a selected input declaring an unsupported authority syntax version
- **WHEN** the configured adapter processes the input
- **THEN** graph construction fails with a deterministic unsupported-syntax-version diagnostic

#### Scenario: Negative - expected authority root is empty
- **GIVEN** policy selecting a root that is required to contain normative authorities but extraction produces none
- **WHEN** graph validation runs
- **THEN** validation fails and MUST NOT report vacuous success
