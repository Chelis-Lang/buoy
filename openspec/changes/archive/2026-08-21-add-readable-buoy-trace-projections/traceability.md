## Requirement Actions

| Requirement ID | Target class | Action | Owner | Buoy command |
|---|---|---|---|---|
| `RTP-001` | CLI parser and query shell | add | CLI owner | `devenv shell -- cargo test -p buoy --all-features --test readable_trace_projections` |
| `RTP-002` | core query and history model | add | core owner | `devenv shell -- cargo test -p buoy-core --all-features trace` |
| `RTP-003` | human renderer and golden output | add | report owner | `devenv shell -- cargo test -p buoy --all-features --test readable_trace_projections human` |
| `RTP-004` | machine schema, renderer, and golden output | add | report owner | `devenv shell -- cargo test -p buoy --all-features --test readable_trace_projections machine` |
| `RTP-005` | Markdown renderer, example, and derived-data warning | add | report owner | `devenv shell -- cargo test -p buoy --all-features --test readable_trace_projections markdown` |
| `RTP-006` | static effect boundary and negative controls | add | shell owner | `devenv shell -- cargo test -p buoy --all-features --test readable_trace_projections effect_boundary` |
| `CAB-006` | source traversal query | revise | core owner | `devenv shell -- cargo test -p buoy --all-features --test content_addressed_bindings` |
| `CAB-013` | successful-bind report schema and parity | add | report owner | `devenv shell -- cargo test -p buoy --all-features --test content_addressed_bindings binding_tree_report` |
| `OSA-005` | OpenSpec format parser and version fixtures | revise | adapter owner | `devenv shell -- cargo test -p buoy --all-features --test openspec_traceability format_v2` |
| `OSA-007` | OpenSpec primary partition and nested trace mappings | revise | adapter owner | `devenv shell -- cargo test -p buoy --all-features --test openspec_traceability partition` |
| `OSA-013` | OpenSpec trace-section parser and source mappings | add | adapter owner | `devenv shell -- cargo test -p buoy --all-features --test openspec_traceability trace_section` |
| `OSA-014` | OpenSpec identity controls and reference parity | add | adapter owner | `devenv shell -- cargo test -p buoy --all-features --test openspec_traceability trace_identity` |
| `ETL-003` | shared external trace parser and adapter parity | revise | adapter owner | `devenv shell -- cargo test -p buoy --all-features --test adapters external_trace` |
| `OBW-003` | traceability template, workflow control, and example | revise | workflow owner | `devenv shell -- cargo test -p buoy --all-features --test openspec_workflow` |
| `OBW-008` | completion snapshot template and negative control | add | workflow owner | `devenv shell -- cargo test -p buoy --all-features --test openspec_workflow` |

## Planned Implementation Relations

No production compact implementation relation is allocated by this planning artifact. Implementation can add reviewed relation rows only after exact target items exist.

| Requirement ID | Relation ID | Consumer | Logical target selector | Action | Owner | Focused Buoy command |
|---|---|---|---|---|---|---|


## Link Review

| Requirement ID | Current link | Disposition | Evidence |
|---|---|---|---|
| `RTP-001` | none | add | `src/cli.rs` and `tests/readable_trace_projections.rs` |
| `RTP-002` | none | add | `crates/buoy-core/src/` and focused core tests |
| `RTP-003` | none | add | `src/cli.rs` and human-output goldens |
| `RTP-004` | none | add | trace-report schema inventory and machine-output goldens |
| `RTP-005` | none | add | Markdown-output goldens and an implementation-owned example |
| `RTP-006` | none | add | effect-boundary and stale-binding controls |
| `CAB-006` | `crates/buoy-core/src/algorithms.rs` and `src/cli.rs` | revise | `tests/content_addressed_bindings.rs` |
| `CAB-013` | `src/cli.rs::BindingTreeReport` | add | frozen bind-report fixtures and fresh trace parity |
| `OSA-005` | `src/adapters/openspec.rs` | revise | OpenSpec configuration fixtures and `tests/openspec_traceability.rs` |
| `OSA-007` | `src/adapters/openspec.rs` | revise | partition controls with nested nonauthority trace mappings |
| `OSA-013` | none | add | version 2 positive and trace-in-authority negative fixtures |
| `OSA-014` | none | add | independent identity controls and frozen digests |
| `ETL-003` | `src/adapters/markdown.rs` | revise | shared parser differential tests in `tests/adapters.rs` |
| `OBW-003` | `openspec/schemas/buoy-bridge/templates/traceability.md` | revise | `tests/openspec_workflow.rs` and workflow examples |
| `OBW-008` | none | add | completion snapshot positive and stale-history negative controls |

## Completion Snapshot

This section is historical. A current Buoy query remains the current-state authority.

This change created no immutable implementation binding record. The completion table remains empty.

| Requirement ID | Canonical record locator | Consumer commit | Final static result |
|---|---|---|---|

## External Targets

This change adds no live provider target. Versioned fixtures use repository-owned example HTTPS values and URNs only.
