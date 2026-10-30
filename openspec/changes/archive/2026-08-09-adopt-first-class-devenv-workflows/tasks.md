## Implementation Tasks

### 1. Add workflow fixtures before implementation

- [x] 1.1 Add a valid resolved-task JSON fixture with the complete baseline and P0-P3 graph.
- [x] 1.2 Add negative task fixtures for malformed JSON, unknown dependencies, cycles, and missing lifecycle edges.
- [x] 1.3 Add negative task fixtures for missing phases, nested phases, duplicate child owners, and direct final children.
- [x] 1.4 Add negative task fixtures for `status`, `execIfModified`, missing roots, and incorrect roots.
- [x] 1.5 Add positive and negative configuration fixtures for CLI agreement, Rust module ownership, imports, and hooks.
- [x] 1.6 Add positive and negative runtime package fixtures for common, focused, generated, and ambient tools.
- [x] 1.7 Revise the ADV-013 fixtures for resolved graph authority and exact phase execution.
- [x] 1.8 Revise the RSC-001, RSC-002, and RSC-004 fixtures for runtime package exposure.
- [x] 1.9 Revise the QAR-001, QAR-005, and QAR-008 fixtures for resolved task data.

### 2. Add the typed workflow checker

- [x] 2.1 Register `check-devenv-task-graph` as a root binary in `Cargo.toml`.
- [x] 2.2 Parse task JSON into typed task names, roots, commands, cache states, and dependency sets.
- [x] 2.3 Reject invalid task data at the parser boundary with stable diagnostics.
- [x] 2.4 Detect unknown dependencies and cycles in deterministic task-name order.
- [x] 2.5 Check the baseline order, phase order, child ownership, graph sink, and lifecycle connection.
- [x] 2.6 Reject cache controls and incorrect roots on acceptance tasks.
- [x] 2.7 Normalize the repository root before report and digest output.
- [x] 2.8 Add checker self-tests that execute every positive and negative fixture.
- [x] 2.9 Add the checker to the exact coverage source inventory and coverage controls.
- [x] 2.10 Add `buoy-devenv` and `devenv tasks run buoy:devenv` as focused workflow commands.
- [x] 2.11 Make `buoy-gate-wiring` call the typed checker without source-text inspection.

### 3. Move the Rust toolchain into the language module

- [x] 3.1 Enable `languages.rust` and set `languages.rust.toolchainFile = ./rust-toolchain.toml`.
- [x] 3.2 Replace direct primary toolchain construction with `config.languages.rust.toolchainPackage`.
- [x] 3.3 Add `rust-analyzer` to the development components in `rust-toolchain.toml`.
- [x] 3.4 Preserve the pinned channel, profile, compiler components, and target.
- [x] 3.5 Move Rustup link creation into a task before `devenv:enterShell`.
- [x] 3.6 Preserve the named Buoy, stable Dylint, and Verus Rustup links.
- [x] 3.7 Add focused checks for Rust, Rust Analyzer, Dylint, Verus, and offline tool discovery.
- [x] 3.8 Update the closure inventory for language-module C, LSP, and toolchain packages.

### 4. Split the Devenv configuration into local modules

- [x] 4.1 Add modules for the environment, hooks, tasks, core scripts, assurance scripts, adversarial scripts, and acceptance scripts.
- [x] 4.2 Pass shared tool packages through one `_module.args` owner.
- [x] 4.3 Move each script body to one module owner without command changes.
- [x] 4.4 Keep only static imports and shared package construction in `devenv.nix`.
- [x] 4.5 Add a duplicate-script-owner negative control.
- [x] 4.6 Compare resolved commands and environment values before and after the split.
- [x] 4.7 Run nixd diagnostics on each changed Nix file.

### 5. Define one authoritative acceptance graph

- [x] 5.1 Add task nodes for versions, workflow checks, format, Clippy, workspace tests, and documentation.
- [x] 5.2 Add explicit serial edges for the baseline and product identity checks.
- [x] 5.3 Replace P0 script orchestration with child task edges and a P0 handoff node.
- [x] 5.4 Replace P1 script orchestration with child task edges and a P1 handoff node.
- [x] 5.5 Replace P2 script orchestration with child task edges and a P2 handoff node.
- [x] 5.6 Replace P3 script orchestration with child task edges and a P3 handoff node.
- [x] 5.7 Connect P0, P1, P2, and P3 in the reviewed order.
- [x] 5.8 Add strict OpenSpec tasks after P3.
- [x] 5.9 Make `buoy:gate` the graph sink before `devenv:enterTest`.
- [x] 5.10 Remove the duplicate acceptance sequence from `enterTest` and `buoy-gate`.
- [x] 5.11 Keep compatibility aliases disconnected from the final graph.
- [x] 5.12 Check that each acceptance task has no `status` or `execIfModified` value.
- [x] 5.13 Check that `devenv test` executes each required task exactly once.

