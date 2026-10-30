## MODIFIED Requirements

### Requirement: CAB-006 Traversable binding references

A binding node MUST expose directed references to its implementation node, OpenSpec requirement node, and atom authority. Clients MUST be able to traverse child OpenSpec nodes from the requirement.

Read-only clients MUST select traversal through a stable requirement ID, stable relation ID, logical implementation path, or exact record digest. Traversal MUST NOT change graph state or current attachments.

#### Scenario: Positive - client follows the binding tree

- **GIVEN** one fresh binding to a requirement with two scenarios
- **WHEN** a client requests traversal from the binding digest
- **THEN** the result includes the Rust item, requirement, both scenarios, their steps, and atom authority

#### Scenario: Positive - client follows a stable requirement ID

- **GIVEN** one requirement with current links in two implementation paths
- **WHEN** a client requests traversal from the requirement ID
- **THEN** the result includes both relations in canonical relation and path order

#### Scenario: Negative - requirement target has the wrong kind

- **GIVEN** a binding whose requirement reference resolves to an OpenSpec scenario node
- **WHEN** binding traversal validates target kinds
- **THEN** validation fails with a deterministic wrong-requirement-kind diagnostic

#### Scenario: Negative - historical record appears current

- **GIVEN** one retained record without a current inline or sidecar attachment
- **WHEN** a read-only client traverses that record
- **THEN** the result marks it historical and no current graph edge originates from it

## ADDED Requirements

### Requirement: CAB-013 Frozen successful-bind tree report

Successful bind operations MUST retain machine schema `buoy/binding-tree-report/v1`. Its relation, record, requirement, atom, path, freshness, and traversal fields MUST keep their published meanings.

A new trace report MUST share fresh-link projection facts with this report. It MUST NOT add stale or historical meanings to the successful-bind schema.

#### Scenario: Positive - fresh bind and trace facts agree

- **GIVEN** one successful bind operation and one trace query for its current relation
- **WHEN** both machine reports are rendered
- **THEN** relation, record, requirement, implementation, path, traversal, and freshness facts agree exactly

#### Scenario: Negative - historical row enters bind output

- **GIVEN** one retained record without a current attachment
- **WHEN** successful-bind report construction receives that historical row
- **THEN** construction fails and MUST NOT reinterpret `buoy/binding-tree-report/v1`
