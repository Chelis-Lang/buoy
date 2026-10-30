# Hardening fixture contracts

This directory contains reviewed, deterministic contracts for the active frozen-v2 encoding of Buoy's frozen semantic model.

## Schema-construction matrix

`schema-construction-fixtures.tsv` is generated from `schema/v2/inventory.tsv` and freezes the pre-implementation construction cases required by hardening task 2.1. It covers:

- a valid-present and omitted-field case for every inventory field;
- explicit `none` and `some` cases for every option field, plus invalid state/variant combinations where applicable;
- explicit empty collection cases, including separate allowed-state and required-state cases;
- every closed enum variant plus one unknown-variant rejection per enum;
- positive and invalid cases for every reference kind;
- exact and mismatched metadata-kind cases for every object; and
- complete and incomplete records for every terminal lifecycle transition.

The expected `SCF-CONSTRUCT-*` codes are stable fixture contracts. Regenerate generated schema fixtures only after reviewing an inventory change:

```text
cargo run --bin check-schema-inventory -- --write-construction-fixtures
```

This command refreshes both the construction matrix and the impact-edge matrix described below. Normal `check-schema-inventory` and `--self-test` runs require byte-exact parity with the inventory.

## Base/head state matrix

`schema-state-fixtures.tsv` is independently authored rather than generated from the inventory. Its closed 39-case set covers positive and negative state comparisons for:

- append-only ID registries and namespace allocations;
- parsed-item identities and source maps;
- repository-owned approval bindings;
- retained terminal policy, debt, and waiver records; and
- explicit base/head artifact-index freshness.

The checker rejects unknown subjects, missing or extra reviewed case IDs, duplicate or unsorted IDs, empty fields, and disagreement between polarity, outcome, and stable diagnostics. Its self-test deliberately removes, duplicates, and corrupts state rows.

## Impact-edge graph pairs

`schema/v2/inventory.tsv` declares each impact edge as an `impact-edge` row naming an existing source field and target schema value. `impact-edge-fixtures.tsv` expands that reviewed authority into:

- one positive propagation graph pair for every declared edge;
- one negative omitted-edge graph pair for every declared edge; and
- positive fixed-point plus negative one-hop-only pairs for atom → formal property → proof, atom → carrier → debt, and assumption → formal property → proof chains.

The current matrix contains 114 declared edges, 234 graph-pair contracts, and three two-hop chains. Byte-exact checking prevents a newly declared edge from silently lacking positive or negative fixture coverage.

## Policy materialization and ratchets

`policy-ratchet-fixtures.tsv` independently freezes 20 base/head cases across scope, evidence role, enforcement mode, authority expectations, derivation requirements, surface evolution, waiver classes, ratchet lineage, supersession, and retirement. Every dimension has one accepted strengthening or reviewed-equivalent case and one rejected hidden-weakening case. The checker requires exact IDs, deterministic order, visible weakening facts, and `CHG-RATCHET-*` diagnostics.

## Exact disposition protocols

`disposition-protocol-fixtures.tsv` freezes one complete accepted case for each of `reaffirmed`, `revised`, `retired`, and `waived`, plus independent rejection cases mutating applicability, approval, base digest, debt, head digest, record, replacement, and waiver state. The resulting 36-case matrix prevents one malformed protocol field from being hidden by otherwise valid disposition data.

## Hardening finite models

`models/` extends the dependency-independent model portfolio with bounded exhaustive tables for artifact-index freshness, complete two-hop impact closure, all ten policy weakening dimensions, governance no-laundering across live and terminal records, exact proof invalidation including correspondence and negative controls, and policy/debt/waiver terminal history. Together with the baseline models, the checker now evaluates 15 models and 88,827 finite states using only the generic Cartesian and transition fixed-point DSL.

## Assurance boundary

These rows freeze inputs and expected outcomes; they do **not** claim that production constructors execute them yet. Behavioral dispatch, exact mutation cardinality, and production/independent result comparison remain unsatisfied until hardening tasks 2.15 through 2.18 implement and negatively self-test the fixture runner.
