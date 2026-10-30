# Readable Trace Projections

## Purpose

Define deterministic, read-only views of current and historical implementation bindings.

## Requirements

### Requirement: RTP-001 Explicit read-only trace selection

Buoy MUST expose one read-only static trace query. The query MUST require exactly one typed requirement ID, relation ID, implementation path, or record-digest selector.

A malformed typed selector or well-formed selector without a matching object MUST return input exit `70`.

#### Scenario: Positive - requirement ID selects current links
- **GIVEN** a current requirement with one fresh implementation binding
- **WHEN** a user selects its stable requirement ID
- **THEN** the trace query returns that current relation and its exact implementation target

#### Scenario: Negative - query has conflicting selectors
- **GIVEN** one trace command with requirement and relation selectors
- **WHEN** CLI parsing runs
- **THEN** parsing fails with usage exit `64` before repository materialization

#### Scenario: Negative - typed selector is malformed
- **GIVEN** one trace command with a malformed stable ID, path, or complete digest
- **WHEN** boundary parsing runs
- **THEN** parsing fails with input exit `70` and no trace projection

#### Scenario: Negative - selected object is absent
- **GIVEN** one valid typed selector that matches no materialized object
- **WHEN** trace selection runs
- **THEN** selection fails with input exit `70` and a deterministic not-found diagnostic

### Requirement: RTP-002 Current and historical relation resolution

The trace query MUST resolve current attachments from the complete source graph. It MUST distinguish retained historical records from current records without making history current.

#### Scenario: Positive - relation has current and historical records
- **GIVEN** one stable relation with an old retained record and one current attachment
- **WHEN** a user requests that relation with history
- **THEN** the trace result identifies one current record and the old historical record in deterministic order

#### Scenario: Negative - old record has no current attachment
- **GIVEN** an immutable record that no current source locator selects
- **WHEN** a user selects that record digest
- **THEN** the result marks it historical and MUST NOT report a current implementation link

### Requirement: RTP-003 Readable human trace view

Human trace output MUST lead with stable IDs, titles, logical paths, item names, source spans, relation names, and freshness. It MUST abbreviate digests only for display.

The output MUST retain the exact primary stale diagnostic and the navigation-only assurance boundary.

#### Scenario: Positive - fresh Rust relation is displayed
- **GIVEN** one fresh binding from an OpenSpec requirement to a Rust function
- **WHEN** the trace query uses human format
- **THEN** the result shows both readable locations, the relation ID, current state, abbreviated record digest, and assurance boundary

#### Scenario: Negative - abbreviated digest hides a mismatch
- **GIVEN** one stale record whose displayed prefix equals another record prefix
- **WHEN** human trace rendering runs
- **THEN** the renderer lengthens the digest or shows the complete digest and retains the exact stale diagnostic

### Requirement: RTP-004 Complete deterministic machine trace view

Machine trace output MUST use versioned schema `buoy/trace-report/v1`. It MUST contain the graph digest, complete object digests, logical paths, source spans, freshness facts, and canonical relation order.

Machine output MUST exclude physical roots, provider state, clocks, locale, terminal dimensions, and traversal order.

The trace report MUST share its fresh-link projection facts with frozen `buoy/binding-tree-report/v1` output. It MUST NOT reinterpret that existing schema.

#### Scenario: Positive - equal mounted snapshots produce equal reports
- **GIVEN** equal logical snapshots under different physical roots
- **WHEN** both trace queries use machine format
- **THEN** they produce byte-identical trace-report JSON with complete digests

#### Scenario: Positive - fresh single link agrees with bind output
- **GIVEN** one fresh relation immediately after a successful bind operation
- **WHEN** bind-tree and trace-report renderers consume the shared projection facts
- **THEN** both reports contain equal relation, requirement, implementation, record, path, and freshness values

#### Scenario: Negative - physical mount path enters a report
- **GIVEN** a machine renderer that emits one caller-supplied absolute root
- **WHEN** report validation runs
- **THEN** validation fails with a deterministic forbidden-trace-field diagnostic

### Requirement: RTP-005 Derived Markdown trace projection

Buoy MUST render a deterministic Markdown projection from the same trace report. The projection MUST contain readable links, current state, relation data, and a derived-data warning.

The trace command MUST write the projection only to standard output. It MUST NOT modify an authority or implementation repository.

#### Scenario: Positive - caller creates an implementation-owned trace page
- **GIVEN** one fresh relation and one explicit trace selector
- **WHEN** the caller requests Markdown format
- **THEN** standard output contains one stable trace page that points to both logical source locations

#### Scenario: Negative - projection is used as authority
- **GIVEN** a Markdown trace projection without its canonical binding record
- **WHEN** static authority validation runs
- **THEN** the projection supplies no authority, freshness, execution, coverage, approval, correctness, or proof

### Requirement: RTP-006 Static and effect-free trace operation

The trace query MUST use saved repository bytes and deterministic static analysis only. It MUST NOT invoke oracles, fetch targets, write files, or inspect provider state.

The command MUST emit a trace projection after successful materialization. A stale selected current attachment or required graph blocker MUST return static-blocker exit `2` after output.

Historical rows without current attachments MUST NOT change a successful exit class.

#### Scenario: Positive - offline trace query succeeds
- **GIVEN** complete saved mounts and no network access
- **WHEN** a user requests a current trace
- **THEN** the command returns the same result as an equal connected environment

#### Scenario: Positive - historical record remains informational
- **GIVEN** one selected historical record without a current attachment
- **WHEN** the trace query emits that record
- **THEN** the projection marks it historical and the command returns success

#### Scenario: Negative - selected current relation is stale
- **GIVEN** one selected current attachment with a changed requirement or implementation subject
- **WHEN** the trace query resolves that relation
- **THEN** it emits the projection, returns exit `2`, and retains `CAB-REQUIREMENT-STALE` or `CAB-SUBJECT-STALE`

### Requirement: RTP-007 Publication-complete Markdown trace identity

The Markdown trace projection MUST label its graph value as the current source-graph digest.

Each current relation row MUST expose the complete semantic atom revision and complete immutable record digest.

The row MUST retain both logical locations and all freshness facets.

The projection MUST identify renderer schema `buoy/readable-trace-markdown/v1`, its trace-report schema, and typed selector.

It MUST retain the derived-data warning and navigation-only assurance boundary.

Any byte-affecting renderer contract change MUST use a new renderer schema version.

Human-readable additions MUST derive from existing `buoy/trace-report/v1` fields. They MUST NOT change that machine schema or frozen bind-report meanings.

#### Scenario: Positive - published page identifies the complete chain

- **GIVEN** one current relation with complete machine trace facts
- **WHEN** Markdown trace rendering runs
- **THEN** the page identifies its source graph, selector, atom revision, record, logical locations, relation, state, and freshness

#### Scenario: Negative - graph domain is ambiguous

- **GIVEN** one Markdown projection that labels a source-graph digest only as `Graph`
- **WHEN** publication projection conformance runs
- **THEN** conformance fails with a deterministic ambiguous-graph-label diagnostic

#### Scenario: Negative - page omits the atom revision

- **GIVEN** one current relation whose Markdown row omits its complete semantic atom revision
- **WHEN** publication projection conformance runs
- **THEN** conformance fails while `buoy/trace-report/v1` remains unchanged

#### Scenario: Negative - renderer bytes change without a new schema

- **GIVEN** one renderer change that alters canonical Markdown bytes under the existing schema
- **WHEN** renderer contract tests run
- **THEN** validation fails before an immutable publication basis uses the stale renderer identity
