## 1. Freeze the spike boundary and fixtures

- [x] 1.1 Add one positive OpenSpec fixture with a document, requirement, two scenarios, and ordered steps.
- [x] 1.2 Add one positive Rust fixture with `#[buoy::implements("<digest>")]` on an exact function item.
- [x] 1.3 Add one canonical implementation-binding record at its digest-derived object-store path.
- [x] 1.4 Add negative fixture mutations for requirement text, scenario steps, Rust body tokens, attribute syntax, record content, and record path.
- [x] 1.5 Add a historical requirement record that remains stored but is absent from the current document root.
- [x] 1.6 Record the expected node, binding, tree-root, and graph digests under `blake3-256`.

## 2. Validate the model before production code

- [x] 2.1 Add a small independent reference model for source-node construction, closure, reachability, and canonical traversal.
- [x] 2.2 Add failing integration tests for compact resolution and traversal from the binding to Rust, requirements, scenarios, steps, and authority.
- [x] 2.3 Add failing mutation tests that require separate stale-requirement and stale-implementation diagnostics.
- [x] 2.4 Add a failing test that proves a binding-attribute replacement does not change the Rust subject identity.
- [x] 2.5 Add a failing separate-root test that requires equal canonical output.
- [x] 2.6 Add Hegel-rs properties for edge-order independence, fixed-point reachability, cycle rejection, and traversal bounds.
- [x] 2.7 Add an exhaustive bounded graph model for up to four nodes and compare it with the planned production closure algorithm.
- [x] 2.8 Run Verus and AeneasVerif feasibility spikes for the pure bounded traversal kernel. Record exclusions before any lane selection.
- [x] 2.9 Review the spike results. If one-hash bidirectional freshness fails, stop before production implementation and revise `design.md`.

## 3. Implement core source-reference values

- [x] 3.1 Add parsed `SourceNodeRef` and `SourceEdgeKind` types at the untrusted-input boundary.
- [x] 3.2 Add `SourceNode`, `SourceEdge`, `SourceGraph`, and canonical digest encodings to `buoy-core`.
- [x] 3.3 Add source-root closure, acyclicity, current reachability, and bounded traversal algorithms.
- [x] 3.4 Add source-node, edge, depth, and visited-node budget types with fail-closed errors.
- [x] 3.5 Add positive, negative, property, and bounded-model tests before each production algorithm.

## 4. Implement content-addressed binding values

- [x] 4.1 Add `ImplementationBinding` with a stable relation ID and exact source and atom references.
- [x] 4.2 Bind the implementation record digest to every field under `buoy/implementation-binding/v1`.
- [x] 4.3 Add validation for record closure, target kinds, authority correspondence, and current reachability.
- [x] 4.4 Add separate deterministic diagnostics for absent records, stale subjects, stale requirements, wrong kinds, mismatched authority, and invalid digest classes.
- [x] 4.5 Extend impact analysis through stable relation IDs and binding source edges.

## 5. Extend the adapter protocol and repository shell

- [x] 5.1 Retain source nodes, source mappings, current roots, and unresolved typed references in every adapter product.
- [x] 5.2 Aggregate all adapter source products before repository-level reference resolution.
- [x] 5.3 Use context-aware graph validation from the static, change, and execute CLI paths.
- [x] 5.4 Add `binding/v1` repository selection and strict object-store path parsing.
- [x] 5.5 Reject compact bindings unless the selected active member has the collision-resistant class.
- [x] 5.6 Preserve deterministic ordering and resource budgets during aggregate construction.

## 6. Extend the OpenSpec and Rust adapters

- [x] 6.1 Materialize OpenSpec document, requirement, scenario, and step source nodes from the pinned parser.
- [x] 6.2 Add ordered `contains` edges and exact requirement-to-atom authority edges.
- [x] 6.3 Prove with mutation tests that a child edit changes containing node identities but not sibling requirement identities.
- [x] 6.4 Parse exactly one digest from `#[buoy::implements("<digest>")]`.
- [x] 6.5 Remove recognized Buoy attributes from Rust subject tokens before subject identity construction.
- [x] 6.6 Emit unresolved binding references with exact Rust source mappings for aggregate resolution.
- [x] 6.7 Retire the verbose implementation attribute only after fixture and migration parity succeeds.

## 7. Add the binding command and traversal output

- [x] 7.1 Add explicit requirement and implementation selectors to `buoy bind`.
- [x] 7.2 Create one immutable record and replace one selected compact attribute without changing an old record.
- [x] 7.3 Make command errors leave all source and object-store files unchanged.
- [x] 7.4 Add machine output for canonical binding-tree traversal.
- [x] 7.5 Add human output that shows the stable relation ID, requirement ID, source paths, scenarios, and freshness facts.

## 8. Update documentation and deterministic artifacts

- [x] 8.1 Document the compact syntax, immutable object store, update process, and stale diagnostics.
- [x] 8.2 Document that fresh implementation links supply navigation only.
- [x] 8.3 Add deterministic golden outputs for valid, stale-requirement, stale-implementation, historical-node, cycle, and wrong-kind cases.
- [x] 8.4 Update schema inventories, migration data, assurance ledgers, and supply-chain records.
- [x] 8.5 Run every focused command from `traceability.md` and record no unresolved requirement action.

## 9. Archive tasks

- [x] 9.1 Run `openspec validate --all --strict --no-interactive`.
- [x] 9.2 Run `openspec schema validate buoy-bridge`.
- [x] 9.3 Run the exact repository acceptance command `devenv test`.
- [x] 9.4 Run the positive fixture twice under separate absolute roots and compare byte-identical machine output.
- [x] 9.5 Run each negative mutation and make sure that only its intended diagnostic causes rejection.
