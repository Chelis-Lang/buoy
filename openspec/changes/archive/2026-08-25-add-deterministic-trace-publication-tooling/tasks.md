## 1. Freeze publication contracts before implementation

- [x] 1.1 Add positive and negative fixtures for every `buoy/trace-publication-plan/v1` field, reference, and bound.
- [x] 1.2 Freeze the `buoy/trace-publication-report/v1` machine contract and malformed vectors.
- [x] 1.3 Freeze `buoy/readable-trace-markdown/v1` and the machine and Markdown evidence contracts.
- [x] 1.4 Freeze the `buoy/package-identity/v1` machine contract and malformed vectors.
- [x] 1.5 Register publication-basis and page-content digest domains with independent encoding vectors.
- [x] 1.6 Add row-conformance fixtures with expected fields split across separate rows.
- [x] 1.7 Add page fixtures for absent, equal, stale, immutable-conflict, and failed-generation states.
- [x] 1.8 Add external-link fixtures for absent slots, canonical slots, migratable legacy fences, mixed legacy sections, duplicates, malformed fences, and authority embedding.
- [x] 1.9 Add path and target-base fixtures for basis, atom, record, wrong-class, traversal, query, graph, page, link, commit, provider, and unknown inputs.
- [x] 1.10 Add path fixtures for regular derived outputs, selected derived outputs, controlled source edits, escapes, links, replacements, and collisions.
- [x] 1.11 Add package fixtures for each digest feature, missing source identity, and ambient-tool leakage.
- [x] 1.12 Add compact-locator fixtures for qualified, raw, wrong-member, change-detection, and extra-field forms.
- [x] 1.13 Add OpenSpec completion fixtures for generated evidence, empty state, conflict, and false current-authority claims.
- [x] 1.14 Add `fixtures/trace-publication/v1/manifest.tsv` with requirement, polarity, command, exit class, and diagnostic fields.
- [x] 1.15 Make its checker fail closed on an omitted TPT, PBD, RTP, ETL, REH, OBW, CAB, or DEV scenario.
- [x] 1.16 Add every REH-012 and REH-013 scenario to the hardening manifest with independent positive and negative oracles.
- [x] 1.17 Extend the hardening manifest checker and self-test through REH-013.

## 2. Add independent models and properties

- [x] 2.1 Implement bounded model `publication-transaction/v1` before the production transaction code.
- [x] 2.2 Cover parse, snapshot, conformance, candidate, guard, write, failure, cancellation, and cleanup states.
- [x] 2.3 Implement bounded model `publication-dependency/v1` before the path and URI validators.
- [x] 2.4 Prove bounded rejection of graph, page, link, page-commit, provider, and physical-path dependencies.
- [x] 2.5 Prove equal normalized basis inputs for absent, canonical, stale, and migratable legacy slots.
- [x] 2.6 Add Hegel-rs properties for plan permutation, root independence, row locality, and deterministic diagnostics.
- [x] 2.7 Add Hegel-rs properties for immutable lifecycle, source guarding, basis completeness, URI joining, and derived-output exclusion.
- [x] 2.8 Add differential tests between production and independent plan, path, URI, report, and evidence encoders.
- [x] 2.9 Run a proof-impact spike before any exported `buoy-core` API change.
- [x] 2.10 Record no new formal lane when implementation remains in the effectful shell.

## 3. Implement typed publication values and parsing

- [x] 3.1 Add closed plan, selector, source-set, output, lifecycle, page-reference, path-template, target-base, and policy types.
- [x] 3.2 Parse every untrusted plan value once at the CLI boundary.
- [x] 3.3 Enforce plan counts and byte budgets before retained allocation.
- [x] 3.4 Implement registered path substitutions for publication-basis, semantic-atom, and binding-record identities.
- [x] 3.5 Reject every unregistered or post-link target dependency during plan parsing.
- [x] 3.6 Add typed publication operation reports with canonical ordering.
- [x] 3.7 Add typed publication evidence with canonical machine and Markdown renderers.
- [x] 3.8 Add collision-resistant publication-basis and page-content digest encoders with independent parity checks.
- [x] 3.9 Add strict package-identity values and machine rendering.

## 4. Implement row conformance and readable projection updates

- [x] 4.1 Expose one internal typed trace report to the publication module.
- [x] 4.2 Enforce exact expected row cardinality before field checks.
- [x] 4.3 Enforce relation, record, atom, node, path, state, freshness, and diagnostics on each same row.
- [x] 4.4 Preserve stale selected rows, exact diagnostics, and exit `2`.
- [x] 4.5 Label the Markdown renderer schema and source-graph value.
- [x] 4.6 Add complete atom revision, record digest, logical locations, state, and freshness to Markdown.
- [x] 4.7 Retain the derived-data warning and navigation-only boundary.
- [x] 4.8 Prove that `buoy/trace-report/v1` and `buoy/binding-tree-report/v1` bytes remain compatible.

## 5. Implement the publication command family

