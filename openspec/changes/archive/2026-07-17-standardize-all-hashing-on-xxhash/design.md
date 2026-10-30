## Context

The current frozen version 1 canonical encoding uses SHA-256 for semantic revisions, raw-artifact bindings, graph and report identities, freshness references, proof and derivation records, fixtures, and independent reference checks. The algorithm is represented in public Rust types, binary tags, canonical text, JSON presentation, documentation, manifests, and committed goldens. Changing it is therefore an identity-schema migration rather than a dependency substitution.

The requested policy is that every project-controlled hashing operation use xxHash and that project code neither compute SHA-family nor BLAKE3 values. Some supply-chain systems prescribe their own integrity formats: Cargo registry checksums, Nix fixed-output/SRI hashes, Git object IDs, upstream release checksums, and signature metadata cannot be changed to xxHash by this project. The design treats those values as opaque external metadata and prevents them from becoming Spec Provenance identities.

Authoritative migration inputs are repository paths and bytes, project configuration, schema identities, parser/tool identities, approval records, and the existing legal lifecycle state. Filesystem traversal order, clocks, network state, locale, process completion order, host paths, and provider state remain non-authoritative. Static graph registration and freshness remain independent from effectful oracle execution, normalized verdicts, and assurance classification.

## Goals / Non-Goals

**Goals:**

- Define one exact repository-owned hash profile: unseeded XXH3-128, meaning seed `0` with the standard default secret.
- Move active identities to a version 2 schema with explicit binary and text forms and no algorithm negotiation.
- Cover all project-controlled semantic digests, artifact bindings, fingerprints, cache keys, fixtures, tools, reference checks, and future project-selected hash-table hashers.
- Remove direct and transitive project hashing paths based on SHA or BLAKE3 and enforce the boundary in the authoritative repository gate.
- Rebuild digest-bearing data deterministically from authoritative source bytes while preserving stable IDs and legal lifecycle states.
- Preserve the `no_std` deterministic functional core, one-way dependency direction, canonical total ordering, fail-closed diagnostics, and evidence/proof distinctions.

**Non-Goals:**

- Provide cryptographic collision resistance, preimage resistance, authenticity, tamper evidence, signatures, or content-addressed security with xxHash.
- Replace ecosystem-prescribed Cargo, Nix, Git, registry, release, or signature integrity formats.
- Reinterpret old digest bytes, support a runtime algorithm selector, or emit mixed-algorithm active graphs.
- Claim that differential tests, official vectors, annotations, migration records, Verus scaffolding, or Aeneas translation prove the XXH3 implementation, collision properties, rustc, LLVM, parsers, operating systems, or natural-language correspondence.
- Change the execution model for effectful oracles or allow oracle availability to affect static identities.

## Decisions

### 1. Standardize on the official XXH3-128 profile

Repository-owned hashing uses XXH3-128 with seed `0` and the standard default secret. The full 128-bit output is retained. Canonical binary bytes use the official XXH128 canonical order: the high 64-bit half in big-endian order followed by the low 64-bit half in big-endian order. Canonical text is `xxh3-128:` followed by exactly 32 lowercase hexadecimal digits.

XXH3-128 is selected over XXH32, XXH64, and XXH3-64 because it remains within the requested family while reducing accidental-collision probability. A custom seed or secret is rejected because it would create hidden algorithm variants and cross-implementation ambiguity. BLAKE3 and SHA-family alternatives are rejected by policy.

The implementation dependency must be an exact Cargo pin with default features disabled and only the `no_std`-compatible XXH3 feature enabled. Its exact source and feature closure are recorded in the schema/tool lock. Official upstream XXH3-128 vectors, including empty, short, stripe-boundary, long, and multi-update inputs, lock the algorithm and byte-order interpretation before project goldens are regenerated.

### 2. Make the change an explicit version 2 identity break

Version 2 uses:

- semantic schema ID `spec-provenance/v2`;
- encoding version `2`;
- digest algorithm enum/binary tag `2`;
- a 16-byte digest payload; and
- `xxh3-128:<32 lowercase hex>` text.

