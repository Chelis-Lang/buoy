## MODIFIED Requirements

### Requirement: CAB-002 One-digest Rust syntax

The Rust adapter MUST accept `#[buoy::implements("<canonical-digest>")]` on an eligible Rust item.

The attribute MUST contain exactly one canonical algorithm-qualified digest and no other field. Its syntax MUST follow `<canonical-name>:<lowercase hexadecimal payload>`.

The digest member MUST equal the repository's active member. The complete digest MUST resolve to exactly one repository-owned implementation binding record.

#### Scenario: Positive - algorithm-qualified compact attribute resolves

- **GIVEN** a Rust item with one `blake3-256:<payload>` implementation-binding digest and matching repository configuration
- **WHEN** the Rust and binding adapters materialize the repository
- **THEN** the complete digest resolves to one binding record attached to that Rust item

#### Scenario: Negative - attribute contains raw payload bytes

- **GIVEN** one compact attribute with lowercase payload bytes and no canonical algorithm name
- **WHEN** the Rust adapter parses the attribute
- **THEN** parsing fails with a deterministic compact-binding-digest diagnostic

#### Scenario: Negative - attribute contains extra metadata

- **GIVEN** a compact attribute with an ID, relation, or second digest
- **WHEN** the Rust adapter parses the attribute
- **THEN** parsing fails with a deterministic compact-binding-syntax diagnostic

### Requirement: CAB-007 Collision-resistant compact locator

The compact locator MUST require one canonical algorithm-qualified digest whose member has the `collision-resistant` assurance class.

The locator member, active repository member, record digest, implementation node, requirement node, and semantic atom revision MUST use that same registered member.

A change-detection member, raw hash payload, or mismatched registered member MUST fail at the configuration or adapter boundary.

Digest equality MUST NOT establish authenticity, authorization, implementation correctness, or proof.

#### Scenario: Positive - BLAKE3 binding is selected

- **GIVEN** a build with `blake3-256` available, one `blake3-256:<payload>` locator, and matching active configuration
- **WHEN** compact binding configuration and source parsing run
- **THEN** configuration succeeds with one collision-resistant member across the complete record

#### Scenario: Negative - XXH3 binding is selected

- **GIVEN** compact binding syntax and active member `xxh3-128`
- **WHEN** configuration is parsed
- **THEN** parsing fails with a deterministic binding-digest-class diagnostic

#### Scenario: Negative - locator and active member differ

- **GIVEN** one `sha-256:<payload>` locator and active member `blake3-256`
- **WHEN** source parsing runs
- **THEN** parsing fails with a deterministic compact-digest-member diagnostic

## RENAMED Requirements

- FROM: `### Requirement: CAB-007 Collision-resistant hash-only locator`
- TO: `### Requirement: CAB-007 Collision-resistant compact locator`
