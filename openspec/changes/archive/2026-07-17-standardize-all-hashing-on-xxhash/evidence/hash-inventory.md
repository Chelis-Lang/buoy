# Version 1 Hashing Inventory

Status: migration input for task 1.1. This inventory describes the repository as scanned on 2026-07-17 before the version 2 hashing implementation. Paths are repository-relative. `schema/v1/inventory.tsv` remains the exhaustive field/domain authority; this document classifies its hashing surfaces and every SHA/BLAKE3 occurrence found by a case-insensitive repository scan.

## Scan boundary

The inventory covered tracked and untracked repository files while excluding `.git/`. The scan classes were:

- algorithm spellings: `sha2`, `sha-?256`, `sha-?512`, and `blake-?3`;
- hashing APIs and entry points: `SemanticDigest`, `DigestAlgorithm`, `semantic_digest`, `Hasher`, `HashMap`, `HashSet`, `fingerprint`, and `cache key`;
- ecosystem metadata fields: Cargo `checksum`, Nix `hash`/`cargoHash`/`narHash`, Git `rev` and lockfile source fragments, release `tag`/`sourceCommit`/`archiveHash`, and signature fields;
- the resolved workspace dependency and feature graph from `cargo metadata` and `cargo tree`.

No BLAKE3 crate, API call, active digest, fixture value, cache/fingerprint implementation, project-selected hash-table hasher, `HashMap`, or `HashSet` exists. BLAKE3 appears only in migration requirements, negative-case planning, and the archived v1 algorithm-migration discussion. No SHA-512 computation or active SHA-512 value exists.

## Repository-owned computations and dependencies

| Class | Exact location | Version 1 behavior | Version 2 disposition |
|---|---|---|---|
| Production computation | `crates/spec-provenance-core/src/value.rs` (`sha2::{Digest, Sha256}` and `Sha256::digest`) | Sole domain-separated semantic digest implementation | Replace with the sole XXH3-128 entry point |
| Reference computation | `crates/spec-provenance-reference/src/lib.rs` (`sha2::{Digest, Sha256}` and `Sha256::digest`) | Independent envelope encoder ending in SHA-256 | Keep envelope independence; replace low-level hash with the locked XXH3-128 crate and state the shared-hasher limitation |
| Direct dependency | `crates/spec-provenance-core/Cargo.toml` | `sha2 = "=0.10.9"`, defaults disabled | Remove |
| Direct dependency | `crates/spec-provenance-reference/Cargo.toml` | `sha2 = "=0.10.9"`, defaults disabled | Remove |
| Direct development dependency | `Cargo.toml` | `sha2 = "=0.10.9"`, defaults disabled | Remove |
| Resolved closure | `Cargo.lock` and the workspace feature graph | `sha2 0.10.9` reaches `cfg-if`, `cpufeatures`, `libc`, `digest`, `block-buffer`, `crypto-common`, `generic-array`, `typenum`, and `version_check` | Remove when no other selected dependency requires it |
| Unicode support checksum | `devenv.nix`, `tools/generate_casefold_table.rs`, `crates/spec-provenance-core/src/casefold_table.rs`, `crates/spec-provenance-core/src/value.rs`, `assurance/v1/toolchain.toml` | Project invokes `nix hash file --type sha256` and propagates the result as a support-data identity | Repository-owned check, not a fixed-output fetch exception; replace with the v2 project hash and move external metadata out of the core |

All application-controlled call sites route through `semantic_digest`; no shell crate implements SHA directly. The current call sites and resulting domains are:

- `crates/spec-provenance-core/src/model.rs`: `atom-revision`, `atom-object`;
- `crates/spec-provenance-core/src/algorithms.rs`: `spec-graph-semantic`;
- `crates/spec-provenance-core/src/report.rs`: `static-report`, `execution-report`, `verdict-set`, `impact-report`;
- `src/repository.rs`: `repository-execution-config`, `raw-artifact`;
- `src/adapters/rust.rs`: `parsed-item-identity`;
- `src/check.rs`: `oracle`;
- `src/bin/spec-provenance.rs`: `change-disposition`, `change-manifest`;
- `crates/spec-provenance-reference/src/lib.rs`: independent envelope authority used by tests and generators;
- `examples/generate_canonical_vectors.rs`, `tests/canonical_vectors.rs`, `tests/hegel_properties.rs`, and `tests/proof_bindings.rs`: generation or verification through production/reference entry points.

The complete v1 domain set subject to recomputation is:

