# Independent finite reference models

These declarative models freeze small, auditable state spaces before production algorithms exist. Production core code must not be imported by their runner.

## Closed model schema

Every model has:

- `schema = "spec-provenance-reference-model/v2"`;
- globally unique `id`;
- `requirement_ids` covered by the model;
- `kind = "truth-table"` or `"transition-system"`; and
- one or more invariants.

Unknown fields, duplicate IDs, duplicate dimension values, unreachable named states, uncovered finite states, and unknown condition names are model errors.

## Truth-table models

A truth-table model declares `[[dimensions]]` with a name and finite string `values`. The runner enumerates the full Cartesian product in declaration order.

A `[[predicates]]` table has a unique name and one or more nested `[[predicates.clauses]]`. A clause is a conjunction of `all` conditions. The predicate is true if any clause is true and false otherwise. An empty clause is true. There is no arbitrary code, negation syntax, implicit coercion, or production-function call.

Conditions are exactly:

- `dimension=value`;
- `dimension!=value`;
- `predicate=true`; or
- `predicate=false`.

Predicate references must be acyclic and may refer only to earlier predicates.

An `[[invariants]]` table contains a stable ID, `if_all` conditions, and `then_all` conditions. For every enumerated state where all `if_all` conditions hold, all `then_all` conditions must hold. Empty `if_all` means every state.

The runner emits predicate values and invariant status per model state. The state identity is the canonical ordered dimension-value tuple, not a hash map order.

## Transition-system models

A transition model declares `[[states]]` with unique name, `terminal` Boolean, and `id_reserved` Boolean; `initial` state names; and explicit `[[transitions]]` with `from`, `event`, and `to`.

No transition is implied. A repeated `(from,event)` pair, unknown state, or outgoing transition from a terminal state is a model error. Fixed-point exploration starts with every initial state and repeatedly applies all explicit outgoing transitions in canonical `(from,event,to)` order until no new state appears.

Transition invariants use these closed relations:

- `all-states-reachable`;
- `terminal-has-no-outgoing`;
- `terminal-never-reaches-live`; and
- `id-remains-reserved`.

## Independence rule

The phase-4 runner may implement only this DSL and generic Cartesian/fixed-point traversal. It cannot call production canonicalization, lifecycle, policy, freshness, aggregation, or proof functions. Later tests compare production output with these model tables as separate oracles.

## Bounds

The declared products are deliberately small. A runner rejects a model whose Cartesian state count differs from `expected_state_count` or exceeds 100,000. Transition exploration rejects a model whose reachable count differs from `expected_reachable_count`.
