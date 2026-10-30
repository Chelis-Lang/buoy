# Adapter and downstream v2 integration

Status: frozen historical contract.

Schema v2 used explicit Markdown and Rust grammar versions. Each v2 digest field accepted only canonical `xxh3-128:<32 lowercase hex>` text.

Malformed, obsolete, uppercase, truncated, padded, unknown-algorithm, and mixed-version digest forms produce stable deterministic diagnostics. Adapters do not call a separate hasher and cannot transform an external Cargo/Nix/Git/upstream checksum into semantic identity. They supply typed values and authoritative bytes inward to the core.

## Historical downstream cutover

The v1-to-v2 cutover required these changes:

1. regenerate configuration/raw-artifact leaves from authoritative bytes;
2. regenerate every dependent object, reference, carrier, policy, proof, graph, report, cache/fingerprint, and expected diagnostic;
3. replace digest-bearing examples and saved outputs with v2 values;
4. retain old stable IDs, lifecycle states, tombstones, replacements, and approvals;
5. produce complete impact dispositions; and
6. reject, rather than reinterpret, every active v1 or mixed binding.

A downstream cache keyed by v1 identity must be invalidated, not relabeled. A rollback restores the entire v1 code/schema/lock/data baseline.

## Evidence boundaries

Production/reference differential tests compare independently encoded envelopes, but both paths use the same pinned low-level `xxhash-rust` implementation. Official upstream vectors anchor that low-level result. Differential equality does not establish independent XXH3 implementation agreement, collision resistance, cryptographic integrity, or correctness for inputs outside the corpus.

Adapter fixtures, Hegel samples, finite models, formal annotations, Verus obligations, Aeneas/Lean replay, proof-record digest equality, and downstream smoke tests each retain their separately declared scope and exclusions. None upgrades another evidence class or proves natural-language meaning, parser/compiler correctness, or native behavior.
