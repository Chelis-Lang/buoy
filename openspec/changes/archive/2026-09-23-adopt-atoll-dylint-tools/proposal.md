## Why

Buoy builds its Dylint runner and driver inputs from a local derivation, `nix/dylint-tools.nix`, and a local driver lock. Other CProof repositories need the same pinned Dylint tools. Separate copies can drift.

The shared private overlay `Chelis-Lang/atoll` now provides the same derivation as `pkgs.dylint-tools`. Buoy can consume that single copy.

## What Changes

- Add `atoll` as a Devenv input in `devenv.yaml`, pinned to an exact revision and applied through its `default` overlay.
- Lock the input in `devenv.lock`. The lock gains only the `atoll` node and its root input entry.
- Take `dylintTools` from `pkgs.dylint-tools` in `devenv.nix`.
- Remove `nix/dylint-tools.nix` and `nix/locks/dylint-driver-6.0.1.Cargo.lock`.
- Rebind `QA-0021` configuration from `nix/dylint-tools.nix` to `devenv.lock` and `devenv.yaml`.
- Regenerate the quality-assurance bindings, scenario execution evidence, obligation ledger, naming inventory, digest correspondence index, and declared source-content digest with their generators.

Goals:

- Keep one reviewed Dylint tool derivation for all CProof consumers.
- Keep the Dylint closure identical: the same derivations and store paths for the tools, the driver source, and the driver vendor.
- Keep `devenv.lock` as the authority for the new pin.

Non-goals:

- Change the Dylint version, the FCIS lint, its fixtures, or its detector scope.
- Change any requirement, assurance class, gate, or task graph edge.
- Change CI workflows. Buoy's workflows are local-only policy jobs.

This change affects only the environment closure. Deterministic static checking and effectful oracle execution are unchanged.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

None. The existing requirements already hold for the moved derivation.

## Impact

Nix must fetch the private `atoll` input once. Local Nix needs a GitHub access token in `access-tokens`. After realization, the closure and every build product are unchanged.

The closure is identical, so no fixture, golden, proof, property, or campaign artifact changes.
