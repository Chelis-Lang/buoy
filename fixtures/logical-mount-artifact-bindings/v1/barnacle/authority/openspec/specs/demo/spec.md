# Demo

## Requirements

### Requirement: OSA-900 Preserve the opaque artifact

The project MUST preserve the exact selected artifact bytes.

#### Scenario: Exact bytes

- **GIVEN** one selected opaque artifact
- **WHEN** repository materialization runs
- **THEN** the source node binds the exact saved bytes
