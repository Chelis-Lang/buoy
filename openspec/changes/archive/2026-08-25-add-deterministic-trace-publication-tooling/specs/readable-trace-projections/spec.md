## ADDED Requirements

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
