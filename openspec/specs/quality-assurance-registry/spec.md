# quality-assurance-registry Specification

## Purpose

Define the truthful inventory, validation, reporting, ownership, and local gate integration for Buoy quality controls.

## Requirements
### Requirement: QAR-001 Versioned and complete quality-control inventory

The repository MUST maintain a versioned machine-readable quality-assurance registry. Each distinct project control MUST have one stable `QA-*` identity.

The inventory MUST account for each QA-role tool in the authoritative Devenv environment. This scope includes common packages and script runtime packages.

The inventory MUST also account for every stable focused command and every QA obligation in phase handoffs or owning OpenSpec artifacts.

Each item MUST have a control row or an explicit non-applicable disposition. Each row MUST declare scope, owner, state, command, evidence, assurance class, assumptions, exclusions, and non-claims.

#### Scenario: Positive - implemented controls are inventoried
- **GIVEN** the repository implements Cargo tests, Hegel properties, Dylint checks, LLVM coverage, finite models, and formal lanes
- **WHEN** the live registry is checked for completeness
- **THEN** each distinct control has one stable row with its actual scope, state, command, owner, and evidence references

#### Scenario: Positive - common and runtime controls are inventoried
- **GIVEN** the environment contains Cargo tests, Hegel properties, Dylint checks, LLVM coverage, finite models, and formal lanes
- **WHEN** registry completeness is checked against resolved Devenv data
- **THEN** each distinct control and QA-role tool has one stable row with its actual exposure mode

#### Scenario: Negative - exposed QA tool is omitted
- **GIVEN** the environment exposes a QA-role tool without a control row or non-applicable disposition
- **WHEN** registry completeness is checked
- **THEN** the check MUST fail with a deterministic missing-control diagnostic that names the tool identity

#### Scenario: Negative - runtime-only QA tool is omitted
- **GIVEN** a focused script declares a QA-role runtime package without a control row or non-applicable disposition
- **WHEN** registry completeness is checked
- **THEN** the check MUST fail with a deterministic missing-control diagnostic that names the tool identity

### Requirement: QAR-002 Orthogonal implementation, enforcement, freshness, execution, and assurance facts
The registry MUST represent implementation state, enforcement state, evidence freshness, execution verdict, and assurance class as independent facts. Implementation state MUST be one of `planned`, `available`, `implemented`, `blocked`, or `retired`; enforcement state MUST be one of `none`, `advisory`, `focused`, or `blocking`; and execution verdict MUST remain `not-run`, `pass`, `fail`, `error`, or `timeout` only when supplied by an applicable normalized result authority. No value on one axis MAY synthesize a stronger value on another axis.

#### Scenario: Positive - installed campaign tool remains available
- **GIVEN** `cargo-mutants` is pinned in the shell but no current Buoy mutation campaign and threshold are implemented
- **WHEN** the registry report is generated
- **THEN** the mutation control is reported as `available` without a current passing execution verdict or blocking enforcement claim

#### Scenario: Negative - installed tool is called implemented
- **GIVEN** a row claims `implemented` solely because its executable is present in the pinned environment
- **WHEN** state evidence is validated
- **THEN** validation MUST fail and MUST NOT infer implementation, freshness, execution success, or assurance from tool availability

### Requirement: QAR-003 Evidence-backed state and lifecycle claims
Every non-planned control state and every enforcement claim MUST be supported by explicit normalized repository-relative authority. `available` MUST bind an exact tool or configuration identity; `implemented` MUST bind executable project behavior and applicable positive and negative controls; `focused` MUST bind a stable focused command; and `blocking` MUST bind inclusion in the exact final gate plus a gate-omission negative control. `blocked` MUST bind a blocker and discharge condition, while `retired` MUST bind terminal rationale and any replacement. A current execution verdict MUST bind exact source, tool, configuration, scope, and normalized result identities.

#### Scenario: Positive - Dylint control is implemented and focused
- **GIVEN** the Dylint FCIS row binds the local lint source, pinned Dylint identity, allowed fixture, expected-rejection fixtures, and `devenv tasks run buoy:fcis`
- **WHEN** the registry is validated
- **THEN** `implemented` and `focused` are accepted independently of the lane's most recent execution verdict

#### Scenario: Negative - historical Hegel result is reported current
- **GIVEN** a Hegel result binds an earlier property source or settings digest
- **WHEN** the current registry and evidence are validated
- **THEN** the result MUST be reported stale and MUST NOT satisfy a current `pass`, implemented-state evidence, or blocking acceptance

### Requirement: QAR-004 Deterministic validation, bindings, and report generation
The registry validator MUST operate offline over explicit repository-owned inputs, normalize every path to repository-relative UTF-8, reject absolute paths and traversal, reject duplicate IDs and bindings, and order controls and diagnostics by canonical control ID bytes then normalized path bytes. It MUST generate deterministic content bindings and a human-readable status report whose bytes exclude timestamps, host paths, locale, process state, durations, network state, raw terminal formatting, and filesystem traversal order. Check mode MUST fail when the committed bindings or report differ from regenerated output.

#### Scenario: Positive - order and root do not affect output
- **GIVEN** identical authoritative input bytes are presented from different absolute roots and in different discovery orders
- **WHEN** bindings and the status report are generated
- **THEN** their normalized repository-relative bytes are identical

#### Scenario: Negative - hand-edited report drifts
- **GIVEN** a maintainer edits a generated status cell without changing the machine-readable registry or evidence
- **WHEN** report check mode runs
- **THEN** validation MUST fail with a deterministic generated-report-drift diagnostic

