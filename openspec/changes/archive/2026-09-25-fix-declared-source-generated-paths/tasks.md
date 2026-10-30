## 1. Reproduce

- [x] 1.1 On `main`, run `nix/tools/derive-source-content-digest.sh --verify` in a Devenv worktree after `devenv test`, then again without `fuzz/artifacts`. Confirm that the first run passes and the second reports `stale-source-content-digest`.
- [x] 1.2 Add the declared-source filter check to `nix/tools/check-derivation-contract.sh` and confirm that it fails against the filter on `main` with `declared-source-admits-uncommitted-paths`.

## 2. Filter

- [x] 2.1 Exclude `fuzz/artifacts`, `.pre-commit-config.yaml`, and `dylint_lints/fcis_core/rust-toolchain.toml` by exact path in `nix/tools/declared-source.nix`.
- [x] 2.2 Confirm that `nix/tools/check-derivation-contract.sh` passes.

## 3. Bound Evidence

- [x] 3.1 Regenerate scenario execution evidence with `check-hardening-manifest --write-execution-evidence`.
- [x] 3.2 Regenerate the obligation ledger with `check-hardening-manifest --ledger-write`.
- [x] 3.3 Regenerate the quality-assurance bindings with `buoy-quality --generate`.
- [x] 3.4 Regenerate the naming inventory with `buoy-product-identity --generate-inventory`.
- [x] 3.5 Regenerate the digest correspondence index with `check-digest-correspondence --write`.
- [x] 3.6 Rerun 3.1 to 3.5 until they change nothing, then re-derive the reviewed digest with `nix/tools/derive-source-content-digest.sh --update`.
- [x] 3.7 Confirm that `--verify` passes with and without `fuzz/artifacts` and the two symlinks, and that a clean export of the commit derives the same digest.

## 4. Focused Validation

- [x] 4.1 Run `devenv tasks run buoy:package-contract --mode single`.
- [x] 4.2 Run `devenv tasks run buoy:obligations --mode single`.
- [x] 4.3 Run `devenv tasks run buoy:quality --mode single`.
- [x] 4.4 Run `devenv tasks run buoy:ledger-green --mode single`.
- [x] 4.5 Run `devenv tasks run buoy:product-identity --mode single`.
- [x] 4.6 Run `devenv tasks run buoy:specs --mode single`.
- [x] 4.7 Run `devenv tasks run buoy:hashing-policy --mode single`.

## 5. Archive

- [x] 5.1 Run `openspec validate --all --strict --no-interactive`.
- [x] 5.2 Run `openspec schema validate buoy-bridge`.
