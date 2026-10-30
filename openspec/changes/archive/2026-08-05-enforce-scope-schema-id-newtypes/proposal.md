## Why

The first phase, `enforce-stable-id-newtype`, typed the uppercase stable object ID and built the shared identifier module `crates/buoy-core/src/identifier.rs`. Three identifier grammars remain raw `String`, still validated imperatively in `crates/buoy-core/src/model.rs`: the lowercase scope ID (`require_scope_id`, about ten sites), the schema ID (`require_schema_id`, about six sites), and the namespace prefix (`prefix_grammar`). This phase finishes the "parse, don't validate" boundary for identifiers and removes the last defect: distinct identifier roles are interchangeable.

`IdAllocation::new(namespace_id, stable_id, …)` (`model.rs:1735`) and `DebtRecord::new(id, …, owner, …)` (`model.rs:4782`) take a scope ID and a stable ID next to each other. After the first phase the stable ID is typed, but the scope ID is still `String`, so a transposed scope-role argument still compiles. Typing the scope, schema, and prefix grammars closes the last gap and lets the identifier-boundary gate cover every role.

This phase depends on the first phase. It reuses the `identifier` module, the `IdentifierError` type, and the deterministic gate, and adds the remaining variants and newtypes.

## What Changes

- Add three validated newtypes to `crates/buoy-core/src/identifier.rs`: `ScopeId` for lowercase authority, owner, namespace, tool, and ratchet IDs; `SchemaId` for schema IDs; and `NamespacePrefix` for namespace prefixes. Each wraps a private NFC `String`, exposes one fallible `new(&str)` constructor that runs the existing grammar, and exposes `as_str`.
- Add the `ScopeIdGrammar` and `SchemaIdGrammar` variants to `IdentifierError`, each returning the frozen code `SCF-CONSTRUCT-SCOPE-ID` or `SCF-CONSTRUCT-SCHEMA-ID`. `NamespacePrefix` reuses the scope diagnostic family for its prefix grammar.
- Move `scope_id_grammar`, `schema_id_grammar`, and `prefix_grammar` behind the newtype constructors. The parse step becomes the only place that validates these identifiers. The grammar bytes and the diagnostic codes do not change.
- **BREAKING**: Change every model constructor that takes a scope, schema, or namespace-prefix identifier from `impl Into<String>` plus a `require_*` call to the typed parameter (`ScopeId`, `SchemaId`, or `NamespacePrefix`). The break is contained to this repository, because no external crate consumes `buoy-core`.
- **BREAKING**: Change the namespace-prefix argument of `stable_id_has_prefix` from `&str` to `&NamespacePrefix`, so both arguments of the prefix comparison are typed after this phase.
- Remove the `require_scope_id` and `require_schema_id` free functions from `model.rs`. After this phase, `model.rs` contains no identifier grammar call and no `impl Into<String>` identifier parameter.
- Extend the `src/identifier_policy.rs` gate to fail closed on an `impl Into<String>` scope, schema, or prefix parameter and on a `require_scope_id` or `require_schema_id` call outside the identifier module.
- **Non-goals:** any identifier grammar, byte, or diagnostic code change; any change to canonical encoding, revision digests, migration, or freshness; a newtype for free-text fields such as statements, rationales, media types, or tool versions; a new runtime dependency; relaxing `no_std`; re-doing the stable-id work that the first phase completed.

## Capabilities

### New Capabilities

- `scope-schema-id-newtypes`: the `ScopeId`, `SchemaId`, and `NamespacePrefix` validated newtypes, their single parse boundaries, the type-level separation of every remaining identifier role, the typed namespace-prefix lookup, and the extension of the deterministic gate to the scope, schema, and prefix roles.

### Modified Capabilities

<!-- None. The scope, schema, and prefix grammars, their diagnostic codes, and the canonical bytes do not change, so no spec-graph requirement changes. -->

## Impact

- **Core**: `crates/buoy-core/src/identifier.rs` (three new newtypes and two new error variants) and `crates/buoy-core/src/model.rs` (every constructor with a scope, schema, or prefix parameter, the `IdAllocation`/`NamespaceRegistry` scope and prefix fields, the `stable_id_has_prefix` prefix argument, and the removed `require_scope_id` and `require_schema_id` helpers).
- **Facade**: the compatibility-facade crate `crates/spec-provenance-core/src/lib.rs` re-exports `buoy_core::*`, so the new types export without change.
- **Shell and tools**: every caller in `src/` and `tools/` that builds a scope-, schema-, or prefix-keyed model object now parses its identifier once with the matching newtype constructor.
- **Tests and fixtures**: unit tests, property tests, and fixtures that construct these objects update their identifier arguments. Negative grammar tests keep the same expected diagnostic codes.
- **Assurance**: no committed digest changes, because canonical bytes are unchanged. The gate obligation added in the first phase widens to cover the new roles.
