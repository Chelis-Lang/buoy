## ADDED Requirements

### Requirement: SRG-001 Content-addressed source nodes

Each parsed source node MUST have one digest that binds its schema ID, source-item kind, canonical repository path, semantic payload, and ordered child references. Source offsets and declared formatting trivia MUST NOT affect this digest.

#### Scenario: Positive - equal nodes have equal digests
- **GIVEN** two source nodes with equal canonical fields under separate absolute roots
- **WHEN** the core calculates both node digests
- **THEN** both digests are equal

#### Scenario: Negative - semantic payload changes
- **GIVEN** one current source node
- **WHEN** one semantic payload byte changes
- **THEN** the node digest changes and the prior digest does not identify the current node

### Requirement: SRG-002 Typed directed source edges

The source graph MUST represent each relationship as a typed directed edge between exact source-node digests. The initial edge vocabulary MUST contain `contains`, `implements`, and `authority`.

Each `contains` edge MUST include a nonnegative child position. Positions under one parent MUST be unique and contiguous.

#### Scenario: Positive - OpenSpec hierarchy forms a tree
- **GIVEN** one standing spec with a requirement, scenario, and ordered steps
- **WHEN** the adapter constructs source edges
- **THEN** `contains` edges connect the document, requirement, scenario, and steps in source order

#### Scenario: Negative - child position is duplicated
- **GIVEN** one parent with two `contains` edges at the same position
- **WHEN** source-graph validation runs
- **THEN** validation fails with a deterministic duplicate-child-position diagnostic

### Requirement: SRG-003 Current-root reachability

Each source graph MUST declare its current root set. A source reference MUST be current only when its target is reachable from that root set.

A retained historical node MAY support diagnostics. A historical node MUST NOT satisfy current reference closure.

#### Scenario: Positive - current requirement is reachable
- **GIVEN** a binding that references a requirement under a current OpenSpec document root
- **WHEN** source-reference closure runs
- **THEN** the requirement reference is current

#### Scenario: Negative - old requirement remains in the object store
- **GIVEN** an old requirement node in the object store but absent from every current root
- **WHEN** a binding references the old node
- **THEN** validation fails with a deterministic stale-source-reference diagnostic

### Requirement: SRG-004 Acyclic bounded traversal

The source graph MUST be acyclic. Traversal MUST use explicit depth and visited-node budgets from repository configuration.

Traversal output MUST use canonical edge order and MUST not depend on insertion order.

#### Scenario: Positive - binding tree is traversed
- **GIVEN** a current implementation binding with requirement and subject edges
- **WHEN** a client traverses from the binding node
- **THEN** traversal returns the subject, requirement, scenarios, steps, and atom authority in canonical order

#### Scenario: Negative - edge introduces a cycle
- **GIVEN** a source graph in which a descendant references an ancestor through a traversable edge
- **WHEN** graph validation runs
- **THEN** validation fails with a deterministic source-cycle diagnostic

### Requirement: SRG-005 Closed source-reference aggregation

Repository-level source-graph construction MUST aggregate all selected adapter products before reference resolution. Missing, conflicting, ambiguous, stale, cyclic, and wrong-kind references MUST fail closed.

Equal nodes, edges, and roots MUST aggregate idempotently. Digest-equal nodes with unequal preimages MUST fail closed.

#### Scenario: Positive - separate adapters join one graph
- **GIVEN** one Rust product and one OpenSpec product in the same materialized snapshot
- **WHEN** repository-level aggregation runs
- **THEN** a binding can reference nodes from both products

#### Scenario: Negative - one adapter resolves references alone
- **GIVEN** a Rust binding whose OpenSpec target appears in another selected file
- **WHEN** per-file materialization cannot see that target
- **THEN** the adapter retains the unresolved typed reference for repository-level resolution and MUST NOT report success for closure
