Replace the bespoke supply-chain checker and its committed closure inventory with devenv-native mechanisms. The current design re-encodes the pins that `devenv.lock` and `Cargo.lock` already hold. That second inventory adds no reproducibility and drifts on every input bump and every tool edit. This change removes the duplication and keeps only the controls that add value beyond the locks.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

**reproducible-supply-chain**

- Remove the closure-inventory control (RSC-004). `devenv.lock`, `Cargo.lock`, and Git already provide immutable pins and provenance. A separate committed inventory adds a synchronization surface without an independent guarantee, because it copies the narHash and checksum values from the locks.
- Reframe the immutable-closure control (RSC-001) to trust `devenv.lock` and `Cargo.lock` as the authoritative pins. Each specialized script must still bind its runtime packages in Devenv configuration.
- Slim the controlled-update control (RSC-005) to lock and vendor updates. Remove the inventory-regeneration requirement.
- Keep the clean-sandbox offline-acceptance control (RSC-002). Provide it through a Nix build sandbox, or through empty `CARGO_HOME` and `RUSTUP_HOME` with the network disabled.
- Keep the advisory, license, source, and duplicate control (RSC-003). Run it as a Devenv task that calls `cargo-deny` against the pinned offline advisory database and registry index.
- Keep the closure-size control (RSC-006). Run it as a Devenv task that reports `nix path-info` sizes.

## Impact

- Delete `tools/check_supply_chain.rs`, `assurance/supply-chain/v1/closure-inventory.tsv`, `assurance/supply-chain/v1/repository-inputs.tsv`, `assurance/supply-chain/v1/update-bindings.tsv`, and `assurance/supply-chain/v1/non-cargo-inputs.tsv`.
- Replace the `buoy:supply-chain` script with small Devenv tasks for `cargo-deny`, the offline sandbox build, and the closure-size report.
- Update the Devenv workflow checker (`tools/check_devenv_task_graph.rs`) assertions that describe the supply-chain script and its runtime packages.
- Remove the RSC-004 requirement and scenario rows from `assurance/v1/scenario-requirement-executions.tsv` and `assurance/v1/obligation-ledger.tsv`. Regenerate the digest-bound evidence.
- Keep the pinned advisory database, the pinned registry index, and the `deny.toml` policy.
- No change to the semantic core, canonical identity, or the digest algorithm.
