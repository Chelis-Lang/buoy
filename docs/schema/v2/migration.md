# Version 1 to version 2 migration and rollback

The hashing change is intentionally breaking. A v1 `sha256:` value identifies only its original `spec-provenance/v1` preimage. It is never truncated, padded, byte-reused, transformed, or relabeled as `xxh3-128:`.

## Offline cutover

The migration driver operates without network, provider, clock, locale, or environment-derived semantic input. It:

1. validates the complete declared v1 source shape;
2. reads authoritative configuration, object, review, and source bytes;
3. regenerates v2 leaf identities from those bytes;
4. regenerates semantic objects, references, carriers, policies, derivations, assumptions, properties, proof records, graphs, and reports in deterministic dependency order;
5. preserves stable IDs, terminal lifecycle state, tombstones, replacements, and required approvals; and
6. emits exactly one ordered revised, reaffirmed, retired, or waived impact disposition for every changed old/new binding.

Missing bytes, stale nested bindings, missing approvals, cycles, duplicate IDs, illegal lifecycle changes, partial impact closure, duplicate dispositions, obsolete active digests, and mixed v1/v2 data all fail closed.

## Reader and writer policy

Active constructors, parsers, adapters, reports, fixtures, examples, and writers accept or emit only version 2 semantic identities. There is no dual-read or dual-write interval and no fallback algorithm. Historical v1 data is consumed only by the explicit migration boundary and remains labeled v1.

## Rollback

The immutable v1 baseline is Git commit `9b1b7740c390aeaa58b491b2cefc9ee8c694fce2`. Rollback means restoring that complete baseline and its dependency lock, schema, code, fixtures, records, and tools. Selecting a subset of v1 files, relabeling v2 data, or combining v1 and v2 artifacts is not rollback and is rejected as mixed-version state.
