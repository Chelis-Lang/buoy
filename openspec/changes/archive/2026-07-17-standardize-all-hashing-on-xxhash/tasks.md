## 1. Baseline and Algorithm Lock

- [x] 1.1 Inventory every repository-owned SHA/BLAKE3 computation, dependency, digest field, fixture, generator, record, cache/fingerprint path, and documentation claim; separately classify ecosystem-mandated Cargo, Nix, Git, release, and signature metadata by exact syntax and field.
- [x] 1.2 Resolve and record an exact licensed `no_std`-compatible XXH3-128 dependency pin, feature closure, upstream algorithm identity, official vector source, and the limits of the independent-reference strategy.
- [x] 1.3 Archive a clean validated version 1 baseline and record the authoritative repository bytes, configuration, schema/tool identities, approvals, lifecycle states, and rollback boundary required for migration.
- [x] 1.4 Freeze the version 2 contract for schema ID `spec-provenance/v2`, encoding version `2`, algorithm tag `2`, seed `0`, standard default secret, official XXH128 canonical byte order, and canonical `xxh3-128:` text.

## 2. Test-First Evidence and Proof Spikes

- [x] 2.1 Commit positive official XXH3-128 vectors for empty, short, stripe-boundary, long, one-shot, and incremental inputs plus negative vectors for wrong variants, seeds, secrets, half order, and byte order before implementing production hashing.
- [x] 2.2 Add positive canonical digest round-trip fixtures and negative SHA, BLAKE3, unknown-tag, wrong-length, uppercase, nonhexadecimal, truncated, padded, and observed-collision fixtures with exact stable diagnostics.
- [x] 2.3 Add positive complete-migration fixtures and negative fixtures for missing authoritative bytes, missing approvals, stale nested bindings, illegal lifecycle changes, mixed versions, duplicate IDs, and partial impact closure.
- [x] 2.4 Add positive and negative hashing-policy fixtures covering core, reference, tools, generators, manifests, transitive features, documentation, cache/fingerprint helpers, and project-selected hash-table hashers before implementing the policy checker.
- [x] 2.5 Add external-integrity boundary fixtures that accept only exact Cargo/Nix/Git/upstream metadata fields as opaque shell data and reject broad path exceptions or promotion into `SemanticDigest`.
- [x] 2.6 Add Hegel-rs properties for deterministic one-shot hashing, incremental/one-shot parity, canonical text/binary round trips, exercised input sensitivity, parser rejection, and migration idempotence, with fixed seeds and replay records.
- [x] 2.7 Define and run an identified finite reference model for active/obsolete/malformed digest acceptance and migration lifecycle preservation; record its finite state space, transition relation, bounds, and checked invariants before production implementation.
- [x] 2.8 Run equivalent Verus and AeneasVerif spikes for the versioned envelope, XXH128 half/byte-order conversion, parser length/tag invariants, and migration lifecycle wrapper, explicitly excluding XXH3 internals and collision claims.
- [x] 2.9 Compare the proof-spike results and record the primary formal-verification lane, exact bindings, unsupported constructs, assumptions, and exclusions before changing production algorithms.

## 3. Version 2 Functional Core and Reference

- [x] 3.1 Add the exact XXH3-128 dependency/features to the `no_std` core and reference test closure while keeping the shell-to-core dependency direction one-way.
- [x] 3.2 Implement the version 2 `DigestAlgorithm` and `SemanticDigest` binary/text representation, strict parsing, official canonical byte order, and deterministic malformed-value diagnostics against the prewritten fixtures.
- [x] 3.3 Implement the version 2 domain-separated canonical preimage envelope and sole core hashing entry point using seed `0` and the standard default secret against official and Hegel vectors.
- [x] 3.4 Implement observed same-digest/different-preimage collision detection wherever both canonical preimages are simultaneously available, preserving duplicate rejection and total ordering.
- [x] 3.5 Update canonical value encoding/decoding, object/reference ordering, schema inventory, and all digest-bound core algorithms for the 16-byte algorithm-tag-2 value without changing static/execution/assurance separation.
- [x] 3.6 Update the independent reference crate to encode version 2 preimages without importing production code, document any shared low-level hasher limitation, and make differential tests pass against official vectors.
- [x] 3.7 Update presentation JSON, human rendering, parser/adaptor diagnostics, and public API examples to emit and accept only canonical active-schema XXH3-128 identities.
- [x] 3.8 Remove direct `sha2`, SHA-family, and BLAKE3 hashing dependencies and code paths from production, development, migration, fixture-generation, and reference hashing closures.

