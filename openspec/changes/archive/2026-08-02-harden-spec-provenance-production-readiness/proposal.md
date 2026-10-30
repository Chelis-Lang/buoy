## Why

The initial repository-independent system passes its development oracle, but a source-bound audit at commit `763d7f084aac` found only 75.29% line, 71.43% function, and 50.68% branch coverage across Cargo-instrumented production files; the standalone fixture/model checker and Dylint implementation were outside that measurement. The same audit found that 112 declared fixture cases are manifest-validated but not behaviorally executed, Hegel properties and exhaustive models do not compare with production, mutation controls are illustrative rather than source mutation campaigns, and formal bindings detect drift without proving representation correspondence. This change closes those false-green, schema, hostile-input, resource, process, proof, and clean-machine gaps before the system is treated as production-ready or used as the blocking foundation for host integrations.

## What Changes

### Goals

- Make every public value, constructor, reference, lifecycle record, source-map entry, report fact, change record, and proof binding conform completely to the frozen v1 field and invariant tables.
- Derive transitive impacts and policy weakening from explicit base/head graphs rather than trusting manifest summaries.
- Implement canonical v1 binary report bytes and retain JSON/human output as deterministic derived presentation.
- Materialize complete Markdown and Rust semantic objects instead of retaining partially interpreted host payloads.
- Harden repository traversal, Unicode/path handling, resource limits, and effectful oracle execution against races, deadlocks, runaway descendants, and unbounded output.
- Make every frozen fixture operation and mutation executable against independent and production authorities, and reject manifest-only evidence that never checks behavior.
- Require production/reference Hegel differential properties and production/model conformance for every inventoried pure semantic obligation.
- Enforce source-bound 100% line, region, function, and branch coverage across production crates, standalone validators, and Dylint detectors while retaining mutation, model, and proof evidence as separate stronger channels.
- Add local fuzzing, differential checks, automated mutation tests, current-host reproducibility, and supply-chain and offline-closure gates.
- Maintain a machine-readable assurance-obligation ledger that exposes point-test, property, finite-model, mutation, formal, platform, and explicit-exclusion disposition without upgrading one evidence class into another.
- Preserve the `no_std` one-way core boundary, OpenSpec reference-only boundary, provider neutrality, exact evidence scopes, and no-laundering semantics.

### Non-goals

- Changing published v1 meanings to fit current implementation shortcuts.
- Adding Chelis-specific capability rows or provider semantics to the generic core.
- Claiming fuzzing, mutation scores, structural checks, exhaustive finite models, or source-level proofs establish broader assurance than their exact scopes.
- Making raw logs, clocks, provider state, unsaved buffers, or network responses canonical inputs.

### Changes

- **BREAKING:** Replace minimally validated constructors and generic declaration wrappers with complete validated v1 values; callers must supply every required binding explicitly.
- **BREAKING:** Make canonical binary report output authoritative and version the existing JSON projection as a derived presentation schema.
- Add graph-derived policy-ratchet, reaffirmation, waiver, replacement, derivation, artifact-freshness, and proof-binding validation.
- Add descriptor-relative no-follow snapshotting where supported, pinned full Unicode case folding, versioned glob semantics, explicit resource budgets, and deterministic budget diagnostics.
- Add process-group isolation, concurrent bounded output draining, semantic result normalization, and descendant cleanup for oracle execution.
- Vendor or Nix-package the complete Cargo/formal closure and prove clean-machine offline execution.
- Add executable fixture and mutation oracles, source-inventory coverage closure, production differential properties and models, coverage-guided fuzz targets, and local mutation-score gates.
- Add an independent canonical checker, proof-obligation accounting, current-host platform observations, and dependency-policy gates.

## Capabilities

### New Capabilities

- `schema-conformance`: Complete frozen-v1 value surfaces, validated construction, source maps, registries, lifecycle history, artifact freshness, and schema-to-API parity.
- `derived-governance-analysis`: Graph-derived transitive impacts, policy ratchets, disposition closure, reaffirmation/waiver protocols, and proof invalidation.
- `canonical-report-encoding`: Authoritative v1 binary static, execution, and impact reports with independently checked derived JSON and human rendering.
- `adapter-schema-completeness`: Full schema-owned Markdown/Rust object materialization, exact attachment/item identity, versioned glob selection, and adapter totality.
- `repository-execution-hardening`: Race-resistant snapshots, Unicode/path conformance, resource budgets, process-tree isolation, bounded logs, and semantic oracle normalization.
- `adversarial-verification`: Executable fixture truthfulness, complete local coverage, production differential properties and models, local fuzzing and mutation tests, current-host checks, fault injection, formal-binding drift controls, and assurance-obligation accounting.
- `reproducible-supply-chain`: Vendored or Nix-fixed offline dependency closure, advisory/license/source policy, and clean-machine reproducibility evidence.

### Modified Capabilities

None. The completed `establish-spec-provenance-system` deltas are reconciled under `openspec/specs/` as the frozen v1 baseline. This change retains `ADDED` operation classification for its seven orthogonal hardening capabilities; it does not reinterpret the baseline capability meanings.

## Impact

- Changes most public APIs in `buoy-core`, adapter outputs, report rendering, CLI machine output, repository snapshotting, and oracle orchestration.
- Adds executable fixture and mutation infrastructure, complete local coverage, independent property and model authorities, fuzz corpora, mutation infrastructure, and proof-obligation records.
- Adds current-host checks, dependency-policy configuration, fixed sources, and deterministic fixtures. Heavy assurance commands run locally, not in CI.
- Requires migration of existing proof records, report goldens, adapter fixtures, and integration callers to complete v1 bindings.
- Keeps all host-specific integration code, including Chelis, outside this production-hardening change.
