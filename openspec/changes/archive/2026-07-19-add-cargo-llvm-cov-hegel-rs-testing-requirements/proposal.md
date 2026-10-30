## Why

The repository already uses `cargo-llvm-cov` and Hegel-rs, but their acceptance requirements are spread across implementation scripts, assurance records, and broader hardening plans. It also serves as a repository-level dogfood target for `cargo-llvm-cov`, pinned local Dylint rules, and the functional-core/imperative-shell (FCIS) split: coverage includes its own complete Rust source inventory, the Dylint rules check its own architecture, and the production crate layout must continue to obey the boundary those rules enforce. A focused contract is needed so coverage and property-test results cannot become false-green assurance claims through incomplete source scope, rounded thresholds, ambient Hegel settings, reference-only properties, or stale evidence.

## What Changes

### Goals

- Define `cargo-llvm-cov` (the tool intended by “cargo-llvm”) as the pinned coverage authority and require complete reviewed source inventory coverage with exact line, region, function, and branch accounting.
- Make repository self-application explicit and fail closed: Buoy MUST NOT exempt its own core, imperative shell, local Dylint detectors, or FCIS fixtures from their applicable focused and aggregate gates.
- Define fail-closed coverage behavior for missing targets, zero denominators, unsupported platform aggregation, exclusions, and any metric below 100%.
- Define deterministic Hegel-rs execution requirements, including the exact upstream identity, fixed settings, committed generator domains, production-versus-independent-reference assertions, replay information, and source/configuration evidence bindings.
- Require focused positive and negative controls for both testing lanes and include both lanes in the offline acceptance gate.
- Keep deterministic static validation of inventories, pins, and evidence bindings distinct from effectful execution of instrumented binaries and sampled properties.

### Non-goals

- Do not treat source coverage as behavioral completeness, exhaustive exploration, or proof.
- Do not treat a passing Hegel sample as exhaustive checking or formal verification.
- Do not change production APIs, canonical graph semantics, or assurance-class definitions.
- Do not permit a similarly named coverage or property-testing framework to substitute for the selected pinned tools.
- Do not treat successful dogfooding in this repository as proof that `cargo-llvm-cov`, Dylint, or the FCIS pattern is complete or suitable for every project.

## Capabilities

### New Capabilities

- `llvm-coverage-testing`: Reviewed source scope, pinned `cargo-llvm-cov` execution, exact metric acceptance, supported-platform aggregation, deterministic evidence, and negative controls.
- `hegel-property-testing`: Pinned Hegel-rs identity, deterministic sampled execution, production/reference property obligations, replay and evidence bindings, and negative controls.

### Modified Capabilities

None.

## Impact

- Affects the coverage and property tasks exposed through `devenv.nix` and the final offline acceptance gate.
- Affects `assurance/v1/source-inventory.tsv`, coverage baseline/result records, property inventories/runs, toolchain bindings, and related validators.
- Affects Hegel test modules such as `tests/hegel_properties.rs` and dependency-independent reference authorities.
- Documents Buoy as a dogfood consumer of `cargo-llvm-cov`, Dylint, and its enforced FCIS architecture without taking ownership of those upstream tools or duplicating baseline FCIS obligations.
- Retains `hegeltest` as a development-only dependency outside the `no_std` core closure and requires the existing Cargo/Nix lock closure to remain pinned and offline-resolvable.
- Owns the detailed coverage/property slice of production-hardening P0; completed `LCOV-*` and `HEGEL-*` obligations map exactly once to hardening tasks 2.10, 2.19, 2.21, and 2.22 without changing their completion state automatically.
