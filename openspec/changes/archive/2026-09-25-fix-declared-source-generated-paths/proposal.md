## Why

The reviewed digest in `nix/consumer-package-v1.source-content.json` covers the declared source closure from `nix/tools/declared-source.nix`. That closure also takes in three paths that no commit contains:

- `.pre-commit-config.yaml`, a Nix store symlink from the Devenv git hooks.
- `dylint_lints/fcis_core/rust-toolchain.toml`, a Nix store symlink for the fcis lint.
- `fuzz/artifacts/<target>`, empty directories that cargo-fuzz creates when `buoy:adversarial` fuzzes.

So the digest depends on the worktree, not only on the commit. On `main`, `--verify` passes only in a Devenv worktree that has been fuzzed. Without `fuzz/artifacts` it reports `stale-source-content-digest`, and a clean export derives a third value. The symlink targets differ between systems, so one reviewed value cannot hold on two systems.

## What Changes

- Exclude `fuzz/artifacts`, `.pre-commit-config.yaml`, and `dylint_lints/fcis_core/rust-toolchain.toml` from the declared source closure, each by its exact path from the repository root. `nix/supply-chain.nix` already excludes the two generated files.
- Re-derive the reviewed source-content digest with `nix/tools/derive-source-content-digest.sh --update`.
- Add a check to `nix/tools/check-derivation-contract.sh`, which `buoy:package-contract` runs. It evaluates the real filter for the three excluded paths and for committed paths with the same names.
- Regenerate the scenario execution evidence, the obligation ledger, the quality-assurance bindings, the naming inventory, and the digest correspondence index with their generators. Only rows that bind the derivation-contract script or a regenerated artifact change.

Goals:

- One commit has one reviewed digest: in a clean export, in a Devenv shell, and after `devenv test`.
- Every committed path stays in the declared source closure.

Non-goals:

- Change what the package builds, its inputs, its variants, or the lock format.
- Change `derive-source-content-digest.sh` or the digest encoding.
- Replace the filter with a Git-tracked file set.
- Add a gate, a task, or a task graph edge, or change any requirement or assurance class.

This change narrows the declared source closure, checks that it stays narrow, and refreshes its reviewed digest. Deterministic static checking and effectful oracle execution are unchanged.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

None. PBD-001 and PBD-005 already require a reviewed digest that identifies the package source.

## Impact

The reviewed digest changes once. After that, `--update` gives the same value in any checkout of the same commit. A branch that also changes the reviewed digest must re-derive it after a rebase.
