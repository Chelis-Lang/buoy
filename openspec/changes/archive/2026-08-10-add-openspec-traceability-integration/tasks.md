## 1. Ownership and Baseline

- [x] 1.1 Record this change as the sole owner of the schema v3 source-item protocol.
- [x] 1.2 Update the Chelis change to reference this protocol without duplicate tasks or evidence.
- [x] 1.3 Record dependencies on the identifier-newtype changes and the digest-algorithm change.
- [x] 1.4 Inventory every source-item field, encoder, adapter, fixture, report, and impact edge.
- [x] 1.5 Record the breaking schema-v3 cutover and the absence of runtime legacy readers.
- [x] 1.6 Record the schema v3 IDs, adapter IDs, parser version, and configuration schema IDs.

## 2. Schema v3 Controls Before Implementation

- [x] 2.1 Add positive fixtures for each initial `SourceItemKind` value.
- [x] 2.2 Add negative fixtures for malformed, duplicate, undeclared, and cross-adapter kind values.
- [x] 2.3 Add positive schema-v3 fixtures for the Markdown and Rust source-item kinds.
- [x] 2.4 Add negative fixtures that reject numeric kind tags and unversioned kind text.
- [x] 2.5 Add compile-fail cases that reject raw kind strings at public core boundaries.
- [x] 2.6 Add Hegel-rs properties for kind parsing, descriptor membership, set order, and round-trip encoding.
- [x] 2.7 Add an exhaustive reference model for descriptor declarations and emitted kind membership.
- [x] 2.8 Add a Verus proof spike for selected v3 kind and parsed-item encoding functions.
- [x] 2.9 Add an AeneasVerif spike for the same pure subset.
- [x] 2.10 Record proof assumptions, exclusions, translator limits, and selected proof scope.

## 3. Schema v3 Implementation

- [x] 3.1 Add the schema v3 inventory, decoder budgets, precedence rules, and cutover contract.
- [x] 3.2 Add the `SourceItemKind` newtype with one parse boundary and private storage.
- [x] 3.3 Add v3 parsed-item identity encoding with the versioned kind ID field.
- [x] 3.4 Add descriptor kind declarations and duplicate-free canonical order.
- [x] 3.5 Reject each emitted item kind that its adapter descriptor does not declare.
- [x] 3.6 Move the built-in Markdown adapter to `markdown/fence/v1` under v3.
- [x] 3.7 Move the built-in Rust adapter to `rust/item/v1` under v3.
- [x] 3.8 Remove runtime legacy readers, numeric item-kind tags, and compatibility paths.
- [x] 3.9 Run the Hegel-rs properties and the exhaustive model against the production implementation.
- [x] 3.10 Complete the accepted formal obligations for the selected pure subset.

## 4. OpenSpec Adapter Controls Before Implementation

- [x] 4.1 Add positive fixtures for valid standing specs with stable requirement IDs.
- [x] 4.2 Add negative fixtures for proposals, designs, tasks, active deltas, and archived changes.
- [x] 4.3 Add negative fixtures for missing, malformed, duplicate, and unreserved requirement IDs.
- [x] 4.4 Add Unicode fixtures for exact requirement and scenario source spans.
- [x] 4.5 Add negative fixtures for malformed requirement headings and three-hash scenario headings.
- [x] 4.6 Add revision fixtures for allowed layout changes and semantic scenario changes.
- [x] 4.7 Add metadata fixtures for kinds, scopes, proposed state, ratification, and approval bindings.
- [x] 4.8 Add negative fixtures that reject archive-derived lifecycle and provider-derived approval.
- [x] 4.9 Add totality fixtures for purpose text, ordinary prose, unknown sections, and unsupported format versions.
- [x] 4.10 Add root, locale, input-order, and provider-state permutation fixtures.
- [x] 4.11 Add Hegel-rs properties for requirement extraction, statement construction, and deterministic output order.
- [x] 4.12 Add an exhaustive parser-state model for the supported heading hierarchy.

## 5. OpenSpec Adapter Implementation

- [x] 5.1 Add parsed configuration types for capability metadata, lifecycle, approval, and registry bindings.
- [x] 5.2 Add the pinned CommonMark parser path for saved OpenSpec bytes.
- [x] 5.3 Extract complete requirement sections with exact original-byte spans.
- [x] 5.4 Construct canonical requirement statements from titles, prose, scenario names, and scenario bodies.
- [x] 5.5 Materialize one Buoy atom and source mapping for each valid requirement.
- [x] 5.6 Add nonnormative dispositions for supported OpenSpec text outside requirements.
- [x] 5.7 Add stable diagnostics for every malformed, forbidden, unsupported, or incomplete input class.
- [x] 5.8 Register `openspec/v1` with `openspec.requirement/v1` in the adapter registry.
- [x] 5.9 Add explicit selector support for repository-local standing specs only.
- [x] 5.10 Merge OpenSpec adapter products through the existing deterministic graph path.
- [x] 5.11 Run all adapter fixtures, Hegel-rs properties, and the exhaustive parser model.

