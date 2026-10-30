## Why

Current inline Rust implementation links repeat IDs, revisions, navigation data, and TOML syntax. The adapter regenerates the implementation-item digest during each run, so a source edit does not make the inline link stale.

Buoy already parses OpenSpec requirements and scenarios as nested source items. However, the CLI discards this structure before repository-level validation. A compact content address can identify one immutable binding record and expose its references as a traversable graph.

## What Changes

- Add content-addressed implementation binding records. Each record references one exact implementation item and one exact OpenSpec requirement item.
- Accept one binding digest in a Rust attribute, for example `#[buoy::implements("blake3-256:...")]`.
- Exclude Buoy attributes from the implementation subject identity to prevent a digest cycle.
- Retain parsed OpenSpec requirement and scenario items in a canonical source graph.
- Represent source relationships as directed digest references. This permits traversal from a binding to its implementation, requirement, scenarios, and atom authority.
- Require each binding digest to resolve to one canonical record. Require each record reference to resolve within the current source graph.
- Emit deterministic diagnostics for missing records, stale implementation items, stale requirements, ambiguous targets, cycles, and wrong item kinds.
- Add a deterministic command that creates a new immutable binding record and updates the compact source attribute.
- Keep static registration and freshness separate from execution verdicts and assurance classes.

### Goals

- Reduce each inline Rust link to one digest.
- Detect requirement and implementation changes during static validation.
- Support tree traversal from implementation code to OpenSpec scenarios and atom authority.
- Preserve deterministic, offline, root-independent behavior.

### Non-Goals

- Do not infer that an implementation satisfies a requirement.
- Do not make an implementation link satisfy evidence or proof obligations.
- Do not add effectful oracle execution to static graph construction.
- Do not add cross-repository filesystem access. A caller must supply one materialized snapshot.
- Do not add a Nix syntax adapter in this change.

## Capabilities

### New Capabilities

- `content-addressed-implementation-bindings`: Defines immutable binding records, compact source references, current-graph resolution, and reference traversal.
- `source-reference-graph`: Defines canonical parsed-source nodes, directed digest edges, roots, closure, and traversal.

### Modified Capabilities

- `openspec-source-adapter`: Retains requirement and scenario nodes with explicit parent relationships.
- `extensible-source-items`: Adds source-node retention and reference closure to the adapter protocol.
- `spec-graph`: Adds content-addressed implementation bindings without changing their navigational assurance boundary.

## Impact

The core gains generic source-reference values and deterministic traversal algorithms. The core remains `no_std` and receives only parsed values.

The Rust and OpenSpec adapters emit source nodes and binding references. The shell resolves repository-wide references after all selected files are materialized.

The CLI retains source products for static validation. The frozen verbose implementation attribute remains unchanged until an explicit migration task removes it.

The exact acceptance oracle is `devenv test`. Success requires all tasks and tests to complete with no diagnostics or coverage regressions.
