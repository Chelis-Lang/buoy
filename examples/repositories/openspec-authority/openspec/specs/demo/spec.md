# Demo Capability

## Purpose

This example defines one proposed requirement for the OpenSpec adapter.

## Requirements

### Requirement: DEMO-001 Deterministic greeting

The service MUST return the configured greeting without provider access.

#### Scenario: Positive - configured greeting
- **GIVEN** a saved repository with one configured greeting
- **WHEN** the service reads the saved configuration
- **THEN** the service returns the configured greeting

#### Scenario: Negative - provider state is absent
- **GIVEN** no network or provider state
- **WHEN** the service reads the saved configuration
- **THEN** the result remains unchanged

## Notes

Buoy treats this section as nonauthority text.
