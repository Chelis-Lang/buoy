## Why

Consumer repositories currently need local shell tools to store readable trace pages, synchronize external links, inspect reports, and obtain a pinned Buoy executable.

This split gives consumers logic that Buoy must own. It also permits several faults:

- unpinned executable substitution,
- unsafe output paths,
- text-based JSON checks,
- nondeterministic diagnostics,
- cache races.

Buoy already owns the source selectors, report schemas, OpenSpec parser, link identity, and trace projection. Buoy must also own publication and consumer packaging.

## What Changes

- Add a versioned trace-publication plan and one Buoy-owned command family for check, generation, and evidence operations.
- Keep `buoy trace` read-only and isolate every local write behind an explicit publication generation operation.
- Parse machine reports into strict types and enforce row cardinality, stable IDs, logical paths, freshness, diagnostics, and assurance boundaries together.
- Generate tracked Markdown pages from the existing trace projection with race-resistant, no-follow, atomic replacement.
- Generate or update only an OpenSpec version 2 nonauthority external-link block from the current semantic atom revision.
- Compute one publication-basis digest from all page-affecting pre-link inputs.
- Require immutable targets to contain the publication basis, with optional atom or record path grouping.
- Reject derived outputs that enter any selected source set or depend on a graph, page, or page-commit digest.
- Require each controlled source edit to match one selected file and preserve its normative semantics.
- Require saved source-edit candidates before page, evidence, or final publication checks run.
- Render deterministic machine and Markdown evidence for OpenSpec completion snapshots. Do not copy immutable record bodies.
- Add a Buoy-owned pinned package interface for Nix and Devenv consumers with machine-readable package identity.
- Test local package variants in Buoy acceptance while downstream package and publication tasks remain advisory.
- Correct the compact Rust locator contract and documentation to match the canonical algorithm-qualified digest that `buoy bind` emits and parses.
- Add contract, property, model, mutation, filesystem-race, cross-root, offline, and consumer-package tests before production implementation.
- Keep all generated links and pages limited to navigation and impact data.
- **Non-goals:** provider publication, network target checks, automatic commits, approvals, pushes, merges, or correctness claims.
- **Non-goals:** Barnacle-specific commands, schemas, paths, IDs, or policy in the Buoy repository.
- **Non-goals:** change `buoy/trace-report/v1` or frozen `buoy/binding-tree-report/v1` meanings.

## Capabilities

### New Capabilities
- `trace-publication-tooling`: Versioned plans and Buoy-owned local operations for trace-page parity, generation, external-link synchronization, strict conformance, and evidence rendering.
- `pinned-buoy-distribution`: A reproducible Buoy package interface that lets consumers pin and invoke Buoy without consumer-owned fetch or build scripts.

### Modified Capabilities
- `readable-trace-projections`: Preserve read-only trace behavior and make the Markdown projection self-describing for publication.
- `external-trace-links`: Add deterministic generation from current atom revisions, basis-keyed page paths, and normalized target-base joins.
- `repository-execution-hardening`: Apply descriptor-relative no-follow and bounded atomic-write controls to generated publication artifacts.
- `openspec-buoy-workflow`: Generate mechanical completion evidence and nonauthority link data. Retain human planning and approval decisions.
- `content-addressed-implementation-bindings`: Define the compact locator as one canonical algorithm-qualified digest and align all examples and diagnostics.
- `devenv-workflow`: Keep downstream publication and package tasks explicit, advisory, and outside automatic acceptance.

## Impact

- Adds a new shell-side publication module, plan parser, command family, safe writer, deterministic diff renderer, and evidence renderer.
- Adds versioned plan, Markdown-renderer, basis, operation-report, evidence, page-content, and package-identity schemas without a new graph object or assurance class.
- Adds a Buoy-owned Nix package interface, Devenv consumption examples, and package-identity command surface.
- Adds OpenSpec adapter edit planning that cannot change normative spans.
- Adds filesystem and concurrency tests for alternate executables, symlinked ancestors, component swaps, interrupted writes, and concurrent generation.
- Adds downstream migration guidance that removes consumer-owned trace scripts after consumers adopt the pinned Buoy package.
- Leaves existing static, change, execute, bind, and trace command behavior compatible.
