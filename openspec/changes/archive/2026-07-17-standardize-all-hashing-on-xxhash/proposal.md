## Why

The repository currently binds canonical identities and several supporting artifacts to SHA-256, while the requested hashing policy is to use xxHash consistently and to prohibit repository-owned SHA or BLAKE3 hashing. A single versioned algorithm contract will remove mixed hashing behavior and make every application-controlled digest predictable across the core, reference implementation, adapters, tests, records, and documentation.

## What Changes

- **BREAKING** Introduce a new semantic schema/domain version whose only supported repository-owned digest algorithm is XXH3-128, encoded as the full 128-bit value with explicit algorithm identity.
- **BREAKING** Recompute all application-controlled semantic identities, raw-artifact digests, proof and derivation bindings, fixture digests, cache keys, and test vectors from canonical preimages; existing `sha256:` identities are not reinterpreted as xxHash identities.
- Replace SHA-based hashing in the deterministic core and independent reference implementation with independently exercised XXH3-128 implementations and remove runtime/development SHA and BLAKE3 hashing dependencies.
- Reject SHA, BLAKE3, unknown, truncated, incorrectly sized, and noncanonical digest forms wherever the active schema accepts a repository-owned digest.
- Add a deterministic migration procedure that regenerates affected records from authoritative source bytes and emits explicit diagnostics when only an obsolete digest is available.
- Add a repository policy check that prevents new repository-owned SHA/BLAKE3 hashing code, dependencies, schema values, fixtures, or documentation claims.
- Preserve domain separation, schema binding, unambiguous encoding, deterministic ordering, static checking, and the distinction between evidence and proof.
- Treat externally mandated package-manager and fetcher integrity fields (for example Cargo registry checksums and Nix fixed-output/SRI hashes) as opaque supply-chain metadata, not application hashing. They are not converted or presented as Spec Provenance identities because those ecosystems do not permit substituting xxHash.

### Goals

- Make XXH3-128 the sole algorithm used for every digest computed or validated under repository-owned schemas and workflows.
- Remove SHA and BLAKE3 implementations from project-owned runtime, development, and reference hashing paths.
- Make the breaking identity migration explicit, deterministic, testable, and fail-closed.

### Non-goals

- Claim cryptographic collision, preimage, authenticity, signature, or tamper-resistance properties for xxHash.
- Replace externally prescribed integrity algorithms in Cargo, Nix, Git, registries, release metadata, or upstream toolchain records.
- Treat scaffolding, tests, annotations, digest equality, or migration output as mathematical proof.
- Change effectful oracle execution or permit ambient state to influence static identities.

## Capabilities

### New Capabilities

- `xxhash-policy`: Defines the repository-wide XXH3-128 algorithm profile, prohibited hashing algorithms, migration behavior, enforcement checks, and external-integrity boundary.

### Modified Capabilities

- `deterministic-artifacts`: Replaces the active repository-owned SHA-256 identity requirement with versioned, domain-separated XXH3-128 identities and explicit fail-closed migration rules.

## Impact

- Affected public API and formats include `DigestAlgorithm`, `SemanticDigest`, canonical binary digest tags and lengths, canonical text, parser diagnostics, presentation JSON, schema/domain identifiers, and all digest-bearing records.
- Affected implementation areas include `spec-provenance-core`, the independent reference crate, adapters, report rendering, tools, tests, fixtures, proof/derivation records, assurance metadata, and hashing-policy documentation.
- Cargo manifests and lockfiles will remove direct `sha2`/BLAKE3 hashing dependencies and add a pinned `no_std`-compatible XXH3-128 implementation or a reviewed in-tree equivalent.
- Every repository-owned golden digest and freshness binding changes. Downstream consumers must migrate atom revisions and other digest-bearing data instead of reusing v1 values.
- External package/fetch integrity metadata may still contain ecosystem-defined SHA strings, but project code must not compute those values or accept them as canonical Spec Provenance identities.
