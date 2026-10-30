# Demo

## Requirements

### Requirement: DEMO-001 Typed kinds

The adapter MUST emit a typed kind.

#### Scenario: Positive - valid kind

- **WHEN** extraction runs
- **THEN** one object exists

## Publication: demo-link

This section also records maintainer commentary that Buoy does not own.

```buoy-external-trace-v1
id = "demo-link"
atoms = ["DEMO-001@blake3-256:aaaa"]
relation = "documents"
target = "https://example.invalid/trace/blake3-256/stale.md"
```