```text
repository-execution-config, raw-artifact, repository-config,
repository-snapshot, snapshot-file, schema-tool-lock, atom-revision,
atom-object, tombstone, id-registry, decision-surface-domain,
surface-disposition, decision-surface, evidence-carrier,
parsed-item-identity, implementation-link, derivation, artifact-binding,
coverage-policy-revision, coverage-policy-object, debt-record, waiver,
reaffirmation, obligation-reference, oracle, normalized-verdict, verdict-set,
assumption, finite-model, formal-property, proof-record, source-map-entry,
source-map, spec-graph-semantic, spec-graph-complete, change-manifest,
change-disposition, impact-set, static-report, execution-report, impact-report
```

## Digest representation and fields

`crates/spec-provenance-core/src/value.rs` owns the v1 `DigestAlgorithm::Sha256`, 32-byte `SemanticDigest`, constructor, strict `sha256:` parser, renderer, binary tag `1`, binary length `32`, and canonical preimage algorithm enum `1`. `src/presentation.rs` emits the `sha256` JSON algorithm label. `src/adapters/markdown.rs`, `src/adapters/rust.rs`, and `src/bin/spec-provenance.rs` parse canonical digest text into this type.

`schema/v1/inventory.tsv` exhaustively declares the following 51 digest-bearing fields. Every listed field is a repository-owned semantic digest and must be regenerated rather than relabeled:

```text
object-reference.expected-object-digest (digest)
atom-revision-reference.expected-revision (digest)
approval-binding.record-digest (digest:raw-artifact)
tool-identity.executable-digest (digest:raw-artifact)
tool-identity.configuration-digest (digest)
id-allocation.allocation-digest (digest:raw-artifact)
repository-snapshot.repository-config-digest (digest:repository-config)
repository-snapshot.schema-tool-lock-digest (digest:schema-tool-lock)
snapshot-file.raw-artifact-digest (digest:raw-artifact)
atom-object.atom-revision-digest (digest:atom-revision)
tombstone.final-revision (digest:atom-revision)
decision-surface.domain-digest (digest:decision-surface-domain)
evidence-carrier.parsed-item-digest (digest:parsed-item-identity)
evidence-carrier.execution-config (digest)
evidence-carrier.corpora (set:digest)
parsed-item-identity.enclosing-item (option:digest:parsed-item-identity)
implementation-link.parsed-item-digest (digest:parsed-item-identity)
derivation.configuration-digest (digest)
artifact-binding.artifact-digest (digest)
coverage-policy-object.revision-digest (digest:coverage-policy-revision)
reaffirmation.base-object-digest (digest)
reaffirmation.head-object-digest (digest)
oracle.configuration-digest (digest)
proof-record.verifier-configuration (digest)
spec-graph-complete.semantic-graph-digest (digest:spec-graph-semantic)
spec-graph-complete.source-map-digest (digest:source-map)
spec-graph-complete.repository-config-digest (digest:repository-config)
spec-graph-complete.schema-tool-lock-digest (digest:schema-tool-lock)
change-manifest.base-graph (digest:spec-graph-complete)
change-manifest.head-graph (digest:spec-graph-complete)
change-disposition.base-digest (option:digest)
change-disposition.head-digest (option:digest)
change-disposition.reaffirmation-digest (option:digest:reaffirmation)
surface-fact.domain-digest (digest:decision-surface-domain)
registration-fact.parsed-item-digest (digest:parsed-item-identity)
registration-fact.scope-digest (digest)
freshness-fact.expected-bindings (set:digest)
freshness-fact.current-bindings (set:digest)
debt-waiver-fact.discharge-condition-digest (digest)
derivation-fact.scope-digest (digest)
proof-fact.scope-digest (digest)
evidence-fact.claim-scope-digest (digest)
disposition-closure-fact.base-digest (option:digest)
disposition-closure-fact.head-digest (option:digest)
static-report.complete-graph-digest (digest:spec-graph-complete)
execution-report.static-graph-digest (digest)
execution-report.verdict-set-digest (digest:verdict-set)
impact-report.base-graph (digest:spec-graph-complete)
impact-report.head-graph (digest:spec-graph-complete)
impact-report.change-manifest (digest:change-manifest)
impact-report.impact-set (digest:impact-set)
```

The Rust model additionally stores these values in object metadata, exact object references, atom revision references, approvals, tool identities, artifact indexes, graph maps, freshness facts, proof/derivation facts, reports, and impact dispositions. Those are typed realizations of the inventory rows above, not separate algorithm exceptions.

## Fixtures, generators, records, and tests

### Generated or committed active-v1 digest data

- `fixtures/hardening/v1/canonical-vectors.tsv`
- `fixtures/hardening/v1/canonical-report-goldens.tsv`
- `fixtures/hardening/v1/static-report.json`
- `fixtures/hardening/v1/static-report.txt`
- `fixtures/v1/baselines/deterministic-static.toml`
- `fixtures/v1/det/DET-004.toml`
- `fixtures/adapters/markdown/positive.md`
- `fixtures/adapters/rust/positive.rs`
- `fixtures/adapters/rust/links.rs`
- `proofs/core/derivation-record.toml`
- `proofs/core/proof-record.toml`

