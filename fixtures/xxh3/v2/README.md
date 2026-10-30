# Official XXH3-128 profile vectors

These fixtures precede production XXH3 integration. They pin the version 2 low-level algorithm independently of project canonical-envelope goldens.

## Source

- Upstream: `Cyan4973/xxHash`
- Release: `v0.8.3`
- Revision: `e626a72bc2321cd320e953a0ccf1584cad60f363`
- Positive array: `tests/sanity_test_vectors.h`, `XSUM_XXH128_testdata`
- Input algorithm and incremental checks: `tests/sanity_test.c`
- Vector generator: `tests/sanity_test_vectors_generator.c`
- Canonical conversion: `xxhash.h`, `XXH128_canonicalFromHash`

`tests/sanity_test.c` declares `GPL-2.0-only`; the generated vector header has no displayed SPDX line. The selected factual vector rows are retained with upstream revision and source indexes. This attribution does not treat the Rust crate's BSL-1.0 license as the license of upstream test material.

## Deterministic input

`xxhash-sanity-buffer/v0.8.3` is generated exactly as upstream `fillTestBuffer`:

1. initialize unsigned 64-bit `byteGen = 2654435761` (`PRIME32`);
2. emit the high byte `byteGen >> 56`;
3. set `byteGen = byteGen * 11400714785074694797` with unsigned 64-bit wrapping;
4. repeat until `input_length` bytes have been emitted.

No input bytes come from files, clocks, locale, random devices, or network state.

## Result interpretation

Upstream vector pairs are `{ low64, high64 }`. `canonical_hex` is `high64` as 16 lowercase hexadecimal digits followed by `low64` as 16 lowercase hexadecimal digits. This equals `XXH128_canonicalFromHash` and Rust `u128::to_be_bytes()` for `xxhash_rust::xxh3::xxh3_128`.

Positive rows require variant XXH3-128, seed `0`, the standard default secret, and all 128 bits. Incremental chunk lengths must sum exactly to `input_length`; `chunks = 0` means an empty update sequence.

Negative rows are detector controls, not alternative accepted identities. Implementations must reject the configured profile or conversion with the exact `expected_code` before a candidate can become `SemanticDigest` data.

## Version 2 fixture inventory

- `official-xxh3-128-vectors.tsv`: upstream one-shot and incremental algorithm vectors.
- `negative-profile-vectors.tsv`: wrong variant, seed, secret, truncation, and byte-order controls.
- `canonical-digest-roundtrip.tsv`: canonical text and binary digest forms.
- `malformed-digest-vectors.tsv`: fail-closed parser and observed-collision diagnostics.
- `migration-cases.tsv` and `migration/`: complete, idempotent migration plus fail-closed mutation classes.
- `policy-cases.tsv` and `policy/`: paired syntax fixtures for every repository-owned hashing role.
- `external-integrity-cases.tsv` and `external/`: exact external shell fields and prohibited semantic promotion.
- `hegel-replay.toml`: fixed seeds, case counts, and replay command for the v2 property suite.

Every negative row carries its required stable diagnostic code. `tools/check_xxh3_vectors.rs --self-test` mutates each diagnostic class and proves that the fixture validator fails closed.
