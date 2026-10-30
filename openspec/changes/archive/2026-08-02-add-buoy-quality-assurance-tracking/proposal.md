## Why

Buoy has implemented and planned quality controls across tests, Hegel-rs properties, local Dylint rules, exact LLVM coverage, finite models, formal verification, mutation testing, fuzzing, and supply-chain checks, but their status is distributed across task definitions, assurance records, documentation, and overlapping OpenSpec changes. A focused, machine-checked registry is needed to answer what is merely available, what is implemented and executable, what is current, and what actually blocks the authoritative gate without turning one fact into a broader assurance claim.

## What Changes

### Goals

- Add a versioned repository-owned registry of Buoy testing and code-quality controls with stable IDs, category, implementation state, focused command, scope, enforcement state, evidence bindings, assurance class, trust boundary, and owning OpenSpec obligation.
- Seed the registry from the current repository, including Rust formatting and linting, Cargo tests, Hegel-rs, Dylint FCIS enforcement, `cargo-llvm-cov`, frozen fixtures and goldens, differential/reference checks, finite models, Verus, Aeneas/Lean, fuzzing, mutation testing, Miri/sanitizers, dependency policy, and offline reproducibility.
- Validate that configured, executable, current, and blocking states remain independent and are supported by repository facts; reject tools that are installed but reported as implemented, focused lanes reported as blocking when omitted from `devenv test`, stale evidence reported as current, and duplicate ownership of another change's obligation.
- Generate a deterministic human-readable status report from the validated registry and document how maintainers update it when controls are added, strengthened, blocked, deferred, or retired.
- Add registry validation and generated-report drift checks to the authoritative offline gate.

### Non-goals

- Do not implement the planned fuzz, mutation, Miri, sanitizer, platform, or supply-chain controls tracked by the registry.
- Do not duplicate or automatically complete obligations owned by `add-cargo-llvm-cov-hegel-rs-testing-requirements` or `harden-spec-provenance-production-readiness`.
- Do not treat tool availability, configuration, source presence, a historical pass, or registry membership as a current execution verdict or proof.
- Do not change Buoy's product APIs, canonical graph semantics, evidence classes, or OpenSpec reference-only boundary.

## Capabilities

### New Capabilities

- `quality-assurance-registry`: Truthful, deterministic inventory, validation, reporting, ownership, and gate integration for Buoy's testing and code-quality controls.

### Modified Capabilities

None.

## Impact

- Adds a versioned machine-readable registry under `assurance/v1/`, deterministic positive and negative fixtures, and a focused registry validator/generator under `tools/`.
- Adds a generated maintainer-facing status document under `docs/assurance/` and update instructions for new or changed controls.
- Adds a focused `buoy:quality` task and wires registry checks into local `devenv test`. The task inventories other lanes but does not execute them.
- References existing task definitions, source and property inventories, proof records, toolchain bindings, trust-boundary documentation, and active OpenSpec changes without taking ownership of their implementation work.
