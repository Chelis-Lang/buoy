## 1. Resolve blocking decisions and freeze the registry contract

- [x] 1.1 Confirm that no repository consumes Buoy and that every workspace crate still sets `publish = false`. Record the check, because it is the premise for regenerating rather than migrating.
- [x] 1.2 Answer design Open Question 1 and fix the member set for this change. Reserve every tag either way.
- [x] 1.3 Answer design Open Question 2: select a reviewed `sha2` dependency or a promoted in-core implementation for the `sha-256` member.
- [x] 1.4 Freeze the registry table: canonical name, numeric tag, output length, assurance class, and complete parameters for every member. Record tags `0` and `1` as permanently obsolete.
- [x] 1.5 Freeze the allowlisted adapter-module paths and the exact preimage-to-bytes signature that each module exposes.
- [x] 1.6 Freeze the project-configuration field name, its accepted values, and the exact unavailable-algorithm and multiple-active-member diagnostics.
- [x] 1.7 Inventory every committed artifact that carries a digest value. Record which generator owns each one, and flag any digest-bearing file that no generator reproduces. Record the expected regeneration scope as a reviewed count.

## 2. Freeze paired positive and negative fixtures before implementation

Every requirement below needs both polarities present and failing for the right reason before any production code lands.

- [x] 2.1 DIG-001 positive: a registry and schema inventory that agree exactly on name, tag, length, and assurance class in both directions. Negative: a row that reassigns obsolete tag `0` or tag `1`, a registry member missing from the inventory, and an inventory variant missing from the registry.
- [x] 2.2 DIG-002 positive: published known-answer vectors for every selected member from each algorithm's official test suite, with source and revision recorded, plus canonical text and binary round trips at each exact declared length. Negative: obsolete tags, unknown tags, unknown names, uppercase prefixes, nonhexadecimal digits, truncated payloads, and a payload length that contradicts its declared tag.
- [x] 2.3 DIG-003 positive: one declared, registered, available active member that produces byte-identical output across two runs and two absolute roots. Negative: zero declared members, two declared members, an unregistered member, and a member whose build feature is absent.
- [x] 2.4 DIG-004 positive: a repository where every digest path reaches the registry and every hashing implementation sits behind its own allowlisted adapter module. Negative: a hand-written hash outside an adapter module, a direct hashing-crate call outside an adapter module, an adapter module computing a member other than its own, and a compute request under an inactive member. Include the retired `tools/content_digest.rs` implementation as a permanent negative fixture.
- [x] 2.5 DIG-005 positive: an active-member change that regenerates every object from authoritative source bytes and produces byte-identical output across two runs. Negative: comparing, ordering, relabeling, or substituting digests across two members, and a right-length payload carrying the wrong tag.
- [x] 2.6 DIG-006 positive: a report citing exactly the active member's declared assurance class. Negative: a collision-resistance claim on a `change-detection` member, and an authenticity, tamper-evidence, or proof claim under any member including a `collision-resistant` one.
- [x] 2.7 DIG-007 positive: an index generated twice that is byte-identical and in the same total order, and whose every row survives recomputation from authoritative source bytes. Negative: a row whose recorded digest contradicts recomputation, a row keyed by a digest value rather than a stable object ID, a row for an object absent from the graph, and an index stored in a nonreproducible binary container.
- [x] 2.8 DIG-008 positive: deleting the index leaves canonical graph and report bytes unchanged. Negative: an object with unavailable authoritative source bytes paired with an index row that records a digest for it, and a freshness check that reads the index.
- [x] 2.9 XXH-005 positive: an ecosystem integrity value verified outside the core and classified as opaque. Negative: a Cargo, Nix, and Git value promoted to a repository-owned digest field, including one whose prefix matches a registered member name.
- [x] 2.10 Add deterministic Hegel-rs properties for per-member round trips, cross-root and permutation independence, preimage sensitivity to domain, field, and order changes, parser rejection of noncanonical forms, and regeneration idempotence under committed seeds. Keep these on the unbounded surfaces that enumeration cannot cover.
- [x] 2.11 Build a fail-closed fixture manifest that maps every DIG and revised XXH scenario to its polarity, its stable diagnostic, and a focused command. Confirm that the manifest lists both polarities for every requirement and that no scenario is unmapped.
- [x] 2.12 Confirm each negative fixture fails for its own stable diagnostic rather than an incidental error, by asserting the exact diagnostic code and not merely a nonzero exit.

