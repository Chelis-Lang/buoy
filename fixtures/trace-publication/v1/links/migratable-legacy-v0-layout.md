# Demo

## Requirements

### Requirement: DEMO-001 Typed kinds

The adapter MUST emit a typed kind.

#### Scenario: Positive - valid kind

- **WHEN** extraction runs
- **THEN** one object exists

## Publication: demo-link

```buoy-external-trace-v1
# Legacy v0 emission layout, retained here as a real migration input.
#
# This differs STRUCTURALLY from the canonical slot, not merely in digest
# values: the keys appear in the original v0 emission order (target first,
# id last), use compact `key="value"` spacing rather than `key = "value"`,
# carry a legacy-only `generated_by` key the current renderer no longer
# emits, and declare more than one atom entry.
target="https://example.invalid/trace/blake3-256/legacy-v0.md"
relation="documents"
atoms=["DEMO-001@blake3-256:0000","DEMO-001@blake3-256:0001"]
id="demo-link"
generated_by="buoy-external-trace/v0"
```
