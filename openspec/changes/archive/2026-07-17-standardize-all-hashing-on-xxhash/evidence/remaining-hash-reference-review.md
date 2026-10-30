# Remaining SHA/BLAKE3 reference review

Status: post-cutover task 5.4 review, 2026-07-17.

A case-insensitive repository scan for `sha2`, `sha-256`, `sha256`, and `blake3` was reviewed together with the syntax-aware Rust/TOML/documentation scan and resolved `cargo metadata --locked --offline` feature graph. The active policy checker reports zero findings. No workspace manifest, lock package, resolved feature, Rust identifier/path, command invocation, generated v2 artifact, proof binding, report, presentation, or active semantic fixture computes or accepts SHA-family or BLAKE3 identity.

## Allowed remaining classes

| Class | Exact scope | Why retained |
|---|---|---|
| External Cargo registry/upstream metadata | Cargo `checksum`; declared upstream `checksum_algorithm`, `checksum`, and `signature` fields; package-checksum tables in `docs/tooling/hegel-rs.md` | Opaque values interpreted by Cargo or the named upstream release process, never `SemanticDigest` |
| External Nix metadata | `locked.narHash` in `devenv.lock`; `hash`, `sourceHash`, and `cargoHash` in reviewed Nix fetch/derivation syntax; corresponding exact tables in `docs/tooling/formal-toolchains.md` | Opaque Nix fixed-output/SRI fields verified by Nix |
| Immutable v1 archive | `schema/v1/`, `docs/schema/v1/`, `fixtures/v1/`, `fixtures/hardening/v1/`, `assurance/v1/`, and baseline commit evidence | Preserves original labels/bytes for audit and whole-baseline rollback; never active input |
| Migration history and boundary | Superseded v1 OpenSpec decisions; this change's proposal/design/delta/evidence; `src/migration.rs`; `tools/generate_v2_fixture_suite.rs`; `fixtures/xxh3/v2/migration/complete-v1-input.toml`; v2 migration documentation | Names the obsolete source shape so authoritative bytes can be regenerated; never transforms or relabels old digest bytes |
| Negative fixtures and controls | `fixtures/xxh3/v2/{malformed-digest-vectors.tsv,policy/negative,external/negative}`; policy/migration case tables; adapter, migration, and Hegel rejection tests; exact marker inventories in their checkers | Must spell prohibited forms to prove deterministic rejection |
| Policy classifier literals | `src/hashing_policy.rs`, `src/bin/spec-provenance-hashing-policy.rs`, `src/external_integrity.rs`, and `tools/check_xxh3_vectors.rs` | Syntax-aware detection, metadata negative controls, and exact Nix-field recognition; literals are not hashing APIs |

`fixtures/v2/` retains `reinterpret-sha256-identity-under-other-algorithm` only as the name of a negative mutation. Its active deterministic baseline declares `digest_algorithm = "xxh3-128"` and the accepted fact is `digest-algorithm=xxh3-128`.

## Enforcement evidence

- `spec-provenance-hashing-policy --self-test`: all 18 positive/negative syntax fixtures plus two resolved-feature controls pass.
- `spec-provenance-hashing-policy`: zero prohibited or unclassified active findings.
- External-integrity tests accept only exact Cargo/Nix/Git/upstream field syntax and reject broad exceptions and semantic promotion.
- Workspace metadata contains only the exact `xxhash-rust 0.8.17` `xxh3` project hashing dependency; no `sha2` or `blake3` package/feature is selected.
- Active digest parsers accept only algorithm tag 2/16 bytes or lowercase `xxh3-128:` plus 32 hex digits; obsolete forms occur only in rejection and migration paths.

This review does not claim that text search proves program behavior. The zero result is combined with parsed Rust syntax, parsed manifests/metadata, fixture negatives, external-field parsers, and active parser tests. External checksum validity remains the owning ecosystem's responsibility.
