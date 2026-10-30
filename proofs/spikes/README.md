# Nonproduction formal-lane spikes

These nonproduction proofs compare Verus and AeneasVerif/Lean on equivalent small integrity obligations. They are architecture and translation-sentinel evidence only; production correspondence requires separate exact bindings and review.

## Obligation matrix

| Obligation | Verus source | Aeneas Rust source | Lean proof |
|---|---|---|---|
| terminal lifecycle cannot return live | `lifecycle_terminality` | `transition` | `lifecycle_terminality` |
| decision-surface missing member is incomplete | `surface_missing_member_is_incomplete` | `surface_complete` | `surface_missing_member_is_incomplete` |
| stale carrier/derivation bindings prevent freshness | `stale_bindings_prevent_freshness` | `carrier_fresh`, `derivation_fresh` | `stale_bindings_prevent_freshness` |
| policy scope/role growth is monotonic | `policy_growth_is_monotonic` | `policy_monotonic` | `policy_growth_is_monotonic` |
| missing impact disposition prevents closure | `missing_impact_prevents_closure` | `impact_closed` | `missing_impact_prevents_closure` |
| debt/waiver/reaffirmation cannot alter tested assurance | `process_records_do_not_launder` | `aggregate_tested` | `process_records_do_not_launder` |
| field tag/length encoding remains distinguishable | `field_tag_is_unambiguous`, `field_length_is_unambiguous` | `field_header` | `field_header_binds_tag_and_length` |
| the v3 envelope binds schema, encoding, and registry tags | `versioned_envelope_binds_v3_registry` | `versioned_envelope_valid` | `versioned_envelope_binds_v3_registry` |
| XXH128 canonical output preserves high-half then low-half big-endian byte order | `xxh128_half_and_byte_order` | `canonical_xxh128_bytes` | `xxh128_half_and_byte_order` |
| only the frozen digest type tag, algorithm tag, and length are accepted | `parser_length_and_tag_invariants` | `digest_shape_valid` | `parser_length_and_tag_invariants` |
| migration cannot alter lifecycle state | `migration_lifecycle_wrapper_preserves_state` | `migrated_lifecycle` | `migration_lifecycle_wrapper_preserves_state` |
| a source-item version must be positive | `version_zero_is_not_a_source_item_kind` | `source_item_kind_valid` | comparison spike only |
| parsed-item fields bind the source-item kind | `parsed_item_identity_binds_source_item_kind` | `parsed_item_fields` | comparison spike only |
| reachability depends on the edge set, not its order | `step_depends_only_on_edge_membership` | `reachable_step` | comparison spike only |
| a reachability step is extensive and monotone | `step_is_extensive`, `step_is_monotone` | `reachable_step` | comparison spike only |
| a fixed point is closed under every outgoing edge | `fixed_point_is_closed_under_edges` | `is_fixed_point`, `reachable_closure` | comparison spike only |
| a self loop or ancestor back edge is rejected | `a_self_loop_is_rejected`, `a_back_edge_to_an_ancestor_is_rejected` | `has_self_loop`, `reachable_cycle` | comparison spike only |
| a traversal visits no more nodes than the graph holds | `visited_is_bounded_by_the_node_set` | `visited_count` | comparison spike only |
| every budget is required and exceeding one rejects | `an_admitted_traversal_respects_every_budget`, `exceeding_any_budget_is_rejected` | `within_budgets` | comparison spike only |
| disjoint prefixes make logical projection injective | `disjoint_prefixes_make_projection_injective` | `logical_projection` | `logical_projection_binds_prefix_and_relative` |
| one prefix preserves relative-path identity | `one_prefix_preserves_relative_identity` | `logical_projection` | `logical_projection_binds_prefix_and_relative` |
| the attachment-key preimage binds the relation | `attachment_key_preimage_binds_the_relation` | `attachment_key_input` | `attachment_key_input_binds_relation` |

“Equivalent” means both lanes prove the same finite logical claim, not that their encodings, trusted implementations, or downstream guarantees are equivalent.

## Files

- `verus/integrity.rs`: positive Verus obligations.
- `verus/source_item_kind.rs`: source-item structure spike.
- `verus/bounded_traversal.rs`: bounded source-traversal feasibility spike.
- `verus/logical_mounts.rs`: logical projection and attachment-key preimage obligations.
- `verus/logical_mounts_negative.rs`: deliberately false equal-input injection control.
- `aeneas/rust/bounded_traversal.rs`: bounded source-traversal comparison subset. It is outside the extracted `src/` crate, so it does not change the committed generated Lean. `tests/bounded_source_traversal_model.rs` includes it and compares it with the exhaustive four-node model.
- `verus/integrity_negative.rs`: deliberately false waiver-laundering control; Verus must reject it for the expected postcondition reason.
- `aeneas/rust/`: safe sequential Rust translation targets.
- `aeneas/rust/source_item_kind.rs`: source-item comparison subset.
- `aeneas/rust/logical_mounts.rs`: logical projection and attachment-key translation source.
- `aeneas/integrity.llbc`: reviewed Charon extraction; its self-recorded destination path is noncanonical, so the gate regenerates LLBC and compares the derived Lean instead of comparing LLBC bytes directly.
- `aeneas/generated/`: committed LLBC-derived Lean output; regeneration must be byte-identical after removing only the two unavailable nonsemantic linter options and enum `discriminant` attributes.
- `aeneas/lean/Aeneas.lean`: bound minimal external model for the generated `Result`, `U8`, and `U64` surface; it is not the full backend library.
- `aeneas/lean/IntegrityProofs.lean`: positive Lean obligations over generated Aeneas functions.
- `aeneas/lean/IntegrityNegative.lean`: deliberately false waiver-laundering control; Lean must reject it without `sorry`.

## Trust and exclusions

Verus results trust the pinned Verus distribution, its encoding, and bundled Z3. Aeneas results trust Charon extraction, LLBC, Aeneas translation, the committed minimal external model, and Lean's kernel. Neither lane proves prose correspondence, adapters, rustc/LLVM, native code, or the other lane.

A spike must not contain `sorry`, `admit`, an unbound external model, unsafe Rust, concurrency, filesystem access, process access, or live dependencies. Generated Aeneas output is reviewed as a derivation rather than independent evidence.

The v3 spikes prove only envelope discrimination, registry length rules, structural half and byte order, and lifecycle wrapper preservation. They exclude hash internals, arithmetic correctness, collision properties, authenticity, and any claim that equal digest outputs imply equal preimages.

The bounded-traversal spikes prove only the named set-theoretic and arithmetic claims over an abstract node and edge relation. Their exclusions are recorded in `openspec/changes/add-content-addressed-openspec-bindings/evidence/bounded-traversal-formal-spikes.md`.

The logical mount spikes prove segment-sequence projection and relation-preimage construction only. They exclude path parsing, filesystems, hash internals, and collision resistance.

The exact command is `buoy-formal-spikes`, invoked by `devenv tasks run buoy:properties`. The primary-lane review is recorded in `docs/architecture/adr-0001-primary-formal-lane.md`.
