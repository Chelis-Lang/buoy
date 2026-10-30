## ADDED Requirements

### Requirement: ESI-007 Retained source-node products

Each adapter product MUST retain every semantic source node required for repository-level reference closure. The shell MUST NOT discard source nodes before graph validation.

A retained source node that creates no semantic object MUST still have a declared source-item kind and an exact parser-backed span.

#### Scenario: Positive - OpenSpec scenario is retained
- **GIVEN** an OpenSpec scenario that creates no atom
- **WHEN** repository-level source aggregation runs
- **THEN** the scenario node remains available for requirement-tree traversal

#### Scenario: Negative - source mappings are discarded
- **GIVEN** one implementation binding and adapter products without retained source nodes
- **WHEN** repository-level closure runs
- **THEN** validation fails and MUST NOT accept the binding from object fields alone

### Requirement: ESI-008 Unresolved typed adapter references

An adapter MAY emit an unresolved typed digest reference when another selected adapter owns the target. The repository shell MUST resolve all such references after aggregate materialization.

#### Scenario: Positive - Rust references OpenSpec
- **GIVEN** a Rust attribute that references a binding record from another selected root
- **WHEN** all adapter products are aggregated
- **THEN** the shell resolves the reference against the aggregate source graph

#### Scenario: Negative - aggregate target is absent
- **GIVEN** an unresolved reference whose target appears in no selected product
- **WHEN** aggregate resolution completes
- **THEN** validation fails with a deterministic unresolved-source-reference diagnostic
