# Bounded Traversal Formal Spikes

These are feasibility spikes for task 2.8. They report what each formal lane can
express for the pure bounded source-traversal kernel before any lane selection.
They do not create a `formally-proved` assurance fact for production code.

## Scope

The kernel under study is the pure part of decision 3 in `design.md`: source-root
closure, acyclicity, current reachability, and bounded traversal. It excludes
digests, parsers, the object store, the filesystem, and the CLI.

The executable model, its seeded properties, and its exhaustive bounded model are
in `tests/bounded_source_traversal_model.rs`.

## Files

- `proofs/spikes/verus/bounded_traversal.rs`: nine Verus obligations.
- `proofs/spikes/aeneas/rust/bounded_traversal.rs`: the safe sequential Rust
  comparison subset.

## Obligations

| Claim | Verus | Aeneas subset |
|---|---|---|
| edge-order independence | `step_depends_only_on_edge_membership` | `reachable_step` |
| step is extensive | `step_is_extensive` | `reachable_step` |
| step is monotone | `step_is_monotone` | `reachable_step` |
| a fixed point is edge-closed | `fixed_point_is_closed_under_edges` | `is_fixed_point` |
| a self loop is rejected | `a_self_loop_is_rejected` | `has_self_loop` |
| an ancestor back edge is rejected | `a_back_edge_to_an_ancestor_is_rejected` | `reachable_cycle` |
| visited nodes are bounded by the node set | `visited_is_bounded_by_the_node_set` | `visited_count` |
| an admitted traversal respects every budget | `an_admitted_traversal_respects_every_budget` | `within_budgets` |
| exceeding any budget rejects | `exceeding_any_budget_is_rejected` | `within_budgets` |

## Feasibility result

Verus expresses the closure and budget claims directly over `ISet` and `nat`,
with no loop invariant and no termination measure. The pinned `vstd` reserves
`Set` for finite sets, so the closure lemmas use `ISet` and only the cardinality
bound uses `Set` with `lemma_len_subset`. Replay is `verus
proofs/spikes/verus/bounded_traversal.rs`, reporting `9 verified, 0 errors`.

The Aeneas lane cannot take the same shape. The comparison subset therefore
represents a four-node graph as one bitmask per node and unrolls the
reachability fixed point to the node bound. That removes every loop, allocation,
iterator, and collection. The unrolling is checked against the exhaustive model
by `aeneas_comparison_subset_agrees_with_the_exhaustive_four_node_model`, which
compares closure, fixed point, visited count, cycle verdict, self-loop verdict,
and budget admission over all 1,048,576 four-node graph and root-set pairs.

## Assumptions

The Verus obligations model a source graph as an abstract node set and edge
relation over `nat`. They assume that production node identity is injective, so
that distinct source nodes have distinct indices.

The Aeneas subset assumes exactly four nodes. Four unrolled steps reach the fixed
point at that bound; the unrolling is not valid for a wider graph.

## Exclusions

Neither lane proves any of the following:

- the executable Rust kernel in `tests/bounded_source_traversal_model.rs`; no
  symbol-level correspondence record binds the spike to that code
- the worklist, Kahn, and depth-relaxation algorithms actually used by the model,
  including their termination on cyclic input
- canonical child ordering by `(relation, position, target digest)`; the spikes
  order by node index only
- digest computation, collision resistance, or that equal digests imply equal
  source nodes
- CommonMark, Rust, and TOML parser correctness
- object-store path parsing, filesystem behavior, or absolute-root independence
- freshness, authority correspondence, or any stale-binding diagnostic
- allocation behavior, arithmetic overflow in the production types, rustc, or
  LLVM

The Aeneas lane additionally excludes, for this kernel: `Vec`, `BTreeSet`,
iterators, closures, generics, trait objects, and any unbounded loop. No Charon
extraction, no LLBC, no generated Lean, and no Lean proof exists for this
subset; it is Rust-level comparison evidence only. The committed
`proofs/spikes/aeneas/generated/Integrity.lean` is unchanged because the subset
lives outside the extracted `src/` crate.

## Lane selection

This spike records feasibility only. It does not select a lane and does not
change ADR-0001. A production claim requires a proof record that binds exact
source, property, assumptions, toolchain, and proof output, per the trust and
proof boundaries in `design.md`.
