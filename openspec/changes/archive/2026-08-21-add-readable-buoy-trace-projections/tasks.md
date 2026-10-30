## 1. Establish the baseline and identities

- [x] 1.1 Verify the clean baseline with the focused binding, adapter, workflow, and example tests.
- [x] 1.2 Verify that `RTP-001` through `RTP-006` are unique across standing and delta specifications.
- [x] 1.3 Add the readable trace projection capability to the schema and naming inventories.
- [x] 1.4 Freeze the version 1 OpenSpec fixture identities before parser changes.
- [x] 1.5 Confirm that no production compact relation is required by `traceability.md`.

## 2. Add fixtures before production code

- [x] 2.1 Add positive CLI fixtures for each typed trace selector.
- [x] 2.2 Add negative CLI fixtures for absent, conflicting, malformed, and unresolved selectors.
- [x] 2.3 Add current, historical, stale-requirement, and stale-implementation relation fixtures.
- [x] 2.4 Add human trace goldens with readable paths, spans, freshness, and assurance text.
- [x] 2.5 Add machine trace goldens with complete digests and no physical roots.
- [x] 2.6 Add Markdown trace goldens with stable headings, links, and derived-data warnings.
- [x] 2.7 Add digest-prefix collision fixtures that require complete human digest output.
- [x] 2.8 Add OpenSpec version 2 fixtures for safe trace sections and exact source mappings.
- [x] 2.9 Add negative OpenSpec fixtures for trace fences inside requirement spans.
- [x] 2.10 Add cross-adapter fixtures with equal valid and malformed external trace fences.
- [x] 2.11 Add workflow fixtures for planned relation rows and historical completion snapshots.
- [x] 2.12 Confirm that every new negative fixture fails before production implementation.

## 3. Add independent models and properties

- [x] 3.1 Add an independent finite model for current and historical record classification.
- [x] 3.2 Add Hegel-rs properties for trace order, root independence, and selector permutation.
- [x] 3.3 Add an exhaustive digest-prefix model for human abbreviation collisions.
- [x] 3.4 Add independent OpenSpec identity checks for trace-section edits and requirement edits.
- [x] 3.5 Record that this change selects no Verus or AeneasVerif proof lane.

## 4. Implement the functional-core query

- [x] 4.1 Add closed trace selector and trace projection types to `buoy-core`.
- [x] 4.2 Implement current attachment selection without file or adapter dependencies.
- [x] 4.3 Implement explicit historical record selection without current-state promotion.
- [x] 4.4 Implement canonical trace row ordering and duplicate rejection.
- [x] 4.5 Run the core model, property, and focused unit tests.

## 5. Implement the CLI and trace views

- [x] 5.1 Add `buoy trace` usage text and boundary parsing for every selector.
- [x] 5.2 Add command-specific format parsing while existing commands retain their format set.
- [x] 5.3 Map core references to logical paths, parser names, and exact source spans.
- [x] 5.4 Add one shared internal row builder for bind and trace reports.
- [x] 5.5 Add the versioned `buoy/trace-report/v1` machine structure.
- [x] 5.6 Implement deterministic machine JSON with complete identities.
- [x] 5.7 Implement human output with collision-safe abbreviated digests.
- [x] 5.8 Implement Markdown output on standard output only.
- [x] 5.9 Emit projections before selected-current stale exits.
- [x] 5.10 Keep historical-only query results successful.
- [x] 5.11 Verify fresh single-link parity with `buoy/binding-tree-report/v1`.
- [x] 5.12 Run all readable trace projection tests and goldens.

## 6. Implement OpenSpec version 2 trace sections

- [x] 6.1 Add explicit `openspec/standing-spec/v2` configuration parsing.
- [x] 6.2 Retain the version 1 source-item schema for equal authority nodes under both formats.
- [x] 6.3 Move external trace document parsing into one shared adapter helper.
- [x] 6.4 Parse version 2 trace fences only after requirement boundary construction.
- [x] 6.5 Reject every recognized trace fence that intersects a requirement span.
- [x] 6.6 Nest trace source mappings only inside primary nonnormative spans.
- [x] 6.7 Add external links and mappings to `OpenSpecOutput` and shell materialization.
- [x] 6.8 Prove cross-version authority identity with frozen fixtures and independent checks.
- [x] 6.9 Run the OpenSpec, adapter, external-link, and source-graph tests.

## 7. Update the OpenSpec workflow and examples

- [x] 7.1 Expand the `traceability.md` template with stable relation and target fields.
- [x] 7.2 Add the historical completion snapshot and its current-state warning.
- [x] 7.3 Reject fabricated completion digests and forbidden provider fields in workflow controls.
- [x] 7.4 Update the workflow example with planned and completed trace rows.
- [x] 7.5 Add an implementation-owned Markdown trace page example.
- [x] 7.6 Update CLI, OpenSpec, binding, migration, and assurance documentation.
- [x] 7.7 Update deterministic schema, report, fixture, naming, and hardening inventories.

## 8. Run focused validation

- [x] 8.1 Run `devenv shell -- cargo test -p buoy-core --all-features trace`.
- [x] 8.2 Run `devenv shell -- cargo test -p buoy --all-features --test readable_trace_projections`.
- [x] 8.3 Run `devenv shell -- cargo test -p buoy --all-features --test content_addressed_bindings`.
- [x] 8.4 Run `devenv shell -- cargo test -p buoy --all-features --test openspec_traceability`.
- [x] 8.5 Run `devenv shell -- cargo test -p buoy --all-features --test adapters`.
- [x] 8.6 Run `devenv shell -- cargo test -p buoy --all-features --test openspec_workflow`.
- [x] 8.7 Run `devenv tasks run buoy:examples`.
- [x] 8.8 Verify equal trace output under separate physical roots and input orders.
- [x] 8.9 Verify that trace commands make no repository, provider, or oracle change.

## 9. Complete acceptance and archive preparation

- [x] 9.1 Update the completion snapshot only for immutable records created during implementation.
- [x] 9.2 Run `openspec validate --all --strict --no-interactive`.
- [x] 9.3 Run `openspec schema validate buoy-bridge`.
- [x] 9.4 Run `cargo fmt --all -- --check` through the pinned Devenv shell.
- [x] 9.5 Run the exact acceptance oracle `devenv test` and require exit code `0`.
- [x] 9.6 Verify that every task is complete before archive synchronization.
