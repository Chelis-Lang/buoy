# Implementation trace

Derived data. Buoy remains the current-state authority. Regenerate this page from a current query.

- Renderer schema: `buoy/readable-trace-markdown/v1`
- Trace-report schema: `buoy/trace-report/v1`
- Selector: `relation-id BGLC-RENDER-CATALOG`
- Source graph: `blake3-256:29fce2a3aa4972e09809482995a4f33636a11c2cac45ea87192bf9d49d7e01a5`
- History included: false

## Requirement

- `BGLC-003` at openspec/specs/demo/spec.md:93-608

## Implementation

- src/lib.rs:0-98 (`rust/item/v1`)

## Relations

| State | Relation | Navigation | Record | Requirement | Atom revision | Requirement path | Implementation path | Freshness |
|---|---|---|---|---|---|---|---|---|
| current | `BGLC-RENDER-CATALOG` | implements | `blake3-256:0e857e33e387325fbf457e426bc42286d9d99926b19f3efa3c203795cfeeb6c4` | `BGLC-003` | `blake3-256:a352b07c143fff7a0aa8e2a57d65fba2ee2d59635fdb20191f68fe93ef21c87f` | openspec/specs/demo/spec.md | src/lib.rs | record=true attachment=true implementation=true requirement=true authority=true |

## Boundary

This projection supplies navigation and impact only. It establishes no coverage, execution, proof, approval, correctness, or prose-to-code correspondence.
