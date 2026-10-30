## ADDED Requirements

### Requirement: SRG-006 Logical-mount graph aggregation

Repository graph construction MUST aggregate source products from every declared logical mount before reference resolution.

A node path MUST contain its logical prefix. Physical root identity and per-mount acceptance state MUST NOT enter the graph.

#### Scenario: Positive - mounted adapters join one graph
- **GIVEN** one raw-artifact subject in a project mount and one OpenSpec requirement in an authority mount
- **WHEN** repository graph aggregation runs
- **THEN** one binding resolves both nodes and traverses their joined graph references

#### Scenario: Negative - unresolved cross-mount target
- **GIVEN** one sidecar that names a requirement digest absent from every declared mount
- **WHEN** repository graph closure runs
- **THEN** validation fails with one deterministic unresolved-requirement diagnostic

### Requirement: SRG-007 Mount-order invariant graph identity

Source products, roots, edges, authorities, attachments, and diagnostics MUST use canonical order after logical path projection.

Input mount order, physical traversal order, and adapter completion order MUST NOT change graph identity or diagnostic precedence.

#### Scenario: Positive - mount permutations agree
- **GIVEN** one complete mounted repository with two input mount orders
- **WHEN** both source graphs are constructed
- **THEN** graph digests, traversals, and machine diagnostics are byte-identical

#### Scenario: Negative - path collision precedes unresolved references
- **GIVEN** inputs with both one logical path collision and one unresolved binding reference
- **WHEN** repository validation runs under any mount order
- **THEN** every order reports the deterministic logical-path-collision diagnostic first
