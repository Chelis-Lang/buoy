# Red-state evidence before production implementation

Status: captured after tasks 2.1 through 3.5 and before task 4.1.

## Commands

```text
devenv shell -- cargo test -p buoy --all-features --test readable_trace_projections
devenv shell -- cargo test -p buoy --all-features --test openspec_v2_identity
devenv shell -- cargo test -p buoy --all-features --test bounded_trace_projection_model
devenv shell -- cargo test -p buoy --all-features --test trace_hegel_properties
```

## Results

| Suite | Passed | Failed | Meaning |
|---|---|---|---|
| `readable_trace_projections` | 8 | 11 | The fixture tables and formal-lane record hold. Each command, adapter, and template contract waits for production code. |
| `openspec_v2_identity` | 2 | 5 | Version 1 parity and the unsupported-format control hold. Each version 2 identity control waits for the adapter. |
| `bounded_trace_projection_model` | 5 | 0 | the independent classification and abbreviation models are complete |
| `trace_hegel_properties` | 3 | 0 | the sampled order, root, and selector properties are complete |

## Primary failure reasons

Each failing test states one exact missing behavior.

- `buoy trace` is absent. The command parser returns usage exit `64` for every trace invocation.
- `openspec/standing-spec/v2` is absent. The adapter configuration parser returns `SPG-OPENSPEC-CONFIG-FORMAT`.
- The `traceability.md` template lacks the planned relation and completion snapshot fields.

## Boundary

This record states test polarity before implementation. It supplies no coverage, execution, proof, approval, or correctness evidence.
