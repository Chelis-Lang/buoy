## ADDED Requirements

### Requirement: CAB-009 Out-of-line artifact attachment

A raw-artifact binding MUST use one repository-owned attachment sidecar. The sidecar MUST bind one stable relation ID, logical artifact path, source-node digest, and binding-record digest.

Its canonical path MUST derive from the relation ID under the active collision-resistant member. A binding digest change MUST NOT change the sidecar path.

#### Scenario: Positive - sidecar selects one immutable record
- **GIVEN** one current opaque artifact and one sidecar whose fields match a current implementation binding record
- **WHEN** binding attachment validation runs
- **THEN** the sidecar attaches that record to the exact artifact source node

#### Scenario: Negative - sidecar path names another relation
- **GIVEN** one sidecar whose path-derived relation identity differs from its content
- **WHEN** attachment parsing runs
- **THEN** parsing fails with a deterministic attachment-path diagnostic

### Requirement: CAB-010 Unique current artifact attachment

One stable relation ID MUST have at most one current sidecar. Equal duplicate sidecars can collapse only when every canonical field is equal.

Old immutable binding records MUST remain valid stored objects. An old record MUST NOT become current without a current inline or sidecar attachment.

#### Scenario: Positive - sidecar update preserves history
- **GIVEN** one relation with an old object record and a sidecar that selects a new object record
- **WHEN** repository validation runs
- **THEN** only the new record is current and both immutable object files remain stored

#### Scenario: Negative - two sidecars select one relation
- **GIVEN** two unequal current sidecars for one stable relation ID
- **WHEN** repository aggregation runs
- **THEN** aggregation fails with a deterministic duplicate-artifact-attachment diagnostic

### Requirement: CAB-011 Atomic raw-artifact bind update

`buoy bind` MUST accept an explicit logical artifact selector. It MUST require the existing requirement, relation, and navigation inputs.

The command MUST write one new immutable object and replace only the selected relation sidecar. It MUST leave artifact bytes unchanged.

Any parse, validation, write, synchronization, or replacement failure MUST restore the prior sidecar and remove only the uncommitted new object.

#### Scenario: Positive - reviewed artifact receives a binding
- **GIVEN** one current opaque artifact, one current requirement, and explicit relation inputs
- **WHEN** `buoy bind` selects the artifact
- **THEN** it writes one immutable record, updates one sidecar, and leaves the selected artifact byte-identical

#### Scenario: Negative - sidecar replacement fails
- **GIVEN** a valid new record and an injected failure before atomic sidecar replacement
- **WHEN** the binding transaction runs
- **THEN** it reports a deterministic transaction diagnostic and restores the exact prior repository state

### Requirement: CAB-012 Artifact binding freshness

Static validation MUST require the mounted artifact node, requirement node, atom revision, sidecar, and immutable record to agree exactly.

A physical root change alone MUST NOT affect freshness. A logical path, artifact byte, requirement, atom, sidecar, or record change MUST affect freshness.

#### Scenario: Positive - physical root moves
- **GIVEN** one fresh artifact binding and equal mounted bytes under another physical root
- **WHEN** static validation uses the same logical mount declarations
- **THEN** the binding remains fresh and supplies navigation only

#### Scenario: Negative - opaque artifact bytes change
- **GIVEN** one fresh sidecar attachment to an opaque artifact
- **WHEN** one artifact byte changes without a new binding transaction
- **THEN** validation fails with the deterministic stale-implementation diagnostic
