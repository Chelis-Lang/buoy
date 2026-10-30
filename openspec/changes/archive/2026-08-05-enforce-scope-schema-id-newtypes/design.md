## Context

`buoy-core` is the `no_std` functional core. The first phase, `enforce-stable-id-newtype`, added `crates/buoy-core/src/identifier.rs` with the `StableId` newtype, the `IdentifierError` type carrying `SCF-CONSTRUCT-STABLE-ID`, and `From<IdentifierError> for ConstructionError`. It also added the deterministic shell gate `src/identifier_policy.rs`. This phase reuses all of that.

Three grammars remain in `crates/buoy-core/src/model.rs` as free functions with `require_*` wrappers: `scope_id_grammar` for lowercase authority, owner, namespace, tool, and ratchet IDs; `schema_id_grammar` for schema IDs; and `prefix_grammar` for namespace prefixes. Every constructor for these roles takes `impl Into<String>` and calls the matching `require_*`.

The imperative shell depends inward on the core and owns all effects. The canonical bytes bind each identifier as `CanonicalValue::Text`. The grammars, the NFC rule, and the diagnostic codes `SCF-CONSTRUCT-SCOPE-ID` and `SCF-CONSTRUCT-SCHEMA-ID` are frozen. This phase moves the remaining validation to type boundaries and changes no byte or code.

## Goals / Non-Goals

**Goals:**

- Represent the scope, schema, and namespace-prefix grammars with the `ScopeId`, `SchemaId`, and `NamespacePrefix` newtypes and one construction path each.
- Make every identifier role type-distinct, so a transposed argument of any role fails to compile.
- Type the last `&str` lookup argument, the namespace prefix of `stable_id_has_prefix`.
- Remove the last identifier grammar call from `model.rs`.
- Extend the existing gate to the new roles.
- Keep the core `no_std`, keep the one-way `model -> identifier` dependency, and keep every canonical byte and diagnostic code identical.

**Non-Goals:**

- No stable-id work; the first phase completed it.
- No grammar, NFC rule, byte, or diagnostic code change.
- No newtype for free-text fields (statement, rationale, media type, tool version).
- No change to revision digests, object digests, migration, freshness, or assurance class.
- No new runtime dependency and no formal proof obligation. Verus and AeneasVerif target the digest and canonical-value kernel, not the identifier constructors, so this change adds no proof binding and invalidates none.

## Decisions

**Extend the existing `identifier.rs` module and `IdentifierError`, do not add a second module.**
`ScopeId`, `SchemaId`, and `NamespacePrefix` join `StableId` in `crates/buoy-core/src/identifier.rs`, and `IdentifierError` gains `ScopeIdGrammar` and `SchemaIdGrammar` variants whose `code` method returns the frozen codes. One module keeps the one-way `model -> identifier` dependency and gives the gate one allowlisted location for every grammar. Alternative considered: a module per role. Rejected as needless surface for four small newtypes that share one error and one gate.

**`NamespacePrefix` reuses the scope diagnostic family.**
`prefix_grammar` has no distinct construction diagnostic today; a failed prefix surfaces as a registry error. `NamespacePrefix::new` returns the scope-id error variant, because the prefix grammar is the scope grammar restricted to the registry, and this keeps the code set frozen. Alternative considered: a new prefix code. Rejected, because it would add a diagnostic that no current behavior emits.

**Each newtype exposes `new(&str) -> Result<Self, IdentifierError>`, `as_str`, and the standard derives.**
This matches `StableId` and `RepositoryPath` and lets each identifier serve as a `BTreeSet`/`BTreeMap` key. Construction runs NFC then the matching grammar, then stores the normalized `String`.

**Model constructors take the typed identifier by value; the prefix lookup takes the borrowed type.**
Each scope, schema, and prefix parameter becomes `ScopeId`, `SchemaId`, or `NamespacePrefix`. `stable_id_has_prefix(id: &StableId, prefix: &NamespacePrefix)` types both arguments. A caller with a `&str` parses once at its own boundary. `require_scope_id` and `require_schema_id` are deleted from `model.rs`.

**Cross-role separation is now total.**
With every identifier role typed, `IdAllocation::new(namespace: ScopeId, stable_id: StableId, …)` and `DebtRecord::new(id: StableId, …, owner: ScopeId, …)` reject a transposed argument at compile time. This is the last defect the two phases set out to remove. Alternative considered: a runtime role check. Rejected, because the type system removes the need for one.

**Extend `src/identifier_policy.rs` rather than add a second gate.**
The gate gains rejection rules for an `impl Into<String>` scope, schema, or prefix parameter and for a `require_scope_id` or `require_schema_id` call outside the identifier module. It keeps its normalized, totally ordered, fail-closed findings. Alternative considered: a separate gate per role. Rejected, because one gate over one module is simpler to reason about.

**Acceptance oracle.** The phase is accepted when three conditions hold together: the `buoy-core` test suite passes, the extended `src/identifier_policy.rs` gate returns an empty finding set on a tree where `model.rs` contains no identifier grammar call and no `impl Into<String>` identifier parameter, and a full regeneration of the committed `fixtures/` and assurance digests produces a byte-identical, empty diff. The empty diff is the exact proof that moving the remaining identifier validation to type boundaries changed no canonical byte.

## Risks / Trade-offs

- **Wide mechanical edit across `src/` and `tools/` call sites.** → Each edit wraps a `&str` in the matching newtype constructor. The grammar and error are unchanged, so a converted call site fails only where it already would have failed at runtime.
- **`NamespacePrefix` sharing the scope diagnostic could surprise a reader.** → The design records the reuse and its reason. No code changes, because the prefix path never emitted a distinct construction code.
- **A constructor mixes two scope-role arguments, for example a namespace and an owner.** → Both are `ScopeId`, so the type system does not separate two values of the same role. This is out of scope; the grammar is identical and the canonical field tags already keep them positionally distinct.
- **The gate can be evaded by an unusual spelling.** → The gate allowlists exactly one module for every grammar and scans for the parameter shape. A rustc-lint follow-up can later prove the property over the AST.
- **Digest drift risk if a newtype normalizes differently.** → Construction runs the same NFC and grammar bytes as the `require_*` functions today, and `as_str` returns those exact bytes. The acceptance oracle's empty regeneration diff is the gate against drift.
