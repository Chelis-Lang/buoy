## ADDED Requirements

### Requirement: SCF-001 Complete frozen-v1 schema parity
The system MUST maintain a reviewed machine-readable inventory of every frozen v1 object, nested value, enum tag, field tag, required/optional field, collection kind, digest domain, reference kind, lifecycle state, report fact, and diagnostic family, and MUST fail the parity gate when the public implementation or executable fixture manifest omits or reinterprets an inventory entry.

#### Scenario: Positive - implementation covers the complete inventory
- **GIVEN** the reviewed v1 inventory and the current public values, encoders, validators, reports, and fixture manifest
- **WHEN** schema parity is checked
- **THEN** every inventory entry maps to one exact implementation and positive/negative fixture contract

#### Scenario: Negative - generic wrapper omits required fields
- **GIVEN** a v1 object implemented only as stable ID plus opaque payload while the inventory requires semantic fields and references
- **WHEN** schema parity is checked
- **THEN** the gate fails with deterministic missing-field and missing-validator diagnostics

### Requirement: SCF-002 Validated complete construction
Every public semantic value MUST have private fields and a validated constructor that either returns one complete schema-valid value or deterministic structured construction errors. A constructor MUST NOT accept a caller-supplied object kind inconsistent with the concrete type, silently default a required field, deduplicate an invalid set, or use an empty string for absence.

#### Scenario: Positive - complete policy construction
- **GIVEN** every required coverage-policy field, explicit options/empty sets, correct enum values, and canonical references
- **WHEN** the policy constructor runs
- **THEN** it returns one complete value whose digest-bound fields match the v1 inventory

#### Scenario: Negative - mismatched metadata kind
- **GIVEN** construction of an `EvidenceCarrier` using metadata tagged as `Oracle`
- **WHEN** validated construction runs
- **THEN** construction fails and MUST NOT create a value later repaired by graph validation

### Requirement: SCF-003 Stable identities, registries, lifecycle, and source maps
The graph MUST implement the complete append-only ID registry, namespace allocation history, atom/tombstone separation, policy/debt/waiver terminal history, parsed-item identities, source-map entries, source roles, approval bindings, and exact lifecycle transitions defined by v1. Terminal records MUST remain permanently reserved across base/head validation.

#### Scenario: Positive - terminal history and source movement coexist
- **GIVEN** a retained withdrawn tombstone, unchanged final revision, complete registry allocation, and an unrelated live atom whose authority moved without semantic edits
- **WHEN** base/head history and source maps are validated
- **THEN** the terminal ID remains reserved while the moved atom preserves its revision and changes only source-map/complete-graph identity

#### Scenario: Negative - terminal governance record disappears
- **GIVEN** a base graph containing a discharged waiver or retired policy and a head graph omitting that terminal record
- **WHEN** lifecycle history is validated
- **THEN** validation fails with a deterministic terminal-history-removal diagnostic

### Requirement: SCF-004 Explicit current artifact bindings
Repository-artifact bindings in derivations and proofs MUST be evaluated against an explicit canonical snapshot/artifact index. The system MUST NOT assume a repository artifact is current merely because its path and recorded digest are structurally well formed.

#### Scenario: Positive - bound artifact matches current snapshot
- **GIVEN** a repository-artifact binding whose canonical path, raw-artifact domain, media type, representation, and full digest match the selected snapshot
- **WHEN** derivation or proof freshness is evaluated
- **THEN** that binding is current while execution and assurance remain independently evaluated

#### Scenario: Negative - selected path has new bytes
- **GIVEN** a proof or derivation retaining a prior artifact digest while the selected snapshot contains different bytes at the same canonical path
- **WHEN** freshness is evaluated
- **THEN** the binding is stale and MUST NOT support consistency or formal assurance

### Requirement: SCF-005 Complete formal proof bindings
A production proof record MUST bind exact atom revisions, approved property, implementation representation, assumptions, translator/verifier/solver executable digests, wrapper and configuration artifacts, external models, generated artifacts, proof sources, negative-control source and expected diagnostic, normalized verdict, scope, and exclusions. Any bound change MUST invalidate freshness.

#### Scenario: Positive - selected Verus proof is exactly current
- **GIVEN** the current executable implementation source, Verus proof, bundled Z3, wrappers, configuration, assumptions, negative control, scope, and exclusions all match the proof record
- **WHEN** proof freshness and acceptance are evaluated
- **THEN** only the exact approved property may be reported `formally-proved`

#### Scenario: Negative - solver executable changes
- **GIVEN** an otherwise accepted proof whose bundled Z3 executable digest differs from its record
- **WHEN** proof freshness is evaluated
- **THEN** the proof becomes stale even if the verifier command still reports success

### Requirement: SCF-006 Deterministic construction and parity diagnostics
Schema construction, parity, registry, lifecycle, source-map, artifact-binding, and proof-binding failures MUST use stable structured diagnostic codes and schema-defined total ordering without raw host paths or tool text in canonical output.

#### Scenario: Positive - invalid objects arrive in different orders
- **GIVEN** the same set of malformed values supplied in different insertion orders and absolute roots
- **WHEN** construction/parity diagnostics are rendered
- **THEN** canonical diagnostic bytes are identical

#### Scenario: Negative - constructor returns parser text as identity
- **GIVEN** a malformed object whose parser emits host/version-specific prose
- **WHEN** canonical diagnostics are built
- **THEN** the prose is confined to a noncanonical detail channel and MUST NOT affect diagnostic identity or ordering
