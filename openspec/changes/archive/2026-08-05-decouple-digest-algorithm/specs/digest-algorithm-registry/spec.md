## ADDED Requirements

### Requirement: DIG-001 Closed digest-algorithm registry
The core MUST define one closed digest-algorithm registry. Each registered member MUST bind an immutable canonical name, an immutable numeric tag, an exact output length in bytes, and a declared assurance class. The registry MUST reserve tag `2` for `xxh3-128` at 16 bytes, tag `3` for `blake3-256` at 32 bytes, and tag `4` for `sha-256` at 32 bytes. Tags `0` and `1` MUST remain permanently obsolete and MUST NOT be reassigned. The registry MUST NOT accept a member from outside the core, and the schema inventory MUST list every registered member with its exact tag and length.

#### Scenario: Positive - the schema inventory matches the registry
- **GIVEN** the closed registry and the active schema inventory
- **WHEN** the schema-inventory gate compares every registered member against every declared enum variant
- **THEN** the gate succeeds only when the names, tags, lengths, and assurance classes agree exactly in both directions

#### Scenario: Negative - an obsolete tag is reassigned
- **GIVEN** a registry definition or schema inventory row that assigns tag `0` or tag `1` to any member
- **WHEN** the schema-inventory gate validates the registry
- **THEN** the gate fails closed with a deterministic obsolete-tag diagnostic and MUST NOT accept the definition

### Requirement: DIG-002 Algorithm-bound digest representation
A semantic digest MUST carry its registered algorithm together with exactly the number of payload bytes that the algorithm declares. The canonical text form MUST be `<canonical-name>:<lowercase hexadecimal payload>`. The canonical binary form MUST encode the algorithm tag and the declared payload length, and its total envelope length MUST follow from the declared length. A digest whose payload length does not equal its algorithm's declared length MUST NOT be representable. Parsers MUST accept every registered member and MUST reject unknown names, unknown tags, obsolete tags, wrong lengths, uppercase text, nonhexadecimal digits, and truncated values.

#### Scenario: Positive - every member round trips in both forms
- **GIVEN** one valid digest for each registered member
- **WHEN** each digest is serialized to canonical text and canonical binary and parsed again
- **THEN** every round trip preserves the same algorithm, the same declared length, and all payload bytes

#### Scenario: Negative - a payload length contradicts its algorithm tag
- **GIVEN** a canonical binary value that declares the `blake3-256` tag with a 16-byte payload, or a canonical text value that declares `xxh3-128` with 64 hexadecimal digits
- **WHEN** the value is parsed
- **THEN** parsing fails with a stable digest-length diagnostic and MUST NOT truncate, pad, or reinterpret the payload

### Requirement: DIG-003 Single pinned active algorithm
Each repository MUST declare exactly one active registered member in its project configuration. The active member MUST be available in the running build, and the configuration MUST fail closed with a deterministic unavailable-algorithm diagnostic that names the missing build feature when it is not. Canonical output MUST record the active member. Two runs over identical repository bytes, identical configuration, identical schema, and identical pinned inputs MUST produce byte-identical canonical output. A repository MUST NOT declare zero active members or more than one active member.

#### Scenario: Positive - a pinned active member produces stable output
- **GIVEN** one repository snapshot, one declared active member, and its available build feature
- **WHEN** the deterministic static check runs twice under separate absolute roots
- **THEN** both runs produce byte-identical canonical output that records the same active member

#### Scenario: Negative - the configuration names an unavailable member
- **GIVEN** a project configuration that declares an active member whose build feature is absent, or that declares zero or two active members
- **WHEN** the configuration is loaded
- **THEN** loading fails closed with a deterministic diagnostic that names the exact fault and MUST NOT fall back to another member

### Requirement: DIG-004 Registry-mediated computation boundary
Every repository-owned digest MUST be computed through the core registry using the active member. Each registered member MUST reach its hashing implementation through exactly one allowlisted core adapter module, and that module MUST expose only a preimage-to-output-bytes operation for its own member. Repository code outside an allowlisted adapter module MUST NOT call a hashing implementation directly, MUST NOT contain its own hash implementation, and MUST NOT compute a digest under an inactive member. A request to compute under an inactive registered member MUST fail closed with a deterministic inactive-algorithm diagnostic.

