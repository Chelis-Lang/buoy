# Devenv Workflow

## Purpose

Define the pinned project environment, task graph, Git hooks, task roots, and runtime package boundaries for local acceptance.

## Requirements

### Requirement: DEV-001 Devenv CLI and module version agreement

The repository MUST set `require_version: true` in `devenv.yaml`. The Devenv CLI MUST match the module version in `devenv.lock` before evaluation.

#### Scenario: Positive - matching CLI loads the environment
- **GIVEN** the CLI version matches the pinned Devenv module version
- **WHEN** a maintainer runs `devenv shell`, `devenv tasks run`, or `devenv test`
- **THEN** Devenv evaluates the repository configuration

#### Scenario: Negative - mismatched CLI attempts acceptance
- **GIVEN** the CLI version does not match the pinned Devenv module version
- **WHEN** a maintainer runs an authoritative command
- **THEN** Devenv MUST fail before any project task supplies an acceptance result

### Requirement: DEV-002 Rust language module owns the primary toolchain

The configuration MUST enable `languages.rust` and MUST set `languages.rust.toolchainFile` to `rust-toolchain.toml`. Project code MUST use `config.languages.rust.toolchainPackage` as the primary Rust package.

The toolchain file MUST retain the pinned channel, profile, compiler components, and target. It MUST include `rust-analyzer` as a development component.

Rustup aliases for Dylint and Verus MUST resolve to their declared Nix packages. They MUST NOT download a toolchain or select an ambient toolchain.

#### Scenario: Positive - module and Rustup use pinned packages
- **GIVEN** the Rust language module reads the committed toolchain file
- **WHEN** Rust, Dylint, Verus, or the Rust language server starts
- **THEN** each tool resolves through its declared Nix package without network access

#### Scenario: Negative - a second primary toolchain bypasses the module
- **GIVEN** project configuration constructs another primary Rust package directly from `rust-overlay`
- **WHEN** the Devenv configuration check runs
- **THEN** the check MUST reject the duplicate toolchain authority

### Requirement: DEV-003 Local modules own distinct configuration areas

The root `devenv.nix` MUST compose local Nix modules for the environment, scripts, tasks, hooks, and acceptance lifecycle. Each script MUST have one module owner.

The module split MUST preserve all stable command names and resolved environment values. A module path MUST remain repository-relative.

#### Scenario: Positive - imported modules preserve resolved behavior
- **GIVEN** all local modules and the same pinned inputs
- **WHEN** Devenv evaluates the root configuration
- **THEN** the resolved commands, task edges, environment values, and packages match their declared contracts

#### Scenario: Negative - two modules define one script body
- **GIVEN** two local modules define the executable body for the same stable script
- **WHEN** Devenv evaluates the configuration
- **THEN** evaluation MUST fail instead of selecting an implicit owner

### Requirement: DEV-004 Repository tasks use one explicit root

Every `buoy:*` task MUST set `cwd` to `config.git.root`. Each task MUST use repository-relative paths from that root.

A focused task MUST produce the same semantic result from the repository root and any repository subdirectory. Host absolute paths MUST NOT enter canonical output.

#### Scenario: Positive - focused task starts in a subdirectory
- **GIVEN** a maintainer starts in a repository subdirectory
- **WHEN** the maintainer runs a focused `buoy:*` task
- **THEN** the task runs from `config.git.root` and produces the root-run result

#### Scenario: Negative - task inherits an external directory
- **GIVEN** a repository task has no explicit root and reads a relative fixture path
- **WHEN** the caller starts outside the repository root
- **THEN** the configuration check MUST reject the task before it can supply evidence

### Requirement: DEV-005 One task graph defines local acceptance

One Devenv task graph MUST define the complete local acceptance flow. `devenv test` MUST reach that graph through `devenv:enterTest`.

The graph MUST execute baseline checks, product-identity checks, P0, P1, P2, P3, and strict OpenSpec checks exactly once. Explicit edges MUST preserve the reviewed order.

`enterTest` and `buoy-gate` MUST NOT contain separate copies of the acceptance command sequence. Acceptance tasks MUST NOT use `status` or `execIfModified`.

#### Scenario: Positive - final lifecycle runs the graph once
- **GIVEN** every task and dependency edge is present in the resolved task file
- **WHEN** `devenv test` runs offline
- **THEN** each required task runs once in the reviewed order and final success requires every task to succeed

#### Scenario: Negative - duplicate shell sequence bypasses the graph
- **GIVEN** `enterTest` contains a direct phase command that also exists in the acceptance graph
- **WHEN** the gate structure check runs
- **THEN** the check MUST reject the duplicate execution path with a stable diagnostic

### Requirement: DEV-006 Resolved task data is the gate structure authority

The stable focused environment command MUST be `devenv tasks run buoy:devenv`. It MUST run workflow self-tests and the live configuration check.

Gate structure checks MUST parse `DEVENV_TASK_FILE` into a typed task graph. They MUST NOT infer task ownership from source text in `devenv.nix`.

The checker MUST reject unknown dependencies, cycles, missing phases, phase nesting, duplicate child ownership, direct final-child edges, and missing lifecycle connection. Diagnostics MUST use stable task names.

