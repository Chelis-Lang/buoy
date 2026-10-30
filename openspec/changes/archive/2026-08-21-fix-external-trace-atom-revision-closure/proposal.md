## Why

A current external trace link fails graph validation because generic object closure compares its semantic atom revision with the atom object digest. These two digest domains are intentionally different, so valid OpenSpec version 2 links receive `SPG-REF-DANGLING-OR-STALE`.

## Goals

- Validate external links against exact semantic atom revisions.
- Reject an atom object digest when it appears as a semantic revision.
- Preserve existing external-link identities, reports, source mappings, and assurance limits.
- Add an end-to-end regression for an OpenSpec version 2 `documents` link.

## Non-Goals

- Change atom, requirement, or trace-link canonical identity.
- Change external-link syntax, relations, targets, or reports.
- Fetch provider state or add a new assurance class.
- Change implementation-binding freshness.
- Add an execution oracle or formal proof claim.

## What Changes

- Stop encoding semantic atom revisions as generic atom object references.
- Include external trace links in the dedicated atom-revision closure check.
- Update the core freshness tests to distinguish atom revisions from atom object digests.
- Add a saved-snapshot regression for one current and one stale version 2 link.
- Document the two digest domains and the deterministic stale diagnostic.

## Capabilities

### New Capabilities

- None.

### Modified Capabilities

- `external-trace-links`: Validate each link against `Atom::revision()` without reinterpretation as `Atom::meta().digest()`.

## Impact

The change affects `ExternalTraceLink` metadata references, graph validation, focused tests, examples, documentation, and generated assurance evidence.

The algorithm file digest change requires bounded campaign replays and maintenance of an existing whole-file proof binding. It adds no formal scope for external links.

No public machine schema changes. No provider permissions or network access changes. The canonical acceptance oracle remains `devenv test`.
