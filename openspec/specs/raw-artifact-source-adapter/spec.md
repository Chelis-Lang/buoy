# Raw Artifact Source Adapter

## Purpose

Define opaque saved-byte source nodes for files that Buoy does not parse as a programming language.

## Requirements

### Requirement: RAS-001 Opaque saved-byte source node

The built-in `artifact/v1` adapter MUST emit one `artifact/file/v1` source node for each selected regular file.

The item identity MUST bind the logical path, exact saved bytes, fixed opaque media type, and versioned saved-byte representation.

#### Scenario: Positive - opaque file becomes a current subject
- **GIVEN** one selected Nix file under an `artifact/v1` selector
- **WHEN** adapter materialization runs
- **THEN** it emits one current full-file source node without parsing Nix syntax

#### Scenario: Negative - one saved byte changes
- **GIVEN** one current opaque file source node
- **WHEN** one selected byte changes
- **THEN** the source-node digest changes and a binding to the prior node becomes stale

### Requirement: RAS-002 Fixed opaque interpretation

The adapter descriptor MUST declare `artifact.opaque-bytes/v1`, `artifact/file/v1`, `application/octet-stream`, and `saved-bytes/v1`.

The adapter MUST NOT infer language semantics, normalize line endings, decode text, or execute the selected artifact.

#### Scenario: Positive - equal bytes under different physical roots
- **GIVEN** equal saved bytes at equal logical relative paths in separate snapshots
- **WHEN** the host uses different physical root directories before logical projection
- **THEN** the adapter emits equal source identities because only the logical path and fixed opaque representation enter identity

#### Scenario: Negative - configuration requests semantic parsing
- **GIVEN** `artifact/v1` configuration that requests a language, parser, or text normalization
- **WHEN** configuration parsing runs
- **THEN** parsing fails with a deterministic artifact-configuration diagnostic

### Requirement: RAS-003 Complete full-file source mapping

Each opaque source node MUST map to the complete selected byte range. The mapping MUST use byte offsets and the canonical logical path.

The adapter MUST emit no atom authority, implementation link, or assurance object from opaque bytes.

#### Scenario: Positive - binary file maps without UTF-8
- **GIVEN** selected bytes that are not valid UTF-8
- **WHEN** `artifact/v1` materializes the file
- **THEN** one full-file source mapping covers all bytes without text decoding

#### Scenario: Negative - adapter emits partial coverage
- **GIVEN** an artifact product whose source span omits one selected byte
- **WHEN** SDK product validation runs
- **THEN** validation fails with a deterministic incomplete-source-mapping diagnostic

### Requirement: RAS-004 Forbidden authority role

An `artifact/v1` selector MUST use the forbidden authority expectation. Opaque files MUST NOT become requirement authorities.

#### Scenario: Positive - artifact selector forbids authority
- **GIVEN** one opaque artifact selector with the forbidden authority expectation
- **WHEN** repository configuration parsing runs
- **THEN** parsing accepts the selector as a nonnormative implementation source

#### Scenario: Negative - artifact selector requires authority
- **GIVEN** one opaque artifact selector with optional or required authority
- **WHEN** repository configuration parsing runs
- **THEN** parsing fails with a deterministic artifact-authority diagnostic
