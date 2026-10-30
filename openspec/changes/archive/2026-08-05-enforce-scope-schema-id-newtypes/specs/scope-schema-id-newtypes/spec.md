## ADDED Requirements

### Requirement: SSD-001 Scope, schema, and prefix newtypes at one construction boundary
The core MUST represent the remaining identifier grammars with distinct newtypes: `ScopeId` for lowercase authority, owner, namespace, tool, and ratchet IDs; `SchemaId` for schema IDs; and `NamespacePrefix` for namespace prefixes. Each newtype MUST hold a private normalized string and MUST expose one fallible constructor that applies NFC normalization and the frozen grammar for that role. A constructor MUST return the typed value on success and a deterministic diagnostic on failure. `ScopeId` and `NamespacePrefix` MUST return the frozen code `SCF-CONSTRUCT-SCOPE-ID`, and `SchemaId` MUST return `SCF-CONSTRUCT-SCHEMA-ID`. After construction, an invalid identifier of these roles MUST be unrepresentable.

#### Scenario: Positive - a valid scope or schema identifier parses into its newtype
- **GIVEN** a string that satisfies the frozen grammar for the scope, schema, or prefix role
- **WHEN** the matching newtype constructor runs
- **THEN** it returns the typed identifier whose canonical text equals the NFC-normalized input

#### Scenario: Negative - an invalid scope or schema identifier is rejected at the boundary
- **GIVEN** a string that violates the frozen grammar or is not NFC for the scope, schema, or prefix role
- **WHEN** the matching newtype constructor runs
- **THEN** it fails with the frozen code for that role and returns no typed value

### Requirement: SSD-002 Every identifier role is type-distinct
After this phase, no model constructor, method, or helper MUST accept an unvalidated string for a scope, schema, or namespace-prefix role, and no public identifier parameter MUST use a shared `String` or `&str` type for two distinct roles. The type system MUST reject a value of one identifier role where a different role is expected, at compile time.

#### Scenario: Positive - each role uses its own type
- **GIVEN** a constructor that takes a namespace `ScopeId` and an object `StableId`
- **WHEN** a caller passes the correct typed value for each parameter
- **THEN** construction accepts the arguments and no runtime role check is needed

#### Scenario: Negative - a transposed scope-role identifier fails to compile
- **GIVEN** a constructor that takes a `ScopeId` and a `StableId` in that order
- **WHEN** a caller passes the `StableId` where the `ScopeId` is expected
- **THEN** the code does not compile

### Requirement: SSD-003 Typed namespace-prefix lookup and no remaining grammar calls
The namespace-prefix comparison MUST take the borrowed `NamespacePrefix` as its prefix argument, so both arguments of the comparison are typed. After this phase, `crates/buoy-core/src/model.rs` MUST contain no `require_scope_id` call, no `require_schema_id` call, and no `impl Into<String>` identifier parameter for any role.

#### Scenario: Positive - the prefix comparison takes typed arguments
- **GIVEN** a `StableId` and a `NamespacePrefix`
- **WHEN** the prefix comparison runs with both borrowed typed values
- **THEN** it returns whether the stable ID carries the namespace prefix

#### Scenario: Negative - a raw prefix slice is unavailable
- **GIVEN** a raw string that is not a `NamespacePrefix`
- **WHEN** code attempts the prefix comparison with that slice
- **THEN** the code does not compile, because the comparison accepts only the borrowed `NamespacePrefix`

### Requirement: SSD-004 Canonical bytes and diagnostic codes are preserved
The `ScopeId`, `SchemaId`, and `NamespacePrefix` newtypes MUST serialize to the identical canonical text bytes as the prior `String` fields, MUST apply the identical NFC and grammar rules, and MUST return the identical stable diagnostic codes `SCF-CONSTRUCT-SCOPE-ID` and `SCF-CONSTRUCT-SCHEMA-ID`. No object revision digest, object digest, canonical graph identity, migration output, or freshness verdict MUST change as a result of this representation change.

#### Scenario: Positive - regeneration produces a byte-identical diff
- **GIVEN** the committed fixtures and assurance digests before this change
- **WHEN** every canonical artifact is regenerated after the scope, schema, and prefix identifiers become newtypes
- **THEN** the regeneration produces a byte-identical, empty diff

#### Scenario: Negative - a grammar or code drift is rejected
- **GIVEN** a candidate build in which a scope, schema, or prefix newtype normalizes differently or returns a changed diagnostic code
- **WHEN** the identifier grammar tests and the fixture regeneration oracle run
- **THEN** they fail with the changed byte or code reported and the change is not accepted

### Requirement: SSD-005 The boundary gate covers every identifier role
The imperative shell gate MUST fail closed if the model source reintroduces an untyped boundary for any identifier role. In addition to the stable-id rules, the gate MUST reject any `impl Into<String>` scope, schema, or prefix parameter on a model constructor and any `require_scope_id` or `require_schema_id` call outside the one allowlisted identifier module. Findings MUST be normalized, totally ordered, and aggregated fail-closed.

#### Scenario: Positive - a fully typed tree yields an empty finding set
- **GIVEN** a model source in which every identifier parameter of every role is a newtype and all grammar checks sit inside the allowlisted identifier module
- **WHEN** the identifier-boundary gate scans the repository
- **THEN** it succeeds with a deterministic empty finding set

#### Scenario: Negative - a reintroduced scope or schema boundary is reported
- **GIVEN** a model constructor that takes `impl Into<String>` for a scope or schema identifier, or a `require_scope_id` or `require_schema_id` call outside the allowlisted module
- **WHEN** the identifier-boundary gate scans the repository
- **THEN** it emits the applicable stable findings in total order and the aggregate gate fails
