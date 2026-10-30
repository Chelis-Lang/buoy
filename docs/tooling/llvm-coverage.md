# Exact LLVM source-coverage lane

Status: active structural source-execution evidence.

## Authority and command

The only blocking coverage authority is `cargo-llvm-cov 0.8.5` from the immutable devenv closure, with the repository’s pinned Rust/LLVM toolchain, Cargo/Nix locks, and `assurance/v1/coverage-command.toml`.

Run the focused lane with:

```text
devenv tasks run buoy:coverage
```

The lane uses `--locked --offline` and checks the exact tool version. It executes positive and negative controls and instruments the complete local surface. It requires exact integer equality for lines, regions, functions, and branches.

Maintainers must select this task explicitly. `devenv test` and CI do not execute LLVM instrumentation.

## Reviewed source scope

`assurance/v1/source-inventory.tsv` is the machine-readable authority. Each row has a stable target ID, normalized repository-relative path, kind, supported-platform scope, instrumentation class, disposition, and rationale. Its in-scope executable classes include:

- workspace libraries, binaries, tests required as executable evidence, and platform backends;
- separately invoked examples and maintainer generators;
- Cargo-reachable standalone validators;
- shared executable tool modules;
- the separate Dylint manifest; and
- generated executable sources.

A `not-applicable` disposition is accepted only for machine-checked sources with no executable regions. Broad directory/module exclusions, missing executable sources, duplicate IDs/paths, and unknown classes fail closed.

The self-test retains the positive and negative export fixtures.
It does not use ownership probes that discard their results.
Its ownership controls check an owned row, a missing owner, and unreadable inputs.

## Artifact ownership and current-host scope

Workspace, per-example, focused-standalone, and Dylint reports have separate artifact ownership. This prevents unexecuted objects from inflating denominators. Every artifact binds the same pinned tool, command configuration, source inventory, execution verdict, and current-host scope.

Normalization accepts a covered inventory row only when a bound export artifact owns it.
After the lane writes every export binding, the instrumented checker runs `--check-ownership` over the authoritative inventory and all bindings.
This check fails with `LCOV-SOURCE-MISSING` for a covered row without an owner.
This run produces the checker artifact export, so the export measures the real ownership routes.

An isolated Chelis artifact uses all features and the collision-resistant binding suites with default features disabled.
It clears the compiler wrapper so Cargo executes and measures the native grammar build script.
The same artifact executes the real CLI for each feature image.
A no-feature control checks the missing-member error and unchanged source bytes.
CLI contract tests use separate fixture-execution sources.
The controls also check binding repair, conversion, collisions, read-only writes, configuration limits, cross-image references, and runner convergence.

Campaign controls use `BUOY_CAMPAIGN_AS_OF` or the current UTC date.
Controlled expiry tests use fixed input records.

Trace contracts run with both collision-resistant members.
The byte golden remains specific to the XXH3 build.

Local acceptance merges compatible artifacts from one current host. Noncurrent hosts receive explicit non-applicable ledger dispositions. Repository-relative paths and LLVM regions are normalized without rounding or double counting.

## Exact acceptance and controls

For every included file, each metric with a nonzero count must satisfy:

```text
covered == count
```

A file can have zero branches when its code has no branch.
Each aggregate metric must have a positive count and equal covered and total counts.

Displayed percentages are never authoritative. The focused fixture matrix independently rejects missing sources, invalid dispositions, uncovered line/region/function/branch counts, absent or zero denominators, duplicate regions, incompatible identities, missing platforms, malformed results, and stale bindings with sorted stable `LCOV-*` diagnostics and exact expected cardinalities.

## Deterministic evidence lifecycle

After every executable control and artifact succeeds, the lane emits the canonical normalized result to `assurance/v1/coverage-result.tsv`, validates it, and independently recomputes it with `--check-current`. Rows bind source, tool, configuration, inventory, and export digests plus exact integer counts, registration, freshness, execution, and assurance class.

The accepted red baseline remains immutable historical context in `assurance/v1/red-baseline.toml`; it is not relabeled as current. Raw LLVM JSON, profiles, temporary bindings, host paths, timestamps, durations, traversal order, locale text, and terminal formatting are noncanonical and are removed after normalization.

Any authoritative source, tool, configuration, inventory, or current-host change makes a prior result stale. Only a new complete local run can restore `current/pass`.

Maintainers select `buoy:coverage-evidence` explicitly.
This fast task validates the recorded result against current repository inputs.

The task fails on stale or inexact evidence without a new LLVM run.

## Assurance boundary

Exact four-metric closure means only that every declared executable source region was observed under the registered commands and artifact scope. It does **not** establish behavioral completeness, exhaustive input exploration, absence of defects, rustc/LLVM/OS correctness, cryptographic properties, Verus/AeneasVerif representation correspondence, or natural-language-to-code/property proof.
