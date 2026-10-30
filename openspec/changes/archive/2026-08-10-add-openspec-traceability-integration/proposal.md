## Why

OpenSpec requirements cannot enter the Buoy graph. Therefore, code, tests, issues, pull requests, and documents cannot link to the current behavior source.

The current parsed-item protocol supports only Markdown fences and Rust items. It cannot give an OpenSpec requirement a truthful source identity.

## What Changes

- Add an optional shell adapter that reads saved OpenSpec standing specs and creates Buoy requirement atoms.
- Read only `openspec/specs/**/*.md`. Do not treat active or archived change artifacts as atom authority.
- Require one stable ID in each requirement heading. Include the requirement text and all scenarios in the atom revision.
- Add explicit adapter configuration for requirement kind, scope, lifecycle state, approval, and stable ID allocation.
- **BREAKING**: Add semantic schema v3 with an extensible, versioned parsed-item kind protocol.
- Use a schema-v3 breaking cutover. Do not retain legacy parsed-item readers or identities.
- Add external trace links for issues, pull requests, and documents. These links provide navigation and no assurance.
- Keep provider state, issue state, labels, authors, timestamps, and network results outside canonical inputs.
- Add a project-level OpenSpec workflow schema with a traceability artifact before implementation tasks.
- Add OpenSpec project rules and operation guidance for stable IDs, link updates, and Buoy checks.
- Add deterministic checks for missing, duplicate, malformed, unsupported, and stale links.

### Goals

- Make an OpenSpec standing requirement a first-class Buoy atom.
- Link exact atom revisions to parsed code items, evidence carriers, and external references.
- Detect link drift after requirement, code, test, or configuration changes.
- Reuse the generic parsed-item protocol for future host adapters, including Chelis.

### Non-Goals

- Do not parse proposals, designs, tasks, active deltas, or archived changes as atom authority.
- Do not infer ratification or approval from an OpenSpec archive operation.
- Do not use live issue or pull-request state as authority, evidence, or gate status.
- Do not make OpenSpec, a provider client, a parser, or a network library a `buoy-core` dependency.
- Do not claim scenario-level test coverage in the first adapter version.
- Do not publish a community schema in this change.

## Capabilities

### New Capabilities

- `extensible-source-items`: Defines schema v3 parsed-item kinds, adapter declarations, cutover rules, and source identity.
- `openspec-source-adapter`: Defines deterministic OpenSpec requirement extraction, metadata configuration, authority rules, source spans, and diagnostics.
- `external-trace-links`: Defines informational links from exact atom revisions to issues, pull requests, and documents.
- `openspec-buoy-workflow`: Defines the project schema, traceability artifact, project rules, operation guidance, and enforcement boundary.

### Modified Capabilities

- `spec-graph`: Adds adapter-declared source items and external trace links to graph closure, navigation, and total adapter disposition.
- `deterministic-artifacts`: Adds external trace links and new parsed items to deterministic identity and change-impact output.
- `product-identity`: Replaces the absolute OpenSpec reference-only rule with an optional shell-adapter boundary for standing specs under schema v3.

## Impact

- Core schema and model: `schema/v3/`, `crates/buoy-core/src/model.rs`, canonical encoders, readers, graph closure, and impact edges.
- Adapter shell: `src/adapter_sdk.rs`, a new OpenSpec adapter, selector configuration, source maps, and diagnostics.
- Trace links: new repository syntax, parser support, reports, fixtures, and navigation output.
- OpenSpec workflow: `openspec/schemas/buoy-bridge/`, `openspec/config.yaml`, generated skills, and archive guidance.
- Tests and evidence: positive and negative adapter fixtures, cutover fixtures, property cases, mutation cases, and deterministic report goldens.
- Active changes: the generic source-item protocol becomes a prerequisite for the Chelis adapter and needs one ownership decision.
