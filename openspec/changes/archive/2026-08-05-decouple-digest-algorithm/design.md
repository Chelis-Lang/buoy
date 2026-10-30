## Context

`buoy-core` is a `no_std` deterministic kernel. It computes every repository-owned identity through one private function, `hash_canonical_preimage`, which calls `xxhash_rust::xxh3::xxh3_128` directly. The result flows into `SemanticDigest`, a `Copy` type with a fixed `[u8; 16]` field and `const` constructors.

Four constants encode the algorithm choice into the wire format:

- The canonical text prefix `xxh3-128:` in `parse_canonical` and `canonical_text`.
- The algorithm tag `2_u16` in `canonical_binary` and `parse_canonical_binary`.
- The declared payload length `16_u16` and the total envelope length `29` in the same two functions.
- The algorithm tag `2_u16` inside `semantic_preimage_from_encoded_record`, which binds the algorithm into every digest preimage.

The last constant matters most. The algorithm identity is already part of the preimage. The design therefore extends an existing binding rather than introducing a new one.

Schema v1 used tag `1` for a 32-byte `sha256` digest. Schema v2 replaced it with tag `2` for a 16-byte `xxh3-128` digest. A variable-length digest is therefore a shape the schema inventory carried before, not a new concept.

No repository consumes Buoy yet. Every workspace crate sets `publish = false`, and Chelis remains a planned integration. The only v2 artifacts that exist are this repository's own generated evidence files. That fact removes the compatibility constraint that would otherwise dominate this design, and it settles the schema-version decision below.

The enforcement side is `src/hashing_policy.rs`. It scans Rust sources, Cargo manifests, resolved dependency metadata, documentation, and generated values. It rejects the identifiers `sha2`, `sha256`, and `blake3`, the prefixes `sha256:` and `blake3:`, and the nondeterministic hashers `DefaultHasher` and `RandomState`.

The scanner does not catch `tools/content_digest.rs`. That file implements SHA-256 by hand and emits `sha-256:` bindings that the assurance ledger and the execution-evidence records consume today. It declares no banned identifier, and its prefix spelling does not match the banned `sha256:` prefix. Its only correctness evidence is a self-test against two NIST vectors.

## Goals / Non-Goals

**Goals:**

- Move the algorithm choice from a type-level constant to a closed registry with one pinned active member.
- Keep the default build byte-identical in behavior and identical in dependency closure to the current XXH3-128 build.
- Let an adopter select `blake3-256` or `sha-256` through project configuration and one Cargo feature.
- Keep the `no_std`, allocation-light, `Copy`, panic-free character of `SemanticDigest`.
- Preserve the deterministic-output guarantee: identical bytes, configuration, schema, and pinned toolchain produce identical canonical output.
- Replace the blanket algorithm ban with an enforceable boundary rule that the hand-written SHA-256 would have failed.
- Give every registered member its own known-answer vectors and its own declared assurance class.

**Non-Goals:**

- Changing the default active algorithm.
- Supporting more than one active algorithm in one repository at one time.
- Making the algorithm a per-object, per-domain, or per-call parameter.
- Using cross-member equality or order as identity, freshness, or migration evidence.
- Runtime algorithm negotiation between a producer and a consumer.
- Claiming that selecting `blake3-256` or `sha-256` alone establishes authenticity, tamper evidence, or proof.

## Decisions

### Closed registry with reserved tags, not an open trait

`DigestAlgorithm` stays a closed `#[repr(u16)]` enum. It gains `Blake3_256 = 3` and `Sha256 = 4` beside the existing `Xxh3_128 = 2`. Each member exposes a canonical name, a tag, and an exact output length through `const` accessors.

Tags `0` and `1` stay permanently obsolete. Tag `1` named a 32-byte `sha256` digest in schema v1. Reusing it for the new `sha-256` member would make a v1 artifact and a v3 artifact indistinguishable at the tag level. The new member therefore takes tag `4`.

Alternative: a `DigestAlgorithm` trait with a generic `SemanticDigest<A>` or a `dyn Hasher` injection. Rejected. A generic parameter would infect every model type, every report type, and every proof obligation in the kernel. A trait object would add indirection and a `?Sized` boundary to a type that must stay `Copy` and `const`-constructible, and it would let a caller supply an unregistered algorithm, which defeats the closed-enum discipline the schema inventory depends on.

### Bounded 32-byte buffer with an algorithm-derived length