`fixtures/v1/README.md` describes the version 1 corpus and is retained only as archival migration context after cutover.

### Generators

- `examples/generate_canonical_vectors.rs` computes reference SHA-256 and formats `sha256:`.
- `examples/generate_report_goldens.rs` constructs 32-byte SHA values and emits report goldens.
- `examples/generate_presentation_goldens.rs` constructs 32-byte SHA values and emits presentation goldens.
- `tools/generate_casefold_table.rs` copies the project-owned Unicode SHA support checksum into generated core data.

### Tests containing v1 constructors or expected values

- `tests/core_contract.rs`
- `tests/hegel_properties.rs`
- `tests/adapters.rs`
- `tests/integration.rs`
- `tests/canonical_differential.rs`
- `tests/report_encoding.rs`
- `tests/proof_bindings.rs`
- `tests/canonical_vectors.rs` (indirectly checks committed SHA values)

These are positive v1 data today. After cutover, obsolete spellings may remain only in explicitly negative parser/migration/policy fixtures.

## Documentation and planning claims

The following files assert or explain SHA-256 as the active/frozen v1 identity and therefore require v2 updates or an explicit archival label:

- `README.md`
- `docs/schema/v1/README.md`
- `docs/schema/v1/atoms.md`
- `docs/schema/v1/canonical-encoding.md`
- `docs/testing/golden-and-mutation-policy.md`
- `docs/assurance/trust-boundaries.md`
- `docs/tooling/hegel-rs.md`
- `docs/tooling/hegel-properties.md`
- `docs/tooling/formal-toolchains.md`
- `docs/tooling/reference-models.md`
- `openspec/specs/deterministic-artifacts/spec.md`
- `openspec/changes/establish-spec-provenance-system/design.md`
- `openspec/changes/establish-spec-provenance-system/tasks.md`
- `openspec/changes/establish-spec-provenance-system/specs/deterministic-artifacts/spec.md`
- `openspec/changes/harden-spec-provenance-production-readiness/specs/canonical-report-encoding/spec.md`
- `openspec/changes/add-cargo-llvm-cov-hegel-rs-testing-requirements/design.md`

The current change's proposal, design, specs, tasks, and this inventory intentionally mention SHA/BLAKE3 as migration explanation or prohibited/negative values; they are not active identity claims.

## Exact external-integrity exceptions

These values are opaque imperative-shell or ecosystem metadata. The exception applies only to the exact syntax/field shown; it is not a path-wide allowlist.

| Ecosystem | Exact accepted syntax and field | Current locations | Boundary |
|---|---|---|---|
| Cargo registry | `[[package]]` with registry `source = "registry+…"` and `checksum = "<64 lowercase hex>"` | `Cargo.lock`; `dylint_lints/fcis_core/Cargo.lock`; fixture Cargo lockfiles | Cargo alone verifies; never parse as `SemanticDigest` |
| Nix flake input | JSON node field `locked.narHash = "sha256-<Nix base64>"` | `devenv.lock` | Nix alone verifies |
| Nix fixed-output fetch | `fetchurl`/`fetchFromGitHub`/`rustPlatform.buildRustPackage` fields `hash` or `cargoHash` with SRI text | `nix/formal-tools.nix`; `nix/dylint-tools.nix` | Nix alone verifies; inherited aliases such as `archiveHash`/`sourceHash` remain opaque presentation of the same fetch hash |
| Git source | Cargo dependency `rev = "<40 lowercase hex>"`; lock source `git+<URL>?rev=<hex>#<hex>` | `dylint_lints/fcis_core/Cargo.toml`; `dylint_lints/fcis_core/Cargo.lock` | Git/Cargo object identity only |
| Upstream release/source | `tag`, `sourceCommit`, release URL/tag, and archive/source hash fields attached to a fetch declaration | `nix/formal-tools.nix` | Opaque provenance/fetch metadata, not a semantic digest |
| Signature metadata | No matching repository field or value found | none | No exception exists until an exact schema field is introduced |

The SHA value used for `unicode/16.0.0/CaseFolding.txt` is deliberately excluded from this table: it is currently computed by a project task and propagated into core/support records, so it is repository-owned and must migrate.

## Cache and hash-table conclusion

No repository source defines a digest-derived cache key, fingerprint helper, project-selected `BuildHasher`, `HashMap`, or `HashSet`. `SemanticDigest` derives Rust's generic `Hash` trait, but no repository-owned hash-table construction consumes it. Transitive crates named `hashbrown` and `rustc-hash` occur only in resolved third-party/tooling graphs and are not semantic digest algorithms; the later dependency-policy checker must classify their reachability and selected features rather than reject their names as substrings.
