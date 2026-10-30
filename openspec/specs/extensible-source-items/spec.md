# Extensible Source Items

## Purpose

Define versioned source-item kinds, adapter declarations, source mappings, and the schema-v3 cutover.

## Requirements

### Requirement: ESI-001 Versioned source-item kind identity
Semantic schema v3 MUST identify each parsed item with one validated, versioned source-item kind ID. The parsed-item digest MUST bind that ID.

#### Scenario: Positive - OpenSpec requirement has a truthful kind
- **GIVEN** an OpenSpec adapter descriptor that declares `openspec/requirement/v1`
- **WHEN** the adapter materializes a valid standing requirement
- **THEN** the parsed-item identity binds `openspec/requirement/v1` and the exact semantic item bytes

#### Scenario: Negative - OpenSpec requirement uses the Markdown fence kind
- **GIVEN** an OpenSpec requirement heading that is not a fenced code block
- **WHEN** an adapter labels it as `markdown/fence/v1`
- **THEN** materialization fails with a deterministic source-item-kind diagnostic

### Requirement: ESI-002 Descriptor-bound kind declarations
Each adapter descriptor MUST declare a nonempty, duplicate-free source-item kind set. The shell MUST reject each materialized item with an undeclared kind.

#### Scenario: Positive - adapter uses its declared kind
- **GIVEN** an adapter descriptor with one valid source-item kind declaration
- **WHEN** the adapter emits an item with that kind
- **THEN** descriptor validation accepts the item

#### Scenario: Negative - adapter invents a kind after registration
- **GIVEN** a registered adapter descriptor that does not declare `host/item/v1`
- **WHEN** the adapter emits an item with `host/item/v1`
- **THEN** materialization fails with a stable undeclared-kind diagnostic

### Requirement: ESI-003 Parsed kind type boundary
The shell MUST parse untrusted kind text into `SourceItemKind` at the descriptor boundary. Core APIs MUST accept only the parsed type.

#### Scenario: Positive - parsed kind enters the core
- **GIVEN** a valid versioned kind ID in an adapter descriptor
- **WHEN** the shell constructs the descriptor
- **THEN** later core calls receive `SourceItemKind` without a second grammar validation

#### Scenario: Negative - raw kind text bypasses the boundary
- **GIVEN** a caller with an unvalidated string for a source-item kind
- **WHEN** the caller tries to construct a v3 parsed-item identity directly
- **THEN** the public type interface rejects the raw string path

### Requirement: ESI-004 Schema-v3 breaking cutover
The v3 profile MUST use versioned text kinds. It MUST NOT include a runtime reader for legacy numeric item-kind tags.

#### Scenario: Positive - all active adapters use v3 kinds
- **GIVEN** the schema-v3 profile and all built-in adapters
- **WHEN** the repository regenerates parsed-item identities
- **THEN** every identity uses a descriptor-declared versioned text kind

#### Scenario: Negative - legacy numeric tag enters v3
- **GIVEN** a legacy identity with a numeric item-kind tag
- **WHEN** a v3 reader receives that identity
- **THEN** the reader rejects it and does not reinterpret the tag

### Requirement: ESI-005 Complete source mapping for new kinds
Each materialized semantic object MUST have an exact parser-backed source mapping. The mapping MUST contain the item identity, source span, and syntax capability.

#### Scenario: Positive - OpenSpec atom has one source mapping
- **GIVEN** a valid OpenSpec requirement that creates one atom
- **WHEN** the adapter product is validated
- **THEN** the atom has an exact requirement item, original-byte span, and declared syntax capability

#### Scenario: Negative - semantic object lacks a source mapping
- **GIVEN** an adapter product with one atom and no source mapping for that atom
- **WHEN** the adapter product is validated
- **THEN** materialization fails with a deterministic missing-source diagnostic

### Requirement: ESI-006 Extensible but closed adapter protocol
A new host adapter MUST add a versioned descriptor kind without a new core enum variant. Every accepted kind MUST remain descriptor-declared and fixture-backed.

#### Scenario: Positive - two host adapters use separate kinds
- **GIVEN** OpenSpec and Chelis adapters with distinct versioned kind declarations
- **WHEN** both adapters materialize saved inputs
- **THEN** the core accepts both parsed types without host-specific enum variants

#### Scenario: Negative - arbitrary kind text enters the graph
- **GIVEN** a syntactically valid kind ID with no registered descriptor or fixture contract
- **WHEN** a materialized graph references that kind
- **THEN** graph construction fails closed with a deterministic unsupported-kind diagnostic

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

### Requirement: ESI-009 Registered opaque artifact kind

The built-in registry MUST declare `artifact/v1` with exactly one `artifact/file/v1` source-item kind and one `artifact.opaque-bytes/v1` capability.

The kind MUST use the existing parsed-item, source-node, source-mapping, budget, and canonical-order contracts.

#### Scenario: Positive - registry materializes opaque bytes
- **GIVEN** one valid `artifact/v1` configuration and one selected file
- **WHEN** the built-in registry dispatches the adapter
- **THEN** the product contains one declared `artifact/file/v1` node with a complete source mapping

#### Scenario: Negative - product emits a semantic language kind
- **GIVEN** an `artifact/v1` product that emits a Rust, OpenSpec, Markdown, or unregistered item kind
- **WHEN** SDK product validation runs
- **THEN** validation fails with the deterministic undeclared-source-item-kind diagnostic