## 3. Build and prove the finite model

- [x] 3.1 Declare the model identity per EVD-007: model ID, revision digest, state space, event alphabet, transition relation, declared bounds, and checked invariants.
- [x] 3.2 Implement the digest-acceptance sub-space at 64 states, over tag class, payload length, text case, hex validity, and operation.
- [x] 3.3 Implement the active-member-resolution sub-space at 12 states, over declared count, registration, and feature availability.
- [x] 3.4 Implement the transition and index lifecycle sub-space at 4096 states, over the twelve declared bits including the four correspondence-index bits.
- [x] 3.5 Write the independent `model_*` transition relation separately from the `implementation_*` path, and compare both outcomes at every enumerated state.
- [x] 3.6 Check the regeneration-soundness invariant: a transition succeeds only when authoritative source bytes and every required approval are present.
- [x] 3.7 Check the index non-authority invariant for both the independent model relation and the separate implementation relation. For every state, each identity outcome must equal its outcome with all index bits cleared. Limit the exhaustive claim to the declared model and bounds.
- [x] 3.8 Check the index-verification invariant as a separate projection, so a contradicted or digest-keyed row rejects at the index gate without altering the identity outcome.
- [x] 3.9 Check the preservation and idempotence invariants on every accepted transition.
- [x] 3.10 Add one injected negative control per invariant: index input reaching each identity relation, a contradicted index row accepted, a transition accepted with missing bytes or missing approval, and a mutated stable ID in an accepted target. Assert that every control makes the runner fail with a diagnostic naming the exact state.
- [x] 3.11 Add a `--self-test` mode that runs every injected control and fails if any control passes.
- [x] 3.12 Report exact enumerated state counts, accepted and rejected counts, and the fixed-point result. Reject an `exhaustively-checked` claim when exploration stops at an undeclared cutoff.
- [x] 3.13 Replace `tools/check_xxh3_migration_model.rs` with this model, and update its `check_exact_coverage.rs` and hardening-manifest registrations.

## 4. Implement the registry and the widened digest in the core

- [x] 4.1 Add the closed `DigestAlgorithm` registry with `const` name, tag, length, and assurance-class accessors, and with tags `0` and `1` rejected.
- [x] 4.2 Widen `SemanticDigest` to a 32-byte buffer with an algorithm-derived valid length. Keep `Copy`, `Ord`, `const` construction, and `no_std`. Make an invalid length unrepresentable.
- [x] 4.3 Replace `parse_canonical` and `canonical_text` with registry-driven forms that accept every registered member.
- [x] 4.4 Replace the fixed 29-byte `canonical_binary` and `parse_canonical_binary` with an algorithm-derived envelope length.
- [x] 4.5 Add one allowlisted adapter module per selected member, each exposing only a preimage-to-output-bytes operation, behind its own Cargo feature. Default the feature set to `xxh3-128` alone.
- [x] 4.6 Replace `hash_canonical_preimage` with an active-member dispatch that fails closed on an inactive or unavailable member.
- [x] 4.7 Bind the active member's tag into `semantic_preimage_from_encoded_record`, and move the encoding version to version 3 from the design's schema-version decision.
- [x] 4.8 Update `raw_artifact_digest`, `observe_canonical_preimages`, and the digest-observation collision path for the widened representation.
- [x] 4.9 Add an `ActiveAlgorithm` value that the shell constructs and passes inward. Add no global, static, or environment read to the core.
- [x] 4.10 Regenerate `UNICODE_CASE_FOLD_SOURCE_DIGEST` in `crates/buoy-core/src/casefold_table.rs` through the registry.

## 5. Wire the shell, invert the policy gate, and retire the in-tree hash

- [x] 5.1 Add the project-configuration field for the active member, with fail-closed loading for the task 1.6 diagnostics.
- [x] 5.2 Update `src/presentation.rs` to render the algorithm from the registry rather than from a match on one variant.
- [x] 5.3 Update `src/migration.rs` and `src/adapters/schema.rs` for the widened digest and the new schema version.
- [x] 5.4 Rewrite `src/hashing_policy.rs` from identifier bans to the structural boundary rule: reject every hashing implementation and direct hashing-crate call outside an allowlisted adapter module, under any spelling.
- [x] 5.5 Keep the `DefaultHasher` and `RandomState` nondeterministic-hasher findings unchanged, and keep the external-integrity syntax classification field-specific.
- [x] 5.6 Replace the documentation-claim check: reject a claim that names any member other than the declared active member, and reject an assurance claim that exceeds the active member's declared class.
- [x] 5.7 Route every `tools/content_digest.rs` caller through the registry, then delete the hand-written SHA-256 implementation.
- [x] 5.8 Confirm that the rewritten gate emits a finding for the deleted implementation when it is restored as the task 2.4 fixture.

