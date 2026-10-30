## ADDED Requirements

### Requirement: SPG-012 Content-addressed implementation binding nodes

The graph MUST represent each compact implementation binding as one immutable binding node identified by its complete digest. The node MUST retain exact source and authority references.

The binding node MUST remain navigational. It MUST NOT satisfy evidence, execution, or proof obligations.

#### Scenario: Positive - fresh binding supports navigation
- **GIVEN** one current binding node with closed source and atom references
- **WHEN** the graph report is rendered
- **THEN** the report exposes its traversal links without adding an assurance class

#### Scenario: Negative - binding is treated as evidence
- **GIVEN** one ratified atom with only a fresh content-addressed implementation binding
- **WHEN** coverage aggregation runs
- **THEN** the atom remains behaviorally uncovered

### Requirement: SPG-013 Binding impact propagation

Change analysis MUST report a changed implementation binding when either referenced source node or the atom revision changes. Impact traversal MUST follow the binding node to dependent governed objects.

#### Scenario: Positive - implementation node changes
- **GIVEN** base and head graphs with different implementation nodes for one stable relation
- **WHEN** change analysis runs
- **THEN** the implementation binding is directly changed and its dependents are transitively impacted

#### Scenario: Negative - source change is hidden by attribute reuse
- **GIVEN** a changed implementation item with its old compact attribute
- **WHEN** change and freshness validation run
- **THEN** validation reports the stale binding and MUST NOT report an unchanged relation
