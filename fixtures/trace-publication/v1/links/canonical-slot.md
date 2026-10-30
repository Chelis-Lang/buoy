# Demo

## Requirements

### Requirement: DEMO-001 Typed kinds

The adapter MUST emit a typed kind.

#### Scenario: Positive - valid kind

- **WHEN** extraction runs
- **THEN** one object exists

## Publication: demo-link

```buoy-external-trace-v1
id = "demo-link"
atoms = ["DEMO-001@blake3-256:bbbb"]
relation = "documents"
target = "https://example.invalid/trace/blake3-256/cccc.md"
```
