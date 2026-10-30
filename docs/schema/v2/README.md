# Buoy schema v2

Status: frozen historical schema. Schema v3 is the active profile.

## Historical contract

The v2 schema identity is `spec-provenance/v2`. Its frozen machine inventory is [`../../../schema/v2/inventory.tsv`](../../../schema/v2/inventory.tsv). Its decoder budgets are in [`../../../schema/v2/resource-budgets.toml`](../../../schema/v2/resource-budgets.toml).

- [`canonical-hashing.md`](canonical-hashing.md) freezes the canonical digest value and semantic preimage profile.
- [`migration.md`](migration.md) defines the breaking v1-to-v2 cutover and rollback boundary.
- [`adapters-and-integration.md`](adapters-and-integration.md) defines v2 digest parsing and cutover boundaries.
- [`../v1/`](../v1/) is the immutable v1 specification. Its digest shape, preimage, active-schema statements, and reports are obsolete for v2.

## Precedence

1. `canonical-hashing.md` controls digest values, preimage envelopes, byte ordering, text rendering, and observed-collision behavior.
2. `schema/v2/inventory.tsv` controls semantic fields, tags, collections, ordering, references, lifecycle, report facts, and diagnostics.
3. `schema/v2/resource-budgets.toml` controls decoder resource limits.
4. The frozen v2 fixtures and XXH3 vectors demonstrate acceptance and rejection. They cannot override the frozen contract.
5. Version 1 and OpenSpec artifacts explain history. They cannot change or relax v2.

A contradiction is a blocking schema defect. Implementations must fail closed rather than select a convenient version, algorithm, length, or ordering.
