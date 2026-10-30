## Context

`nix/tools/declared-source.nix` defines `source` with `lib.cleanSourceWith` over the repository root. Its filter removes the reviewed-digest lock, `nix/measurements`, `.drain-*`, `.buoy-product-check-*`, `*.profraw`, and the basenames `.devenv`, `.direnv`, `.git`, `.pi`, `target`, and `result`. Everything else in the worktree enters the closure, committed or not. `nix/tools/derive-source-content-digest.sh` hashes that closure, and `nix/consumer-package-v1.nix` builds from it.

Three uncommitted paths appear in an ordinary worktree:

- The Devenv git hooks write `.pre-commit-config.yaml` as a symlink to a generated configuration in the Nix store.
- `nix/devenv/environment.nix` writes `dylint_lints/fcis_core/rust-toolchain.toml` through `files`, also as a symlink into the store.
- In `devenv test`, `buoy:adversarial` runs cargo-fuzz. cargo-fuzz passes each target an `-artifact_prefix` under `fuzz/artifacts/<target>/` and creates that directory even when no crash occurs. `buoy:package-contract` runs after `buoy:adversarial`.

Both symlinks are gitignored. `fuzz/artifacts` is not ignored, but Git records no empty directory.

## Goals / Non-Goals

**Goals:**

- Make the declared source closure a function of the commit.
- Keep one filter as the single definition that both the digest derivation and the package `src` use.

**Non-Goals:**

- Change the package derivation, the realization scripts, or the task graph.
- Change the functional-core boundary. `buoy-core` stays `no_std` and inward-only. The shell still owns all effects.
- Change Verus or AeneasVerif scope, proofs, or exclusions.

## Decisions

### 1. Exclude each path exactly, from the repository root

The filter compares `toString path` with `toString ../../fuzz/artifacts`, `toString ../../.pre-commit-config.yaml`, and `toString ../../dylint_lints/fcis_core/rust-toolchain.toml`. This is the form of the existing `../measurements` rule.

A basename rule would remove committed files. The repository commits a root `rust-toolchain.toml` and the fixture directory `fixtures/logical-mount-artifact-bindings/v1/artifacts`.

### 2. Keep the explicit denylist

The filter stays a list of named exclusions. A Git-tracked file set would also drop source files that a developer has not added yet, and it would change what the package builds in a dirty worktree. That is a larger change than this defect needs.

### 3. Check the real filter in the derivation contract

`nix/tools/check-derivation-contract.sh` already asserts values that Nix computes from the real package function, and `buoy:package-contract` runs it in every `devenv test`. It now also evaluates `(import nix/tools/declared-source.nix { }).source.filter` with `nix-instantiate`. It asks for the verdict on each excluded path and on three committed paths with the same names, and fails with `declared-source-admits-uncommitted-paths` on any other answer. On `main` the filter keeps all six paths, so the check fails there.

A Rust control in `tests/` would also work, but the mutation campaign binds the whole `tests` directory. Any change there requires a fresh campaign and a new campaign date in the ledger. The gate script has no such coupling.

### 4. Regenerate bound evidence, then the digest

The twelve PBD scenario rows bind the derivation-contract script. The generators regenerate each artifact that binds the changed bytes:

- `check-hardening-manifest --write-execution-evidence` refreshes the scenario execution evidence.
- `check-hardening-manifest --ledger-write` refreshes the obligation ledger rows.
- `buoy-quality --generate` refreshes the `QA-0064` binding of the ledger.
- `buoy-product-identity --generate-inventory` refreshes the naming inventory rows for the regenerated lines.
- `check-digest-correspondence --write` refreshes the correspondence rows for the ledger, the bindings, and the scenario evidence.

Another pass changes nothing. `nix/tools/derive-source-content-digest.sh --update` then rewrites the reviewed constant last, because every regenerated file is in the declared source closure. The new value equals the digest that a clean export of the commit derives, and it stays the same with or without `fuzz/artifacts` and the two symlinks.

## Trust Boundaries

The reviewed constant stays the only expected value. Verification still derives the closure's content hash independently and fails closed on any difference. The change narrows what the closure contains. It adds no fallback and no second definition.

## Acceptance Oracle

The final oracle stays `devenv test`. It succeeds only when every automatic phase passes, including `buoy:package-contract`. The focused checks for this change are `nix/tools/check-derivation-contract.sh` and `nix/tools/derive-source-content-digest.sh --verify` with and without the generated paths.

## Risks / Trade-offs

- A future tool could write another uncommitted path into the worktree. The digest would again depend on the worktree until the filter excludes that path.
- The closure excludes `fuzz/artifacts` even if someone commits a crash reproducer there. The package build never reads that directory.
