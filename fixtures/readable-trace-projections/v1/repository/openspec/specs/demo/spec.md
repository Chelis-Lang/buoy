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