#### Scenario: Positive - resolved graph has exact phase ownership
- **GIVEN** each focused child belongs to one phase and P0 through P3 form the reviewed chain
- **WHEN** the gate structure checker parses the resolved task file
- **THEN** it accepts the graph and reports no source-file location as task identity

#### Scenario: Negative - one child belongs to two phases
- **GIVEN** two phase tasks depend on the same focused child as direct phase ownership
- **WHEN** the gate structure checker parses the resolved task file
- **THEN** it MUST fail with one deterministic duplicate-owner diagnostic

### Requirement: DEV-007 Git hooks give fast local checks

Devenv MUST manage Git hooks through the pinned `git-hooks` input. The hooks MUST check Rust format, Nix format, and OpenSpec artifacts.

A hook MUST use only declared packages and repository configuration. A hook MUST NOT run coverage, fuzz, mutation, formal, adversarial, or full acceptance lanes.

`devenv test` MUST run equivalent or stronger checks. Git hook success MUST NOT supply an execution verdict for a blocking assurance control.

#### Scenario: Positive - formatted commit passes fast checks
- **GIVEN** changed Rust, Nix, and OpenSpec files satisfy their focused checks
- **WHEN** the Git hook runs
- **THEN** the hook exits successfully without execution of a heavy assurance lane

#### Scenario: Negative - hook replaces final acceptance
- **GIVEN** all Git hooks pass but one blocking acceptance lane fails
- **WHEN** production readiness is evaluated
- **THEN** the hook result MUST NOT replace `devenv test` or supply the missing lane result

### Requirement: DEV-008 Scripts declare specialized runtime packages

Each script that uses a specialized assurance tool MUST declare that tool as a runtime package. The default shell MUST contain only common development tools.

The final acceptance graph MUST realize every required runtime package from the pinned closure. Removal from the default shell MUST NOT remove the tool from inventory or evidence bindings.

#### Scenario: Positive - focused formal script receives its tools
- **GIVEN** Aeneas, Lean, Verus, and Z3 are absent from the common package list
- **WHEN** the focused formal script runs
- **THEN** its runtime package set supplies the exact pinned tools

#### Scenario: Negative - script finds an ambient specialized tool
- **GIVEN** a focused script omits its required runtime package and the host provides a command with the same name
- **WHEN** the script or supply-chain check runs
- **THEN** the command MUST fail closed and MUST NOT accept the ambient tool

### Requirement: DEV-009 Explicit selection for coverage and mutation checks

The resolved Devenv graph MUST expose these focused tasks:

- `buoy:coverage`
- `buoy:coverage-evidence`
- `buoy:mutation-campaign`
- `buoy:mutation-replay`

These tasks and their compatibility aliases MUST have no dependency path to `devenv test`, a phase task, or another automatic task.

Automatic acceptance MUST NOT start LLVM instrumentation, `cargo-mutants`, the recorded coverage check, or the named mutation replay.

Coverage-bearing source rows MUST retain manual coverage dispositions. The focused coverage task MUST continue to select these rows.

Automatic ledger validation MUST classify manual coverage rows as not applicable. Stale manual coverage evidence MUST NOT block `devenv test`.

The task graph checker MUST reject an automatic dependency on any of these focused tasks.

#### Scenario: Positive - maintainer selects a focused task
- **GIVEN** the Devenv environment exposes the four focused tasks
- **WHEN** a maintainer explicitly selects one task
- **THEN** only that task starts its declared check or campaign

#### Scenario: Positive - automatic acceptance excludes focused checks
- **GIVEN** the four focused tasks exist without automatic dependencies
- **WHEN** `devenv test` runs
- **THEN** it MUST complete without those tasks

#### Scenario: Positive - stale manual coverage remains outside acceptance
- **GIVEN** a manual coverage row has stale recorded evidence
- **WHEN** automatic ledger validation runs
- **THEN** it MUST classify that row as not applicable
- **AND** the explicit coverage task MUST still select that row

#### Scenario: Negative - a phase depends on a focused task
- **GIVEN** one phase has a direct or transitive dependency on a listed focused task
- **WHEN** the task graph checker validates the resolved graph
- **THEN** it MUST fail with one stable manual-task dependency diagnostic

#### Scenario: Negative - a focused task becomes an automatic prerequisite
- **GIVEN** an automatic task depends on `buoy:coverage-evidence` or `buoy:mutation-replay`
- **WHEN** automatic acceptance is validated
- **THEN** the graph check MUST reject the dependency

### Requirement: DEV-010 Advisory downstream publication tasks

A downstream task introduced by this capability MUST remain explicit and advisory.

This rule applies to downstream publication checks, publication generation, evidence rendering, and consumer-package realization.

Such a task MUST have no direct or transitive dependency path to `devenv test`, a phase task, or another automatic task.

Buoy repository acceptance MUST test the owned implementation through its declared local test graph. It MUST NOT treat a downstream task result as execution evidence.

#### Scenario: Positive - maintainer selects publication check

- **GIVEN** one downstream environment with an explicit Buoy publication-check task
- **WHEN** a maintainer selects that task
- **THEN** the task runs without changing the consumer acceptance graph

#### Scenario: Negative - consumer test depends on publication check

- **GIVEN** one downstream `devenv test` graph with a path to a Buoy publication task
- **WHEN** task-graph policy validation runs
- **THEN** validation fails with a deterministic automatic-publication-task diagnostic
