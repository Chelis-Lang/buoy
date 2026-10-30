## Why

The OpenSpec adapter does not disposition all selected syntax. Text can remain uncovered or enter the wrong requirement revision without a diagnostic.

The adapter also lacks the required diagnostic for unsupported OpenSpec format versions. The user guide documents a requirement-heading form that the parser rejects.

These gaps affect deterministic static authority. They do not affect oracle execution.

## What Changes

- Define complete source disposition for every byte and supported syntax node in a selected standing specification.
- Keep purpose text, ordinary prose, and supported unknown sections nonnormative when the schema permits them.
- Reject unsupported syntax and unsupported format versions with stable diagnostics.
- Stop each requirement span at the next peer or higher-level section boundary.
- Reject uncovered text between the requirements heading and the first requirement.
- Add positive and negative fixtures for section boundaries, unsupported syntax, format versions, and exact byte coverage.
- Correct the documented requirement-heading form to `### Requirement: <stable-id> <title>`.
- Reconcile assurance records with the focused test results.

### Goals

- Make source disposition total and deterministic.
- Prevent unrelated sections from changing an atom revision.
- Fail closed on unsupported authority formats.
- Keep the documented heading syntax equal to the parser contract.

### Non-Goals

- Do not add workflow enforcement for `OBW-003` or `OBW-007`.
- Do not add repository-wide OpenSpec self-ingestion.
- Do not change external trace-link semantics.
- Do not add provider or network inputs.
- Do not change effectful oracle execution.
- Do not claim that test success proves parser correctness.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `openspec-source-adapter`: Tighten total disposition, section boundaries, format-version rejection, diagnostics, and documentation parity.

## Impact

- Adapter implementation: `src/adapters/openspec.rs` and the shell adapter validation path.
- Tests and fixtures: `tests/openspec_traceability.rs` and `fixtures/openspec/v1/`.
- Documentation: `docs/schema/v3/openspec-traceability.md`.
- Assurance records: focused OpenSpec adapter evidence and generated obligation records.
- Compatibility: previously accepted ambiguous or unsupported standing-spec input can now fail with a stable diagnostic.
