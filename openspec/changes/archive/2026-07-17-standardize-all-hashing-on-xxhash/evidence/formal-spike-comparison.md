# Version 2 formal-spike comparison and lane decision

## Result and decision

The authoritative replay command is:

```text
devenv shell -- spec-provenance-formal-spikes
```

Accepted result on 2026-07-17:

```text
verification results:: 13 verified, 0 errors
formal-spikes: verus=13/13 aeneas-lean=11/11 negative-controls=2/2
```

The version 2 additions prove equivalent finite claims in both lanes for:

1. binding schema version `2`, encoding version `2`, and algorithm tag `2` in one envelope decision;
2. preserving byte order inside the high half followed by the low half of an abstract XXH128 canonical value;
3. accepting only canonical-value type tag `12`, algorithm tag `2`, and digest length `16`; and
4. preserving each lifecycle variant through the migration wrapper.

**Verus remains the primary formal-verification lane.** It directly verifies the smallest source/specification surface and has the smaller translation and external-model trust boundary. AeneasVerif/Charon/Lean remains the mandatory comparative translation sentinel. This reaffirms `docs/architecture/adr-0001-primary-formal-lane.md`; it does not broaden that ADR's claims.

## Exact bindings

### Verus lane

- Positive source/spec/proofs: `proofs/spikes/verus/integrity.rs`
- Negative control: `proofs/spikes/verus/integrity_negative.rs`
- Verus release: `0.2026.07.12.0b42f4c`
- Verus source commit: `0b42f4cee92a178937608cf55e512371d2fd8cd4`
- Verus Rust toolchain: `1.96.0`
- Solver: Z3 bundled in the pinned Verus release
- Package and archive hashes: `nix/formal-tools.nix`

### AeneasVerif lane

- Safe sequential Rust source: `proofs/spikes/aeneas/rust/src/lib.rs`
- Rust manifest/lock: `proofs/spikes/aeneas/rust/Cargo.toml`, `proofs/spikes/aeneas/rust/Cargo.lock`
- Charon extraction: `proofs/spikes/aeneas/integrity.llbc`
- Normalized Aeneas derivation: `proofs/spikes/aeneas/generated/Integrity.lean`
- Bound external model: `proofs/spikes/aeneas/lean/Aeneas.lean`
- Positive proofs: `proofs/spikes/aeneas/lean/IntegrityProofs.lean`
- Negative control: `proofs/spikes/aeneas/lean/IntegrityNegative.lean`
- Aeneas nightly source commit: `678ffd3b797b2a6d8e3568a0bfdd5d027038afad`
- Charon source commit: `74166d896db4ae4f63101c13c22072fb2022092e`
- Lean: `4.30.0-rc2`, source commit `3dc1a088b6d2d8eafe25a7cd7ec7b58d731bd7cc`
- Nix source hashes and generation command: `devenv.lock`, `nix/formal-tools.nix`, and `devenv.nix`

Regeneration must reproduce the normalized Lean source byte-for-byte. Normalization removes only unavailable nonsemantic linter settings and generated enum `discriminant` attributes; it does not rewrite definitions.

## Supported and unsupported constructs

The common supported subset is safe, sequential, total, ambient-input-free logic over booleans, bounded lifecycle enums, scalar tags and lengths, and structural records/sequences. The Aeneas spike additionally depends on its explicitly bound minimal `Result`, `U8`, and `U64` model.

Unsupported by this decision are unsafe Rust, concurrency, mutable statics, filesystem/process/network access, live dependencies, panics, unbounded recursion, unsupported loop control, arbitrary external models, and production code outside the listed bindings. No `sorry` or `admit` is permitted.

## Assumptions

- Numeric constants `2`, `12`, and `16` correspond to the frozen v2 schema, canonical-value digest tag, algorithm tag, and digest width.
- The `Octets` supplied to the Aeneas half-order wrapper and the `Seq<int>` supplied to the Verus byte selector already represent each 64-bit half in big-endian byte order.
- Lifecycle variants in the spike correspond to the production lifecycle variants selected for migration.
- Verus/Z3 accepts the intended specification encoding; Charon/Aeneas translates the bound Rust faithfully; the minimal Lean model represents only the generated constructs used here; and Lean's kernel checks the resulting terms.
- The negative controls demonstrate detector behavior only.

## Explicit exclusions

Neither lane proves XXH3 internals, arithmetic correctness of XXH3, seed/default-secret implementation behavior, absence of collisions, collision resistance, preimage resistance, authenticity, tamper evidence, parser implementations, migration completeness, prose correspondence, rustc, LLVM, native code, operating systems, or external package-manager integrity. The byte-order obligations prove structural placement only, not that a third-party hasher computed the supplied halves. Official upstream vectors remain the algorithm and byte-order anchor.
