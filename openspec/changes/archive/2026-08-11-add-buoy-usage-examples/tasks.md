## Implementation Tasks

### 1. Freeze example controls before implementation

- [x] 1.1 Add a manifest for every positive and negative example case.
- [x] 1.2 Freeze stable diagnostic codes, exit classes, commands, and expected machine-output bytes.
- [x] 1.3 Add absent-file controls for the Rust examples and example index.
- [x] 1.4 Add malformed configuration and invalid graph controls for the Rust examples.
- [x] 1.5 Add reserved-ID and active-change-path controls for OpenSpec authority.
- [x] 1.6 Add closed-relation and stale-revision controls for external trace links.
- [x] 1.7 Add complete and incomplete disposition controls for the change example.
- [x] 1.8 Add structural controls for every required `buoy-traceability` template field.
- [x] 1.9 Add ready and blocked controls for the reusable schema artifact graph.
- [x] 1.10 Add controls that reject Buoy-internal rules and provider-state fields.

### 2. Add compiled Rust examples

- [x] 2.1 Add `examples/core_graph.rs` with typed constructors and deterministic graph diagnostics.
- [x] 2.2 Add the adjacent invalid-relation path required by `BUE-002`.
- [x] 2.3 Add `examples/repository_check.rs` with configuration parsing at the input boundary.
- [x] 2.4 Add saved repository snapshots, static checks, and exact machine report output.
- [x] 2.5 Compile both examples under all workspace lints and enabled features.

### 3. Add complete command-line repositories

- [x] 3.1 Add `examples/repositories/openspec-authority` with explicit authority configuration.
- [x] 3.2 Add one reserved requirement with positive and negative scenarios.
- [x] 3.3 Add trace blocks for `tracks`, `discusses`, `documents`, and `reviews`.
- [x] 3.4 Add `examples/repositories/stale-external-link` with the reviewed stale revision.
- [x] 3.5 Add `examples/repositories/change-check` with exact base and head roots.
- [x] 3.6 Add complete and incomplete materialized change manifests.
- [x] 3.7 Add deterministic machine-output goldens for each command example.
- [x] 3.8 Add a control that recomputes all committed graph digest bindings.

### 4. Add the reusable OpenSpec project

- [x] 4.1 Add `examples/openspec-workflow/openspec/config.yaml` with `schema: buoy-traceability`.
- [x] 4.2 Add the project-local `buoy-traceability` schema and artifact dependencies.
- [x] 4.3 Add generic proposal, specification, design, traceability, and task templates.
- [x] 4.4 Require stable requirement IDs and explicit positive and negative scenarios.
- [x] 4.5 Require complete requirement actions and current-link review in `traceability.md`.
- [x] 4.6 State that repository configuration owns lifecycle, reserved IDs, scope, atom kind, and approvals.
- [x] 4.7 State that active OpenSpec changes are noncanonical plans.
- [x] 4.8 Verify schema selection and the complete artifact flow in a temporary example change.

### 5. Add documentation and repository gates

- [x] 5.1 Add `examples/README.md` with prerequisites, commands, expected results, and assurance limits.
- [x] 5.2 Link the example index from `README.md`.
- [x] 5.3 Document synchronized candidate head roots for OpenSpec change checks.
- [x] 5.4 Document the separation of static facts, oracle receipts, and assurance classes.
- [x] 5.5 Add `buoy:examples` to the resolved devenv task graph.
- [x] 5.6 Add example sources to exact source and coverage inventories where required.
- [x] 5.7 Run every focused command from `traceability.md`.
- [x] 5.8 Run `devenv tasks run buoy:examples` and require exit code `0`.
- [x] 5.9 Run `devenv tasks run buoy:devenv` and verify the final task dependency.
- [x] 5.10 Detach `buoy:coverage-evidence` from every automatic task and phase.
- [x] 5.11 Detach `buoy:mutation-replay` from every automatic task and phase.
- [x] 5.12 Keep manual coverage rows selectable without blocking the automatic ledger.

## Archive Tasks

- [ ] 6.1 Run `openspec validate --all --strict --no-interactive`.
- [ ] 6.2 Run `openspec schema validate buoy-bridge`.
- [ ] 6.3 Run the exact repository acceptance command `devenv test`.