`SemanticDigest` becomes a `[u8; 32]` buffer plus its algorithm. The valid length is a function of the algorithm, so the struct stores no separate length field and no invalid state is representable. Accessors return `&self.buffer[..self.algorithm.length()]`.

The type stays `Copy`, `Ord`, and `const`-constructible. `Ord` supplies deterministic ordering for complete tagged values in canonical containers. Semantic equality and order use checked methods that reject different members with `DIG-ALGORITHM-MISMATCH`. The type grows from 17 bytes to 33 bytes before padding. That cost is acceptable in a type that is already cloned freely.

Alternative: `heapless::Vec` or an `alloc::Vec<u8>` payload. Rejected. A heap payload would break `Copy`, add an allocation to every digest, and make `Ord` non-trivial. Alternative: one enum variant per length. Rejected as a duplication of the algorithm enum.

### One active member, declared in configuration, enforced at the boundary

The active member is a project-configuration field. The core exposes an `ActiveAlgorithm` value that the shell constructs once and passes into every hashing entry point. The core does not read a global, a static, or an environment variable.

Parsing and computation split deliberately:

- Parsers accept every registered member, so a tool can read an artifact that another repository produced. This is what makes migration and cross-repository inspection possible.
- Computation uses only the active member. A request to compute under an inactive member fails closed with a deterministic diagnostic.

Alternative: a Cargo feature alone selects the algorithm. Rejected as the sole mechanism. A compile-time-only choice makes cross-member property tests impossible in one test binary, and it hides the active choice from the canonical artifacts that must record it.

Alternative: configuration alone, with all three hashers always compiled. Rejected as the sole mechanism. It would add `blake3` and `sha2` to the default `no_std` closure and to the formal-verification surface for every adopter, including adopters who never leave XXH3-128.

The two mechanisms therefore combine. Each hasher sits behind its own Cargo feature. The default feature set is `xxh3-128` alone. If configuration names a member whose feature is absent, construction fails closed with an unavailable-algorithm diagnostic that names the required feature.

### Per-algorithm adapter modules are the only place a hashing crate appears

Each member gets one small adapter module in `buoy-core`, for example `value::hash::blake3`. The module has one job: take a preimage slice, return the raw output bytes. The hashing-policy scanner allowlists those exact module paths and nothing else.

This is the rule the hand-written SHA-256 in `tools/content_digest.rs` fails. The rule tests where hashing happens, not which identifier appears. A second implementation outside the registry is a finding regardless of its spelling, so the current evasion cannot recur.

### Schema v3 and full regeneration

Extending a closed enum changes the schema. The schema inventory declares `digest-algorithm` as `closed`, and `tools/check_schema_inventory.rs` asserts exactly one variant. The change therefore bumps the canonical schema ID from `spec-provenance/v2` to `spec-provenance/v3`.

The encoding version inside the preimage moves from `2` to `3`. Every existing digest value changes, including every `xxh3-128:` value in a repository that keeps XXH3-128.

The alternative is to extend the closed enum in place and keep encoding version `2`. That option preserves every existing digest byte, because the algorithm tag already sits in the preimage and existing values keep tag `2`. Its entire benefit is avoided regeneration.

That benefit is worth nothing here. No external artifact carries a v2 identity, and this repository's v2 values all come from generators. The in-place option would therefore buy a generator run in exchange for two different closed-enum definitions both claiming to be v2, in a repository whose archived history documents what v2 means. The design takes v3.

A third option is to redefine v2 rather than bump it, since v2 was never published either. Rejected. The same schema ID would name two different encodings across this repository's own git history, which contradicts the provenance story that the archived `2026-07-17-standardize-all-hashing-on-xxhash` change records.

Requirement XXH-004 already defines the regeneration semantics this needs: regenerate from authoritative canonical source bytes, never transform, truncate, pad, or relabel old digest bytes. A later algorithm switch reuses the same path, so the machinery is written once. That path matters for future adopters even though this repository can reach v3 by regeneration alone.

### Requirement IDs stay stable, the capability name does not change here

`assurance/v1/obligation-ledger.tsv` and `assurance/v1/scenario-requirement-executions.tsv` key thousands of rows on `XXH-001` through `XXH-006` and on scenario slugs derived from the current scenario names. The requirement IDs therefore stay unchanged, and this change modifies their content in place.

The capability folder keeps the name `xxhash-policy` even though the policy is no longer xxHash-specific. Renaming the folder would rewrite every `openspec/specs/xxhash-policy/spec.md` path in the ledger and in `assurance/product-identity/v1/naming-inventory.tsv`. That rename is a separate, mechanical, separately reviewed change.

