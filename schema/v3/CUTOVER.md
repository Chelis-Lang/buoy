# Schema v3 Cutover

Schema v3 replaces the numeric source item kind with the `SourceItemKind` newtype.

This release uses a breaking cutover. It does not read, migrate, or reinterpret an older parsed-item identity.

Adoption requires source reingestion. The reingestion process creates new parsed-item identities and dependent graph digests.

Each adapter descriptor declares its complete source-item kind set. Materialization rejects a kind outside that set.

The built-in kinds are:

- `markdown/fence/v1`
- `markdown/external-trace/v1`
- `openspec/requirement/v1`
- `openspec/scenario/v1`
- `rust/item/v1`

External adapters can add versioned kinds without a new core enum variant.
