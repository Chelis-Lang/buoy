## 1. Closure Neutrality

- [x] 1.1 Evaluate the local `nix/dylint-tools.nix` derivation and the atoll `pkgs.dylint-tools` on the locked devenv-nixpkgs revision.
- [x] 1.2 Confirm equal derivation paths and equal `dylint-tools`, `dylintDriverVendor`, and `dylintDriverSource` output paths.

## 2. Devenv Input

- [x] 2.1 Add the pinned `atoll` input with the `default` overlay to `devenv.yaml`.
- [x] 2.2 Run `devenv update atoll` and confirm that `devenv.lock` gains only the `atoll` node and its root input entry.
- [x] 2.3 Set `dylintTools = pkgs.dylint-tools;` in `devenv.nix`.
- [x] 2.4 Remove `nix/dylint-tools.nix` and `nix/locks/dylint-driver-6.0.1.Cargo.lock`.
- [x] 2.5 Confirm that `cargo-dylint dylint --version` reports 6.0.1 and that `BUOY_DYLINT_DRIVER_SOURCE` and `BUOY_DYLINT_DRIVER_VENDOR` name existing store paths.

## 3. Assurance Bindings

- [x] 3.1 Rebind `QA-0021` configuration to `devenv.lock` and `devenv.yaml`.
- [x] 3.2 Regenerate scenario execution evidence with `check-hardening-manifest --write-execution-evidence`.
- [x] 3.3 Regenerate the obligation ledger with `check-hardening-manifest --ledger-write`.
- [x] 3.4 Regenerate the quality-assurance bindings and report with `buoy-quality --generate`.
- [x] 3.5 Regenerate the naming inventory with `buoy-product-identity --generate-inventory`.
- [x] 3.6 Regenerate the digest correspondence index with `check-digest-correspondence --write`.
- [x] 3.7 Rerun 3.2 to 3.6 until they change nothing, then regenerate the declared source-content digest with `nix/tools/derive-source-content-digest.sh --update`.
- [x] 3.8 Confirm that no live repository reference names a removed file.

## 4. Focused Validation

- [x] 4.1 Run `devenv tasks run buoy:fcis`.
- [x] 4.2 Run `devenv tasks run buoy:quality` and compare the generated bindings and report with the committed bytes.
- [x] 4.3 Run `devenv tasks run buoy:obligations`.
- [x] 4.4 Run `devenv tasks run buoy:closure-size`.
- [x] 4.5 Run `nix/tools/derive-source-content-digest.sh --verify`.

## 5. Archive

- [x] 5.1 Run `openspec validate --all --strict --no-interactive`.
- [x] 5.2 Run `openspec schema validate buoy-bridge`.
