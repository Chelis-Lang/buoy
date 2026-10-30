## Requirement Actions

| Requirement ID | Target class | Action | Owner | Buoy command |
|---|---|---|---|---|
| `SRG-001` | core code and test | add | core owner | `devenv shell -- cargo test -p buoy-core source_node` |
| `SRG-002` | core code and test | add | core owner | `devenv shell -- cargo test -p buoy-core source_edge` |
| `SRG-003` | core code and test | add | core owner | `devenv shell -- cargo test -p buoy-core source_reachability` |
| `SRG-004` | core code and property test | add | core owner | `devenv shell -- cargo test -p buoy-core source_traversal` |
| `SRG-005` | shell code and integration test | add | shell owner | `devenv shell -- cargo test --all-features --test content_addressed_bindings aggregate` |
| `CAB-001` | core code and test | add | core owner | `devenv shell -- cargo test -p buoy-core implementation_binding` |
| `CAB-002` | adapter code and fixture | add | Rust adapter owner | `devenv shell -- cargo test --all-features --test content_addressed_bindings compact_attribute` |
| `CAB-003` | adapter code and test | add | Rust adapter owner | `devenv shell -- cargo test --all-features --test content_addressed_bindings subject_identity` |
| `CAB-004` | repository code and fixture | add | repository owner | `devenv shell -- cargo test --all-features --test content_addressed_bindings object_store` |
| `CAB-005` | shell code and mutation test | add | shell owner | `devenv shell -- cargo test --all-features --test content_addressed_bindings binding_freshness` |
| `CAB-006` | report code and test | add | report owner | `devenv shell -- cargo test --all-features --test content_addressed_bindings traversal` |
| `CAB-007` | configuration code and test | add | configuration owner | `devenv shell -- cargo test --all-features --test content_addressed_bindings digest_class` |
| `CAB-008` | command code and test | add | CLI owner | `devenv shell -- cargo test --all-features --test content_addressed_bindings bind_command` |
| `OSA-010` | adapter code and fixture | add | OpenSpec adapter owner | `devenv shell -- cargo test --test openspec_traceability source_tree` |
| `OSA-011` | adapter code and test | add | OpenSpec adapter owner | `devenv shell -- cargo test --test openspec_traceability atom_correspondence` |
| `OSA-012` | adapter code and mutation test | add | OpenSpec adapter owner | `devenv shell -- cargo test --test openspec_traceability child_propagation` |
| `ESI-007` | adapter protocol and test | add | adapter owner | `devenv shell -- cargo test --all-features --test adapters retained_source_nodes` |
| `ESI-008` | adapter protocol and test | add | adapter owner | `devenv shell -- cargo test --all-features --test adapters unresolved_reference` |
| `SPG-012` | core graph and test | add | core owner | `devenv shell -- cargo test -p buoy-core implementation_binding` |
| `SPG-013` | impact code and test | add | core owner | `devenv shell -- cargo test -p buoy-core binding_impact` |

## Command Coverage

Each command above runs under the exact acceptance oracle, which builds every
feature. A compact locator needs the collision-resistant class, so every
`content_addressed_bindings` command names `--all-features`; the default
`xxh3-128` build contains no collision-resistant member and compiles those
contracts out.

Through task 6, all commands run tests except `CAB-008`. Task 7 owns the
`buoy bind` command for that requirement.

Task 5 defines the retained source-product protocol and repository aggregation.
Task 6 supplies OpenSpec nodes, roots, authorities, Rust subjects, and compact
attachments. Repository scans now report freshness for complete bindings.

The verbose implementation-attribute reader remains for frozen v1
compatibility. Compact and verbose implementation attributes cannot occur on
the same item. The adapter removes both forms from source-node identity.

## Graph-Validation Lanes

Task 5.3 runs context-aware graph validation from the one shared check path that
the static, change, and execute commands use. The repository declares only the
lanes it has an authority source for.

Declared lanes: source-map currency and original-byte extents, from the exact
adapter source mappings and the snapshot file lengths; and parsed-item closure
and staleness, from every parsed item the selected adapters retained.

Undeclared lanes: stable-ID registry allocation, approval authority, approval
permission, approval artifact currency, and host-schema registration. A
repository snapshot carries none of those authorities, so running them would
report an absent authority as a failed one.

The parsed-item lane reports `SCF-SOURCE-MAP-ENCLOSING` when a retained item
names an enclosing item that the same snapshot does not retain. The Rust adapter
retains only items that carry a provenance attribute, so an attribute inside an
inline module reports this fact today. Task 6 must retain every enclosing item
so that the retained snapshot is closed under enclosure.

## Link Review

All requirement IDs in this change are new. No current implementation, evidence, proof, or external trace link references these IDs.

The implementation phase must search the graph and assurance ledgers again before archive. Any new current link requires an exact disposition.

## External Targets

The change adds no external target. The OpenSpec and Rust nodes remain repository-owned source references inside one materialized snapshot.
