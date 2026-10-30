## MODIFIED Requirements

### Requirement: XXH-001 Sole repository-owned hashing algorithm
The system MUST compute every hash under a repository-owned schema or workflow through the core digest-algorithm registry, using the one declared active member. The registry MUST fix the complete parameters of each member, including for `xxh3-128` the seed `0` and the standard XXH3 default secret. The system MUST retain the full declared output width of the active member, MUST encode the digest bytes in big-endian order, and MUST NOT compute a repository-owned digest under any inactive member or under any algorithm, seed, secret, or output width that the registry does not declare.

#### Scenario: Positive - the same canonical preimage is hashed twice
- **GIVEN** the same domain-separated canonical preimage is supplied to two conforming project hashing paths under one declared active member
- **WHEN** each path computes its repository-owned digest through the registry
- **THEN** each emits the same full-width result for that member using the member's exact registered parameters

#### Scenario: Negative - a project path selects another algorithm
- **GIVEN** a project-controlled digest, fingerprint, cache key, fixture generator, or reference path configured for an inactive registered member, an unregistered algorithm, a truncated output width, a custom seed, or a custom secret
- **WHEN** the hashing-policy gate evaluates the path
- **THEN** the gate fails closed with a deterministic prohibited-hash-algorithm diagnostic

### Requirement: XXH-002 Canonical XXH3-128 identity profile
The active schema MUST identify repository-owned digests with semantic schema ID `spec-provenance/v3`, encoding version `3`, the active member's registered algorithm tag, and canonical text `<canonical-name>:<lowercase hexadecimal payload>`. Binary digest values MUST contain exactly the member's declared byte count in the member's official canonical order, which for `xxh3-128` is the high 64-bit half in big-endian order followed by the low 64-bit half in big-endian order. Parsers MUST accept every registered member and MUST reject obsolete tags, unknown tags, unknown names, wrong lengths, uppercase or nonhexadecimal text, and truncated values. When unequal canonical preimages with the same digest are simultaneously available, construction MUST fail with a deterministic observed-collision diagnostic.

#### Scenario: Positive - canonical binary and text forms agree
- **GIVEN** a valid full-width result for each registered member under the version 3 schema
- **WHEN** each result is serialized to binary and canonical text and parsed again
- **THEN** both round trips preserve the same algorithm tag and all declared digest bytes in that member's official canonical order

#### Scenario: Negative - obsolete or malformed digest is supplied
- **GIVEN** an active-schema input containing an obsolete tag `0` or tag `1` value, a `sha512:` digest, an unknown algorithm name, an uppercase name prefix, a truncated payload, a wrong-length binary value, or a payload length that contradicts its declared tag
- **WHEN** the input is parsed or validated
- **THEN** validation fails with a stable digest diagnostic and MUST NOT guess, truncate, pad, or reinterpret the value

#### Scenario: Negative - observed collision has both preimages
- **GIVEN** two unequal canonical preimages and equal computed digest bytes under one active member are simultaneously available during construction
- **WHEN** the system compares the resulting identities
- **THEN** construction fails with a deterministic observed-collision diagnostic and MUST NOT deduplicate the objects

### Requirement: XXH-003 Complete project hashing coverage and enforcement
The authoritative repository gate MUST inventory project-owned manifests, source, tools, generators, schemas, fixtures, records, and documentation. It MUST reject every hashing implementation and every direct hashing-crate call that sits outside an allowlisted core adapter module, under any identifier spelling or digest prefix spelling. Each registered member MUST reach its implementation through exactly one allowlisted adapter module. The gate MUST also reject generated active-schema values under an inactive member, dependency features that enable an unselected member, nondeterministic hashers including `DefaultHasher` and `RandomState`, and claims that an inactive member establishes a Buoy identity. The inventory MUST cover semantic object revisions, canonical graph and report identities, raw-artifact bindings, proof and derivation bindings, assurance and execution-evidence bindings, oracle configurations, fixtures and corpora, test vectors, migration outputs, fingerprints, cache keys, and any project-selected hash-table hasher. Findings MUST be normalized, totally ordered, and aggregated fail-closed.

#### Scenario: Positive - repository uses only the approved hashing profile
- **GIVEN** every project-controlled hashing path reaches the registry, every hashing implementation sits behind its own allowlisted adapter module, every active digest-bearing artifact carries the declared active member, and all external-integrity exceptions are syntax-classified
- **WHEN** the authoritative hashing-policy gate scans the repository and dependency graph
- **THEN** it succeeds with a deterministic empty finding set

