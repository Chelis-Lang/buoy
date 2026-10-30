## ADDED Requirements

### Requirement: ESI-009 Registered opaque artifact kind

The built-in registry MUST declare `artifact/v1` with exactly one `artifact/file/v1` source-item kind and one `artifact.opaque-bytes/v1` capability.

The kind MUST use the existing parsed-item, source-node, source-mapping, budget, and canonical-order contracts.

#### Scenario: Positive - registry materializes opaque bytes
- **GIVEN** one valid `artifact/v1` configuration and one selected file
- **WHEN** the built-in registry dispatches the adapter
- **THEN** the product contains one declared `artifact/file/v1` node with a complete source mapping

#### Scenario: Negative - product emits a semantic language kind
- **GIVEN** an `artifact/v1` product that emits a Rust, OpenSpec, Markdown, or unregistered item kind
- **WHEN** SDK product validation runs
- **THEN** validation fails with the deterministic undeclared-source-item-kind diagnostic