The existing domain names and canonical value algebra remain structurally stable unless a separate schema change requires otherwise, but the version and algorithm fields in every preimage ensure that no version 1 identity equals a version 2 identity by reinterpretation. Active parsers accept only the version 2 profile. SHA, BLAKE3, unknown, uppercase, truncated, padded, and wrong-length forms produce stable structured diagnostics.

A dual-read/dual-write active mode was rejected because it would make equality, ordering, freshness, and graph closure depend on negotiation. Historical version 1 material may be retained outside active version 2 inputs as archival bytes, but active objects are regenerated from authoritative source rather than converted from an old digest. When both canonical preimages are present and equal digest bytes are observed for unequal preimages, construction fails with an explicit deterministic collision diagnostic; this detects observed collisions but is not a cryptographic guarantee.

### 3. Keep one hashing entry point in the functional core

`spec-provenance-core` remains `#![no_std]`, safe, sequential, and free of ambient effects. It owns the canonical preimage envelope, XXH3-128 invocation, digest value, parser, rendering, and comparison semantics. The dependency closure must continue to build for the configured bare-metal target. It may use invocation-local mutation but no ambient mutable state or callable public inputs.

The root package, adapters, CLI, parsers, migration driver, policy scanner, and effectful oracle runners remain in the imperative shell. Shell crates depend inward on the core and MUST NOT reimplement or select a semantic hasher. They supply explicit bytes and typed fields to the core. The core never depends on shell or adapter crates and never reads files, environment, clocks, network, locale, processes, or provider state.

`spec-provenance-reference` remains independent of the production core and independently encodes preimages. Differential tests compare its outputs with production outputs. If both paths use the same pinned low-level XXH3 crate, the documentation states that differential evidence covers envelope and integration independence, while official vectors—not differential agreement—anchor the low-level algorithm result. A separate hashing implementation is not claimed unless its source and dependency independence are demonstrated.

### 4. Enforce complete scope with a syntax-aware inventory

A repository-owned checker inventories Cargo metadata and dependency features plus Rust source, project schemas, fixtures, proof and derivation records, generated active values, tools, scripts, and normative documentation. It classifies findings by semantic role rather than banning substrings. The checker rejects:

- SHA/BLAKE3 hashing crates or transitive features reachable from project hashing paths;
- project calls or implementations that compute prohibited algorithms;
- active digest fields with prohibited prefixes, tags, or sizes;
- generators, tests, examples, cache/fingerprint helpers, or reference paths using another algorithm; and
- documentation claiming SHA/BLAKE3 is an active Spec Provenance identity.

External exceptions are field- and syntax-specific: for example, Cargo's `checksum` field and a Nix fetcher's fixed-output `hash` field. A whole-file or whole-directory exemption is forbidden. External values remain opaque shell metadata, are never parsed as `SemanticDigest`, and may enter canonical data only where a schema explicitly binds them as opaque text. Diagnostic records use normalized repository-relative paths, stable codes, structured fields, and the existing total order; all findings are emitted before the gate fails.

### 5. Regenerate records in dependency order

Migration begins from a clean, archived version 1 snapshot and runs offline. It performs these phases:

1. Add version 2 value types, preimage envelope, parser diagnostics, official vectors, and negative malformed vectors.
2. Update the independent reference path and establish production/reference parity for version 2 encoding.
3. Regenerate leaf raw-artifact and configuration bindings from authoritative bytes.
4. Regenerate semantic objects, references, carriers, policies, derivations, assumptions, formal properties, and proof records in dependency order.
5. Regenerate canonical graphs, reports, fixtures, presentation goldens, proof-binding records, and documentation examples.
6. Run complete impact closure so every changed binding is explicitly revised, reaffirmed, retired, or waived under the active policy.
7. Enable the hashing-policy checker and remove obsolete hashing dependencies and project-owned active-schema values.