- [x] 5.1 Add `buoy publication check` with one artifact selector and machine output.
- [x] 5.2 Add `buoy publication generate` with one required artifact selector.
- [x] 5.3 Add `buoy publication evidence` with one machine evidence value.
- [x] 5.4 Reject unknown operations, conflicting selectors, and undeclared artifacts before materialization.
- [x] 5.5 Ignore executable-selection environment variables in every publication operation.
- [x] 5.6 Keep `buoy trace` read-only and preserve all existing exit classes.
- [x] 5.7 Add usage, input, static-blocker, and successful-operation CLI contract tests.

## 6. Implement retained snapshots and source guards

- [x] 6.1 Derive the complete publication snapshot from parsed selectors and explicit logical-mount inputs.
- [x] 6.2 Reuse descriptor-relative no-follow reads for every selected path.
- [x] 6.3 Retain one immutable snapshot for conformance, rendering, and candidate construction.
- [x] 6.4 Require each declared source edit to equal its candidate before any derived-artifact operation.
- [x] 6.5 Return exit `2` before rendering when a source edit is absent or stale.
- [x] 6.6 Reopen and compare selected paths before each output replacement.
- [x] 6.7 Reject changed, removed, replaced, link-like, or newly selected source paths.
- [x] 6.8 Add cross-root, ambient-root, traversal-order, locale, timezone, and caller-directory equality tests.
- [x] 6.9 Add a recursive-selector fixture that detects one additional path against an exact source-set policy.

## 7. Implement safe page checks and writes

- [x] 7.1 Open each output root through one retained directory descriptor.
- [x] 7.2 Descend every output parent through no-follow operations.
- [x] 7.3 Reject selected derived outputs while permitting one semantically guarded selected source edit.
- [x] 7.4 Create unique regular sibling candidates through exclusive descriptor-relative operations.
- [x] 7.5 Implement synchronized create-if-absent behavior for `immutable` outputs.
- [x] 7.6 Implement synchronized same-directory replacement for `replaceable` outputs.
- [x] 7.7 Add normalized output-path locks with bounded wait behavior.
- [x] 7.8 Remove only temporary artifacts owned by the failed operation.
- [x] 7.9 Emit fixed-label deterministic parity details without physical paths or timestamps.
- [x] 7.10 Add parent-symlink, component-swap, interrupted-write, cancellation, and concurrent-writer tests.
- [x] 7.11 Add immutable create, equal no-op, conflict, and prior-output-preservation tests.

## 8. Implement cycle-safe external-link generation

- [x] 8.1 Retain normative boundaries, link fence spans, section spans, and owned separator spans.
- [x] 8.2 Resolve the selected current atom before external-link closure checking.
- [x] 8.3 Find zero or one publication slot and the fixed boundary before other nonauthority sections.
- [x] 8.4 Remove any present slot and insert equal canonical marker bytes at that boundary.
- [x] 8.5 Omit only the normalized link's diagnostics during pre-link conformance.
- [x] 8.6 Compute the complete publication basis from the normalized saved snapshot.
- [x] 8.7 Render the basis-keyed page path and join it with the normalized target base.
- [x] 8.8 Require the publication-basis path encoding in every immutable target.
- [x] 8.9 Restrict version 1 publication-managed external links to relation `documents`.
- [x] 8.10 Render one canonical standalone heading and `buoy-external-trace-v1` fence.
- [x] 8.11 Replace or insert the complete publication slot and preserve all other bytes.
- [x] 8.12 Reject migration of a legacy section that contains unrelated nonwhitespace bytes.
- [x] 8.13 Rematerialize candidate bytes through the normal OpenSpec version 2 adapter.
- [x] 8.14 Compare all normative spans, requirement nodes, atom IDs, and atom revisions before writing.
- [x] 8.15 Verify equal normalized snapshot bytes before and after slot generation.
- [x] 8.16 Run complete graph and semantic atom-revision closure checks on the candidate.
- [x] 8.17 Reject constant, atom-only, graph, page, link, provider, and commit keys for immutable targets.
- [x] 8.18 Add integration fixtures that save the link candidate before page generation and integrate the page first.
- [x] 8.19 Add controls that reject atom-object digests and page-commit cycles.

## 9. Implement generated completion evidence

- [x] 9.1 Require every non-evidence artifact to pass parity before a completion payload renders.
- [x] 9.2 Render exact publication identities from the retained typed report.
- [x] 9.3 Exclude record bodies, provider fields, commits, users, approvals, clocks, and physical paths.
- [x] 9.4 Mark generated evidence as historical data whose recorded freshness creates no assurance claim.
- [x] 9.5 Define marker-delimited generated evidence and separate human completion blocks.
- [x] 9.6 Update the `buoy-bridge` template and `traceability_control.rs` for both blocks.
- [x] 9.7 Retain read-only parser compatibility with archived legacy four-column completion tables.
- [x] 9.8 Update workflow tests, malformed fixtures, and template assertions for both formats.
- [x] 9.9 Migrate `examples/openspec-workflow/completed-traceability.md` and `examples/openspec-workflow/openspec/schemas/buoy-traceability/templates/traceability.md`.
- [x] 9.10 Keep owners, commits, reviews, approvals, and task completion human-authored.
- [x] 9.11 Reject manual digest rows that conflict with generated evidence.
- [x] 9.12 Keep pre-implementation generated blocks empty without placeholder identities.
- [x] 9.13 Migrate this change's completion section to the generated format before final archive.