Every scenario name also stays unchanged. Only scenario bodies change, so no ledger row re-keys and every affected row needs a digest rebind only.

### Per-member assurance class

The assurance boundary becomes a property of the member, not a global constant. The registry declares each member as `change-detection` or `collision-resistant`.

- `xxh3-128` is `change-detection`. Every current XXH-006 prohibition continues to apply to it.
- `blake3-256` and `sha-256` are `collision-resistant`. A report may state collision resistance for them.

No member may claim authenticity, tamper evidence, signature strength, or proof status from digest equality alone. Those claims need a separately defined mechanism, and XXH-006 keeps forbidding them for every member. Selecting a cryptographic member raises the collision claim only.

### The correspondence index is keyed by stable object ID, and it is committed text

An active-member change rewrites every digest. External references to old digests, and human review of what an algorithm change did, both need a translation record. The index supplies it.

The key is the stable object ID, not the old digest. XXH-004 already requires that stable object IDs survive migration, so the old-to-new mapping falls out as a join on that key.

Keying by old digest was rejected. It would make the index answer the question "what is the new identity for this old digest?", which is exactly the input that XXH-004's negative scenario requires to fail. An object whose authoritative source bytes are gone would acquire a target identity from a lookup table instead of failing validation. DIG-008 therefore forbids the index as an identity input in every direction, and requires that deleting the index leave canonical identity unchanged.

The index is a generated artifact, so it registers as a `Derivation` under SPG-010 with a generator, a configuration, and a consistency oracle. It inherits freshness and staleness detection from that machinery rather than inventing its own. Every row is verifiable by recomputing both digests from authoritative source bytes, which makes the index checkable rather than trusted.

Storage is a committed, totally ordered TSV, consistent with every other evidence artifact in the repository.

Alternative: an embedded key-value store such as redb. Rejected on four grounds. First, the repository's central gate is byte-identical canonical output across two runs and two absolute roots, and a copy-on-write B-tree file's bytes depend on page allocation, free-page tracking, and transaction history. Byte-reproducibility is not a property that redb documents, and the core determinism gate must not rest on an undocumented one. Second, an algorithm change is precisely when a reviewer must read the diff, and a binary container renders as one opaque changed blob. Third, the scale does not call for it: the entire assurance corpus is roughly 16,000 rows and 20 MB of TSV, and the index holds about one row per digest-bound object. Fourth, `buoy-core` is `no_std` with no ambient effects and a formal-verification lane, so a database could live only in the shell and the core would still receive plain data.

A future incremental-recheck cache over a large repository is a different artifact. It would be a performance cache, explicitly noncanonical, uncommitted, and safe to delete at any time. An embedded store is a reasonable choice there, and that choice stays out of this change.

### The finite model proves index non-authority exhaustively

`tools/check_xxh3_migration_model.rs` already establishes the repository pattern: an independent `model_*` transition relation, a separate `implementation_*` path, exhaustive enumeration of a bitmask product, outcome comparison at every state, injected-fault negative controls, and a report with exact counts. The new model follows that pattern and replaces the existing one.

The model splits into three enumerated sub-spaces.

**Digest acceptance, 64 states.** The product of tag class (active-registered, inactive-registered, obsolete, unknown), payload length (matches declared, mismatched), text case (canonical lowercase, noncanonical), hex validity (valid, invalid), and operation (parse, compute). Acceptance requires a registered tag, a matching declared length, canonical lowercase, valid hex, and for `compute` the active member. This covers DIG-002, DIG-003, DIG-004, and XXH-002.

**Active-member resolution, 12 states.** The product of declared member count (0, 1, 2), registration (registered, unregistered), and build-feature availability (present, absent). Acceptance requires exactly one declared, registered, available member. This covers DIG-003.

**Transition and index lifecycle, 4096 states.** A twelve-bit mask over: authoritative source bytes available, required approval present, duplicate stable ID, observed collision, illegal lifecycle change, stale reference, mixed member, incomplete impact closure, index row present, index row contradicts recomputation, index keyed by digest, index absent.

Total declared bound: 4172 states, fully enumerable in well under a second.

The last four bits are the reason the model exists. The central invariant is stated as an equality over the whole space:

> for every state `s`, `identity_outcome(s) == identity_outcome(s with all index bits cleared)`

The runner checks this equality for both the independent model relation and the separate implementation relation across the declared state space. This result establishes index non-authority only for model `buoy-digest-transition/v1` and its declared bounds. The end-to-end correspondence control separately confirms that production identity output does not read the index. Neither result extends beyond its bound source paths or state model.