#### Scenario: Negative - prohibited hashing is hidden in a non-core path
- **GIVEN** a development tool, transitive dependency feature, fixture generator, evidence binding path, documentation example, cache helper, or reference crate that implements a hash by hand, calls a hashing crate outside an allowlisted adapter module, or endorses an inactive member for a repository-owned identity
- **WHEN** the authoritative hashing-policy gate scans the repository and dependency graph
- **THEN** it emits the applicable stable findings in total order and the aggregate gate fails

### Requirement: XXH-004 Deterministic breaking migration
Migration to a new schema version or to a new active member MUST regenerate every repository-owned digest from its authoritative canonical source bytes and the target version preimage. It MUST NOT transform, truncate, pad, or relabel old digest bytes, and it MUST NOT reinterpret a payload under a different algorithm tag. The migration MUST preserve stable object IDs and legal lifecycle states while replacing all revision, freshness, derivation, proof, policy, fixture, and report bindings as one closed change set. An object for which authoritative source bytes or required approval are unavailable MUST remain unmigrated and MUST fail active-schema validation.

#### Scenario: Positive - complete authoritative inputs are migrated
- **GIVEN** authoritative bytes, schema inputs, and required approval records for every digest-bound object in a repository snapshot, and one declared target active member
- **WHEN** the deterministic migration runs twice
- **THEN** both runs produce byte-identical target records with preserved stable IDs, legal lifecycle states, and recomputed bindings under the target member

#### Scenario: Negative - only an obsolete digest is available
- **GIVEN** a digest-bound object for which migration has an old digest but lacks authoritative source bytes or a required approval record
- **WHEN** migration or active-schema validation runs
- **THEN** it fails with a deterministic incomplete-migration diagnostic and MUST NOT infer a target identity

### Requirement: XXH-005 External integrity metadata boundary
An ecosystem-prescribed integrity value that the project cannot choose, including a Cargo registry checksum, Nix fixed-output or SRI hash, Git object ID, upstream release checksum, or signature metadata, MUST be treated as opaque supply-chain metadata in the imperative shell. Such a value MUST NOT be computed by project code, parsed as a `SemanticDigest` under any registered member, accepted as a repository-owned object identity, or allowed to influence canonical static output except as explicitly schema-bound opaque text. A registered member's canonical name prefix MUST NOT make an external value eligible for promotion. Exceptions in the hashing-policy inventory MUST be field- and syntax-specific rather than path-wide or substring-wide.

#### Scenario: Positive - package manager verifies its own integrity field
- **GIVEN** a lockfile or fetch declaration containing an ecosystem-required integrity value
- **WHEN** the external package manager or fetcher verifies that value outside the deterministic core
- **THEN** the repository hashing-policy gate classifies it as opaque external metadata and no Buoy identity is created from it

#### Scenario: Negative - external checksum is promoted to semantic identity
- **GIVEN** an ecosystem integrity string is passed to the canonical digest parser or copied into a repository-owned digest field, including a string whose prefix matches a registered member name
- **WHEN** static validation runs
- **THEN** validation rejects the value with a deterministic trust-boundary diagnostic

### Requirement: XXH-006 Non-cryptographic assurance boundary
The system MUST document and report the declared assurance class of the active member. It MUST NOT claim collision resistance or preimage resistance for a member whose declared class is `change-detection`, which includes `xxh3-128`. It MUST NOT claim authenticity, tamper evidence, signature strength, or proof status from digest equality under any member, including a member whose declared class is `collision-resistant`. Static registration, freshness, execution verdict, and assurance class MUST remain separate facts under every member, and any authenticity requirement MUST be satisfied by an independently defined mechanism rather than by the digest algorithm.

#### Scenario: Positive - matching digest supports freshness only
- **GIVEN** a carrier whose expected revision under the active member equals the current canonical atom revision
- **WHEN** freshness is evaluated
- **THEN** the system reports the digest binding as current while retaining separate execution and assurance facts

#### Scenario: Negative - matching digest is presented as security or proof
- **GIVEN** a report or policy that claims collision resistance for a `change-detection` member, or that treats matching digest values, scaffolding, tests, or annotations as authenticity, tamper evidence, or mathematical proof under any member
- **WHEN** the report or policy is validated
- **THEN** validation fails with a deterministic unsupported-assurance-claim diagnostic
