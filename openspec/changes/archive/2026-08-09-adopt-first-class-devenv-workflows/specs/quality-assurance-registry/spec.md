## MODIFIED Requirements

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
