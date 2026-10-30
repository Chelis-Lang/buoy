# formal-proof-records Specification

## Purpose

Define correspondence approval, complete and reproducible proof bindings, exact invalidation, fail-closed acceptance, and narrow formal trust boundaries.

## Requirements

### Requirement: FPR-001 Human-approved formalization correspondence
A formal property MUST identify the exact atom revisions it refines and a review artifact approving the prose-to-property correspondence. The system MUST report that approval as review evidence rather than proof of semantic equivalence.

#### Scenario: Positive - correspondence is approved
- **GIVEN** a formal property linked to exact atom revisions with a valid approval artifact
- **WHEN** formal-property registration is validated
- **THEN** the property is eligible for proof execution under its declared assumptions

#### Scenario: Negative - property is inferred from prose
- **GIVEN** a generated formal property without an approval artifact
- **WHEN** formal-property registration is validated
- **THEN** the property cannot support a formally-proved atom claim

### Requirement: FPR-002 Complete proof bindings
A proof record MUST bind exact atom revisions, formal property digest, implementation representation digest, assumption IDs and digests, translator identities and configurations, verifier identity and configuration, external model digests, proof-source digest, normalized verdict, scope, and exclusions.

#### Scenario: Positive - complete current bindings
- **GIVEN** an accepted verifier result whose proof record contains every required current binding
- **WHEN** the proof record is validated
- **THEN** it is complete and eligible for freshness evaluation

#### Scenario: Negative - external model binding is absent
- **GIVEN** an Aeneas proof that relies on a handwritten external Rust model absent from its proof record
- **WHEN** the proof record is validated
- **THEN** validation fails and no formally-proved claim is emitted

### Requirement: FPR-003 Exact proof invalidation
A proof record MUST become stale when any bound atom, property, implementation representation, assumption, translator, verifier, configuration, external model, generated artifact, or proof source changes.

#### Scenario: Positive - all proof bindings remain equal
- **GIVEN** a previously accepted proof record whose current bindings all equal the recorded bindings
- **WHEN** proof freshness is evaluated
- **THEN** the proof remains fresh

#### Scenario: Negative - verifier changes
- **GIVEN** a previously fresh proof record
- **WHEN** the verifier version or executable digest changes
- **THEN** the proof becomes stale even if the property and implementation are unchanged

### Requirement: FPR-004 Proof acceptance is fail-closed
The system MUST report `formally-proved` only when formalization correspondence is approved, every binding is complete and fresh, the pinned verifier runs offline and accepts, required negative proof fixtures fail for the intended reason, and assumptions and exclusions are present.

#### Scenario: Positive - current proof is accepted
- **GIVEN** a complete fresh proof record and passing positive and negative proof oracles under pinned tools
- **WHEN** formal assurance is aggregated
- **THEN** the exact scoped property may be reported as `formally-proved`

#### Scenario: Negative - stale proof command still passes
- **GIVEN** a verifier command that accepts current proof source but the recorded atom binding is stale
- **WHEN** formal assurance is aggregated
- **THEN** the result is reported stale and MUST NOT be reported as formally proved

### Requirement: FPR-005 Explicit trust boundaries and exclusions
Every formal claim MUST state trusted tools, assumptions, implementation representation, and excluded downstream semantics. A proof over Rust source, MIR, LLBC, or compiler IR MUST NOT imply correctness of rustc, LLVM, emitted C, HIP, Metal, machine code, or external models without separate bound evidence.

#### Scenario: Positive - source-level scope is explicit
- **GIVEN** a Verus proof over supported Rust source
- **WHEN** its proof report is rendered
- **THEN** the report names Verus and Z3 trust and excludes unverified rustc and LLVM translation

#### Scenario: Negative - IR proof claims backend correctness
- **GIVEN** a proof over an internal compiler IR without translation validation or verified backend evidence
- **WHEN** its claim is rendered
- **THEN** the system rejects a claim that emitted backend code is formally proved

### Requirement: FPR-006 Supported verified-kernel subset
The initial formally verified integrity kernel MUST use safe sequential Rust and MUST isolate unsafe code, concurrency, filesystem access, process execution, and parser adapters outside the proved functions. Aeneas-targeted code MUST avoid unsupported nested-loop exits, and all external definitions MUST have explicit models and bindings.

#### Scenario: Positive - pure lifecycle function
- **GIVEN** a total safe sequential lifecycle transition function in the selected verifier subset
- **WHEN** the formal lane verifies its transition invariant
- **THEN** the proof record may bind that exact executable function and property

#### Scenario: Negative - proof target uses an unsupported effect
- **GIVEN** a proposed proof target that performs filesystem access or concurrency inside the integrity function
- **WHEN** proof eligibility is evaluated
- **THEN** the function is excluded from the initial verified kernel until a sound supported model is approved

### Requirement: FPR-007 Reproducible formal toolchain
The formal toolchain MUST use immutable offline-resolvable pins and integrity digests for Verus, Z3, Charon, Aeneas, the selected proof backend, external models, and generated templates that affect a claim.

#### Scenario: Positive - pinned offline verification
- **GIVEN** all selected formal tools and models are available under immutable verified pins
- **WHEN** the formal oracle runs without network access
- **THEN** it produces the schema-defined normalized verdict and reproducible proof record

#### Scenario: Negative - live branch dependency
- **GIVEN** a proof workflow that resolves a verifier or model from an unpinned live branch
- **WHEN** proof acceptance is evaluated
- **THEN** the workflow is nonreproducible and cannot support a formally-proved claim

### Requirement: FPR-008 Formal lane selection by proved spikes
Before selecting a primary formal lane, the project MUST implement equivalent nonproduction proof spikes for lifecycle, decision-surface completeness, carrier and derivation freshness, coverage-policy and debt/waiver/reaffirmation no-laundering, change-impact closure, and canonical field encoding in Verus and AeneasVerif or record a reviewed blocker that makes one lane infeasible.

#### Scenario: Positive - both spikes are compared
- **GIVEN** equivalent required integrity-kernel proof obligations exercised under pinned Verus and AeneasVerif toolchains
- **WHEN** maintainability, faithfulness, assumptions, subset support, and CI cost are reviewed
- **THEN** an architecture decision selects at least one primary lane with explicit rationale

#### Scenario: Negative - primary lane is selected by preference alone
- **GIVEN** no proved spike or reviewed infeasibility record for one candidate lane
- **WHEN** a primary formal lane is proposed
- **THEN** selection remains incomplete and production formal claims cannot depend on it
