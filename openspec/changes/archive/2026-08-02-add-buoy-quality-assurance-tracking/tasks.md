## 1. Test-First Registry Contracts

- [x] 1.1 Add a complete positive registry fixture covering every category, implementation state, enforcement state, evidence state, execution verdict, assurance class, ownership form, and lifecycle form.
- [x] 1.2 Add negative schema and identity fixtures for missing required fields, unknown enum values, malformed or reused `QA-*` IDs, duplicate controls, duplicate bindings, and unaccounted QA-role tools or commands.
- [x] 1.3 Add negative state fixtures proving a pinned tool is not automatically implemented, an implementation is not automatically focused or blocking, and registration or a historical result is not automatically a current pass.
- [x] 1.4 Add positive and negative evidence fixtures for implemented, available, blocked, retired, focused, and blocking controls, including missing implementation paths, stale digests, absent positive/negative controls, and omitted gate commands.
- [x] 1.5 Add positive and negative ownership fixtures for one focused owner plus related changes, ambiguous owners, duplicate evidence credit, checkbox-only implementation claims, retired-ID reuse, and incomplete replacement mappings.
- [x] 1.6 Add deterministic-output and no-laundering fixtures covering discovery-order permutations, alternate absolute roots, absolute/traversing paths, timestamps and host fields, report drift, sampled-as-exhaustive claims, coverage-as-proof claims, and incomplete formal bindings.

## 2. Registry Parser and Validator

- [x] 2.1 Define the versioned `quality-assurance-controls/v1` schema and root-shell Rust values for controls, state axes, commands, ownership, evidence references, trust scope, lifecycle, and diagnostics without adding registry dependencies to `buoy-core`.
- [x] 2.2 Implement strict TOML parsing and local validation for closed fields, stable IDs, normalized repository-relative paths, unique rows/bindings, required state-specific fields, and deterministic diagnostics.
- [x] 2.3 Implement completeness checks that account for QA-role tool bindings, stable QA tasks/scripts, phase-handoff obligations, and explicit non-applicable dispositions without treating discovery as implementation.
- [x] 2.4 Implement evidence-backed state validation for `available`, `implemented`, `blocked`, `retired`, `focused`, and `blocking`, including applicable positive/negative controls and exact gate-registration checks.
- [x] 2.5 Implement source/tool/configuration/result freshness validation that preserves `not-run`, `pass`, `fail`, `error`, and `timeout` and rejects stale results as current.
- [x] 2.6 Implement single-owner, related-change, replacement, retirement, and no-duplicate-credit validation without using OpenSpec task checkboxes as technical evidence.
- [x] 2.7 Implement assurance-scope validation for structural, tested, sampled, bounded exhaustive, and formal claims, including exact finite-model fields and Verus/AeneasVerif proof bindings and exclusions.
- [x] 2.8 Add `buoy-quality --self-test` and make every task 1 positive fixture pass and every negative fixture fail with its exact stable diagnostic and expected cardinality.

## 3. Current Buoy QA Portfolio

- [x] 3.1 Audit the pinned shell, `devenv` tasks/scripts, assurance records, tests, proofs, fixtures, documentation, and active/completed OpenSpec artifacts to produce a reviewed list of current and planned controls with one owner each.
- [x] 3.2 Add registry rows for code hygiene and ordinary verification, including rustfmt, compiler checks and warnings, Clippy, Cargo tests, rustdoc, OpenSpec validation, and any available-but-unused nextest surface.
- [x] 3.3 Add registry rows for structural and deterministic controls, including the bare-metal `no_std` build, Dylint FCIS, schema and hashing policy validators, frozen positive/negative fixtures and goldens, and independent reference checks.
- [x] 3.4 Add registry rows for Hegel-rs sampled properties, exact LLVM source coverage, independent finite models, mutation tripwires, and their focused change ownership without duplicating `LCOV-*` or `HEGEL-*` obligations.
- [x] 3.5 Add registry rows for Verus and Aeneas/Charon/Lean with exact proof or translation-sentinel scope, bindings, assumptions, correspondence state, negative controls, and exclusions.
- [x] 3.6 Add conservative registry rows for fuzz campaigns, `cargo-mutants`, Miri/sanitizers, fault injection, cross-platform checks, `cargo-deny`, and clean offline closure, classifying each from repository evidence as planned, available, implemented, or blocked.
- [x] 3.7 Review every row against `harden-spec-provenance-production-readiness`, `add-cargo-llvm-cov-hegel-rs-testing-requirements`, completed baseline changes, and the assurance ledger schema; resolve ambiguous ownership and retain unsatisfied work as visible nonpassing state.

## 4. Deterministic Bindings and Status Report

- [x] 4.1 Implement deterministic content-binding generation over the registry's authoritative implementation, configuration, result, documentation, and ownership paths using normalized ordering and the selected digest policy.
- [x] 4.2 Implement stale-binding detection and regeneration modes without interpreting ecosystem-specific hashes, package checksums, Git commits, or Nix hashes as Buoy semantic identities.
- [x] 4.3 Implement deterministic Markdown rendering ordered by category and control ID with implementation, enforcement, command, freshness, execution verdict, owner, assurance scope, trust statement, and limitations.
- [x] 4.4 Generate and review `docs/assurance/quality-assurance-status.md` and committed binding artifacts; exclude timestamps, host paths, durations, raw logs, locale, network state, and provider metadata.
- [x] 4.5 Add check mode and golden tests proving byte-identical output across ordering and root permutations and exact failure when the committed report or bindings drift.

## 5. Commands, Gate, and Maintainer Workflow

- [x] 5.1 Add the stable `buoy-quality` script and `devenv tasks run buoy:quality` task to run fixture generation checks, validator self-tests, live validation, binding freshness, and byte-exact report checks offline.
- [x] 5.2 Add a gate-omission negative control proving a control declared `blocking` is rejected when its exact focused lane is absent from the final command chain.
- [x] 5.3 Wire `buoy:quality` into local `devenv test` without treating registry success as execution of any child lane.
- [x] 5.4 Document the maintainer workflow for adding, strengthening, blocking, deferring, replacing, and retiring controls, including one-owner selection, regeneration, focused validation, and stale-result handling.
- [x] 5.5 Link the generated status report from the repository's assurance and engineering documentation and preserve the functional-core, trust, formal-proof, finite-model, and no-cross-class boundaries.

## 6. Validation and Acceptance

- [x] 6.1 Run formatting, Clippy with warnings denied, root and focused validator tests, every positive/negative registry fixture, and core bare-metal/FCIS boundary checks under the pinned environment.
- [x] 6.2 Run `devenv tasks run buoy:quality` twice from clean temporary output locations and verify identical bindings, report bytes, diagnostics, and exit status.
- [x] 6.3 Run `openspec validate add-buoy-quality-assurance-tracking --strict --no-interactive` and resolve every planning/specification diagnostic.
- [x] 6.4 Run an adversarial review of registry completeness, state truthfulness, ownership, gate claims, assurance scope, deterministic output, and false-green controls; resolve every valid blocking finding.
- [x] 6.5 Run local `devenv test` as the exact offline final oracle. Accept the implementation only when all local lanes execute and exit 0.
