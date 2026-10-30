## Why

Buoy uses Devenv as the local acceptance authority, but its configuration bypasses several first-class Devenv features.

The current `enterTest` script duplicates `buoy-gate` and runs all checks as one shell process. This structure can drift and hides task dependencies.

The project also configures Rust directly, does not require the pinned Devenv version, and gives every shell all assurance tools.

The local-only policy disables CI acceptance. Therefore, the local environment must give fast checks and one exact acceptance path.

## What Changes

- Replace the duplicate acceptance scripts with one Devenv task graph.
- Preserve the exact P0, P1, P2, and P3 order with explicit task dependencies.
- Connect the final acceptance task to `devenv:enterTest`.
- Keep `devenv test` as the only final acceptance command.
- Make focused `buoy:*` commands run their declared dependency subgraphs.
- Check the resolved task file instead of source text in `devenv.nix`.
- Add `devenv tasks run buoy:devenv` as the focused environment command.
- Enable `languages.rust` and use `rust-toolchain.toml` through `languages.rust.toolchainFile`.
- Preserve the Rustup aliases that Dylint and Verus require.
- Require a Devenv CLI version that matches the pinned module version.
- Split the large Devenv configuration into local imported modules.
- Set the repository root as the current directory for each repository task.
- Add lightweight Git hooks for Rust format, Nix format, and focused OpenSpec checks.
- Keep the full acceptance gate outside the commit hook.
- Assign specialized assurance tools to the scripts that use them.
- Keep only common development tools in the default shell.

### Goals

- Give `devenv test` one source of truth.
- Make the phase order and child ownership visible in the resolved task graph.
- Keep focused commands stable for existing specifications and evidence.
- Keep every acceptance command deterministic, pinned, offline, and independent of the caller directory.
- Reduce the default shell package set without reducing the final acceptance closure.
- Give maintainers fast local checks before the full acceptance gate.

### Non-Goals

- Do not change product behavior, canonical schemas, digest identities, or assurance classes.
- Do not add a service, process, container, database, network dependency, or CI acceptance job.
- Do not cache or skip an acceptance task through `status` or `execIfModified`.
- Do not run the full acceptance gate as a Git hook.
- Do not remove the compatibility task names in this change.
- Do not change the current Rust channel, profile, targets, compiler components, or offline Cargo source.
- Do not add a Rust toolchain component except `rust-analyzer` for development.
- Do not claim that the task graph or Git hooks prove product correctness.

## Capabilities

### New Capabilities

- `devenv-workflow`: Defines the pinned environment, module layout, Rust toolchain ownership, task graph, lifecycle connection, task roots, Git hooks, and script package scope.

### Modified Capabilities

- `adversarial-verification`: Replaces the duplicate shell gate with one ordered Devenv task graph under `devenv test`.
- `reproducible-supply-chain`: Binds the Devenv CLI, Rust language module, runtime script packages, and offline acceptance closure.
- `quality-assurance-registry`: Reads resolved task data for command presence, phase ownership, and final-gate inclusion.

## Impact

- Devenv configuration: `devenv.nix`, `devenv.yaml`, `devenv.lock`, `.gitignore`, and new local Nix modules.
- Rust environment: `rust-toolchain.toml`, Rustup state links, Dylint driver discovery, Verus toolchain links, and Rust LSP availability.
- Acceptance flow: `buoy:*` tasks, `devenv:enterTest`, phase dependencies, focused commands, and final gate diagnostics.
- Local checks: generated Git hook configuration and lightweight commit-time commands.
- Assurance checks: task-graph inspection, command-presence checks, phase cardinality checks, and their negative controls.
- Documentation: the README, phase handoffs, enforcement order, and toolchain guidance.
- Evidence: normalized gate records can change only when their task or tool bindings include Devenv configuration identities.
