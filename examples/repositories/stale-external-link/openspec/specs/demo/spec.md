# Stale Link Capability

## Requirements

### Requirement: STALE-001 Revised requirement

The revised service MUST preserve exact requirement revision links.

#### Scenario: Positive - current revision
- **GIVEN** a link with the current requirement revision
- **WHEN** Buoy validates the graph
- **THEN** the link remains current

#### Scenario: Negative - earlier revision
- **GIVEN** a link with an earlier requirement revision
- **WHEN** Buoy validates the graph
- **THEN** Buoy reports a stale reference