## 6. External Trace-Link Controls Before Implementation

- [x] 6.1 Add positive fences for issue, pull-request, document, and review links.
- [x] 6.2 Add negative fences for unknown fields, duplicate fields, empty atoms, and empty targets.
- [x] 6.3 Add URI cases for accepted `https`, accepted `urn`, and every rejected target form.
- [x] 6.4 Add relation cases for each accepted relation and one unknown relation.
- [x] 6.5 Add negative controls that prevent links from satisfying any assurance obligation.
- [x] 6.6 Add provider-state permutation cases with equal repository bytes.
- [x] 6.7 Add stale-link and missing-impact-edge graph pairs.
- [x] 6.8 Add report goldens with permuted input and registration order.
- [x] 6.9 Add Hegel-rs properties for target parsing, relation parsing, order, and revision stability.
- [x] 6.10 Add an exhaustive model for relation and target-form acceptance.

## 7. External Trace-Link Implementation

- [x] 7.1 Add the v3 `ExternalTraceLink` core type and canonical encoding.
- [x] 7.2 Add parsed types for the closed relation set and normalized external targets.
- [x] 7.3 Add strict `buoy-external-trace-v1` fence parsing in the Markdown adapter.
- [x] 7.4 Add exact source mappings for each external trace-link fence.
- [x] 7.5 Add graph closure rules for atom-revision references.
- [x] 7.6 Add freshness rules that bind each exact atom revision.
- [x] 7.7 Add atom-to-trace-link impact edges and required dispositions.
- [x] 7.8 Add separate human and machine navigation output.
- [x] 7.9 Exclude external trace links from every assurance-satisfaction path.
- [x] 7.10 Run all link fixtures, Hegel-rs properties, and the exhaustive model.

## 8. OpenSpec Workflow Integration

- [x] 8.1 Fork `spec-driven` into the project-level `buoy-bridge` schema.
- [x] 8.2 Add the `traceability` artifact after specs and design.
- [x] 8.3 Make tasks depend on the traceability artifact.
- [x] 8.4 Add a traceability template for IDs, metadata, links, removals, owners, and commands.
- [x] 8.5 Add project rules for reserved IDs and positive and negative scenarios.
- [x] 8.6 Add apply guidance for repository-owned link updates and focused Buoy commands.
- [x] 8.7 Add archive guidance for strict OpenSpec validation and the Buoy change gate.
- [x] 8.8 Add negative controls that prove guidance and artifact existence are not enforcement.
- [x] 8.9 Add schema validation and generated-workflow drift validation to the repository gate.
- [x] 8.10 Validate the project schema resolution order against a conflicting user schema fixture.

## 9. Reports, Cutover, and Documentation

- [x] 9.1 Add v3 source items and external trace links to static report schemas.
- [x] 9.2 Add their registration, freshness, navigation, and impact facts to deterministic output.
- [x] 9.3 Record that schema-v3 adoption requires source reingestion without identity migration.
- [x] 9.4 Document the breaking cutover without a rollback reader or identity reinterpretation.
- [x] 9.5 Update the README boundary for optional OpenSpec standing-spec authority under v3.
- [x] 9.6 Update adapter SDK, trust boundary, schema, workflow, and maintainer documentation.
- [x] 9.7 Document requirement-level scope and the exclusion of scenario-level coverage.
- [x] 9.8 Document external links as navigation that supplies no assurance.
- [x] 9.9 Reconcile all affected assurance inventories, owner mappings, and generated reports.

## 10. Acceptance

- [x] 10.1 Regenerate every v3 fixture, vector, report, source inventory, and assurance binding.
- [x] 10.2 Validate that schema v3 rejects each legacy numeric item-kind representation.
- [x] 10.3 Run focused schema v3, adapter SDK, OpenSpec adapter, trace-link, cutover, and workflow controls.
- [x] 10.4 Run all Hegel-rs properties, exhaustive models, formal obligations, and negative controls.
- [x] 10.5 Run `openspec validate add-openspec-traceability-integration --strict --no-interactive`.
- [x] 10.6 Run an adversarial review of authority, lifecycle, stale links, provider isolation, and assurance boundaries.
- [x] 10.7 Run the exact offline acceptance oracle `devenv test`.
- [x] 10.8 Accept implementation only when every required control runs and `devenv test` exits `0`.
