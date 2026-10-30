# Demo Capability

## Requirements

### Requirement: OSA-900 Exact fixture requirement
The adapter MUST materialize this requirement.

```buoy-external-trace-v1
id = "TRACE-INSIDE-REQUIREMENT"
atoms = ["OSA-900@xxh3-128:02020202020202020202020202020202"]
relation = "documents"
target = "https://example.com/traces/inside"
```

#### Scenario: Positive - fixture materializes
- **GIVEN** this standing specification
- **WHEN** the adapter runs
- **THEN** the requirement and scenario identities are exact
