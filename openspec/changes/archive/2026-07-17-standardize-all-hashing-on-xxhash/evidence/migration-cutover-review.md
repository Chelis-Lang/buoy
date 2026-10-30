# Version 2 migration cutover review

Status: reviewed task 5.5 evidence, 2026-07-17.

## Deterministic migration report

[`deterministic-migration-report.tsv`](deterministic-migration-report.tsv) is generated offline by:

```text
cargo run --quiet --bin spec-provenance-migrate -- complete fixtures/xxh3/v2/migration/complete-v1-input.toml
```

The hashing-policy gate regenerates this report and compares it byte-for-byte. The reviewed report contains:

- 12 semantic objects in dependency order 0 through 11;
- all 12 supported semantic object/graph/report kinds represented;
- one preserved terminal ID, `ATOM-MIGRATION-RETIRED`, still a `superseded` tombstone with its declared replacement;
- 16 changed identities covering one configuration, 12 objects, and three approval records; and
- exactly 16 stable-ID-ordered impact dispositions: 10 `revised`, four `reaffirmed`, one `retired`, and one `waived`.

Every v2 output is regenerated from authoritative bytes and typed fields. Old values remain visibly labeled `sha256:` only in the migration-impact source column. No old digest bytes are truncated, padded, transformed, or relabeled. Focused negatives reject missing bytes/approvals, duplicate IDs, stale/cyclic references, illegal lifecycle state, partial/duplicate dispositions, and mixed versions.

## Schema and tool-lock changes

| Surface | V1 archive | Active v2 |
|---|---|---|
| Semantic schema/envelope | `spec-provenance/v1`, encoding 1, algorithm tag 1 | `spec-provenance/v2`, encoding 2, algorithm tag 2 |
| Digest value | 32-byte SHA-256 | 16-byte full XXH3-128, high64-be then low64-be |
| Canonical text | archival `sha256:` plus 64 lowercase hex | active `xxh3-128:` plus 32 lowercase hex |
| Schema inventory | `schema/v1/` retained | `schema/v2/inventory.tsv` and `resource-budgets.toml` |
| Production/reference dependency | archived SHA path at baseline commit | exact `xxhash-rust =0.8.17`, defaults off, feature `xxh3` |
| Unicode support identity | archived v1/Nix metadata | `xxh3-128:9b444623559ccd1a6fb469ec6af6c018` for repository-owned semantic source identity |
| Proof/derivation records | immutable v1 identities | `proofs/core/*.toml` schema and bindings v2 |
| Fixtures/reports | `fixtures/v1/`, `fixtures/hardening/v1/` retained | `fixtures/v2/`, `fixtures/hardening/v2/`, and `fixtures/xxh3/v2/` |

The complete rollback baseline is commit `9b1b7740c390aeaa58b491b2cefc9ee8c694fce2`. Rollback restores that whole commit/lock/schema/data state; it never converts v2 output backward.

## Dependency and license inventory

The only project hashing package selected by workspace metadata is:

| Package | Version/features | Normal transitive dependencies | Registry checksum field | Declared license |
|---|---|---:|---|---|
| `xxhash-rust` | exact `0.8.17`; defaults disabled; `xxh3` only | none | `985eec839aaf2a1270af8f4ebcf63cf9401cfd90f0902f97c28d9f104ffbde72` | Boost Software License 1.0 (`BSL-1.0`) |

It is a normal dependency of the `no_std` core and reference crate and a root development dependency for vector/property checks. `std`, `xxh32`, `xxh64`, and const/custom variants are not selected. The Cargo checksum is opaque registry metadata, not a semantic digest. Workspace metadata selects no `sha2` or `blake3` package/feature.

The low-level vector anchor is upstream xxHash v0.8.3 (`Cyan4973/xxHash@e626a72bc2321cd320e953a0ccf1584cad60f363`). Upstream library code is BSD-2-Clause; the upstream sanity test C source is marked GPL-2.0-only and the generated vector header has no displayed SPDX header. The repository retains attribution and numeric expected-value provenance and does not claim that the Rust crate's BSL license covers upstream test sources. This records observed metadata and is not legal advice.

## Downstream cutover

[`docs/schema/v2/adapters-and-integration.md`](../../../../../docs/schema/v2/adapters-and-integration.md) is the downstream procedure. Consumers must regenerate all digest-bearing leaves and dependents from authoritative bytes, preserve IDs/lifecycle/tombstones/approvals, invalidate v1 cache keys, produce complete impact dispositions, and switch readers/writers/examples atomically. An obsolete or mixed binding is an error, not a compatibility input.

## Review conclusion

The report's ordering, stable IDs, terminal state, references, digest forms, and one-to-one impact closure were reviewed against the fixture and production tests. The dependency/feature policy, external metadata boundary, v2 generator checks, and deterministic report comparison are gate-enforced. This review does not claim cryptographic strength, collision impossibility, source-to-proof correspondence, ecosystem checksum validity, or correctness beyond the tested/modelled scopes.
