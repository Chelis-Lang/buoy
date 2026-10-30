# Exhaustive finite reference-model runs

Status: active v2 model evidence.

## Independence and scope

`tools/check_fixture_manifest.rs` implements only the generic DSL in `fixtures/v2/models/README.md`: finite Cartesian enumeration, condition/predicate evaluation, invariant checking, and explicit transition fixed-point traversal. It imports no production provenance crate and calls no production lifecycle, surface, freshness, policy, impact, aggregation, proof, encoding, or hashing function.

Truth-table models enumerate each closed Cartesian product. The lifecycle model follows every declared transition from its initial states until no new state is found. The checker rejects an unexpected model inventory or bound, more than 100,000 states per model, malformed/cyclic predicates, duplicates, unknown conditions, invariant failures, outgoing terminal transitions, and released terminal IDs.

| Active v2 model | Kind | States | Invariants |
|---|---|---:|---:|
| `model-atom-lifecycle-v2` | transition system | 4 | 4 |
| `model-carrier-derivation-freshness-v2` | truth table | 2,048 | 3 |
| `model-change-impact-closure-v2` | truth table | 256 | 3 |
| `model-coverage-policy-adoption-v2` | truth table | 108 | 2 |
| `model-debt-waiver-reaffirmation-no-laundering-v2` | truth table | 160 | 4 |
| `model-decision-surface-completeness-v2` | truth table | 128 | 2 |
| `model-proof-invalidation-v2` | truth table | 16,384 | 14 |
| `model-required-role-coverage-v2` | truth table | 225 | 3 |
| `model-verdict-aggregation-v2` | truth table | 400 | 4 |

Total: 19,713 finite states and 39 invariant schemas.

`tools/check_xxh3_migration_model.rs` contains a separate 4,172-state model for digest acceptance, active-member resolution, transitions, and index behavior.

The model compares an independent relation with a separate implementation relation inside the same tool. It reruns each accepted target to a fixed point.

The model does not call production migration or digest code. Focused production controls provide separate correspondence evidence.

A pass supplies model-internal exhaustive evidence only. It does not prove production correctness outside the declared states, events, bounds, and invariants.

## Exact oracle

```text
devenv shell -- buoy-fixtures
devenv shell -- buoy-hashing-policy
```

A pass establishes exhaustive evidence only for the exact declared finite state spaces, events, bounds, and invariants. It does not establish behavior outside those spaces, production/reference equivalence, XXH3 collision properties, natural-language correctness, parser correctness, or formal proof. Matching a production digest or output does not strengthen that claim.
