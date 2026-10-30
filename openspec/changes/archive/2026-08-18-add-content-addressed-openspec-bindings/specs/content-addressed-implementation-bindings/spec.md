## ADDED Requirements

### Requirement: CAB-001 Immutable implementation binding record

An implementation binding record MUST bind one navigation relation, one exact implementation source-node digest, one exact OpenSpec requirement source-node digest, and one exact atom-revision reference.

The record digest MUST bind all fields under a versioned domain. The requirement source node and atom reference MUST identify the same materialized authority.

#### Scenario: Positive - record binds both current sides
- **GIVEN** one current Rust item and one current OpenSpec requirement atom
- **WHEN** Buoy constructs an implementation binding record
- **THEN** the record digest identifies both source nodes and the exact atom revision

#### Scenario: Negative - requirement node and atom disagree
- **GIVEN** a binding record whose requirement node maps to a different atom revision
- **WHEN** binding validation runs
- **THEN** validation fails with a deterministic authority-mismatch diagnostic

### Requirement: CAB-002 One-digest Rust syntax

The Rust adapter MUST accept `#[buoy::implements("<digest>")]` on an eligible Rust item. The attribute MUST contain exactly one canonical digest and no other field.

The attribute digest MUST resolve to exactly one repository-owned implementation binding record.

#### Scenario: Positive - compact attribute resolves
- **GIVEN** a Rust item with one canonical binding digest
- **WHEN** the Rust and binding adapters materialize the repository
- **THEN** the digest resolves to one binding record attached to that Rust item

#### Scenario: Negative - attribute contains extra metadata
- **GIVEN** a compact attribute with an ID, relation, or second digest
- **WHEN** the Rust adapter parses the attribute
- **THEN** parsing fails with a deterministic compact-binding-syntax diagnostic

### Requirement: CAB-003 Cycle-free implementation subject identity

The Rust subject digest MUST exclude all recognized Buoy binding attributes from the item token stream. It MUST include every other semantic Rust token, the canonical path, item kind, and enclosing-item identity.

#### Scenario: Positive - generated binding digest changes alone
- **GIVEN** one Rust item whose only edit replaces a Buoy binding digest
- **WHEN** the Rust adapter calculates both subject identities
- **THEN** both subject identities are equal

#### Scenario: Negative - function body changes
- **GIVEN** one linked Rust function
- **WHEN** one semantic body token changes
- **THEN** the subject digest changes and the old binding becomes stale

### Requirement: CAB-004 Repository-owned content-addressed store

Each binding record MUST use the path `buoy/objects/v1/<algorithm>/<payload>.toml`. The filename algorithm and payload MUST equal the calculated record digest.

The store adapter MUST reject mutable aliases, duplicate records, unknown schemas, wrong filenames, and noncanonical record data.

#### Scenario: Positive - immutable record path matches content
- **GIVEN** one canonical binding record at its digest-derived path
- **WHEN** the store adapter materializes it
- **THEN** the record becomes available by its complete digest

#### Scenario: Negative - record content changes in place
- **GIVEN** a binding record whose content does not match its digest-derived path
- **WHEN** the store adapter materializes it
- **THEN** validation fails with a deterministic object-path-digest diagnostic

### Requirement: CAB-005 Bidirectional freshness

Static validation MUST require the binding record, implementation node, requirement node, and atom revision to be current. A change to either source side MUST make the old binding stale.

Freshness MUST remain separate from execution verdicts and assurance classes.

#### Scenario: Positive - both sources remain current
- **GIVEN** a binding whose source nodes and atom revision are reachable in the current graph
- **WHEN** static validation runs
- **THEN** the binding is fresh and supplies navigation only

#### Scenario: Negative - OpenSpec scenario changes
- **GIVEN** a fresh binding to a requirement with one scenario
- **WHEN** the scenario result changes but the Rust attribute remains unchanged
- **THEN** static validation fails with a deterministic stale-requirement diagnostic

#### Scenario: Negative - Rust implementation changes
- **GIVEN** a fresh binding to a Rust item
- **WHEN** the item body changes but the Rust attribute remains unchanged
- **THEN** static validation fails with a deterministic stale-implementation diagnostic

### Requirement: CAB-006 Traversable binding references

A binding node MUST expose directed references to its implementation node, OpenSpec requirement node, and atom authority. Clients MUST be able to traverse child OpenSpec nodes from the requirement.

#### Scenario: Positive - client follows the binding tree
- **GIVEN** one fresh binding to a requirement with two scenarios
- **WHEN** a client requests traversal from the binding digest
- **THEN** the result includes the Rust item, requirement, both scenarios, their steps, and atom authority

#### Scenario: Negative - requirement target has the wrong kind
- **GIVEN** a binding whose requirement reference resolves to an OpenSpec scenario node
- **WHEN** binding traversal validates target kinds
- **THEN** validation fails with a deterministic wrong-requirement-kind diagnostic

### Requirement: CAB-007 Collision-resistant hash-only locator

The hash-only binding syntax MUST require an active digest member with the `collision-resistant` assurance class. Change-detection members MUST fail at the configuration boundary for this feature.

Digest equality MUST NOT establish authenticity, authorization, implementation correctness, or proof.

#### Scenario: Positive - BLAKE3 binding is selected
- **GIVEN** a build with `blake3-256` available and selected
- **WHEN** hash-only binding configuration is parsed
- **THEN** configuration succeeds with the collision-resistant member

#### Scenario: Negative - XXH3 binding is selected
- **GIVEN** hash-only binding syntax and active member `xxh3-128`
- **WHEN** configuration is parsed
- **THEN** parsing fails with a deterministic binding-digest-class diagnostic

### Requirement: CAB-008 Immutable binding update

The binding command MUST create a new record at a new digest-derived path and replace only the selected source attribute. It MUST NOT rewrite an old record in place.

The command MUST require an explicit requirement selector and implementation selector. It MUST not refresh all stale bindings without selection.

#### Scenario: Positive - reviewed implementation edit receives a new binding
- **GIVEN** one stale implementation binding and explicit selectors for both source nodes
- **WHEN** the binding command runs
- **THEN** it writes one new record and replaces the old attribute digest

#### Scenario: Negative - no requirement selector is supplied
- **GIVEN** one stale binding and no explicit requirement selector
- **WHEN** the binding command runs
- **THEN** it fails without changing source or object-store files
