# Content-Addressed Implementation Bindings

## Purpose

Define compact, immutable implementation bindings between current source nodes and exact requirement atom revisions.

## Requirements

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

### Requirement: CAB-014 Current requirement subject inventory

A static report MUST expose one `requirement_subjects` row for each current OpenSpec requirement authority in a diagnostic-free accepted source graph.

A static report with diagnostics MUST omit `requirement_subjects`. It MUST preserve the complete report, diagnostics, and static exit class.

`buoy bind` MUST reject a repository when its current static check contains diagnostics.

Each row MUST contain the stable requirement ID, exact requirement source-node digest, exact atom revision, logical source path, and ordered authority scopes.

Rows MUST use complete canonical digests. They MUST use logical paths and MUST exclude physical roots, provider data, clocks, locale, and traversal order.

Rows MUST sort by stable ID, authority scopes, logical path, source-node digest, and atom revision. Unequal current subjects MUST NOT collapse.

A report with no current OpenSpec requirement authority MUST omit `requirement_subjects`. The inventory MUST NOT change graph identity or add an assurance class.

#### Scenario: Positive - consumer selects an initial binding subject

- **GIVEN** one current standing requirement with stable ID `SDT-001` and no stored implementation binding
- **WHEN** `buoy static --format machine` renders the accepted repository
- **THEN** one requirement-subject row exposes the exact digest that `buoy bind --requirement` accepts

#### Scenario: Positive - physical roots differ

- **GIVEN** equal selected bytes and equal logical mount declarations under two different physical roots
- **WHEN** both static machine reports are rendered
- **THEN** both complete `requirement_subjects` inventories are byte-identical

#### Scenario: Positive - no OpenSpec authority exists

- **GIVEN** an accepted repository with no current OpenSpec requirement authority
- **WHEN** the static machine report is rendered
- **THEN** the report omits `requirement_subjects` without adding a placeholder row

#### Scenario: Negative - a non-requirement authority enters aggregation

- **GIVEN** a retained source authority identifies a node outside `openspec/requirement/v1`
- **WHEN** requirement-subject aggregation runs
- **THEN** the inventory omits that authority without omitting any current OpenSpec requirement row

#### Scenario: Negative - active change data enters the inventory

- **GIVEN** an active or archived OpenSpec change document that is outside current standing authority selectors
- **WHEN** the static machine report is rendered
- **THEN** no requirement-subject row derives from that planning document

#### Scenario: Negative - an inventory row changes graph identity

- **GIVEN** one accepted source graph and its derived requirement-subject inventory
- **WHEN** report rendering adds the derived rows
- **THEN** the report keeps the original graph digest and adds no graph object, edge, or assurance fact
