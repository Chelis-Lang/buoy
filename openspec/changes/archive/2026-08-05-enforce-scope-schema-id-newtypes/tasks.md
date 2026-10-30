## 1. Scope, schema, and prefix newtype tests (test-first)

- [x] 1.1 Add positive unit tests: a valid string for each role parses through `ScopeId::new`, `SchemaId::new`, and `NamespacePrefix::new`, and `as_str` returns the NFC-normalized input.
- [x] 1.2 Add negative unit tests: an invalid or non-NFC string returns `SCF-CONSTRUCT-SCOPE-ID` for the scope and prefix roles and `SCF-CONSTRUCT-SCHEMA-ID` for the schema role, and produces no typed value.
- [x] 1.3 Add differential tests that assert each constructor accepts and rejects exactly the same strings as the current `require_scope_id`, `require_schema_id`, and `prefix_grammar` logic.
- [x] 1.4 Add a canonical-parity test that an object built through the new newtypes encodes to the same `CanonicalValue::Text` bytes as the current `String` fields.

## 2. Extend the identifier module

- [x] 2.1 Move `scope_id_grammar`, `schema_id_grammar`, and `prefix_grammar` from `model.rs` into `crates/buoy-core/src/identifier.rs` unchanged.
- [x] 2.2 Add `ScopeIdGrammar` and `SchemaIdGrammar` variants to `IdentifierError`, and return `SCF-CONSTRUCT-SCOPE-ID` and `SCF-CONSTRUCT-SCHEMA-ID` from `code`.
- [x] 2.3 Add `ScopeId`, `SchemaId`, and `NamespacePrefix`, each with a private `String`, a `new(&str) -> Result<Self, IdentifierError>` constructor that runs NFC then its grammar, an `as_str`, and the standard derives. `NamespacePrefix::new` returns the scope-id error variant.
- [x] 2.4 Confirm the compatibility-facade crate `crates/spec-provenance-core/src/lib.rs` re-exports the three new types and that the crate stays `#![no_std]`.
- [x] 2.5 Confirm tasks 1.1 through 1.4 pass.

## 3. Convert scope, schema, and prefix constructors

- [x] 3.1 Convert the `ScopeId`-role parameters to `ScopeId`: `authority`, `authority_id`, namespace authorities, `owner`, `ratchet_id`, and the tool `id`.
- [x] 3.2 Convert the `SchemaId`-role parameters to `SchemaId`: `schema`, `adapter_id`, `configuration_schema_id`, `encoding_schema_id`, and `language_schema_id`.
- [x] 3.3 Convert the `IdAllocation` and `NamespaceRegistry` namespace and prefix fields and constructor arguments to `ScopeId` and `NamespacePrefix`.

## 4. Type the prefix lookup and remove the last grammar calls

- [x] 4.1 Change the prefix argument of `stable_id_has_prefix` from `&str` to `&NamespacePrefix`.
- [x] 4.2 Remove `require_scope_id` and `require_schema_id` from `model.rs`.
- [x] 4.3 Confirm `crates/buoy-core/src/model.rs` contains no identifier grammar call and no `impl Into<String>` identifier parameter of any role.

## 5. Convert shell, tools, tests, and fixtures

- [x] 5.1 Update every scope-, schema-, or prefix-keyed model-construction call site in `src/` to parse its identifier once with the matching newtype constructor.
- [x] 5.2 Update every such call site in `tools/` the same way.
- [x] 5.3 Update unit tests, property tests, and fixture builders to use the new newtypes, and keep the negative grammar tests bound to the same expected codes.
- [x] 5.4 Confirm the whole workspace builds and the `buoy-core` test suite passes.

## 6. Extend the boundary gate (test-first)

- [x] 6.1 Add negative gate fixtures for the new roles: one `impl Into<String>` scope parameter, one `impl Into<String>` schema parameter, and one `require_scope_id` or `require_schema_id` call outside the allowlisted module, and assert the exact stable findings in total order.
- [x] 6.2 Add a positive gate fixture: a fully typed model source in which every identifier parameter of every role is a newtype, and assert an empty finding set.
- [x] 6.3 Extend `src/identifier_policy.rs` to reject the scope, schema, and prefix boundaries and confirm it keeps normalized, totally ordered, fail-closed findings.
- [x] 6.4 Confirm tasks 6.1 and 6.2 pass.

## 7. Acceptance oracle

- [x] 7.1 Regenerate every committed artifact under `fixtures/` and the assurance digests, and confirm a byte-identical, empty diff.
- [x] 7.2 Run the `buoy-core` test suite, the extended `src/identifier_policy.rs` gate, and the fixture regeneration together, and confirm all three succeed.
- [x] 7.3 Update the identifier and construction documentation to describe the fully typed identifier boundary across every role.
