# Readable trace projections

Status: implemented schema v3 shell projection.

## Inventory classification

`buoy/trace-report/v1` is a deterministic JSON projection. It is not a canonical digest domain.

The canonical schema inventory remains `schema/v3/inventory.tsv`. It lists the source graph, implementation bindings, and attachment values that supply trace facts.

The trace report does not add an object kind, digest domain, assurance class, or external relation.

## Machine report

The machine report contains these top-level fields:

- `schema`: fixed value `buoy/trace-report/v1`.
- `graph`: the complete current source-graph digest.
- `selector`: one typed selector and its complete value.
- `include_history`: the explicit history-selection fact.
- `diagnostics`: the selected static diagnostics.
- `rows`: the canonically ordered trace rows.

Each row contains these facts:

- The current or historical state.
- The stable relation ID and navigation kind.
- The complete immutable record digest.
- The stable requirement ID and complete atom revision.
- The exact implementation and requirement node references.
- The available logical paths, source spans, item kinds, and display names.
- The requirement traversal nodes.
- The separate record, attachment, implementation, requirement, and authority freshness values.

A historical node that is absent from the current source graph retains its complete node reference. Its display location is absent.

## Derived views

The human and Markdown views consume the same complete projection. They are derived output and do not replace canonical graph or binding identities.

Human digest abbreviations are display values only. Machine output always contains complete canonical digests.

Every view states that a trace supplies navigation and impact data only. It does not establish coverage, execution, proof, approval, correctness, or prose-to-code correspondence.
