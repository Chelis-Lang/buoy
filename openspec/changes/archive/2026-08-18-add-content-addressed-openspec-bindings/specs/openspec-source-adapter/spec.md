## ADDED Requirements

### Requirement: OSA-010 Content-addressed OpenSpec source tree

The adapter MUST materialize each selected standing spec as a content-addressed source tree. The tree MUST contain document, requirement, scenario, and step nodes.

The document MUST contain requirements in source order. Each requirement MUST contain its scenarios in source order. Each scenario MUST contain its parsed steps in source order.

#### Scenario: Positive - complete standing-spec tree
- **GIVEN** one standing spec with two requirements and three scenarios
- **WHEN** the OpenSpec adapter materializes the file
- **THEN** the source tree preserves each parent, child, and source position

#### Scenario: Negative - scenario escapes its requirement
- **GIVEN** a malformed source tree whose scenario parent is a different requirement
- **WHEN** OpenSpec tree validation runs
- **THEN** validation fails with a deterministic OpenSpec-parent diagnostic

### Requirement: OSA-011 Requirement node and atom correspondence

Each authoritative requirement node MUST map to exactly one atom. Its stable ID and canonical semantic subtree MUST agree with the atom ID and revision.

A scenario or step node MUST NOT map directly to an atom in the first binding version.

#### Scenario: Positive - requirement maps to its atom
- **GIVEN** one valid OpenSpec requirement source node
- **WHEN** the adapter constructs authority
- **THEN** the node maps to one atom with the same stable ID and semantic revision

#### Scenario: Negative - scenario is used as atom authority
- **GIVEN** an OpenSpec scenario node and no requirement node
- **WHEN** authority mapping runs
- **THEN** validation fails and no atom is created from the scenario

### Requirement: OSA-012 Child changes propagate to requirement identity

A semantic change to a scenario or step MUST change each containing scenario and requirement node digest. Unchanged sibling requirement digests MUST remain stable.

#### Scenario: Positive - one scenario result changes
- **GIVEN** two requirements with separate scenarios
- **WHEN** one result step changes in the first requirement
- **THEN** the first requirement digest changes and the second requirement digest remains equal

#### Scenario: Negative - child changes but parent digest remains equal
- **GIVEN** a requirement with one changed semantic descendant
- **WHEN** the adapter calculates the source tree
- **THEN** validation fails if the requirement digest equals its prior digest
