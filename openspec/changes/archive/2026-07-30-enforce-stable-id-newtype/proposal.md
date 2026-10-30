## Why

`buoy-core` already parses two identity grammars into types: `SemanticDigest` and `RepositoryPath(String)` in `crates/buoy-core/src/value.rs`. Stable object IDs do not follow the same rule. They stay raw `String`, and `crates/buoy-core/src/model.rs` re-validates them at every construction site with `require_stable_id`. This contradicts the project "parse, don't validate" rule and produces two concrete defects that this phase removes.

First, the model exposes unchecked stable-ID bypass constructors. `ObjectRef::from_validated` and `ObligationRef::from_validated` skip validation of a `String` that no type proves valid. Second, stable-ID lookups accept any slice. `IdRegistry::reservation(&self, stable_id: &str)` and `stable_id_has_prefix` take an unvalidated `&str`, so unrelated text can flow in.

This is the first of two phases. It carries the shared identifier-module and gate infrastructure and converts the largest identifier role, the uppercase stable object ID used at about eighteen construction sites. A second phase converts the scope, schema, and namespace-prefix grammars. No repository consumes `buoy-core` yet; every workspace crate sets `publish = false`, so the cost to move validation to a type boundary is near its minimum now.

## What Changes

- Add a `StableId` newtype to `buoy-core` for uppercase object IDs. It wraps a private NFC `String`, exposes one fallible `new(&str)` constructor that runs the existing `stable_id_grammar`, and exposes `as_str` for canonical encoding.
- Add the shared identifier module `crates/buoy-core/src/identifier.rs` and a local `IdentifierError` type that carries the frozen diagnostic code `SCF-CONSTRUCT-STABLE-ID`. Add `From<IdentifierError> for ConstructionError` so model constructors keep emitting the same code.
- Move `stable_id_grammar` behind the `StableId` constructor. The parse step becomes the only place that validates a stable ID. The grammar bytes and the diagnostic code do not change.
- **BREAKING**: Change every model constructor that takes a stable object ID from `id: impl Into<String>` plus `require_stable_id` to `id: StableId`. The break is contained to this repository, because no external crate consumes `buoy-core`.
- Remove the stable-ID `from_validated` string bypass constructors. A caller that holds a `StableId` builds the value without a second parse or an unchecked path. `PolicyRatchetImpact::from_validated` remains temporarily because it accepts a scope ID. The second phase removes that scope-role bypass.
- **BREAKING**: Change stable-ID keyed lookups to accept the borrowed typed identifier. `IdRegistry::reservation` and the stable-ID argument of `stable_id_has_prefix` take `&StableId`, not `&str`.
- Add a deterministic static gate, `src/identifier_policy.rs`. The gate rejects an `impl Into<String>` stable-ID parameter, a stable-ID `from_validated` bypass, or a `require_stable_id` call outside the identifier module. It temporarily allows the scope-role `PolicyRatchetImpact::from_validated` method until the second phase.
- **Non-goals:** the `ScopeId`, `SchemaId`, and `NamespacePrefix` grammars, which the second phase converts; any identifier grammar, byte, or diagnostic code change; any change to canonical encoding, revision digests, migration, or freshness; a newtype for free-text fields such as statements, rationales, or media types; a new runtime dependency; relaxing `no_std`.

## Capabilities

### New Capabilities

- `stable-id-newtype`: the `StableId` validated newtype, the shared identifier module and error type, the single parse boundary for stable object IDs, the removal of stable-ID string bypass constructors, the typed-only stable-ID lookup surface, and the deterministic gate that keeps stable-ID validation on the type boundary.

### Modified Capabilities

<!-- None. The stable-id grammar, its diagnostic code, and the canonical bytes do not change, so no spec-graph requirement changes. -->

## Impact

- **Core**: `crates/buoy-core/src/model.rs` (every constructor with a stable-ID parameter, the two stable-ID bypass methods, typed ID fields and lookups, and the private `require_stable_id` helper) and the new `crates/buoy-core/src/identifier.rs`.
- **Facade**: the compatibility-facade crate `crates/spec-provenance-core/src/lib.rs` re-exports `buoy_core::*`, so `StableId` exports without change.
- **Shell and tools**: every caller in `src/` and `tools/` that builds a stable-id-keyed model object now parses its ID once with `StableId::new`. The change is mechanical at each site, because the grammar and error are unchanged.
- **Tests and fixtures**: unit tests, property tests, and fixtures that construct these objects update their stable-id arguments. Negative grammar tests keep the same expected diagnostic code.
- **Assurance**: no committed digest changes, because canonical bytes are unchanged. The new gate adds one obligation to the hardening ledger.
