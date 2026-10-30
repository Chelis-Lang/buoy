# Final v2 formal replay

Status: task 6.3 replay evidence, 2026-07-17.

## Commands and normalized result

```text
devenv shell -- spec-provenance-core-proofs
devenv shell -- spec-provenance-formal-spikes
```

The production Verus source was accepted, its deliberately false no-laundering postcondition was rejected with `postcondition not satisfied`, and `tests/proof_bindings.rs` confirmed every v2 source/record binding. The comparative replay reported:

```text
formal-spikes: verus=13/13 aeneas-lean=11/11 negative-controls=2/2
```

Charon extraction and Aeneas generation reproduced the normalized committed Lean derivation. Lean accepted the positive theorem groups and rejected the false control with its expected definitional-equality diagnostic. No proof file contains `sorry` or `admit`.

## Exact production bindings

| Binding | Value |
|---|---|
| Implementation | `crates/spec-provenance-core/src/algorithms.rs` / `xxh3-128:3980843a1da3b900e6f580c95657d436` |
| Verus proof | `proofs/core/integrity.rs` / `xxh3-128:8c17f655a33ebaf18221616f92474465` |
| Negative source | `proofs/core/integrity_negative.rs` / `xxh3-128:edb2588a6469f121b00122665376f72c` |
| Proof record | `PROOF-CORE-INTEGRITY-V2`, `proofs/core/proof-record.toml` |
| Derivation | `DERIV-CORE-INTEGRITY-REPRESENTATION-V2`, `proofs/core/derivation-record.toml` |
| Verifier | Verus `0.2026.07.12.0b42f4c`, source `0b42f4cee92a178937608cf55e512371d2fd8cd4` |
| Configuration | `release/macos_aarch64; rust=1.96.0-aarch64-apple-darwin` |
| Solver | Verus distribution's bundled Z3 |

The bound assumptions are safe sequential mathematical value semantics and reviewed correspondence between the named Rust implementation and Verus representation. Correspondence approval is `review-only`; digest equality detects drift but does not prove translation equivalence.

The comparative lane is bound to Aeneas `nightly-2026.07.01` at `678ffd3b797b2a6d8e3568a0bfdd5d027038afad`, Charon `74166d896db4ae4f63101c13c22072fb2022092e`, Lean `v4.30.0-rc2` at `3dc1a088b6d2d8eafe25a7cd7ec7b58d731bd7cc`, the generated Lean artifact, and the minimal external `Result`/`U8`/`U64` model. Exact external Nix hashes remain Nix metadata, not semantic digests.

## Stated exclusions

The replay does **not** establish:

- natural-language entailment or prose-to-code correspondence;
- parser, adapter, filesystem, process, provider, network, clock, or operating-system behavior;
- arbitrary Rust, unsafe code, concurrency, macro expansion, or unsupported generated constructs;
- Charon/Aeneas translation faithfulness or correctness of the full Mathlib-dependent Aeneas backend;
- Verus, Z3, Lean kernel, external model, rustc, LLVM, linker, machine-code, C/HIP/Metal, or hardware correctness;
- XXH3 arithmetic/implementation correctness, collision absence/resistance, preimage resistance, authenticity, tamper evidence, or cryptographic strength; or
- properties outside the named lifecycle, surface, freshness, policy, no-laundering, impact, field-header, v2 envelope, byte-order, parser-shape, and migration-preservation obligations.

The successful replay is exact scoped formal/derivation evidence only. It does not upgrade tests, annotations, generated-byte equality, or other assurance classes.