#### Scenario: Positive - all repository hashing reaches the registry
- **GIVEN** a repository in which every digest-producing path calls the core registry and every hashing implementation sits behind its own allowlisted adapter module
- **WHEN** the hashing-boundary gate scans the repository
- **THEN** the gate succeeds with a deterministic empty finding set

#### Scenario: Negative - a second hash implementation lives outside the registry
- **GIVEN** a tool, generator, test helper, or evidence path that implements a hash by hand or calls a hashing crate outside an allowlisted adapter module, under any identifier spelling or digest prefix spelling
- **WHEN** the hashing-boundary gate scans the repository
- **THEN** the gate emits a stable outside-registry finding for the exact location and the aggregate gate fails

### Requirement: DIG-005 Cross-algorithm isolation
Every canonical digest preimage MUST bind the active member's tag together with the domain tag, the schema version, and the encoded record. A digest computed under one member MUST NOT be compared, ordered, merged, or substituted against a digest computed under another member for freshness, equality, or identity purposes. A change of active member MUST regenerate every repository-owned digest from its authoritative canonical source bytes, and MUST NOT transform, truncate, pad, or relabel the payload bytes of an existing digest.

#### Scenario: Positive - an algorithm change re-identifies from source bytes
- **GIVEN** authoritative canonical source bytes for every digest-bound object and a new declared active member
- **WHEN** the deterministic migration runs twice
- **THEN** both runs produce byte-identical records that carry the new member, preserve stable object IDs, and preserve legal lifecycle states

#### Scenario: Negative - digests from two members are compared
- **GIVEN** a stored digest under one registered member and a recomputed digest under a different registered member
- **WHEN** a freshness check, equality check, or migration compares or relabels them
- **THEN** the operation fails with a deterministic algorithm-mismatch diagnostic and MUST NOT report the binding as current

### Requirement: DIG-006 Per-member assurance class
The registry MUST declare an assurance class for each member. The class `change-detection` MUST apply to `xxh3-128`, and the class `collision-resistant` MUST apply to `blake3-256` and `sha-256`. A report MUST NOT state collision resistance or preimage resistance for a member whose class is `change-detection`. The system MUST NOT read any member class as authenticity, tamper evidence, signature strength, or proof status. Digest equality alone MUST NOT establish these properties. Static registration, freshness, execution verdict, and assurance class MUST remain separate facts under every member.

#### Scenario: Positive - a report states only the declared class
- **GIVEN** an active member with a declared assurance class and a report that cites that exact class
- **WHEN** the report is validated
- **THEN** validation succeeds and the report retains separate registration, freshness, and execution facts

#### Scenario: Negative - a claim exceeds the declared class
- **GIVEN** a report or policy that claims collision resistance for a `change-detection` member, or that claims authenticity, tamper evidence, or proof from digest equality under any member
- **WHEN** the report or policy is validated
- **THEN** validation fails with a deterministic unsupported-assurance-claim diagnostic

### Requirement: DIG-007 Cross-algorithm correspondence index
An active-member change MUST produce a correspondence index that records, for every digest-bound object, its stable object ID, its object kind, and its digest under each involved member. The index MUST be keyed by stable object ID and MUST NOT be keyed by any digest value. Every row MUST be verifiable by recomputing both digests from the object's authoritative canonical source bytes, and a verification gate MUST reject any row that recomputation contradicts. The index MUST be a deterministic generated artifact, registered as a `Derivation` with source bindings, target bindings, generator identity, configuration, and a consistency oracle. The index MUST be stored as a committed, totally ordered, line-oriented text artifact. It MUST NOT be stored in a binary container whose bytes are not reproducible from its logical content.

#### Scenario: Positive - the index is regenerated and verified from source bytes
- **GIVEN** authoritative canonical source bytes for every digest-bound object and a completed active-member change
- **WHEN** the index is generated twice and then verified by recomputing every row under both members
- **THEN** both runs produce byte-identical output in the same total order and the verification gate succeeds with an empty finding set

