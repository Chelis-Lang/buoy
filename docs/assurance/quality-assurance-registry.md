# Quality-assurance registry maintenance

Status: maintainer workflow for `quality-assurance-controls/v1`.

The machine-readable authority is [`../../assurance/v1/quality-assurance-controls.toml`](../../assurance/v1/quality-assurance-controls.toml). The generated status is [`quality-assurance-status.md`](quality-assurance-status.md). Neither registry membership nor a green registry check establishes that an inventoried child lane ran or passed.

## Stable commands

```text
devenv tasks run buoy:quality
devenv test
```

`buoy:quality` runs the complete positive/negative fixture self-test, validates live repository discovery and claims, checks content bindings, checks report bytes, and runs focused Rust contracts offline. It does not execute Hegel, LLVM coverage, Dylint, formal tools, fuzzers, mutants, sanitizers, dependency policy, or other child lanes. `devenv test` remains the only final implementation acceptance oracle.

## Change one control

1. Allocate a never-used `QA-NNNN` identity. Never reuse an identity recorded in `retired_ids`.
2. Select exactly one implementation owner: one active OpenSpec change or one completed baseline authority. Put broader changes in `related_changes`; they do not share ownership or evidence credit.
3. Classify implementation, enforcement, evidence freshness, execution verdict, and assurance independently. A pin supports `available`, not `implemented`; a focused command does not imply `blocking`; registration or a historical result does not imply `pass`.
4. Bind normalized repository-relative implementation, configuration, evidence, positive-control, negative-control, documentation, owner, and result paths. Do not use absolute paths, traversal, raw logs, task checkboxes, timestamps, durations, host fields, provider fields, or ambient process/network state.
5. Run the focused child control when applicable, then run `devenv tasks run buoy:quality`. Regenerate with `cargo run --locked --offline --bin buoy-quality -- --generate` before review.

Review the registry, generated bindings, and generated report in the same diff. Hand edits to generated status cells are rejected.

## State transitions

### Add or strengthen

An `implemented` control requires executable project behavior plus applicable positive and negative controls. `focused` additionally requires an exact stable `devenv tasks run buoy:*` command. `blocking` additionally requires direct structural inclusion in the `devenv test` chain and a gate-omission negative control. Strengthening one axis never silently strengthens freshness, verdict, or assurance class.

### Defer or block

Use `planned` when no exact implementation/tool claim is ready. Use `available` only when an exact pinned tool or configuration is present. Use `blocked` with a repository-owned blocker and explicit discharge condition. Keep the execution verdict `not-run` unless a current normalized result binds exact source, tool, configuration, scope, and result bytes.

### Replace or retire

A retired row keeps its identity, terminal rationale, and replacement or explicit terminal disposition. A replacement row lists the retired identity in `replaces`. Update owner mapping, evidence credit, bindings, report, and applicable gate references together. Retired IDs are never reassigned.

## Freshness and regeneration

Bindings use the repository's active non-cryptographic XXH3-128 identity policy over explicit repository-owned file bytes. Cargo checksums, Nix hashes, Git commits, package hashes, and upstream release hashes remain opaque ecosystem identities; the registry does not reinterpret them as Buoy semantic identities.

A referenced file change makes the committed binding/report stale. Regenerate, inspect the exact diff, and rerun:

```text
cargo run --locked --offline --bin buoy-quality -- --self-test
cargo run --locked --offline --bin buoy-quality -- --check
devenv tasks run buoy:quality
```

A stale execution authority loses its current verdict. It does not fall back to a historical pass.

## Assurance boundaries

- Coverage remains structural source-execution evidence, not behavioral completeness, exhaustiveness, proof, compiler correctness, or defect absence.
- Hegel remains sampled evidence over committed cases/settings/domains, not exhaustive or formal evidence.
- Finite-model claims name exact models, bounds, transitions, and invariants; nothing outside those bounds is covered.
- Verus claims bind exact representation, proof, verifier, bundled solver, configuration, assumptions, correspondence state, negative control, scope, and exclusions.
- Aeneas/Charon/Lean remains a narrow translation sentinel unless separately bound proof evidence establishes more.
- Dylint and bare-metal builds remain structural detector/build evidence within their exact scope.
- OpenSpec remains planning/reference-only and task checkboxes are never technical evidence.

No evidence class promotes another. Functional-core constraints remain one-way: the root shell may read registry inputs and generate status, while `buoy-core` remains `#![no_std]` and contains no registry, TOML, filesystem, process, Hegel, Dylint, coverage, or report-generation dependency.
