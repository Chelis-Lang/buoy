## MODIFIED Requirements

### Requirement: ASC-007 Shell-side adapter SDK and consumer-owned UX
The system MUST expose `buoy.adapter-sdk/v3` from the imperative shell. Each configured adapter MUST declare immutable adapter, parser, schema, capability, configuration, and source-item kind identities. Each adapter MUST consume only canonical saved paths, saved bytes, and explicit configuration. Each adapter MUST emit complete validated core values, parser-backed source maps, nonnormative dispositions, or deterministic diagnostics. Consumer state and registration order MUST NOT alter canonical output. Host parsers, variants, and user-interface behavior MUST NOT enter `buoy-core`.

Schema v3 MUST NOT provide a legacy parsed-item reader, identity migration, opaque fallback, cross-adapter kind, or host-specific core enum.

#### Scenario: Positive - shell-side host adapter is deterministic
- **GIVEN** one versioned host adapter receives identical saved snapshots and explicit configuration under different roots and registration orders
- **WHEN** the shell materializes and validates its complete output
- **THEN** the core values, source maps, diagnostics, canonical graph, and reports are byte-identical while consumer state remains excluded

#### Scenario: Negative - host item uses an undeclared source-item kind
- **GIVEN** an adapter emits a source item kind that its descriptor does not declare
- **WHEN** adapter protocol validation runs
- **THEN** validation fails with `SPG-ADAPTER-UNDECLARED-SOURCE-ITEM-KIND` before graph construction