## 4. Migration and Repository Policy

- [x] 4.1 Implement the offline migration driver to regenerate leaf raw-artifact and configuration bindings from authoritative bytes, never from transformed or relabeled old digest bytes.
- [x] 4.2 Extend migration through semantic objects, references, carriers, policies, derivations, assumptions, properties, proof records, graphs, and reports in deterministic dependency order.
- [x] 4.3 Enforce stable-ID and lifecycle preservation during migration so terminal IDs remain terminal, tombstones remain present, approvals remain required, and incomplete objects fail closed.
- [x] 4.4 Implement complete migration impact closure with deterministically ordered revised, reaffirmed, retired, or waived dispositions for every changed digest binding.
- [x] 4.5 Implement the syntax-aware hashing-policy checker and resolved dependency-feature inspection with stable codes, structured fields, normalized paths, total diagnostic ordering, and fail-closed aggregation.
- [x] 4.6 Implement field-specific opaque external-integrity exceptions in the imperative shell and reject external checksum parsing or use as canonical Spec Provenance identity.
- [x] 4.7 Integrate the hashing-policy checker, its self-tests, and its exact formatting/lint inventory into `spec-provenance:gate` without adding ambient inputs to the deterministic core.

## 5. Artifact Regeneration and Documentation

- [x] 5.1 Regenerate every repository-owned canonical vector, fixture manifest, atom revision, graph/report golden, presentation golden, proof/derivation record, freshness binding, and example as version 2 XXH3-128 data.
- [x] 5.2 Update schema and trust-boundary documentation with the exact XXH3-128 profile, version 2 preimage envelope, migration break, observed-collision behavior, and non-cryptographic assurance limitations.
- [x] 5.3 Update testing, reference-model, Hegel, formal-toolchain, proof, adapter, and downstream integration documentation without claiming tests, annotations, digest equality, Verus, or Aeneas establish excluded properties.
- [x] 5.4 Verify that remaining SHA/BLAKE3 strings occur only in field-specific external supply-chain metadata, archival migration explanations, or negative fixtures and that none is computed or accepted as an active identity.
- [x] 5.5 Produce and review the deterministic migration report, complete impact dispositions, schema/tool lock changes, dependency/license inventory, and downstream version 2 cutover instructions.

## 6. Final Verification and Cutover

- [x] 6.1 Run formatting, clippy, all workspace targets/features, official vectors, malformed/migration/policy negative fixtures, Hegel properties, finite reference models, and production/reference differential tests.
- [x] 6.2 Run the bare-metal `no_std` core build and FCIS structural checks to confirm no outward dependency, ambient effect, mutable static, or callable public input was introduced.
- [x] 6.3 Replay the selected formal checks with exact source, tool, assumption, configuration, and proof-record bindings and report all stated exclusions.
- [x] 6.4 Run the exact offline acceptance oracle `devenv tasks run spec-provenance:gate`; accept the change only when it exits `0`, every expected negative fixture emits its exact diagnostic, and the hashing-policy inventory has no unclassified or prohibited findings.
- [x] 6.5 Switch the active schema, code, records, fixtures, tools, and downstream examples atomically to version 2, or restore the complete archived version 1 baseline without mixed or backward-relabeled data.