#### Scenario: Negative - an index row contradicts recomputation
- **GIVEN** a correspondence index containing a row whose recorded digest for either member does not equal the digest recomputed from that object's authoritative canonical source bytes, or a row keyed by a digest value rather than a stable object ID
- **WHEN** the verification gate evaluates the index
- **THEN** the gate fails closed with a deterministic index-correspondence diagnostic and MUST NOT accept the recorded value

### Requirement: DIG-008 Correspondence index carries no authority
The correspondence index MUST remain a derived observation and MUST NOT be an input to identity. Identity construction, migration, and freshness evaluation MUST NOT read the index, and MUST NOT accept an index row in place of regeneration from authoritative canonical source bytes. A missing index row MUST NOT block regeneration, and a present index row MUST NOT enable an identity that regeneration cannot produce. An object whose authoritative source bytes are unavailable MUST fail active-schema validation even when the index records a digest for it. Deleting the index MUST NOT change any canonical graph or report identity.

#### Scenario: Positive - deleting the index leaves identity unchanged
- **GIVEN** a repository snapshot with a current correspondence index
- **WHEN** the index is deleted and the deterministic static check runs again
- **THEN** the canonical graph and report bytes are unchanged and only the index derivation is reported as absent

#### Scenario: Negative - the index is used to supply a missing identity
- **GIVEN** a digest-bound object whose authoritative canonical source bytes are unavailable, and a correspondence index row that records a digest for that object under the target member
- **WHEN** migration or active-schema validation runs
- **THEN** it fails with the deterministic incomplete-migration diagnostic and MUST NOT adopt the indexed digest as the object identity

### Requirement: DIG-009 Bounded exhaustive model for algorithm transition and index non-authority
The system MUST declare one finite model that covers digest acceptance, active-member resolution, and the algorithm-transition and correspondence-index lifecycle. The model MUST identify its model ID, revision digest, state space, event alphabet, transition relation, declared bounds, and checked invariants, and the runner MUST explore every reachable state until it reaches a fixed point. The model MUST define an independent expected transition relation separately from the implementation path, and MUST compare the two outcomes at every enumerated state.

The checked invariants MUST include the following. Regeneration soundness: a transition succeeds only when authoritative canonical source bytes and every required approval are present. Index non-authority: for every enumerated state, the identity outcome MUST equal the identity outcome of the same state with every correspondence-index input removed. Index verification: a present index row that contradicts recomputation, or an index keyed by a digest value, MUST make the index gate reject, and that verdict MUST remain separate from the identity outcome. Preservation: an accepted transition preserves stable object IDs and legal lifecycle states. Idempotence: re-running an accepted transition on its own target yields an identical target.

The model MUST carry negative controls that inject a divergence for each checked invariant, and each injected control MUST make the runner fail. A run MUST report exact enumerated state counts, accepted and rejected counts, and its fixed-point result, and MUST NOT report an exhaustive claim when exploration stops at an undeclared cutoff.

#### Scenario: Positive - the model reaches a fixed point with every invariant held
- **GIVEN** the declared finite model with its exact state space, transition relation, bounds, and invariants
- **WHEN** the runner enumerates every reachable state and compares the independent model outcome against the implementation outcome
- **THEN** every state agrees, every invariant holds, and the report states `exhaustively-checked` with exact state counts and its fixed-point result within that exact model ID and revision digest

#### Scenario: Positive - index inputs cannot change the identity outcome
- **GIVEN** every enumerated state paired with the same state with all correspondence-index inputs removed
- **WHEN** the runner compares the identity outcome of each pair across the complete state space
- **THEN** the two outcomes are equal at every state, which exhaustively establishes index non-authority within the declared bounds

#### Scenario: Negative - an injected divergence is not detected
- **GIVEN** a negative control that injects an index input into the identity outcome, accepts a contradicted index row, accepts a transition with missing authoritative bytes or missing approval, or mutates a preserved stable ID in an accepted target
- **WHEN** the runner enumerates the state space under that injected control
- **THEN** the runner fails with a deterministic model-divergence or invariant-violation diagnostic naming the exact state, and a run that reports success under any injected control MUST fail the gate

#### Scenario: Negative - exploration stops before a fixed point
- **GIVEN** a run that stops at a depth or state cutoff that the model does not declare
- **WHEN** results are reported
- **THEN** the system MUST NOT report the model as exhaustively checked and MUST NOT bind an exhaustive claim to it
