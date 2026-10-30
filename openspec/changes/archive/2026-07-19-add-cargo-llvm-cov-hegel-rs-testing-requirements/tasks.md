## 1. Test-First Contracts and Fixtures

- [x] 1.1 Add synthetic positive LLVM export fixtures and independent negative exports for a missing source, invalid `not-applicable` source, each uncovered metric, each zero/absent denominator, incompatible tool/configuration/platform identity, duplicate region, and stale binding; record the expected stable diagnostic and cardinality for each fixture.
- [x] 1.2 Add positive and negative source-inventory fixtures covering unique normalized rows, duplicate IDs/paths, unknown instrumentation classes, broad exclusions, standalone validators, separate-manifest Dylint sources, generated executable sources, and missing required-platform artifacts.
- [x] 1.3 Add normalized coverage-result positive goldens plus malformed/nondeterministic records containing absolute paths, host store paths, timestamps, wrong digests, unsorted rows, rounded percentages, and historical results presented as current.
- [x] 1.4 Add positive and negative property-inventory/settings fixtures for exact Hegel identity, unique IDs/seeds, complete generator domains, ambient defaults, helper-only production mappings, production-importing references, missing production APIs, duplicate mappings, and stale bindings.
- [x] 1.5 Add a focused compile/pass probe for the approved Hegel API and a committed expected-failure Hegel property whose explicit example or declared domain deterministically exposes the false assertion.
- [x] 1.6 Add normalized property-result and replay positive goldens plus negative records containing host-dependent fields, incomplete bindings, uncommitted shrink output, unexpected panic/abort, and stale source/settings/reference identities.

## 2. Static Inventories and Validators

- [x] 2.1 Complete `assurance/v1/source-inventory.tsv` with stable IDs, normalized paths, platform scopes, instrumentation classes, and exact dispositions for every production, generator, reference, validator, and Dylint executable source.
- [x] 2.2 Extend the source-inventory validator to reject duplicates, unknown classes, broad exclusions, executable `not-applicable` rows, missing platform scope, and absent exported sources with sorted `LCOV-*` diagnostics; make all task 1 coverage fixtures pass.
- [x] 2.3 Implement deterministic LLVM export normalization and supported-platform aggregation that validates compatible source/tool/configuration bindings, deduplicates regions, preserves integer counts, and rejects missing/zero/nonexact line, region, function, or branch metrics.
- [x] 2.4 Add a source/tool/configuration-bound coverage-result record and freshness validator that separates registration, freshness, and execution verdict and invalidates any historical pass after an authoritative input changes.
- [x] 2.5 Expand `assurance/v1/property-inventory.tsv` and `assurance/v1/property-runs.tsv` (or versioned replacements) with expected polarity, exact Hegel settings, generator domains/limits, explicit edges, production/reference or metamorphic authority, source/configuration digests, replay bindings, and sampled-evidence scope.
- [x] 2.6 Extend `tools/check_property_evidence.rs` to validate the approved package closure, exact deterministic settings, unique IDs/seeds, complete production API mapping, actual production invocation, dependency-independent references, deterministic ordering, replay completeness, and stale result rejection with sorted `HEGEL-*` diagnostics.
- [x] 2.7 Add self-tests proving every task 1 positive fixture passes and every negative coverage/property fixture fails with its exact expected diagnostic and cardinality.

## 3. Complete LLVM Coverage Execution

- [x] 3.1 Make directly compiled standalone validators Cargo-reachable instrumented binary or harness targets without changing their focused command behavior.
- [x] 3.2 Add a pinned `cargo-llvm-cov` lane for the separate Dylint manifest and execute its positive and expected-negative fixtures under the same Rust/LLVM closure.
- [x] 3.3 Extend `buoy-coverage` to instrument the complete workspace/all-target/all-feature surface, every registered example/generator and validator invocation, and every registered separate manifest while preserving profiles for final reporting.
- [x] 3.4 Add required supported-platform artifact ingestion and deterministic merge preflight; fail with `LCOV-PLATFORM-MISSING` when a declared platform-specific executable region lacks a compatible current artifact.
- [x] 3.5 Feed normalized exports into the exact coverage validator, require positive integer equality for all four metrics over every inventory row, and emit a current deterministic coverage-result record only after all negative controls pass.
- [x] 3.6 Add behavior-asserting tests until exact line, region, function, and branch closure is reached without broad exclusions, rounded thresholds, call-only assertions, or removal of reachable behavior solely to satisfy coverage.

