## ADDED Requirements

### Requirement: XXH-001 Sole repository-owned hashing algorithm
The system MUST use XXH3-128 with seed `0` and the standard XXH3 default secret for every hash computed under a repository-owned schema or workflow. It MUST retain all 128 output bits, encode the digest bytes in big-endian order, and MUST NOT compute SHA-family or BLAKE3 hashes in project runtime, development, migration, reference, fixture-generation, or policy-checking code.

#### Scenario: Positive - the same canonical preimage is hashed twice
- **GIVEN** the same domain-separated canonical preimage is supplied to two conforming project hashing paths
- **WHEN** each path computes its repository-owned digest
- **THEN** each emits the same full 128-bit XXH3-128 result using seed `0` and the standard default secret

#### Scenario: Negative - a project path selects another algorithm
- **GIVEN** a project-controlled digest, fingerprint, cache key, fixture generator, or reference path configured for SHA, BLAKE3, another xxHash variant, a custom seed, or a custom secret
- **WHEN** the hashing-policy gate evaluates the path
- **THEN** the gate fails closed with a deterministic prohibited-hash-algorithm diagnostic

### Requirement: XXH-002 Canonical XXH3-128 identity profile
The active schema MUST identify repository-owned digests with semantic schema ID `spec-provenance/v2`, encoding version `2`, digest algorithm tag `2`, and canonical text `xxh3-128:<32 lowercase hexadecimal digits>`. Binary digest values MUST contain exactly 16 bytes in official XXH128 canonical order: the high 64-bit half in big-endian order followed by the low 64-bit half in big-endian order. Parsers MUST reject unknown tags, wrong lengths, uppercase or nonhexadecimal text, truncated values, and every SHA or BLAKE3 prefix. When unequal canonical preimages with the same digest are simultaneously available, construction MUST fail with a deterministic observed-collision diagnostic.

#### Scenario: Positive - canonical binary and text forms agree
- **GIVEN** a valid full XXH3-128 result under the version 2 schema
- **WHEN** the result is serialized to binary and canonical text and parsed again
- **THEN** both round trips preserve the same algorithm tag and all 16 digest bytes in official XXH128 canonical order

#### Scenario: Negative - obsolete or malformed digest is supplied
- **GIVEN** an active-schema input containing a `sha256:`, `sha512:`, `blake3:`, uppercase `XXH3-128:`, truncated `xxh3-128:`, wrong-length binary, or unknown-algorithm digest
- **WHEN** the input is parsed or validated
- **THEN** validation fails with a stable digest diagnostic and MUST NOT guess, truncate, pad, or reinterpret the value

#### Scenario: Negative - observed collision has both preimages
- **GIVEN** two unequal canonical preimages and equal computed XXH3-128 bytes are simultaneously available during construction
- **WHEN** the system compares the resulting identities
- **THEN** construction fails with a deterministic observed-collision diagnostic and MUST NOT deduplicate the objects

### Requirement: XXH-003 Complete project hashing coverage and enforcement
The authoritative repository gate MUST inventory project-owned manifests, source, tools, generators, schemas, fixtures, records, and documentation and MUST reject direct or transitive SHA/BLAKE3 hashing dependencies, hashing APIs, generated active-schema values, or claims that another algorithm establishes a Spec Provenance identity. The inventory MUST cover semantic object revisions, canonical graph/report identities, raw-artifact bindings, proof and derivation bindings, oracle configurations, fixtures and corpora, test vectors, migration outputs, fingerprints, cache keys, and any project-selected hash-table hasher. Findings MUST be normalized, totally ordered, and aggregated fail-closed.

#### Scenario: Positive - repository uses only the approved hashing profile
- **GIVEN** every project-controlled hashing path and active digest-bearing artifact uses the XXH3-128 profile and all external-integrity exceptions are syntax-classified
- **WHEN** the authoritative hashing-policy gate scans the repository and dependency graph
- **THEN** it succeeds with a deterministic empty finding set

