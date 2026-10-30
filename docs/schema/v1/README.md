# Buoy schema v1

Status: schema decision index for phase 2. OpenSpec is the planning/reference workflow used to review these decisions; none of the product formats below are OpenSpec formats or require OpenSpec at runtime.

## Frozen documents

- [`atoms.md`](atoms.md): atom authority syntax, stable IDs, semantic metadata, approval, lifecycle, and revision boundary.
- [`repository-inputs.md`](repository-inputs.md): explicit snapshots, paths, selectors, links, submodules, Unicode, line endings, and Markdown trivia.
- [`canonical-encoding.md`](canonical-encoding.md): canonical value encoding, digest domains and preimages, ordering, duplicates, SHA-256, and migration.
- [`objects.md`](objects.md): validated core objects, nested values, enum tags, references, and graph closure.
- [`reports.md`](reports.md): static/execution/impact facts, neutral change manifests and dispositions, aggregation, and diagnostic families.
- [`decision-surfaces.md`](decision-surfaces.md): domain evolution, completeness, authority citations, and host extension rules. Added by task 2.6.
- [`governance.md`](governance.md): policy ratchets, weakening, debt/waiver discharge, and reaffirmation approvals. Added by task 2.7.
- [`../../../schema/v1/inventory.tsv`](../../../schema/v1/inventory.tsv): machine-checkable frozen-v1 tags, fields, collections, references, lifecycle transitions, report facts, and diagnostic families.
- [`../../../schema/v1/migration-policy.toml`](../../../schema/v1/migration-policy.toml): public-constructor and historical-reader compatibility rules.
- [`../../../schema/v1/resource-budgets.toml`](../../../schema/v1/resource-budgets.toml): explicit default and maximum resource limits with semantic-identity policy.

## Precedence

The active schema identity is `spec-provenance/v1`.

When wording appears to conflict:

1. `canonical-encoding.md` controls binary field identity, digest preimages, and total ordering.
2. The object-specific document controls validation and semantic constraints.
3. `repository-inputs.md` controls path and text normalization before object validation.
4. The active fixture manifest controls demonstrated positive and negative acceptance behavior but cannot override a frozen schema requirement.
5. Design and OpenSpec artifacts explain intent but do not silently alter the product schema.

A real contradiction is a blocking schema defect. Implementations MUST NOT choose a convenient interpretation; the schema is revised and reviewed before fixtures or code proceed.

## Versioning

V1 documents are append-mostly until fixtures begin. Once phase 3 starts, changing a digest-bound field, enum tag, normalization rule, validity condition, or diagnostic meaning requires:

- an explicit schema revision decision;
- impact analysis over existing fixtures and bindings;
- updated positive and negative fixtures in the same change;
- migration behavior for retained identities; and
- no reinterpretation of bytes already identified under `spec-provenance/v1`.

Editorial changes that cannot alter parsing, validation, canonical bytes, ordering, or assurance meaning may retain v1 identity after review.