## 4. Deterministic Hegel-rs Execution

- [x] 4.1 Centralize and validate the authoritative Hegel runner configuration: `Backend::Default`, 256 cases, property-specific committed seed prefix, derandomization, disabled database, and the exact ordered phase list.
- [x] 4.2 Complete each registered generator-domain declaration and explicit edge corpus, including applicable size/depth/rejection limits, before changing any mapped production algorithm.
- [x] 4.3 Complete Hegel production-versus-independent-reference or justified metamorphic properties for every inventoried pure public semantic API, keeping helper-only properties visibly `reference-only`.
- [x] 4.4 Strengthen the reference-independence check so a reference crate importing or re-exporting production semantic implementations fails with `HEGEL-REFERENCE-NOT-INDEPENDENT`.
- [x] 4.5 Add the expected-failure Hegel harness, verify the false property is executed and rejected exactly once, and fail on an unexpected pass, incomplete execution, panic outside the expected protocol, or missing replay identity.
- [x] 4.6 Add focused replay commands for reviewed digest-bound counterexample fixtures and keep raw shrink traces, timings, terminal formatting, local databases, and uncommitted minimized examples outside deterministic evidence.
- [x] 4.7 Emit current normalized property results only after all registered blocking properties and positive/negative controls execute under the exact offline Hegel closure.

## 5. Gate Wiring, Documentation, and Evidence Boundaries

- [x] 5.1 Make `devenv tasks run buoy:coverage` and `devenv tasks run buoy:properties` run their complete focused lanes offline and fail if static registration is mistaken for an execution verdict.
- [x] 5.2 Wire both focused effectful lanes into `devenv test` and add a gate-omission self-test that rejects an aggregate path which validates only manifests or historical passes.
- [x] 5.3 Update coverage and Hegel tooling documentation with exact commands, pins, settings, generator scopes, platform aggregation, freshness/replay lifecycle, stable diagnostics, and the distinction between canonical normalized records and noncanonical raw logs; document Buoy's dogfood role for `cargo-llvm-cov`, pinned local Dylint rules, and the FCIS split, including exact self-applied scope, separate ownership and verdicts, and general-suitability non-claims.
- [x] 5.4 Update trust-boundary and assurance-ledger records so coverage remains structural source-execution evidence and Hegel remains sampled evidence; preserve the red baseline as historical and reject promotion to exhaustive or formal assurance.
- [x] 5.5 Document that these lanes do not establish Verus/AeneasVerif representation correspondence, compiler/LLVM/OS correctness, cryptographic collision resistance, or natural-language-to-property proof.

## 6. Validation and Acceptance

- [x] 6.1 Run formatting, Clippy with warnings denied, validator unit/self-tests, and all focused positive/negative coverage and Hegel fixtures under the pinned environment.
- [x] 6.2 Run `devenv tasks run buoy:properties` and verify every inventory row has a present, current, correctly scoped passing result and every expected-failure control is rejected.
- [x] 6.3 Run `devenv tasks run buoy:coverage` and verify every in-scope source, including the local Dylint detector implementation and executable FCIS fixtures, is present and line, region, function, and branch integer counts are exactly covered with positive denominators; confirm the LLVM coverage verdict remains separate from the FCIS structural result.
- [x] 6.4 Run `openspec validate add-cargo-llvm-cov-hegel-rs-testing-requirements --strict --no-interactive` and resolve every planning/spec diagnostic.
- [x] 6.5 Run the exact offline final acceptance oracle `devenv test`; record success only when both effectful lanes execute, all bindings are current, all expected negative controls are observed, and the command exits 0.
