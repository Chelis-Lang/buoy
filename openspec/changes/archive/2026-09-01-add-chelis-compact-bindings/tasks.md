## 1. Frozen grammar and store boundary

- [x] 1.1 Add positive and negative fixtures for the two-line `binding` record before production code; cover valid attachment, unattached records, duplicate or unknown fields, and malformed addresses.
- [x] 1.2 Add `binding-store` to the parsed adapter configuration; reject absolute paths, traversal, and paths outside every selector root at the configuration boundary.
- [x] 1.3 Extend extraction with the `binding` kind and preserve the total-record grammar, deterministic ordering, and the existing parse-error behavior.

## 2. Address resolution and core materialization

- [x] 2.1 Resolve `<binding-store>/buoy/objects/v1/<algorithm>/<hex>.toml` only through saved snapshot artifacts; verify object bytes against the declared content address before parsing.
- [x] 2.2 Parse `buoy/implementation-binding/v1` objects into the existing core `ImplementationBinding` type and append them to `graph.implementation_bindings`.
- [x] 2.3 Compare each binding subject and requirement with current source bytes; emit deterministic subject and requirement staleness contexts with bound and current digests.
- [x] 2.4 Emit `CHELIS-PROV-BINDING-ADDRESS` for missing, hash-mismatched, or malformed objects; never materialize rejected bytes.
- [x] 2.5 Add focused crate tests and canonical report goldens for address integrity, subject currency, atom-revision currency, and cross-root determinism.

## 3. Sanctioned conversion

- [x] 3.1 Implement a deterministic `bind` dry-run plan that converts one selected inline `implementation-link` into one compact object and a two-line source marker.
- [x] 3.2 Wire `bind --root --config --link <ID> [--write]`; reject unknown IDs, non-link IDs, parse failures, and destination collisions with nonmatching bytes.
- [x] 3.3 Confirm that `bind --write` changes only the selected record block and the object store; keep the attached declaration and all unrelated bytes identical.
- [x] 3.4 Add conversion tests for dry-run immutability, repeat refusal, declaration immutability, and semantic graph equivalence before and after conversion.

## 4. Rebind and trace integration

- [x] 4.1 Extend `rebind` plans with `object-write`, `address`, and `object-remove` entries; generate a successor object instead of editing an immutable object.
- [x] 4.2 Confirm that `rebind --write` updates the source address, writes the successor object, removes only the superseded object, and converges to an empty second plan.
- [x] 4.3 Extend `trace` to expand compact bindings through the store and report the address, object hash, subject, requirement, atom revision, and attached declaration digest.
- [x] 4.4 Add rebind and trace tests for current and stale bindings, tampered objects, successor-object hygiene, and cross-root byte equality.

## 5. Contract pack and documentation

- [x] 5.1 Add frozen cases for `CHELIS-PROV-019..021`, update the manifest, and extend the requirement coverage loop.
- [x] 5.2 Add adversarial-review rows for object substitution, subject laundering, selector-boundary escape, dry-run mutation, and superseded-object retention.
- [x] 5.3 Update the schema registry, fixture README, Chelis trust-boundary document, and CLI documentation for the compact form and its review workflow.
- [x] 5.4 Run every focused Buoy command from `traceability.md` and record the final implementation relations. No compact implementation relation is planned in this upstream change.

## 6. Assurance convergence

- [x] 6.1 Re-run the fuzz and mutation campaigns through sanctioned tools with honest artifact updates.
- [x] 6.2 Run sanctioned regeneration to a fixed point: evidence, ledger, naming inventory, quality, digest correspondence, and the reviewed source-content digest.
- [x] 6.3 Run `openspec validate --all --strict --no-interactive` and `openspec schema validate buoy-bridge`.
- [x] 6.4 Final oracle: full `devenv test` exits 0.

## Implementation notes

### Compact bindings require a collision-resistant active member

Upstream `parse_stored_binding` rejects any stored binding whose locator,
subject, requirement, or atom revision falls outside
`DigestAssuranceClass::CollisionResistant`, and every digest in one stored
binding must share the member the rest of the graph uses. A build whose
active member is `xxh3-128` therefore resolves no compact binding at all and
reports `algorithm-not-collision-resistant`. Adopting the compact form means
pinning `blake3-256` or `sha-256` as the digest feature for the whole host.

The three positive suites (`compact_bindings`, `bind_conversion`,
`binding_regeneration`) compile only when BLAKE3-256 or SHA-256 is active
without `xxh3-128`. A run-time guard was removed because it reported a pass for
work it never did. `binding_change_detection` asserts fail-closed behavior in
the default change-detection build. The `buoy:chelis-compact-bindings` task
lints and tests both collision-resistant configurations, and `buoy:docs`
depends on it.

### The store is a mount prefix over the frozen upstream layout

`binding-store` names a mount prefix, and objects resolve at exactly
`<binding-store>/buoy/objects/v1/<algorithm>/<hex>.toml` and nowhere else.
This reuses `buoy::adapters::extract_binding_in_mount` verbatim, so Chelis
inherits the canonical-form check, the path-digest agreement check, the schema
and relation checks, and the digest class and member checks without
reimplementing any of them, and a Chelis store holds byte-identical objects to
a Rust store.

### The requirement digest is the authority record block

A compact binding records `requirement` as a source-node digest. The Chelis
analogue of the Markdown atom block is the authority record block itself, so
the requirement digest covers exactly those bytes — the
`-- chelis:provenance/v1 authority` marker line through its last field line,
as saved. Revising a statement moves the requirement digest and the atom
revision together.

### Core atom-revision closure

The integration exposed a missing shared closure edge. `validate_graph` now
checks the atom revision in every `implementation_binding`, so the compact and
inline forms both reject stale authority references with
`SPG-REF-DANGLING-OR-STALE`.
