## Why

Buoy accepts one physical repository root. This boundary prevents a project from using a separate normative OpenSpec store without unsafe links or copied paths.

Compact implementation bindings also require Rust attributes. Unsupported languages and generated files cannot become current implementation subjects through a language-neutral path.

Buoy needs deterministic logical mounts and opaque artifact subjects before Barnacle can bind code or generated artifacts to its canonical `../openspec/` store.

## What Changes

### Goals

- Add explicit logical mounts that map separate physical directories into one canonical snapshot namespace.
- Keep physical paths outside canonical identities, reports, diagnostics, and graph bytes.
- Add a built-in raw-artifact adapter that creates one source node from exact saved bytes and declared media metadata.
- Add repository-owned compact sidecars that attach immutable binding records to raw-artifact subjects.
- Extend `buoy bind` with an explicit raw-artifact selector and atomic sidecar update.
- Preserve descriptor-relative reads, path policy, resource budgets, and fail-closed graph aggregation across all mounts.
- Add deterministic fixtures that model a Barnacle source root and a separate OpenSpec authority root.

### Non-goals

- Do not fetch, clone, or update repositories.
- Do not follow symbolic links, submodules, or nested repository links.
- Do not infer language semantics from opaque artifact bytes.
- Do not make a fresh binding evidence of coverage, execution, approval, proof, or correctness.
- Do not add graph federation between independently validated snapshots.
- Do not modify the Barnacle repository in this change.

The shell will open each supplied physical root and build one logical snapshot. Static checking will consume only saved bytes, logical paths, explicit configuration, and registered adapter outputs.

Effectful oracle execution remains separate. No oracle result will affect source-node identity or binding freshness.

## Capabilities

### New Capabilities

- `logical-source-mounts`: Define explicit multi-root snapshot inputs, logical path projection, collision controls, and deterministic mount identity.
- `raw-artifact-source-adapter`: Materialize opaque files as current source nodes without claiming language interpretation.

### Modified Capabilities

- `content-addressed-implementation-bindings`: Add compact out-of-line attachments and raw-artifact bind updates.
- `source-reference-graph`: Aggregate selected products from all logical mounts into one bounded source graph.
- `extensible-source-items`: Register the raw-artifact adapter and its closed source-item kind.
- `repository-execution-hardening`: Apply descriptor, link, race, overlap, and budget controls to each physical mount.

## Impact

The repository configuration schema, CLI root arguments, snapshot builder, adapter registry, binding transaction, graph validation, diagnostics, and schema inventory will change.

Existing single-root configurations will retain their current semantics through an explicit compatibility path. Existing Rust compact bindings and immutable object paths will remain unchanged.

New fixtures, negative controls, canonical vectors, migration records, SDK documentation, and exact assurance inventories will cover the new boundary.