### 6. Make repository task roots explicit

- [x] 6.1 Add a shared task constructor with `cwd = config.git.root`.
- [x] 6.2 Apply the constructor to every `buoy:*` task and compatibility alias.
- [x] 6.3 Run representative focused tasks from the root and a nested directory.
- [x] 6.4 Compare normalized outputs from both caller directories.
- [x] 6.5 Add root checks to `devenv tasks run buoy:devenv`.

### 7. Add pinned lightweight Git hooks

- [x] 7.1 Add the `git-hooks` input to `devenv.yaml` and refresh `devenv.lock`.
- [x] 7.2 Set `require_version: true` in `devenv.yaml`.
- [x] 7.3 Add Rust format, Nix format, and strict OpenSpec hooks.
- [x] 7.4 Declare exact packages for every hook.
- [x] 7.5 Exclude heavy assurance commands from the hook set.
- [x] 7.6 Add `.pre-commit-config.yaml` to `.gitignore`.
- [x] 7.7 Run the hooks against positive and negative fixture changes.
- [x] 7.8 Check that hook success does not create a QA execution verdict.

### 8. Scope specialized tools to focused scripts

- [x] 8.1 Define the common shell package set and each specialized runtime package set.
- [x] 8.2 Assign Dylint tools to `buoy-fcis`.
- [x] 8.3 Assign Aeneas, Lean, Verus, and Z3 to the formal scripts.
- [x] 8.4 Assign `cargo-llvm-cov` to `buoy-coverage`.
- [x] 8.5 Assign `cargo-fuzz` to the fuzz scripts.
- [x] 8.6 Assign `cargo-mutants` to the mutation campaign script.
- [x] 8.7 Assign `cargo-deny` and closure runners to the supply-chain script.
- [x] 8.8 Assign all reported tool packages to the tool-version task.
- [x] 8.9 Remove specialized tools from the common shell package list.
- [x] 8.10 Check each focused script with an empty ambient `PATH` and empty user tool homes.

### 9. Reconcile quality and supply-chain evidence

- [x] 9.1 Add exposure mode to the closure inventory schema and checker.
- [x] 9.2 Regenerate the closure inventory, size bindings, and source manifest.
- [x] 9.3 Include runtime-only tools in quality-registry completeness checks.
- [x] 9.4 Replace source-text command checks with resolved task-graph checks.
- [x] 9.5 Regenerate `assurance/v1/quality-assurance-controls.toml` and the quality status report.
- [x] 9.6 Regenerate ADV-013, RSC-001, RSC-002, RSC-004, QAR-001, QAR-005, and QAR-008 results.
- [x] 9.7 Add DEV-001 through DEV-008 to fixture, execution, and obligation records.
- [x] 9.8 Regenerate `assurance/v1/scenario-requirement-executions.tsv`.
- [x] 9.9 Regenerate `assurance/v1/obligation-ledger.tsv`.
- [x] 9.10 Run `devenv tasks run buoy:closure-size`, which runs the `buoy:deny` and `buoy:offline-acceptance` successor tasks of the removed `buoy:supply-chain` task.
- [x] 9.11 Run `devenv tasks run buoy:quality`.
- [x] 9.12 Run `devenv tasks run buoy:obligations`.

### 10. Update documentation and focused checks

- [x] 10.1 Document the cumulative phase task behavior in `docs/implementation/phase-handoffs.md`.
- [x] 10.2 Update `docs/engineering/enforcement-order.md` for the resolved graph authority.
- [x] 10.3 Update Rust and formal tool guidance for the language module and runtime packages.
- [x] 10.4 Update `README.md` with `devenv tasks run buoy:devenv`.
- [x] 10.5 Document Git hook scope and the separate final acceptance authority.
- [x] 10.6 Run `devenv tasks run buoy:devenv`.
- [x] 10.7 Run `devenv tasks run buoy:gate`.

## Archive Tasks

- [x] Run `openspec validate --all --strict --no-interactive`.
- [x] Run `openspec schema validate buoy-bridge`.
- [x] Run the exact repository acceptance command `devenv test`.
