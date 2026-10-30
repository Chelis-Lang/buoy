## 1. Controls before implementation

- [x] 1.1 Add a negative control that the offline-acceptance task fails when a developer Cargo cache supplies an undeclared dependency.
- [x] 1.2 Add a negative control that the offline-acceptance task fails when an ambient tool supplies a command absent from a script runtime package set.
- [x] 1.3 Add a positive control that `cargo-deny` passes offline against the pinned advisory database, the pinned registry index, and `deny.toml`.
- [x] 1.4 Add a negative control that a wildcard advisory exemption fails the policy.
- [x] 1.5 Add a control that the closure-size task reports deterministic component sizes.

## 2. Add devenv-native supply-chain tasks

- [x] 2.1 Add the `buoy-deny` script that runs `cargo-deny` offline against `deny.toml`, the pinned advisory database, and the pinned registry index.
- [x] 2.2 Add the `buoy:deny` task that runs `buoy-deny`.
- [x] 2.3 Add the `buoy-offline-acceptance` script that runs the final oracle with empty `CARGO_HOME` and `RUSTUP_HOME` and the network disabled.
- [x] 2.4 Add the `buoy:offline-acceptance` task that runs `buoy-offline-acceptance`.
- [x] 2.5 Add the `buoy-closure-size` script that reads `nix path-info -S` on the Devenv profile and reports component sizes.
- [x] 2.6 Add the `buoy:closure-size` task that runs `buoy-closure-size`.
- [x] 2.7 Bind each new script to its runtime packages in Devenv configuration.

## 3. Remove the bespoke checker and inventory

- [x] 3.1 Remove the `buoy:supply-chain` task and the `buoy-supply-chain` script.
- [x] 3.2 Delete `tools/check_supply_chain.rs` and its `Cargo.toml` binary entry.
- [x] 3.3 Delete `assurance/supply-chain/v1/closure-inventory.tsv`.
- [x] 3.4 Delete `assurance/supply-chain/v1/repository-inputs.tsv`, `update-bindings.tsv`, and `non-cargo-inputs.tsv`.

## 4. Reconcile the workflow checker and policy

- [x] 4.1 Update `tools/check_devenv_task_graph.rs` to assert the new supply-chain tasks and their runtime packages, and to drop the removed script.
- [x] 4.2 Update `nix/devenv/workflow-policy.toml` for the new task names and runtime packages.
- [x] 4.3 Remove the RSC-004 authority mapping from `tools/check_hardening_manifest.rs`.

## 5. Reconcile specs and assurance evidence

- [x] 5.1 Sync the `reproducible-supply-chain` delta: remove RSC-004 and revise RSC-005.
- [x] 5.2 Remove the RSC-004 requirement and scenario rows from `assurance/v1/scenario-requirement-executions.tsv` and `assurance/v1/obligation-ledger.tsv`.
- [x] 5.3 Revise the RSC-002, RSC-003, and RSC-006 command bindings for the new task names.
- [x] 5.4 Regenerate the digest-bound execution evidence and obligation ledger.
- [x] 5.5 Update `assurance/v1/quality-assurance-controls.toml` for the new tasks.
- [x] 5.6 Drop `tools/check_supply_chain.rs` from the exact coverage source inventory.

## 6. Documentation

- [x] 6.1 Update `docs/assurance/trust-boundaries.md` to state that `devenv.lock` and `Cargo.lock` hold the verified pins.
- [x] 6.2 Update the supply-chain and workflow documentation for the new tasks.

## 7. Acceptance

- [x] 7.1 Run `buoy:deny`, `buoy:offline-acceptance`, and `buoy:closure-size` green.
- [x] 7.2 Run `buoy-fixtures`, `buoy-obligations`, and `buoy-devenv` green.
- [x] 7.3 Confirm no `source_digest` drift outside the removed RSC-004 rows.

## Archive Tasks

- [x] Run `openspec validate --all --strict --no-interactive`.
- [x] Run `openspec schema validate buoy-bridge`.
- [x] Run the exact repository acceptance command `devenv test`.
