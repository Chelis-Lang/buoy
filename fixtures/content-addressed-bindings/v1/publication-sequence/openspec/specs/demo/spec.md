# Generated lint catalog

## Purpose

Define deterministic catalog output.

## Requirements

### Requirement: BGLC-003 Render a deterministic and complete catalog

The generator MUST render every discovered library and registered lint in stable order.

#### Scenario: Complete catalog
- **GIVEN** two discovered libraries with registered lints
- **WHEN** the generator renders the catalog
- **THEN** each library and lint appears exactly once

#### Scenario: Stable output
- **GIVEN** unchanged repository content
- **WHEN** the generator renders the catalog twice
- **THEN** both outputs are byte-identical

### Requirement: BGLC-004 Reserve a trailing normative anchor

The generator MUST reserve one trailing section so appended nonauthority content never extends a prior requirement's span.

#### Scenario: Trailing anchor holds
- **GIVEN** one publication slot appended after this requirement
- **WHEN** the publication slot is inserted
- **THEN** this requirement's own span absorbs the appended bytes
