# Demo Capability

Purpose text introduces café behavior before any authority section.

## Purpose
This specification defines a deterministic fixture with multibyte text: λ.

## Requirements

This ordinary prose explains the requirement section and is nonnormative.

### Requirement: OSA-900 Exact fixture requirement
The adapter MUST materialize this requirement.

#### Scenario: Positive - fixture materializes
- **GIVEN** this standing specification
- **WHEN** the adapter runs
- **THEN** the requirement and scenario identities are exact

#### Scenario: Negative - active change path
- **GIVEN** equal bytes under an active change path
- **WHEN** the adapter runs
- **THEN** it emits `SPG-OPENSPEC-STANDING-PATH`

## Implementation trace

This section is nonauthority navigation data. Buoy remains the current-state authority.

```buoy-external-trace-v1
id = "TRACE-DEMO-PAGE"
atoms = ["OSA-900@xxh3-128:02020202020202020202020202020202"]
relation = "documents"
target = "https://example.com/traces/osa-900-revised"
```
