## Context

`buoy-core` is the `no_std` functional core. `crates/buoy-core/src/value.rs` already parses two identity grammars into types, `SemanticDigest` and `RepositoryPath(String)`. Each type has one fallible constructor, a private inner field, and an accessor for canonical encoding. The stable-id grammar lives one layer up in `crates/buoy-core/src/model.rs` as the free function `stable_id_grammar` plus its `require_stable_id` wrapper. Every object constructor takes `id: impl Into<String>` and calls `require_stable_id`.

The imperative shell (`src/`, `tools/`) depends inward on the core and owns all effects. The shell already runs deterministic repository gates that emit totally ordered findings, for example `src/hashing_policy.rs` and `src/product_identity.rs`. A custom rustc lint crate `dylint_lints/fcis_core` enforces the functional-core and imperative-shell boundary.

The canonical bytes of every object bind the stable ID as `CanonicalValue::Text`. The grammar, the NFC rule, and the diagnostic code `SCF-CONSTRUCT-STABLE-ID` are frozen. This phase moves validation to a type boundary and builds the shared identifier module that the second phase extends. It does not change any byte or code.

## Goals / Non-Goals

**Goals:**

- Represent the uppercase stable object ID grammar with the `StableId` newtype and one construction path.
- Make an unvalidated stable ID unrepresentable after construction, so no downstream code re-validates.
- Remove unchecked `from_validated` bypasses for stable-ID roles.
- Build the shared `identifier` module and `IdentifierError` that the scope, schema, and prefix phase reuses.
- Keep the core `no_std`, keep the one-way `model -> identifier` dependency, and keep every canonical byte and diagnostic code identical.
- Add one deterministic gate that fails closed if the stable-id boundary regresses.

**Non-Goals:**

- No `ScopeId`, `SchemaId`, or `NamespacePrefix` newtype. The second phase adds these.
- No stable-id grammar, NFC rule, byte, or diagnostic code change.
- No newtype for free-text fields (statement, rationale, remediation).
- No change to revision digests, object digests, migration, freshness, or assurance class.
- No new runtime dependency and no formal proof obligation. Verus and AeneasVerif target the digest and canonical-value kernel, not the identifier constructors, so this change adds no proof binding and invalidates none.

## Decisions

**Place `StableId` and the grammar in a new `crates/buoy-core/src/identifier.rs`, below `model.rs`.**
The grammar and the newtype move into one module that `model.rs` imports, mirroring how `value.rs` holds `RepositoryPath`. This keeps the one-way dependency: `model` uses `identifier`, never the reverse. The module is sized to hold the scope, schema, and prefix newtypes in the second phase, so the shared error and location exist from the start. Alternative considered: keep `StableId` in `model.rs`. Rejected, because the grammar is pure value logic and a separate module gives the gate one exact allowlisted location.

**Introduce a local `IdentifierError` that carries the frozen code.**
`identifier.rs` cannot depend on the model-level `ConstructionError` without inverting the layer order. `IdentifierError::StableIdGrammar` returns `SCF-CONSTRUCT-STABLE-ID` through a `code(self) -> &'static str` method, exactly as `PathError` does. `ConstructionError` gains `From<IdentifierError>` so model constructors keep using `?` and emit the same code. The enum is introduced with the one variant this phase needs; the second phase adds the scope and schema variants.

**`StableId` exposes `new(&str) -> Result<Self, IdentifierError>`, `as_str`, and derives `Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Hash`.**
This matches `RepositoryPath` and lets a `StableId` serve as a `BTreeSet`/`BTreeMap` key. Construction runs NFC then `stable_id_grammar`, then stores the normalized `String`.

**Model constructors take `StableId` by value; stable-ID bearing references become infallible.**
`ObjectRef::new(kind, id: StableId, digest)` no longer parses, so it drops its `Result` and `from_validated` twin. `ObligationRef::new` takes a typed policy ID but stays fallible because it validates the obligation shape. A caller with a `&str` parses once at its boundary with `StableId::new(...)?`, then passes the typed value inward. This structure enforces the single-parse rule. Alternative considered: keep stable-ID `from_validated` methods that take `StableId`. Rejected, because the ordinary constructors already accept typed inputs.

**Stable-id-keyed lookups take the borrowed typed identifier.**
`NamespaceRegistry::reservation(&self, id: &StableId)` replaces the `&str` parameter, and the stable-id argument of `stable_id_has_prefix` becomes `&StableId`. The namespace-prefix argument stays `&str` in this phase and becomes `NamespacePrefix` in the second phase. Alternative considered: leave lookups on `&str`. Rejected, because a raw slice is the exact defect this phase removes.

**Enforce the boundary with a deterministic shell gate, `src/identifier_policy.rs`.**
The gate scans `crates/buoy-core/src/model.rs` and rejects each `impl Into<String>` stable-ID parameter. It also rejects each stable-ID `from_validated` bypass and each `require_stable_id` call outside `crates/buoy-core/src/identifier.rs`. The gate temporarily allows `PolicyRatchetImpact::from_validated`, because its `ratchet_id` is a scope ID. The second phase types that scope role and removes the carve-out. The gate emits normalized findings in a total order with stable codes, as `src/hashing_policy.rs` does. Alternative considered: extend the `dylint_lints/fcis_core` rustc lint to inspect function signatures. A later change can add that lint. The token scanner matches the existing gate pattern and needs no rustc driver changes.

**Acceptance oracle.** The phase is accepted when three conditions hold together: the `buoy-core` test suite passes, the `src/identifier_policy.rs` gate returns an empty finding set on the converted tree, and a full regeneration of the committed `fixtures/` and assurance digests produces a byte-identical, empty diff. The empty diff is the exact proof that moving stable-id validation to the type boundary changed no canonical byte.

## Risks / Trade-offs

- **Wide mechanical edit across `src/` and `tools/` call sites.** → Each edit wraps a `&str` in `StableId::new(...)?`. The grammar and error are unchanged, so a converted call site fails only where it already would have failed at runtime.
- **A constructor that dropped its `Result` breaks callers that used `?`.** → The build surfaces every such site. Infallible construction is the intended payoff, and the fixture no-op diff confirms behavior is unchanged.
- **The token-scan gate can miss an unusual spelling.** → The gate scans defined parameter shapes and allows one identifier module. A later rustc lint can prove the property over the AST.
- **One scope-role bypass remains during this phase.** → The gate allows only `PolicyRatchetImpact::from_validated`. The second phase types its scope ID and removes the carve-out.
- **A constructor mixes a stable ID and a scope-role `String` that stays untyped this phase.** → The stable-id parameter is already `StableId`, so passing it where a `String` is expected does not compile. Full cross-role separation lands when the second phase types the scope role.
- **Digest drift risk if `StableId` normalizes differently.** → Construction runs the same NFC and `stable_id_grammar` bytes as `require_stable_id` today, and `as_str` returns those exact bytes. The acceptance oracle's empty regeneration diff is the gate against drift.
