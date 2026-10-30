## 1. StableId tests (test-first)

- [x] 1.1 Add positive unit tests: a valid stable ID parses through `StableId::new`, and `as_str` returns the NFC-normalized input.
- [x] 1.2 Add negative unit tests: an invalid or non-NFC stable ID returns the frozen code `SCF-CONSTRUCT-STABLE-ID` and produces no typed value.
- [x] 1.3 Add a differential test that asserts `StableId::new` accepts and rejects exactly the same strings as the current `require_stable_id` helper.
- [x] 1.4 Add a canonical-parity test that an object built through `StableId` encodes to the same `CanonicalValue::Text` bytes as the current `String` field.

## 2. Identifier module

- [x] 2.1 Create `crates/buoy-core/src/identifier.rs` and move `stable_id_grammar` into it unchanged.
- [x] 2.2 Add `IdentifierError` with a `StableIdGrammar` variant and a `code(self) -> &'static str` method that returns `SCF-CONSTRUCT-STABLE-ID`.
- [x] 2.3 Add `StableId` with a private `String`, a `new(&str) -> Result<Self, IdentifierError>` constructor that runs NFC then `stable_id_grammar`, an `as_str`, and the derives `Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Hash`.
- [x] 2.4 Register the module in `crates/buoy-core/src/lib.rs`, keep `#![no_std]`, and confirm the compatibility-facade crate `crates/spec-provenance-core/src/lib.rs` re-exports `StableId`.
- [x] 2.5 Add `From<IdentifierError> for ConstructionError` in `model.rs` so constructors keep emitting the frozen code through `?`.
- [x] 2.6 Confirm tasks 1.1 through 1.4 pass.

## 3. Convert stable-id constructors

- [x] 3.1 Change the stable-id parameter of `Atom`, `Tombstone`, `ObjectRef`, and `AtomRevisionRef` from `id: impl Into<String>` plus `require_stable_id` to `id: StableId`.
- [x] 3.2 Convert the remaining stable-ID constructors: `DecisionSurfaceDomain`, `ImplementationLink`, `EvidenceCarrier`, `Derivation`, `CoveragePolicy`, `ObligationRef`, `DebtRecord`, `Waiver`, `Oracle`, `FiniteModel`, `FormalProperty`, `Assumption`, `ProofRecord`, `Reaffirmation`, `ChangeManifest`, and `ChangeDisposition`, including the second stable-ID argument on `Reaffirmation` and `ChangeDisposition`.
- [x] 3.3 Convert the `stable_id` field and constructor argument of `IdAllocation` to `StableId`, and update `IdRegistry` to use that type. Leave the namespace scope and prefix arguments unchanged for the second phase.

## 4. Remove the string bypass and type the stable-id lookups

- [x] 4.1 Delete `ObjectRef::from_validated` and `ObligationRef::from_validated`. Keep `PolicyRatchetImpact::from_validated` until the second phase types its scope-ID parameter.
- [x] 4.2 Update internal model and algorithm callers to build references from an already-typed `StableId`.
- [x] 4.3 Change `IdRegistry::reservation` to take `&StableId`, and change the stable-ID argument of `stable_id_has_prefix` to `&StableId`.
- [x] 4.4 Remove `require_stable_id` from `model.rs`. Confirm that no `impl Into<String>` stable-ID parameter or stable-ID `from_validated` method remains.

## 5. Convert shell, tools, tests, and fixtures

- [x] 5.1 Update every stable-id model-construction call site in `src/` to parse its ID once with `StableId::new` at its own boundary.
- [x] 5.2 Update every stable-id model-construction call site in `tools/` the same way.
- [x] 5.3 Update unit tests, property tests, and fixture builders to use `StableId`, and keep the negative grammar tests bound to `SCF-CONSTRUCT-STABLE-ID`.
- [x] 5.4 Confirm the whole workspace builds and the `buoy-core` test suite passes.

## 6. Stable-id boundary gate (test-first)

- [x] 6.1 Add a positive gate fixture with typed stable-ID parameters, grammar checks in `identifier.rs`, and the allowed scope-role bypass. Assert an empty finding set.
- [x] 6.2 Add negative gate fixtures for one `impl Into<String>` stable-ID parameter, one stable-ID `from_validated` bypass, and one `require_stable_id` call outside the identifier module. Assert the exact stable findings in total order.
- [x] 6.3 Implement `src/identifier_policy.rs` as a deterministic token-scan gate that emits normalized, totally ordered findings and aggregates fail-closed, matching the pattern in `src/hashing_policy.rs`.
- [x] 6.4 Register the gate in the repository check surface and add its obligation row to the hardening ledger.
- [x] 6.5 Confirm tasks 6.1 and 6.2 pass.

## 7. Acceptance oracle

- [x] 7.1 Regenerate every committed artifact under `fixtures/` and the assurance digests, and confirm a byte-identical, empty diff.
- [x] 7.2 Run the `buoy-core` test suite, the `src/identifier_policy.rs` gate, and the fixture regeneration together, and confirm all three succeed.
- [x] 7.3 Update the identifier and construction documentation to describe the typed stable-id boundary and the removed string bypass.
