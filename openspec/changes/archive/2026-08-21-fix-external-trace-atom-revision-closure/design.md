## Context

`Atom` exposes two collision-resistant digests with different meanings:

- `Atom::revision()` identifies semantic requirement content.
- `Atom::meta().digest()` identifies the complete atom object.

`ExternalTraceLink` accepts `AtomRevisionRef`, but its constructor also creates generic `ObjectRef` values from those revision digests. Generic object closure compares each value with the atom object digest.

A valid current link therefore fails when the two domains differ. The dedicated atom-revision closure already compares semantic revisions correctly, but it omits external trace links.

## Goals / Non-Goals

**Goals:**

- Make external-link freshness use semantic atom revisions only.
- Keep canonical link fields and digests unchanged.
- Produce one deterministic stale diagnostic for a wrong or old semantic revision.
- Prove the behavior through core and saved-snapshot regression tests.

**Non-Goals:**

- Change generic `ObjectRef` semantics.
- Change atom construction or either digest domain.
- Change adapters, fence syntax, report schemas, or provider behavior.
- Change the trace command's diagnostic-selection policy.
- Add a formal proof or effectful execution result.

## Decisions

### Keep atom revisions out of generic object references

`ExternalTraceLink::new` will keep the atom-revision set in its canonical fields. It will no longer duplicate those values as generic `ObjectRef` metadata.

This matches `ImplementationLink`, which also stores atom revisions in canonical fields and uses dedicated closure validation.

The change does not alter the link digest. `ObjectMeta` computes that digest from canonical fields, while references support closure checks.

### Extend dedicated atom-revision closure

`validate_atom_reference_closure` will call `validate_atom_references` for every external trace link.

`additional_object_dependencies` will also derive each external link's atom dependencies from the same set. Requirement changes will therefore retain transitive link impact without generic object references.

For current graph `G` and link atom set `A`, freshness is:

```text
fresh(A, G) = every (id, revision) in A exists as
              (Atom::meta().id(), Atom::revision()) in G
```

`Atom::meta().digest()` does not participate in this relation.

The existing diagnostic remains `SPG-REF-DANGLING-OR-STALE`. Canonical diagnostic sorting and deduplication remain unchanged.

### Add regression evidence before production edits

First update the core test so a current link uses `atom.revision()`. Require it to fail before the fix.

Add a negative case that uses `atom.meta().digest()` in the revision field. Require the stale diagnostic after the fix.

Add an end-to-end OpenSpec version 2 repository regression. It will use one current `documents` link with unequal atom revision and object digest values.

No Hegel property test is necessary. The behavior is one finite membership predicate with direct positive and negative examples.

### Preserve the functional-core boundary

All changes remain in `buoy-core` validation and deterministic tests. No shell adapter gains an effect.

The core remains safe, sequential, deterministic, and `no_std`. The root crate continues to depend inward on the core.

### Maintain existing assurance bindings without expanding them

The existing core proof record binds the complete `algorithms.rs` file. The source edit therefore changes its implementation artifact digest.

Replay the selected Verus command before the digest update. The replay must retain 9 verified obligations and 0 errors.

Update the proof and derivation records with the new file digest. Do not add an external-link theorem or expand the approved correspondence scope.

Replay both bounded adversarial campaigns because their source scopes include the changed core file and tests.

### Keep assurance classes separate

A fresh external link provides navigation and impact only. The fix adds no evidence, execution verdict, approval, structural class, or proof class.

Static freshness remains independent from provider availability. Buoy does not fetch the target.

## Alternatives Rejected

- **Put the atom object digest in the fence**: This violates the semantic revision contract and hides requirement drift.
- **Make every atom `ObjectRef` compare against semantic revisions**: This changes generic object-reference semantics for unrelated object types.
- **Accept either digest**: This makes one field ambiguous and weakens stale detection.
- **Suppress the diagnostic in static output**: This hides invalid closure instead of repairing it.

## Risks / Trade-offs

- **Existing malformed links use atom object digests** → They will now fail deterministically and require the documented semantic revision.
- **A second closure path emits duplicate diagnostics** → Generic references are removed, and final diagnostics remain sorted and deduplicated.
- **Canonical identity changes accidentally** → Freeze link digests before the edit and require equality after the edit.
- **Other atom-reference users regress** → Run focused graph contracts and the complete acceptance oracle.

## Validation

Focused validation will run:

```text
devenv shell -- cargo test -p buoy-core --test graph_validation_contract external_trace_link -- --nocapture
devenv shell -- cargo test --test openspec_v2_identity version_two_external_trace_link -- --nocapture
devenv shell -- cargo test --test openspec_traceability external_trace -- --nocapture
devenv tasks run buoy:examples --mode single
```

The exact repository acceptance oracle is:

```text
devenv test
```

Success requires all tests, strict OpenSpec validation, formatting, Clippy, generated assurance checks, and deterministic examples to pass.

## Rollback

Revert the constructor and closure-loop edits together. Revert their regression expectations in the same change.

Do not retain only one half. That state either restores the false stale diagnostic or removes external-link freshness checks.
