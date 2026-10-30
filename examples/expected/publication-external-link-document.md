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

## Publication: link

```buoy-external-trace-v1
id = "link"
atoms = ["BGLC-003@blake3-256:a352b07c143fff7a0aa8e2a57d65fba2ee2d59635fdb20191f68fe93ef21c87f"]
relation = "documents"
target = "https://example.invalid/trace/docs/blake3-256/1230550f0c19b8701908afeb129700d2cb82fa10ba6244089e957d61736f13c4.md"
```
