## Why

Buoy binds every repository-owned identity to XXH3-128 at the type level, not at a boundary. `SemanticDigest` stores exactly 16 bytes. `buoy-core` calls `xxh3_128` directly in `hash_canonical_preimage`. The canonical text prefix `xxh3-128:`, the algorithm tag `2`, and the 29-byte binary envelope are fixed constants in `crates/buoy-core/src/value.rs`.

The hashing-policy gate makes the coupling permanent. `src/hashing_policy.rs` emits a finding for the identifiers `sha2`, `sha256`, and `blake3` anywhere in the repository, and for the active-value prefixes `sha256:` and `blake3:`.

Three consequences follow.

First, no adopter can select a cryptographic digest. XXH3-128 supplies deterministic change detection only. An adopter that needs collision resistance for tamper-evident provenance has no supported path. Requirement XXH-006 correctly forbids the claim, but the architecture also forbids the remedy.

Second, the ban did not remove SHA-256 from the repository. It moved SHA-256 into `tools/content_digest.rs`, a hand-written in-tree implementation that emits `sha-256:` bindings for the assurance ledger and the execution-evidence records. The scanner does not classify that file, because the code declares no `sha2`, `sha256`, or `blake3` identifier and because `sha-256:` does not match the banned `sha256:` prefix. The implementation carries a self-test against two NIST vectors, and nothing more. A second hashing implementation therefore sits in the evidence path, outside every boundary that the policy gate enforces.

Third, the algorithm choice cannot be tested as a variable. Every fixture, vector file, and property test in `fixtures/xxh3/v2/` assumes one algorithm. A future algorithm change would repeat this whole exercise instead of reusing a boundary.

The timing argument is the decisive one. No repository consumes Buoy yet. Every workspace crate sets `publish = false`, and Chelis remains a planned integration rather than a live one. The only digest-bearing artifacts in existence are this repository's own generated evidence files, and a generator run reproduces all of them.

The cost of this change is therefore near its permanent minimum today. Each future adopter, and each release, raises it. A repository whose entire purpose is provenance identity must settle how that identity is chosen before anything depends on the answer.

This change moves the algorithm from a hard-coded constant to a closed registry with one pinned active member. It does not change the active algorithm. XXH3-128 remains the default.

## What Changes

- Add a closed digest-algorithm registry in `buoy-core` that binds each member to a canonical name, a stable numeric tag, and an exact output length. Reserve tag `2` for `xxh3-128`, tag `3` for `blake3-256`, and tag `4` for `sha-256`. Keep tags `0` and `1` permanently obsolete.
- **BREAKING**: Widen `SemanticDigest` from a fixed `[u8; 16]` field to an algorithm-bound length of at most 32 bytes. Replace the fixed 29-byte canonical binary envelope with an algorithm-derived length.
- **BREAKING**: Replace the `SemanticDigest::xxh3_128` constructor and the hard-coded `xxh3-128:` parse prefix with registry-mediated construction and parsing. Parsers accept every registered member. Computation uses only the active member.
- Add one active-algorithm selection that the project configuration declares and that every canonical preimage binds. Compile each hasher behind its own Cargo feature, and default to `xxh3-128` alone, so the current dependency closure does not change unless an adopter opts in.
- **BREAKING**: Bump the canonical schema from `spec-provenance/v2` to `spec-provenance/v3`, and regenerate every repository-owned digest from authoritative source bytes. The break is contained to this repository, because no external artifact carries a v2 identity.
- Re-scope the hashing policy from "XXH3-128 only" to "one pinned active algorithm, reached only through the core registry". Direct hashing-crate calls remain a finding everywhere except the registry's own per-algorithm adapter modules. The nondeterministic-hasher bans on `DefaultHasher` and `RandomState` stay unchanged.
- Retire the hand-written SHA-256 in `tools/content_digest.rs` and route assurance content bindings through the registry.
- Add a cross-algorithm correspondence index that records each digest-bound object's identity under every involved member, keyed by stable object ID. Store it as a committed totally ordered TSV, register it as an SPG-010 `Derivation` with a consistency oracle, verify every row by recomputation from authoritative source bytes, and forbid it as an input to identity, migration, or freshness.
- Add known-answer vectors, round-trip fixtures, and Hegel-rs properties for every registered member, and add negative fixtures for cross-algorithm confusion, wrong-length payloads, and inactive-member computation.
- **Non-goals:** changing the default active algorithm, making the algorithm a per-call or per-object parameter, accepting more than one active algorithm in one repository, comparing digests across algorithms, letting the correspondence index supply an identity that regeneration cannot produce, adding an embedded database or any binary canonical artifact, treating an external integrity value as a repository-owned identity, or claiming that a cryptographic member alone establishes authenticity or proof.

## Capabilities

### New Capabilities

- `digest-algorithm-registry`: the closed algorithm registry, the algorithm-agnostic digest representation, the pinned active-member selection and its binding into every canonical preimage, checked semantic comparison, cross-member parse and rejection behavior, the per-member assurance class, the bounded transition model, and the non-authoritative cross-algorithm correspondence index.

### Modified Capabilities

- `xxhash-policy`: XXH-001 through XXH-006 change from an XXH3-128 mandate to a registry-mediated single-active-algorithm mandate. XXH-003 gains an explicit adapter-module allowlist. XXH-006 makes the assurance boundary a per-member property rather than a global one.
- `deterministic-artifacts`: DET-004 stops naming XXH3-128 as the required active profile and instead requires that the active member, its tag, and its length come from the registry and bind into every preimage.

## Impact

- **Core**: `crates/buoy-core/src/value.rs` (digest type, parsers, canonical binary envelope, preimage construction, `raw_artifact_digest`), `crates/buoy-core/src/casefold_table.rs` (the pinned source-digest constant), and `crates/buoy-core/Cargo.toml` (per-member optional hashing dependencies).
- **Shell**: `src/hashing_policy.rs` (scanner inversion), `src/presentation.rs` (algorithm rendering), `src/migration.rs`, `src/adapters/schema.rs`, and the root `Cargo.toml`.
- **Tools**: `tools/content_digest.rs` (retire the in-tree SHA-256), `tools/check_xxh3_vectors.rs` (generalize across members), `tools/check_xxh3_migration_model.rs` (replace with the bounded transition model), `tools/check_schema_inventory.rs` (replace the single-variant assertion), `tools/check_supply_chain.rs`, `tools/check_exact_coverage.rs`, `tools/check_hardening_manifest.rs`, `tools/generate_property_evidence.rs`, and `tools/generate_v2_fixture_suite.rs`.
- **Schema and fixtures**: a new `schema/v3/inventory.tsv` and a per-member `fixtures/digest/v3/` layout. Retain `fixtures/xxh3/v2/` only for frozen v2 readers, migration, and compatibility controls.
- **Supply chain**: enabling a non-default member adds `blake3` or `sha2` to the closure. `deny.toml`, `assurance/supply-chain/v1/closure-inventory.tsv`, and the license inventory need new rows. The default build closure does not change.
- **Assurance**: every `xxh3-128:` value in the committed evidence regenerates under v3. These files are generated, so the work is a generator run plus a reviewed diff, not a hand migration. The requirement IDs `XXH-001` through `XXH-006` stay stable, so `assurance/v1/obligation-ledger.tsv` and `assurance/v1/scenario-requirement-executions.tsv` keep their keys and only rebind their digests.
- **Formal proofs**: the Verus and AeneasVerif spikes in `proofs/` bind exact source digests. The kernel change invalidates those bindings and requires re-running the selected lane.
