# Validated Version 1 Migration Baseline

Status: authoritative rollback record for task 1.3.

## Immutable repository boundary

| Property | Value |
|---|---|
| Baseline commit | `9b1b7740c390aeaa58b491b2cefc9ee8c694fce2` |
| Baseline commit subject | `chore: checkpoint validated v1 baseline` |
| Commit timestamp | `2026-07-17T20:14:41-04:00` |
| Git tree object | `763b136634efd0ec819fdaf63b1426f5c930691a` |
| Parent commit | `026630c332cd8f6cfbd04f763d128969b4c38046` |
| Active semantic schema | `spec-provenance/v1` |
| Git status after checkpoint | clean (`## main`) |

The Git commit and tree IDs are opaque Git object metadata, not `SemanticDigest` values. The tree is the authoritative byte boundary: every tracked path and exact byte sequence needed to build, test, inspect, or restore this baseline is selected by that tree. No filesystem mtime, host path, untracked file, cache, process output, environment value, clock, or network response is authoritative.

## Validation record

The complete repository gate was run against the exact bytes subsequently committed without intervening semantic changes:

```text
devenv shell -- spec-provenance-gate
```

Result: exit `0` on 2026-07-17. The final OpenSpec phase reported 10 passed and 0 failed items, including this change. Before the successful run, `tools/check_schema_inventory.rs` was formatted without semantic changes; the successful gate then covered formatting, Clippy, workspace tests, schema/reference/Unicode/fixture/property/core/adapter/integration/FCIS checks, and strict OpenSpec validation as defined by the baseline's `devenv.nix`.

The planned v2 acceptance spelling `devenv tasks run spec-provenance:gate` is not present in this v1 baseline. That command gap is not hidden: task 4.7 must register the exact task before v2 acceptance. The v1 baseline command above is the executable gate that existed and passed at the rollback boundary.

## Authoritative configuration and schema inputs

The baseline binds configuration by path and bytes within the tree, including:

- workspace/dependency closure: `Cargo.toml`, `Cargo.lock`, and every workspace/member `Cargo.toml`;
- Nix/devenv closure and shell tasks: `devenv.nix`, `devenv.yaml`, `devenv.lock`, `nix/dylint-tools.nix`, `nix/formal-tools.nix`;
- Rust compiler/toolchain: `rust-toolchain.toml` and the generated Dylint toolchain configuration selected by `devenv.nix`;
- active schema inventory: `schema/v1/inventory.tsv`;
- resource limits: `schema/v1/resource-budgets.toml`;
- public/report migration policy: `schema/v1/migration-policy.toml`;
- schema documentation: `docs/schema/v1/`;
- repository selectors, parser identities, oracle commands, budgets, and concurrency values represented by the committed source/configuration and canonical `repository-execution-config` domain;
- committed Unicode source bytes: `unicode/16.0.0/CaseFolding.txt` and generated `crates/spec-provenance-core/src/casefold_table.rs`.

The v1 schema identity is frozen as encoding version `1`, semantic schema `spec-provenance/v1`, digest algorithm tag `1`, and full SHA-256 values. This is archival baseline meaning only; v2 must not reinterpret those bytes.

## Tool and assurance identities

`assurance/v1/toolchain.toml` is the baseline tool identity inventory. It binds the selected Rust, Unicode, parser/glob, platform API, coverage, fuzz, mutation, dependency-policy, Hegel, Dylint, Verus, Aeneas, Charon, Lean, and Z3 identities and exact revisions where available.

Related authoritative records are:

- `assurance/v1/source-inventory.tsv` — production/generated/standalone source scope;
- `assurance/v1/property-inventory.tsv` and `property-runs.tsv` — fixed property authorities, seeds, bounds, and results;
- `assurance/v1/ledger-schema.toml` — required source/tool/configuration/evidence binding shapes;
- `assurance/v1/phase-handoffs.toml` — P0–P3 and final command contracts;
- `assurance/v1/platform-blockers.toml` — explicit unsupported/blocked platform scope;
- `assurance/v1/red-baseline.toml` — historical assurance baseline and exclusions;
- `proofs/core/proof-record.toml` and `proofs/core/derivation-record.toml` — exact proof/derivation bindings and stated exclusions.

Tool/fetch checksums that are prescribed by Cargo, Nix, or Git remain opaque external-integrity metadata as classified in `evidence/hash-inventory.md`. The project-owned Unicode SHA support identity is part of the v1 archive and must be regenerated under v2 rather than promoted as an exception.

## Approvals and lifecycle state

Required approvals and lifecycle states are not reconstructed from this summary. Their authoritative bytes are the committed schema, models, proof records, and fixture baselines, notably:

- approval shape and digest binding: `schema/v1/inventory.tsv` rows for `approval-binding`, approval authorities, active/terminal approvals, and correspondence approvals;
- live/terminal atom rules: `fixtures/v1/baselines/lifecycle-graph.toml` and `fixtures/v1/models/atom-lifecycle.toml`;
- policy/debt/waiver lifecycle and approvals: `fixtures/v1/baselines/policy-adoption.toml`, `debt-waiver.toml`, `evidence-assurance.toml`, and `surface-graph.toml`;
- formal correspondence and proof approvals: `fixtures/v1/baselines/proof-record.toml`, `formal-lane-selection.toml`, and committed proof records;
- reaffirmation approval: `fixtures/v1/baselines/reaffirmation.toml`;
- canonical positive/negative lifecycle expectations: `fixtures/v1/spg/`, `fixtures/v1/evd/`, `fixtures/v1/fpr/`, and `fixtures/v1/models/`.

The migration must preserve every stable ID, ratified/proposed state, terminal superseded/withdrawn reservation, active/terminal policy/debt/waiver state, tombstone, replacement, and required approval represented by those bytes. A missing approval or authoritative source remains a migration failure.

## Rollback procedure and prohibition

Rollback means restoring the complete baseline tree, not converting individual values:

```text
git switch --detach 9b1b7740c390aeaa58b491b2cefc9ee8c694fce2
devenv shell -- spec-provenance-gate
```

A branch restoration may instead reset a dedicated rollback branch to the commit after preserving any later work. Destructive reset of the active development branch is not implied by this record.

Rollback must restore schema, code, manifests, lockfiles, records, fixtures, tools, proofs, and documentation together. It must not retain v2 active records, relabel XXH3 bytes as v1, relabel SHA bytes as v2, mix algorithm tags or schema versions, or derive either version from obsolete digest bytes alone.
