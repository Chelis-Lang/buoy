## Requirement Actions

| Requirement ID | Target class | Action | Owner | Buoy command |
|---|---|---|---|---|
| `ASC-007` | code and test | revise | adapter owner | `devenv shell -- cargo test --test adapters` |
| `DET-009` | code and test | add | core owner | `devenv shell -- cargo test --workspace --all-features --all-targets` |
| `DET-011` | code and test | add | adapter owner | `devenv shell -- cargo test --test openspec_traceability` |
| `ESI-001` | code and test | revise | core owner | `devenv shell -- cargo test -p buoy-core` |
| `ESI-002` | code and test | revise | adapter owner | `devenv shell -- cargo test --test adapters` |
| `ESI-003` | code and test | revise | core owner | `devenv shell -- cargo test -p buoy-core` |
| `ESI-004` | schema and documentation | replace | schema owner | `devenv shell -- cargo run --bin check-schema-inventory` |
| `ESI-005` | code and test | revise | core owner | `devenv shell -- cargo test -p buoy-core` |
| `ESI-006` | code and test | revise | adapter owner | `devenv shell -- cargo test --test adapters` |
| `ETL-001` | code and test | add | core owner | `devenv shell -- cargo test --test openspec_traceability` |
| `ETL-002` | code and test | add | core owner | `devenv shell -- cargo test --test openspec_traceability` |
| `ETL-003` | code and fixture | add | adapter owner | `devenv shell -- cargo test --test openspec_traceability` |
| `ETL-004` | code and test | add | assurance owner | `devenv shell -- cargo test --test openspec_traceability` |
| `ETL-005` | code and test | add | assurance owner | `devenv shell -- cargo test --test openspec_traceability` |
| `ETL-006` | code and test | add | core owner | `devenv shell -- cargo test --test openspec_traceability` |
| `ETL-007` | code and test | add | report owner | `devenv shell -- cargo test --test openspec_traceability` |
| `OBW-001` | workflow | add | workflow owner | `devenv shell -- cargo test --test openspec_workflow` |
| `OBW-002` | workflow | add | workflow owner | `devenv shell -- cargo test --test openspec_workflow` |
| `OBW-003` | template | add | workflow owner | `devenv shell -- cargo test --test openspec_workflow` |
| `OBW-004` | guidance | add | workflow owner | `devenv shell -- cargo test --test openspec_workflow` |
| `OBW-005` | guidance | add | workflow owner | `devenv shell -- cargo test --test openspec_workflow` |
| `OBW-006` | code and test | add | workflow owner | `devenv shell -- cargo test --test openspec_workflow` |
| `OBW-007` | task and test | add | workflow owner | `devenv shell -- cargo test --test openspec_workflow` |
| `OSA-001` | code and test | add | adapter owner | `devenv shell -- cargo test --test openspec_traceability` |
| `OSA-002` | code and fixture | add | adapter owner | `devenv shell -- cargo test --test openspec_traceability` |
| `OSA-003` | code and test | add | parser owner | `devenv shell -- cargo test --test openspec_traceability` |
| `OSA-004` | code and test | add | parser owner | `devenv shell -- cargo test --test openspec_traceability` |
| `OSA-005` | code and test | add | configuration owner | `devenv shell -- cargo test --test openspec_traceability` |
| `OSA-006` | code and test | add | configuration owner | `devenv shell -- cargo test --test openspec_traceability` |
| `OSA-007` | code and test | add | adapter owner | `devenv shell -- cargo test --test adapters` |
| `OSA-008` | code and test | add | adapter owner | `devenv shell -- cargo test --test openspec_traceability` |
| `OSA-009` | code and test | add | adapter owner | `devenv shell -- cargo test --test openspec_traceability` |
| `PID-009` | code and test | add | product owner | `devenv shell -- cargo test --test integration` |
| `SPG-005` | code and test | revise | core owner | `devenv shell -- cargo test -p buoy-core` |
| `SPG-007` | code and test | revise | parser owner | `devenv shell -- cargo test --test openspec_traceability` |
| `SPG-011` | code and test | revise | adapter owner | `devenv shell -- cargo test --test adapters` |

## Link Review

The change adds new requirement links. It does not revise, retire, or waive an existing link.

## External Targets

The change does not add an external target to its own planning artifacts.