## 6. Migrate schema, artifacts, and assurance evidence

- [x] 6.1 Add `schema/v3/inventory.tsv` with one row per registered member, and update the single-variant assertion in `tools/check_schema_inventory.rs` to a full registry comparison in both directions.
- [x] 6.2 Generalize `tools/check_xxh3_vectors.rs` across members, add the per-member `fixtures/digest/v3/` layout, and retain `fixtures/xxh3/v2/` only for frozen v2 compatibility controls.
- [x] 6.3 Regenerate every committed digest-bound artifact as one closed change set. Reconcile the diff against the task 1.7 inventory, and confirm that no unowned digest-bearing file was left behind. Verify byte-identical output across two runs and across two absolute roots.
- [x] 6.4 Regenerate `assurance/v1/obligation-ledger.tsv` and `assurance/v1/scenario-requirement-executions.tsv`. Confirm that every `XXH-001` through `XXH-006` requirement key and scenario slug is unchanged and that only digest bindings differ.
- [x] 6.5 Register the new `DIG-001` through `DIG-009` requirements and every scenario in the obligation ledger with exact evidence class and scope. Confirm that both polarities appear for every requirement.
- [x] 6.6 Update `tools/check_exact_coverage.rs`, `tools/check_hardening_manifest.rs`, `tools/generate_property_evidence.rs`, and `tools/generate_v2_fixture_suite.rs` for the renamed authorities and per-member fixtures.
- [x] 6.7 Add `deny.toml` license rows, `assurance/supply-chain/v1/closure-inventory.tsv` rows, and update `tools/check_supply_chain.rs` for each optional hashing dependency. Verify that the default closure is unchanged.

## 7. Enable the non-default members and build the correspondence index

- [x] 7.1 Add the `blake3-256` feature, its adapter module, its official vectors, and its fixture set.
- [x] 7.2 Add the `sha-256` feature, its adapter module, its official vectors, and its fixture set, using the task 1.3 decision.
- [x] 7.3 Run the full deterministic static check under each non-default member. Confirm that canonical output differs from the default run in digest values only, and not in record structure, ordering, or diagnostics.
- [x] 7.4 Run an active-member switch end to end, and confirm that it regenerates from source bytes and never relabels payload bytes.
- [x] 7.5 Generate the cross-algorithm correspondence index for that switch as a committed totally ordered TSV, keyed by stable object ID. Verify byte-identical output across two runs and two absolute roots.
- [x] 7.6 Implement the index verification gate that recomputes every row under both members from authoritative source bytes, and fails closed on any contradiction or on a digest-keyed row.
- [x] 7.7 Register the index as an SPG-010 `Derivation` with source bindings, target bindings, generator identity, configuration, and its consistency oracle.
- [x] 7.8 Prove non-authority end to end: confirm that deleting the index leaves canonical graph and report bytes unchanged, and that an object with unavailable source bytes still fails validation when the index records a digest for it.

## 8. Re-establish proofs, documentation, and the final oracle

- [x] 8.1 Treat the kernel change as proof-invalidating. Re-run the selected Verus or AeneasVerif lane against the new source digests, and do not carry forward a stale proof record.
- [x] 8.2 Update `docs/assurance/trust-boundaries.md` and the production-readiness and quality-assurance status documents for the per-member assurance class, the registry boundary, and the index non-authority boundary.
- [x] 8.3 Add deliberate faulty-implementation mutations for tag confusion, length confusion, inactive-member computation, adapter-module escape, and index-influences-identity. Confirm that each mutation fails a named gate.
- [x] 8.4 Run an adversarial review against every DIG and revised XXH requirement, fixture, binding, model invariant, and gate. Resolve every assurance-inflation finding, with attention to any claim that a cryptographic member implies authenticity and any claim that the finite model proves more than its declared bounds.
- [x] 8.5 Run `devenv test` three times as the exact final oracle, once per member feature set with its matching configuration. Require every run to be green and require the default run to remain the committed configuration.
