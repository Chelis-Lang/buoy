## Why

`buoy bind` requires an exact current OpenSpec requirement source-node digest. The static report does not expose that digest before the first binding exists.

`buoy trace --requirement-id` selects stored relations, so it cannot bootstrap a new relation. A consumer must not reimplement or handwrite Buoy identities.

## What Changes

- Add a canonically ordered `requirement_subjects` inventory to static machine reports.
- Include the stable requirement ID, source-node digest, atom revision, logical path, and authority scopes in each row.
- Add the equivalent deterministic lines to human static reports.
- Keep the inventory read-only and derived from the accepted current source graph.
- Add positive and negative fixtures for current selection, ambiguity, ordering, physical-root independence, and non-OpenSpec repositories.
- Update static report goldens, documentation, package source identity, and assurance evidence.

Goals:

- Let a consumer select one current requirement digest before the first `buoy bind` operation.
- Preserve digest-explicit binding commands and current authority checks.
- Keep graph identity, binding identity, and assurance classes unchanged.

Non-goals:

- Add a new write command or implicit binding selector.
- Expose active or archived OpenSpec change documents as current authority.
- Treat the inventory as approval, coverage, execution, proof, correctness, or semantic correspondence.
- Change `buoy trace`, binding-record schemas, or canonical digest domains.

Static checks remain deterministic and effect-free. Effectful oracle execution remains unchanged.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `content-addressed-implementation-bindings`: Expose exact current OpenSpec requirement subjects before the first binding exists.

## Impact

The change affects static report types, renderers, CLI report construction, focused fixtures, goldens, schema documentation, and package source evidence.

Existing static report consumers receive one additive field when current OpenSpec requirements exist. Reports without current OpenSpec requirements omit that field.

The change adds no filesystem write, provider access, oracle execution, network input, or assurance class.
