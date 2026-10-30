# Demo

## Requirements

### Requirement: DEMO-001 Typed kinds

The adapter MUST emit a typed kind.

```buoy-external-trace-v1
id = "demo-link"
atoms = ["DEMO-001@blake3-256:aaaa"]
relation = "documents"
target = "https://example.invalid/trace/blake3-256/one.md"
```

#### Scenario: Positive - valid kind

- **WHEN** extraction runs
- **THEN** one object exists