The index gate outcome is checked as a separate projection of the same state, which keeps DIG-007 verification and DIG-008 non-authority as independent facts rather than one merged verdict.

Each invariant carries its own injected negative control, because an exhaustive check that cannot fail proves nothing. The controls inject an index input into the identity outcome, accept a contradicted index row, accept a transition with missing authoritative bytes or approval, and mutate a preserved stable ID in an accepted target. Every control must make the runner fail, and the self-test asserts exactly that.

Alternative: a property test over random states instead of exhaustive enumeration. Rejected for this model. The state space is small enough to enumerate completely, and EVD-007 permits an `exhaustively-checked` claim only for a fixed-point exploration. Hegel-rs properties still cover the unbounded surfaces, such as round trips over arbitrary byte inputs, where enumeration is impossible.

### Acceptance oracle

The exact final oracle is `devenv test`, run three times, once per member feature set, with the matching configuration. Each run must produce a green result, and the two non-default runs must produce canonical output that differs from the default run in digest values only, not in record structure, ordering, or diagnostics.

## Risks / Trade-offs

- **Full re-identification churn across every committed artifact** → regenerate rather than migrate, run it as one closed change set, and gate it on the existing byte-identical double-run determinism check from XXH-004. No external artifact is affected.
- **A generated file turns out to be hand-edited and does not survive regeneration** → inventory every digest-bearing artifact first (task 1.7), and reconcile the regenerated output against the recorded expected scope before accepting the diff.
- **A cryptographic member gets read as an authenticity guarantee** → keep the per-member class explicit in the registry and in every report, and keep the XXH-006 negative scenario blocking for all members.
- **The adapter-module allowlist becomes a laundering channel** → keep each adapter module to one preimage-to-bytes function, and add a negative fixture for an adapter module that computes anything beyond its own member.
- **A second in-tree hash implementation reappears** → make the scanner rule structural rather than identifier-based, and add the retired `tools/content_digest.rs` implementation as a permanent negative fixture.
- **Cross-algorithm digest confusion** → bind the tag into the preimage, use checked semantic comparison at identity and freshness boundaries, reserve derived `Ord` for canonical tagged-value order, and add negative fixtures for a right-length value carrying the wrong tag.
- **The correspondence index becomes a second source of identity** → key it by stable object ID, verify every row by recomputation, forbid it as an identity input in DIG-008, and test that deleting it leaves canonical output unchanged.
- **The index goes stale against the graph it describes** → register it as an SPG-010 derivation so existing derivation-freshness reporting detects drift, and keep the consistency verdict separate from derivation freshness.
- **The 32-byte buffer wastes 16 bytes for XXH3-128 adopters** → accept the cost; the type is small and the alternative breaks `Copy`.
- **Formal-proof bindings invalidate** → treat the kernel change as proof-invalidating, re-run the selected lane, and do not carry forward a stale proof record.
- **Feature and configuration disagree at run time** → fail closed with a diagnostic that names the missing feature, and add a negative fixture for that exact mismatch.

## Migration Plan

There is no deployed consumer, so this plan is a repository-internal sequence rather than a rollout.

1. Land the registry, the widened `SemanticDigest`, and the v3 encoding behind the default `xxh3-128` feature.
2. Invert the hashing-policy scanner to the boundary rule, and add the allowlisted adapter modules.
3. Route `tools/content_digest.rs` callers through the registry, then delete the hand-written implementation.
4. Regenerate every committed digest-bound artifact as one closed change set, and reconcile the diff against the task 1.7 inventory.
5. Regenerate the assurance ledgers, the property evidence, and the schema inventory.
6. Add the `blake3-256` and `sha-256` features, their vectors, and their fixtures. The default build stays unchanged.
7. Re-run the selected formal-verification lane against the new kernel source digests.

Rollback is `git revert` at any step, because no external state exists. The v3-to-v2 regeneration path is therefore not required for this change. It stays specified in XXH-004 for future adopters, who will not have that luxury.

## Resolved Questions

1. **Initial member set.** The first implementation includes `xxh3-128`, `blake3-256`, and `sha-256`. Tags `2`, `3`, and `4` identify these members. Tags `0` and `1` stay obsolete.
2. **SHA-256 implementation.** The `sha-256` adapter uses the reviewed RustCrypto `sha2` crate. The retired in-tree implementation remains a negative fixture only.
3. **Capability name.** This change keeps the `xxhash-policy` capability folder and the existing requirement headings. A separate mechanical change can rename them.
