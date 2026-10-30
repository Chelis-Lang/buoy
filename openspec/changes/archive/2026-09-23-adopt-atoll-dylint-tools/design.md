## Context

`devenv.nix` built `dylintTools` with `pkgs.callPackage ./nix/dylint-tools.nix { }`. That derivation provides `cargo-dylint` 6.0.1 and two passthru values: `dylintDriverSource` and `dylintDriverVendor`. The vendor imports `nix/locks/dylint-driver-6.0.1.Cargo.lock`.

The value feeds `nix/supply-chain.nix`, `nix/devenv/environment.nix`, and the Dylint scripts. The environment exports `BUOY_DYLINT_DRIVER_SOURCE` and `BUOY_DYLINT_DRIVER_VENDOR` from the passthru values.

`Chelis-Lang/atoll` is an input-less private flake. Its `overlays.default` adds `pkgs.dylint-tools`. The atoll package is a verbatim copy of the local derivation. Only the lock file path differs: the lock moves next to the package file.

## Goals / Non-Goals

**Goals:**

- Consume the shared derivation through the Devenv nixpkgs import.
- Keep every Dylint store path identical.
- Keep all repository bindings current through their generators.

**Non-Goals:**

- Update Dylint, the driver, the lint crate, or the Rust toolchain.
- Add a requirement, an assurance class, or a gate.
- Change the functional-core boundary. `buoy-core` stays `no_std` and inward-only. The shell still owns all effects.
- Change Verus or AeneasVerif scope, proofs, or exclusions.

## Decisions

### 1. Pin atoll as a Devenv overlay input

`devenv.yaml` adds `atoll` with `url: github:Chelis-Lang/atoll/<rev>` and `overlays: [ default ]`. Devenv applies the overlay inside its single nixpkgs import. atoll has no inputs, so it needs no `follows`.

`devenv update atoll` records the revision and content hash in `devenv.lock`. No other input moves.

**Alternatives considered:**

- A `git+ssh` URL. Rejected because every CProof consumer uses the token-based GitHub fetcher.
- Keep a local copy. Rejected because two copies can drift without review.

### 2. Keep the `dylintTools` binding

`devenv.nix` sets `dylintTools = pkgs.dylint-tools;`. Every consumer of the binding stays unchanged. The environment variables and script runtime packages resolve through the same attribute names.

### 3. Prove closure neutrality by evaluation

The same evaluation compares the removed local derivation with the atoll overlay package. Both use devenv-nixpkgs revision `6004ea8c229fe9d41b21c6f4c76bf6c2e10771dd` from `devenv.lock`. The derivation paths are equal. The output paths are also equal:

- `dylint-tools`: `/nix/store/61k5x8wgiy1qlwnghgf6v5vffxxjnw1k-dylint-tools-6.0.1`
- `dylintDriverVendor`: `/nix/store/y5hc1s5g0zg3b5vcb2m0c7ra4m513j6r-cargo-vendor-dir`
- `dylintDriverSource`: `/nix/store/rcybmn2cr9npwcxr932dhm8b4fdimifb-dylint_driver-6.0.1.tar.gz`

Equal derivations mean equal build inputs. The FCIS verdict, closure sizes, and every downstream artifact keep their identities.

### 4. Rebind QA-0021 to the pinning files

`QA-0021` bound `nix/dylint-tools.nix` as configuration. That file no longer exists. `devenv.yaml` names the atoll pin, and `devenv.lock` holds its revision and content hash. `QA-0021` now binds both files. Its scope, state, command, and claims are unchanged.

### 5. Regenerate bound evidence under RSC-005

The lock update changes the bytes of `devenv.yaml`, `devenv.lock`, `devenv.nix`, and the registry. The generators regenerate each artifact that binds those bytes:

- `buoy-quality --generate` regenerates the quality-assurance bindings. The generated status report does not change.
- `check-hardening-manifest --write-execution-evidence` refreshes the scenario execution evidence for the rebound registry.
- `check-hardening-manifest --ledger-write` refreshes the `lock:devenv` and phase-handoff ledger rows.
- `buoy-product-identity --generate-inventory` refreshes the naming inventory rows for the regenerated lines.
- `check-digest-correspondence --write` refreshes the correspondence rows for the regenerated artifacts.
- `nix/tools/derive-source-content-digest.sh --update` refreshes the declared source-content digest last, after another pass of the other generators changes nothing.

The closure is identical. No fixture, golden, parser conformance record, proof binding, property run, or campaign artifact changes.

## Trust Boundaries

`devenv.lock` is the authority for the atoll revision and content hash. Nix verifies the fetched content against that hash.

The GitHub access token only authorizes the initial fetch. The token never enters the repository, the lock, a derivation, or a build product. After realization, acceptance uses the store content and needs no credential.

## Acceptance Oracle

The final oracle stays `devenv test`. It succeeds only when every automatic phase passes. The focused checks for this change are `devenv tasks run buoy:fcis`, `buoy:quality`, `buoy:obligations`, and `buoy:closure-size`.

## Risks / Trade-offs

- A maintainer without access to `Chelis-Lang/atoll` cannot realize a new environment. Existing store content stays usable.
- A future atoll revision changes Buoy's closure only through an explicit `devenv update atoll` and the RSC-005 protocol.