### Requirement: QAR-005 Static registration and effectful gate execution remain separate

The focused registry lane MUST check registration, consistency, freshness, command presence, graph inclusion, and generated artifacts. It MUST NOT claim execution of every control.

A `blocking` claim MUST bind structural inclusion in the resolved task graph for `devenv test`. Source-text presence in a Nix file MUST NOT establish inclusion.

Only the child lane result and the final oracle exit status MAY establish execution success. Historical results, sibling lanes, waivers, and aggregate labels MUST NOT synthesize `pass`.

#### Scenario: Positive - registry passes before one child runs
- **GIVEN** an implemented Hegel lane has valid registration and graph inclusion but no current normalized result
- **WHEN** `devenv tasks run buoy:quality` checks the registry
- **THEN** the registry MAY pass while the Hegel execution verdict remains `not-run`

#### Scenario: Negative - blocking lane is omitted from final gate
- **GIVEN** a control is `blocking` but its focused lane is absent from the exact final command chain
- **WHEN** the gate-omission control and registry check run
- **THEN** both MUST reject the blocking claim with one stable omission diagnostic and no execution success

#### Scenario: Negative - blocking lane exists but lacks a final graph path
- **GIVEN** a blocking focused task exists in resolved task data but no dependency path connects it to `devenv:enterTest`
- **WHEN** the gate-omission control and registry check run
- **THEN** both MUST reject the blocking claim with one stable omission diagnostic and no execution success

### Requirement: QAR-006 Single implementation ownership and explicit lifecycle
Each control implementation obligation MUST name exactly one owning OpenSpec change or completed baseline authority. Related changes and evidence records MAY be cross-referenced but MUST NOT become additional owners, automatically synchronize task completion, or count one result as independent duplicate evidence. Stable control IDs MUST NOT be reused after retirement. Adding, strengthening, blocking, deferring, replacing, or retiring a control MUST update the registry, bindings, report, owner mapping, and applicable gate references together.

#### Scenario: Positive - focused coverage change owns its control
- **GIVEN** the exact LLVM coverage implementation is owned by `add-cargo-llvm-cov-hegel-rs-testing-requirements` and is also referenced by the broader production-hardening change
- **WHEN** ownership is validated
- **THEN** the focused change is the single implementation owner and the hardening reference does not duplicate ownership or evidence credit

#### Scenario: Negative - two active changes claim the same implementation task
- **GIVEN** one control row assigns two independent owners for the same implementation obligation
- **WHEN** owner mappings are validated
- **THEN** validation MUST fail with a deterministic ambiguous-owner diagnostic and MUST NOT infer completion in either change

### Requirement: QAR-007 Exact assurance scope and no cross-class promotion
Every control row and generated report entry MUST state what the control establishes and excludes using Buoy's accepted assurance vocabulary. Source coverage MUST remain structural source-execution evidence; Dylint and target builds MUST remain structural enforcement within their detector scope; Hegel MUST remain sampled property evidence over its committed domains; exhaustive claims MUST identify an exact finite model, bounds, transitions, and invariants; and formal claims MUST bind exact implementation representation, proof source, verifier, solver, configuration, assumptions, correspondence state, negative control, scope, and exclusions. No control MAY promote another control's evidence class or claim natural-language correspondence, compiler/backend correctness, native-code correctness, cryptographic integrity, or defect absence without separate exact evidence.

#### Scenario: Positive - formal lanes retain exact limits
- **GIVEN** current Verus and Aeneas/Lean controls bind their exact source, toolchain, assumptions, models, correspondence state, negative controls, and exclusions
- **WHEN** the status report is generated
- **THEN** it reports their scoped proof or translation-sentinel claims and explicitly excludes unproved rustc, LLVM, native-code, translator, external-model, and prose correspondence claims as applicable

#### Scenario: Negative - Hegel samples are labeled exhaustive
- **GIVEN** a Hegel control executes all committed sampled cases successfully
- **WHEN** its assurance class is validated
- **THEN** validation MUST reject `exhaustively-checked` or `formally-proved` unless separate exact finite-model or proof bindings establish that independent claim

### Requirement: QAR-008 Functional boundary and exact local acceptance commands

Registry parsing, filesystem inspection, task-graph inspection, report generation, and tool bindings MUST remain in the root imperative shell. These functions MUST NOT enter `buoy-core`.

The stable focused command MUST be `devenv tasks run buoy:quality`. The final implementation oracle MUST be local `devenv test`.

Success MUST require exit status 0 after all registered local lanes run offline through the resolved task graph. CI MUST NOT run heavy or full acceptance lanes.

#### Scenario: Positive - focused and final commands succeed
- **GIVEN** complete registry data, current bindings, rejected negative controls, and one final graph path for each blocking control
- **WHEN** `devenv test` runs every required lane offline and exits 0
- **THEN** the change is accepted without an increase to any child assurance scope

#### Scenario: Negative - core acquires registry dependencies
- **GIVEN** `buoy-core` imports registry parsing, filesystem, process, or report functions
- **WHEN** the functional-core boundary and bare-metal target checks run
- **THEN** acceptance MUST fail even if the registry report is current

#### Scenario: Negative - source scan replaces resolved graph inspection
- **GIVEN** a registry check scans `devenv.nix` text but does not parse the resolved task file
- **WHEN** a local import removes one required dependency edge
- **THEN** acceptance MUST fail even if the old command text remains present