## 10. Implement the Buoy-owned consumer package

- [x] 10.1 Add the reviewed `nix/consumer-package-v1.nix` function with its closed input contract.
- [x] 10.2 Build the executable from committed source and `Cargo.lock` with no ambient Rust tools.
- [x] 10.3 Add explicit `xxh3-128`, `blake3-256`, and `sha-256` package variants.
- [x] 10.4 Disable default Cargo features for every explicit package variant.
- [x] 10.5 Require explicit source revision, source content, Rust toolchain, and feature inputs.
- [x] 10.6 Embed operational package identity through reproducible derivation inputs.
- [x] 10.7 Add read-only machine command `buoy package-identity`.
- [x] 10.8 Add offline local-source realization from the declared closure with empty Cargo and Rustup homes.
- [x] 10.9 Run static, change, bind, trace, and publication commands without network access after realization.
- [x] 10.10 Add an advisory private non-flake `git+ssh` fixture with temporary external credentials.
- [x] 10.11 Add blocking internal `buoy:package-contract` and advisory downstream `buoy:consumer-package` tasks.
- [x] 10.12 Bind only `buoy:package-contract` into the resolved `devenv test` graph.
- [x] 10.13 Add a gate-omission negative control for the blocking package task.
- [x] 10.14 Prove that the advisory task has no path to `devenv:enterTest`.
- [x] 10.15 Register the blocking package control with one new stable quality ID and exact ownership.
- [x] 10.16 Record closure size and noncanonical build cost separately.

## 11. Correct compact-locator contracts

- [x] 11.1 Update CAB-002 tests to require one algorithm-qualified canonical digest.
- [x] 11.2 Update CAB-007 tests and names to remove the hash-only syntax claim.
- [x] 11.3 Reject raw digest payloads and active-member mismatches with stable diagnostics.
- [x] 11.4 Update schema documents, examples, fixtures, source comments, and migration guidance.
- [x] 11.5 Remove the obsolete planned status from readable trace-projection documentation.

## 12. Run adversarial and regression gates

- [x] 12.1 Fuzz plan, path-template, target-base, operation-report, evidence, and package-identity parsers under explicit bounds.
- [x] 12.2 Run the publication transaction and dependency models to fixed points.
- [x] 12.3 Run all Hegel-rs publication properties with committed seeds and settings.
- [x] 12.4 Run mutation tests for every cycle, source guard, row check, and write-prevention branch.
- [x] 12.5 Prove that a substitute executable cannot generate or approve page bytes.
- [x] 12.6 Prove that no symlinked ancestor can redirect a publication write.
- [x] 12.7 Prove equal diagnostics across temporary roots, timestamps, locales, and operation order.
- [x] 12.8 Prove failed and concurrent operations preserve prior committed outputs.
- [x] 12.9 Run all noncredentialed focused commands from `traceability.md`.
- [x] 12.10 Regenerate and verify the hardening manifest, requirement cap, and self-test for REH-012 and REH-013.
- [x] 12.11 Refresh affected assurance inventories, ledgers, and proof bindings without expanding formal claims.
- [x] 12.12 Update the quality registry and gate paths for each new control classified as blocking.

## 13. Document and demonstrate consumer migration

- [x] 13.1 Document the publication plan, command family, operation reports, evidence, and exit classes.
- [x] 13.2 Document immutable basis-keyed pages and replaceable current aliases.
- [x] 13.3 Document path-template, target-base, URI-join, and rejected-cycle rules.
- [x] 13.4 Document safe output behavior, source guards, locks, cleanup, and platform limits.
- [x] 13.5 Document package variants, private source pins, updates, rollback, and offline use.
- [x] 13.6 Add a generic mounted example with no Barnacle-specific value.
- [x] 13.7 Add exact generated page, link, operation-report, evidence, and package-identity goldens.
- [x] 13.8 Document downstream removal of fetch, page, link, and report shell tools.
- [x] 13.9 State that all publication artifacts provide navigation and impact data only.
- [x] 13.10 Keep every downstream publication and package task explicit and outside automatic acceptance.

## 14. Final acceptance and archive

- [x] 14.1 Run Rust formatting, lint, unit, integration, property, model, fuzz, and mutation checks.
- [x] 14.2 Run the offline local-source package check and command matrix.
- [x] 14.3 Run `openspec validate --all --strict --no-interactive`.
- [x] 14.4 Run `openspec schema validate buoy-bridge`.
- [x] 14.5 Run the exact repository acceptance command `devenv test`.
- [x] 14.6 Require every publication and package requirement to have positive and negative evidence.
- [x] 14.7 Generate the final completion evidence without manual volatile digests.
- [x] 14.8 Sync all delta specifications into standing specifications.
- [x] 14.9 Archive the completed change through the reviewed OpenSpec workflow.