Stable IDs do not change merely because the algorithm changes. Lifecycle transitions remain governed by their existing state machine: migration preserves live, deprecated, superseded, and withdrawn states; it cannot reactivate a terminal ID, erase a tombstone, invent a replacement, or bypass an approval. Missing authoritative bytes, missing approval, duplicate IDs, stale nested bindings, or incomplete impact dispositions fail closed in deterministic total order.

Rollback restores the complete pre-migration version 1 snapshot and tooling together. It does not relabel version 2 data as version 1, mix versioned records, or retain partially migrated active graphs.

### 6. Preserve trust and assurance boundaries

XXH3-128 equality is used only as deterministic change-detection and binding evidence. Registration, freshness, normalized execution verdict, and assurance class remain separate fields and calculations. A matching digest can support freshness but cannot establish authenticity, correspondence, successful execution, exhaustive coverage, or formal proof.

Formal work may verify selected pure envelope construction, canonical encoding, byte-order conversion, ordering, and state-transition wrappers against exact source and tool bindings. Verus and AeneasVerif results do not prove the third-party XXH3 implementation, absence of collisions, compiler correctness, generated machine code, external package managers, parsers, operating systems, or natural-language meaning. Any authenticity requirement must use a separately specified signature or trust mechanism whose ecosystem-prescribed integrity metadata remains outside the semantic digest type.

### 7. Use one authoritative acceptance oracle

The exact acceptance oracle is:

```text
devenv tasks run spec-provenance:gate
```

Implementation is accepted only when this command exits `0` offline and the gate includes formatting/linting, all workspace targets and features, the bare-metal `no_std` core build, official XXH3 vectors, malformed and migration negative fixtures, production/reference differential tests, regenerated golden checks, schema inventory, proof/derivation freshness checks, and the repository hashing-policy inventory with no unclassified or prohibited findings. Expected negative fixtures must fail with their exact stable diagnostic codes; their failure is evidence of detector behavior, not proof of completeness.

## Risks / Trade-offs

- **[XXH3-128 is non-cryptographic and has a smaller output than SHA-256]** → State the limitation in schemas and reports, reject cryptographic claims, retain all 128 bits, detect observed same-digest/different-preimage pairs when both preimages are available, and use a separate mechanism for authenticity.
- **[Every existing repository-owned digest changes]** → Treat the cutover as version 2, regenerate from authoritative bytes in dependency order, require complete impact dispositions, and prohibit partial or in-place conversion.
- **[Crate APIs may disagree about 128-bit half/byte ordering]** → Define official XXH128 canonical order explicitly and lock it with upstream boundary vectors and incremental/one-shot parity tests.
- **[A textual ban would reject mandatory lockfile and Nix hashes]** → Use syntax-aware, field-specific external exceptions and reject path-wide allowlists.
- **[A transitive dependency can reintroduce prohibited hashing]** → Inspect the resolved dependency and feature graph in the authoritative gate, not manifests alone.
- **[Shared low-level xxHash code weakens differential independence]** → Make the limitation explicit, anchor the low-level result in official vectors, and restrict differential claims to independently implemented envelopes unless a genuinely independent hasher is added.
- **[Formal backends may not support or scale to XXH3 internals]** → Keep the hash implementation outside claimed proof scope and prove only selected wrappers and invariants with exact exclusions.
- **[Rollback can create mixed active records]** → Roll back schema, code, records, fixtures, and tools as one snapshot; reject mixed algorithm tags and schema versions.

## Migration Plan

1. Archive and validate a clean version 1 baseline.
2. Implement and test version 2 digest primitives and diagnostics without enabling mixed active output.
3. Update the reference path and policy checker; prove their positive and negative fixtures first.
4. Regenerate all digest-bearing artifacts from authoritative bytes in dependency order and review complete change impact.
5. Remove prohibited project hashing dependencies and paths, update downstream examples, and switch the active schema atomically.
6. Run the authoritative acceptance oracle offline and retain the migration report as review evidence.
7. If acceptance fails after cutover, restore the complete version 1 baseline; do not convert version 2 bytes backward.

## Open Questions

None. The algorithm variant, seed/secret, canonical byte order, schema version, external-integrity boundary, migration strategy, and acceptance oracle are fixed by this design.