#### Scenario: Negative - prohibited hashing is hidden in a non-core path
- **GIVEN** a development tool, transitive dependency feature, fixture generator, documentation example, cache helper, or reference crate computes or endorses SHA or BLAKE3 for a repository-owned identity
- **WHEN** the authoritative hashing-policy gate scans the repository and dependency graph
- **THEN** it emits the applicable stable findings in total order and the aggregate gate fails

### Requirement: XXH-004 Deterministic breaking migration
Migration to version 2 MUST regenerate every repository-owned digest from its authoritative canonical source bytes and version 2 preimage; it MUST NOT transform, truncate, pad, or relabel old digest bytes. The migration MUST preserve stable object IDs and legal lifecycle states while replacing all revision, freshness, derivation, proof, policy, fixture, and report bindings as one closed change set. An object for which authoritative source bytes or required approval are unavailable MUST remain unmigrated and MUST fail active-schema validation.

#### Scenario: Positive - complete authoritative inputs are migrated
- **GIVEN** authoritative bytes, schema inputs, and required approval records for every digest-bound object in a repository snapshot
- **WHEN** the deterministic migration runs twice
- **THEN** both runs produce byte-identical version 2 records with preserved stable IDs, legal lifecycle states, and recomputed XXH3-128 bindings

#### Scenario: Negative - only an obsolete digest is available
- **GIVEN** a digest-bound object for which migration has an old digest but lacks authoritative source bytes or a required approval record
- **WHEN** migration or active-schema validation runs
- **THEN** it fails with a deterministic incomplete-migration diagnostic and MUST NOT infer a version 2 identity

### Requirement: XXH-005 External integrity metadata boundary
An ecosystem-prescribed integrity value that the project cannot choose, including a Cargo registry checksum, Nix fixed-output or SRI hash, Git object ID, upstream release checksum, or signature metadata, MUST be treated as opaque supply-chain metadata in the imperative shell. Such a value MUST NOT be computed by project code, parsed as a `SemanticDigest`, accepted as a repository-owned object identity, or allowed to influence canonical static output except as explicitly schema-bound opaque text. Exceptions in the hashing-policy inventory MUST be field- and syntax-specific rather than path-wide or substring-wide.

#### Scenario: Positive - package manager verifies its own integrity field
- **GIVEN** a lockfile or fetch declaration containing an ecosystem-required integrity value
- **WHEN** the external package manager or fetcher verifies that value outside the deterministic core
- **THEN** the repository hashing-policy gate classifies it as opaque external metadata and no Spec Provenance identity is created from it

#### Scenario: Negative - external checksum is promoted to semantic identity
- **GIVEN** an ecosystem integrity string is passed to the canonical digest parser or copied into a repository-owned digest field
- **WHEN** static validation runs
- **THEN** validation rejects the value with a deterministic trust-boundary diagnostic

### Requirement: XXH-006 Non-cryptographic assurance boundary
The system MUST document and report that XXH3-128 provides deterministic change detection only and MUST NOT claim collision resistance, preimage resistance, authenticity, tamper evidence, signature strength, or proof status from digest equality. Static registration, freshness, execution verdict, and assurance class MUST remain separate facts, and any authenticity requirement MUST be satisfied by an independently defined mechanism rather than by xxHash.

#### Scenario: Positive - matching digest supports freshness only
- **GIVEN** a carrier whose expected XXH3-128 revision equals the current canonical atom revision
- **WHEN** freshness is evaluated
- **THEN** the system may report the digest binding as current while retaining separate execution and assurance facts

#### Scenario: Negative - matching digest is presented as security or proof
- **GIVEN** a report or policy attempts to treat matching XXH3-128 values, scaffolding, tests, or annotations as authenticity, collision resistance, or mathematical proof
- **WHEN** the report or policy is validated
- **THEN** validation fails with a deterministic unsupported-assurance-claim diagnostic
