## Requirement Actions

| Requirement ID | Target class | Action | Owner | Buoy command |
|---|---|---|---|---|
| `LSM-001` | configuration code and fixture | add | configuration owner | `devenv shell -- cargo test --all-features --test logical_mounts explicit_mount_set` |
| `LSM-002` | snapshot code and golden | add | repository owner | `devenv shell -- cargo test --all-features --test logical_mounts physical_path_independence` |
| `LSM-003` | configuration code and property test | add | configuration owner | `devenv shell -- cargo test --all-features --test logical_mounts closed_namespace` |
| `LSM-004` | repository code and integration test | add | repository owner | `devenv shell -- cargo test --all-features --test logical_mounts joined_snapshot` |
| `LSM-005` | repository code and boundary fixture | add | repository owner | `devenv shell -- cargo test --all-features --test logical_mounts aggregate_budgets` |
| `RAS-001` | adapter code and fixture | add | adapter owner | `devenv shell -- cargo test --all-features --test artifact_bindings opaque_source_node` |
| `RAS-002` | adapter descriptor and canonical vector | add | adapter owner | `devenv shell -- cargo test --all-features --test artifact_bindings fixed_opaque_interpretation` |
| `RAS-003` | source mapping and negative control | add | adapter owner | `devenv shell -- cargo test --all-features --test artifact_bindings complete_byte_mapping` |
| `RAS-004` | configuration code and fixture | add | configuration owner | `devenv shell -- cargo test --all-features --test artifact_bindings forbidden_authority` |
| `CAB-009` | core value, adapter code, and fixture | add | binding owner | `devenv shell -- cargo test --all-features --test artifact_bindings sidecar_attachment` |
| `CAB-010` | repository aggregation and history fixture | add | binding owner | `devenv shell -- cargo test --all-features --test artifact_bindings current_sidecar` |
| `CAB-011` | CLI transaction and failure controls | add | CLI owner | `devenv shell -- cargo test --all-features --test artifact_bindings bind_artifact_transaction` |
| `CAB-012` | freshness checks and mutation control | add | binding owner | `devenv shell -- cargo test --all-features --test artifact_bindings artifact_freshness` |
| `SRG-006` | source graph aggregation and integration test | add | graph owner | `devenv shell -- cargo test --all-features --test logical_mounts cross_mount_graph` |
| `SRG-007` | canonical ordering and property test | add | graph owner | `devenv shell -- cargo test --all-features --test logical_mounts mount_order_identity` |
| `ESI-009` | adapter registry and SDK control | add | adapter owner | `devenv shell -- cargo test --all-features --test adapters raw_artifact_registry` |
| `REH-009` | descriptor snapshot code and race control | add | repository owner | `devenv shell -- cargo test --all-features --test integration mount_descriptor_reads` |
| `REH-010` | global budget accounting and failure control | add | repository owner | `devenv shell -- cargo test --all-features --test integration mount_budget_failure` |
| `REH-011` | oracle configuration and isolation test | add | execution owner | `devenv shell -- cargo test --all-features --test integration oracle_mount_isolation` |

## Command Coverage

Every focused command uses the workspace lockfile and the repository toolchain. Each new integration test contains exact positive and negative requirement controls.

The implementation phase must also run these commands:

- `devenv shell -- cargo test --workspace --all-targets --all-features --locked --offline`
- `devenv shell -- cargo clippy --workspace --all-targets --all-features --locked --offline -- -D warnings`
- `devenv tasks run buoy:schema`
- `devenv tasks run buoy:fixtures`
- `devenv tasks run buoy:properties`
- `devenv tasks run buoy:coverage`
- `openspec validate --all --strict --no-interactive`
- `devenv test`

The coverage command must report exactly 100% lines, regions, functions, and branches. The final acceptance oracle is `devenv test` with exit status zero.

## Link Review

All requirement IDs in this change are new. No current implementation, evidence, proof, or external trace link references these IDs.

Existing links for the modified capabilities remain unchanged. The implementation phase must search the graph and assurance ledgers again before archive.

Any new current link requires an exact retain, revise, retire, or waive disposition.

## External Targets

This change adds no external target. A separate Barnacle change will pin the released Buoy revision and configure the first mounted-store pilot.
