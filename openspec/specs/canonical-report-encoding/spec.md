# Canonical Report Encoding

## Purpose
This specification defines canonical report encoding, decoding, identity, and migration.

## Requirements

### Requirement: CRE-001 Complete v1 domain encoding
The canonical encoder MUST implement every frozen v1 digest/report domain and exact field tag in `docs/schema/v1/canonical-encoding.md`, including explicit absent options, empty sequences/sets, algorithm identity, domain separation, and complete orthogonal fact sets. No semantic default or unknown field MAY be omitted or reinterpreted.

#### Scenario: Positive - complete static report is encoded
- **GIVEN** a static report containing complete graph identity, every frozen fact set, explicit empty sets/options, and canonical diagnostics
- **WHEN** v1 binary encoding runs
- **THEN** fields appear exactly once in ascending tags and produce the independently frozen bytes/digest

#### Scenario: Negative - empty waiver facts are omitted
- **GIVEN** a report encoder that elides the required empty debt/waiver fact set
- **WHEN** encoding is compared with the v1 field table
- **THEN** parity/golden validation fails and MUST NOT accept the shorter bytes as equivalent

### Requirement: CRE-002 Strict canonical decoder
The v1 decoder MUST consume exactly one complete canonical value and MUST reject unknown type/algorithm/enum/field tags, wrong payload lengths, integer overflow, duplicate or out-of-order fields, omitted fields, non-NFC text, duplicate or unsorted set encodings, invalid digest lengths, noncanonical options, nesting/count budget violations, and trailing bytes.

#### Scenario: Positive - valid report round trip
- **GIVEN** independently generated canonical v1 report bytes within configured budgets
- **WHEN** the bytes decode and re-encode
- **THEN** the resulting bytes are exactly identical

#### Scenario: Negative - valid prefix has trailing bytes
- **GIVEN** one valid canonical report followed by extra bytes
- **WHEN** strict decoding runs
- **THEN** decoding fails with a deterministic trailing-bytes diagnostic and MUST NOT return the prefix value

### Requirement: CRE-003 Authoritative binary report identity
Static, execution, and impact report identity MUST be established only by the canonical v1 binary encoding. JSON and human output MUST be deterministic derived presentations that bind or display the authoritative binary report digest and MUST NOT silently replace binary identity.

#### Scenario: Positive - JSON derives from one binary report
- **GIVEN** one canonical binary report and fixed JSON presentation schema
- **WHEN** JSON rendering runs under different locales and terminal environments
- **THEN** JSON bytes are identical and contain the same authoritative report digest

#### Scenario: Negative - JSON key order establishes identity
- **GIVEN** two semantically equivalent JSON objects with different key ordering
- **WHEN** identity is requested
- **THEN** neither JSON byte sequence is used directly; identity remains the bound canonical binary digest

### Requirement: CRE-004 Complete orthogonal report facts
Canonical reports MUST preserve every frozen authority, surface, registration, policy, freshness, debt/waiver, derivation, proof, evidence, disposition-closure, verdict, assumption, exclusion, scope, and diagnostic fact without replacing them with counts or one optimistic summary.

#### Scenario: Positive - waived timeout remains explicit
- **GIVEN** a timed-out required carrier with an active valid waiver
- **WHEN** static and execution reports are encoded
- **THEN** timeout, unsatisfied obligation, waiver validity/process authorization, carrier freshness, scope, and assurance classes remain separate fields

#### Scenario: Negative - report emits only aggregate success
- **GIVEN** passing siblings and one stale required negative carrier
- **WHEN** a renderer attempts to emit only one successful aggregate status
- **THEN** report validation fails because required orthogonal facts are absent

### Requirement: CRE-005 Independent encoding authority and malformed vectors
A second canonical implementation MUST share no production encoder/decoder modules and MUST generate positive and negative vectors for every type tag, field table, collection rule, digest domain, report kind, malformed boundary, and algorithm identity. Production and independent outputs MUST match exact bytes for valid values and intended diagnostic families for invalid values.

#### Scenario: Positive - independent report bytes agree
- **GIVEN** one generated valid graph/report value
- **WHEN** production and independent encoders run
- **THEN** their complete bytes and SHA-256 identities match

#### Scenario: Negative - both implementations call one helper
- **GIVEN** an independent checker whose dependency graph includes the production canonical encoder
- **WHEN** independence policy is checked
- **THEN** the differential evidence is rejected and MUST NOT satisfy the independent-oracle obligation

### Requirement: CRE-006 Explicit report compatibility migration
Any future report field, encoding, or digest algorithm change MUST use a new schema/domain and explicit compatibility mapping. Readers MUST preserve published v1 bytes/digests and MUST fail rather than reinterpret them under new rules.

#### Scenario: Positive - later report schema is introduced
- **GIVEN** a reviewed v2 report with new fields and a mapping artifact for retained identities
- **WHEN** v1 history is read
- **THEN** original v1 bytes/digests remain unchanged and addressable

#### Scenario: Negative - configuration switches v1 to another hash
- **GIVEN** a v1 report digest and a configuration selecting a different algorithm without migration
- **WHEN** report identity is validated
- **THEN** validation fails with a deterministic schema/algorithm mismatch
