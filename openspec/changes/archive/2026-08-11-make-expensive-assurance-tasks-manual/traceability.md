## Requirement Actions

| Requirement ID | Target class | Action | Owner | Buoy command |
|---|---|---|---|---|
| `DEV-009` | configuration | add the closed manual-task policy and detach coverage from P0 | task-graph owner | `devenv tasks run buoy:devenv` |
| `DEV-009` | code | reject lifecycle reachability for each manual task | task-graph owner | `devenv tasks run buoy:devenv` |
| `DEV-009` | fixture | add positive and negative manual-task graph cases | task-graph owner | `devenv tasks run buoy:devenv` |
| `LCOV-006` | configuration | move effectful coverage outside automatic phase execution | coverage owner | `devenv tasks run buoy:coverage` |
| `LCOV-006` | code | add a fast current-source and exact-metric evidence check | assurance owner | `devenv tasks run buoy:coverage-evidence` |
| `LCOV-006` | evidence | retain complete blocking validation in ledger closure | assurance owner | `devenv tasks run buoy:ledger-green` |
| `LCOV-006` | documentation | document manual execution and automatic evidence validation | documentation owner | `devenv tasks run buoy:specs` |
| `ADV-003` | configuration | declare the mutation campaign as a manual task | adversarial owner | `devenv tasks run buoy:devenv` |
| `ADV-003` | evidence | retain automatic mutation replay and artifact freshness checks | adversarial owner | `devenv tasks run buoy:mutation-replay` |
| `ADV-003` | documentation | distinguish campaign execution from deterministic replay | documentation owner | `devenv tasks run buoy:specs` |

## Link Review

| Requirement ID | Current link | Disposition | Evidence |
|---|---|---|---|
| `DEV-009` | none | add | `tools/check_devenv_task_graph.rs` |
| `LCOV-006` | `buoy:coverage` to P0 task edge | retire | `nix/devenv/tasks.nix` |
| `LCOV-006` | coverage source and result bindings | retain | `assurance/v1/source-inventory.tsv` and `assurance/v1/coverage-result.tsv` |
| `ADV-003` | `buoy:mutation-replay` to P1 task edge | retain | `nix/devenv/tasks.nix` |
| `ADV-003` | detached `buoy:mutation-campaign` task | revise | `nix/devenv/workflow-policy.toml` |

## External Targets

None.
