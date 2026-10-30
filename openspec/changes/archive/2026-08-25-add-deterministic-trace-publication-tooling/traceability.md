## Requirement Actions

| Requirement ID | Target class | Action | Owner | Buoy command |
|---|---|---|---|---|
| `TPT-001` | schema and parser | add publication-plan boundary | Buoy shell | `devenv shell -- cargo test --locked --test trace_publication_contract` |
| `TPT-002` | CLI | add publication command family | Buoy shell | `devenv shell -- cargo test --locked --test trace_publication_contract` |
| `TPT-003` | conformance | add typed row-local checks | Buoy shell | `devenv shell -- cargo test --locked --test trace_publication_contract` |
| `TPT-004` | snapshot | add selector-derived retained snapshot | Buoy shell | `devenv shell -- cargo test --locked --test trace_publication_contract` |
| `TPT-005` | page output | add parity and generation lifecycles | Buoy shell | `devenv shell -- cargo test --locked --test trace_publication_contract` |
| `TPT-006` | OpenSpec edit | add nonauthority link synchronization | Buoy shell | `devenv shell -- cargo test --locked --test external_link_publication` |
| `TPT-007` | dependency policy | add output and target cycle checks | Buoy shell and core model | `devenv shell -- cargo test --locked --test trace_publication_contract` |
| `TPT-008` | evidence | add machine and Markdown evidence | Buoy shell | `devenv shell -- cargo test --locked --test publication_evidence` |
| `TPT-009` | diagnostics | add canonical publication diagnostics | Buoy shell | `devenv shell -- cargo test --locked --test trace_publication_contract` |
| `TPT-010` | trust boundary | enforce generic ownership and assurance | Buoy shell | `devenv shell -- cargo test --locked --test publication_evidence` |
| `TPT-011` | report schema | add operation report | Buoy shell | `devenv shell -- cargo test --locked --test trace_publication_contract` |
| `PBD-001` | Nix package | add Buoy-owned package function | Buoy Nix | `devenv tasks run buoy:package-contract` |
| `PBD-002` | package variants | add explicit digest-feature variants | Buoy Nix | `devenv tasks run buoy:package-contract` |
| `PBD-003` | package identity | add machine version report | Buoy shell and Nix | `devenv shell -- cargo test --locked --test package_identity` |
| `PBD-004` | offline package | remove runtime source discovery | Buoy Nix | `devenv tasks run buoy:package-contract` |
| `PBD-005` | package checks | add blocking local acceptance and advisory private integration | Buoy Nix and workflow | `devenv tasks run buoy:package-contract` |
| `RTP-007` | Markdown projection | add publication-complete identities | Buoy shell | `devenv shell -- cargo test --locked --test readable_trace_projections` |
| `ETL-008` | external link | add current-revision generation | Buoy adapter and shell | `devenv shell -- cargo test --locked --test external_link_publication` |
| `ETL-009` | external target | add basis-keyed target policy | Buoy shell | `devenv shell -- cargo test --locked --test external_link_publication` |
| `REH-012` | filesystem | add no-follow output writer | Buoy shell | `devenv shell -- cargo test --locked --test publication_filesystem` |
| `REH-013` | concurrency | add output locking and cleanup | Buoy shell | `devenv shell -- cargo test --locked --test publication_filesystem` |
| `OBW-008` | completion evidence | replace manual identities with generated evidence | Buoy workflow | `devenv shell -- cargo test --locked --test openspec_workflow` |
| `OBW-009` | planning policy | exclude volatile planned identities | Buoy workflow | `devenv shell -- cargo test --locked --test openspec_workflow` |
| `OBW-010` | operation ownership | direct consumers to Buoy commands | Buoy workflow | `devenv shell -- cargo test --locked --test openspec_workflow` |
| `CAB-002` | Rust adapter | require algorithm-qualified locator syntax | Buoy Rust adapter | `devenv shell -- cargo test --locked --test content_addressed_bindings` |
| `CAB-007` | binding policy | rename and align collision-resistant locator meaning | Buoy core and shell | `devenv shell -- cargo test --locked --test content_addressed_bindings` |
| `DEV-010` | task graph | keep downstream publication and package tasks advisory | Buoy workflow | `devenv shell -- cargo test --locked --test task_graph_contract` |

## Planned Implementation Relations

This change plans no compact implementation relation.

| Requirement ID | Relation ID | Consumer | Logical target selector | Action | Owner | Focused Buoy command |
|---|---|---|---|---|---|---|

## Link Review

No current external trace link selects an affected standing requirement.

| Requirement ID | Current link | Disposition | Evidence |
|---|---|---|---|
| `TPT-001` | none | add | publication-plan fixtures and `trace_publication_contract` |
| `TPT-002` | none | add | CLI effect-boundary contracts |
| `TPT-003` | none | add | typed row-local conformance fixtures |
| `TPT-004` | none | add | retained-snapshot and source-edit prerequisite controls |
| `TPT-005` | none | add | page lifecycle and exact-parity controls |
| `TPT-006` | none | add | OpenSpec publication-slot edit controls |
| `TPT-007` | none | add | publication-basis and dependency models |
| `TPT-008` | none | add | machine and Markdown evidence contracts |
| `TPT-009` | none | add | deterministic diagnostic goldens |
| `TPT-010` | none | add | ownership and assurance-boundary controls |
| `TPT-011` | none | add | publication-operation report contracts |
| `PBD-001` | none | add | Nix package interface checks |
| `PBD-002` | none | add | explicit package-variant checks |
| `PBD-003` | none | add | package-identity schema contracts |
| `PBD-004` | none | add | offline command matrix |
| `PBD-005` | none | add | package task-graph and isolation controls |
| `RTP-007` | none | add | Markdown renderer contract goldens |
| `ETL-008` | none | add | generated external-link controls |
| `ETL-009` | none | add | basis-keyed target controls |
| `REH-012` | none | add | no-follow publication filesystem controls |
| `REH-013` | none | add | publication locking and cleanup controls |
| `OBW-008` | none | revise | generated completion evidence controls |
| `OBW-009` | none | add | stable planning-policy controls |
| `OBW-010` | none | add | mechanical-operation ownership controls |
| `CAB-002` | none | revise | compact-locator adapter controls |
| `CAB-007` | none | rename and revise | collision-resistant locator controls |
| `DEV-010` | none | add | downstream task-graph controls |

## Completion Snapshot

This section is historical. Buoy remains the current-state authority. Run a new query to read current state.

No link action is complete.

<!-- BEGIN buoy-trace-publication-evidence-v1 -->
<!-- END buoy-trace-publication-evidence-v1 -->

| Owner | Consumer commit | Review decision |
|---|---|---|

## External Targets

This change records no concrete external target. Consumer plans own normalized target bases and rendered repository-owned page paths.

Buoy MUST NOT fetch a rendered target during static, trace, publication, evidence, or acceptance operations.
