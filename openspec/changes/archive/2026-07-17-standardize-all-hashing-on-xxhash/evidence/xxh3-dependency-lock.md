# XXH3-128 Dependency and Upstream Lock

Status: decision record for task 1.2, captured 2026-07-17. This file records the dependency selected for later implementation; task 3.1 performs the manifest and lockfile change.

## Selected Rust dependency

```toml
xxhash-rust = { version = "=0.8.17", default-features = false, features = ["xxh3"] }
```

| Property | Locked value |
|---|---|
| Crate | `xxhash-rust` |
| Exact version | `0.8.17` |
| crates.io archive checksum | `985eec839aaf2a1270af8f4ebcf63cf9401cfd90f0902f97c28d9f104ffbde72` |
| Repository revision corresponding to 0.8.17 | `DoumanAsh/xxhash-rust@6c4963aa0b63161afc789b8e85860222447d7f0e` |
| License | Boost Software License 1.0 (`BSL-1.0`) |
| Rust edition | 2018 |
| Normal dependencies | none |
| Enabled features | `xxh3` only |
| Disabled features | `std`, `xxh32`, `xxh64`, `const_xxh32`, `const_xxh64`, `const_xxh3` |
| Allocation requirement | none from the crate; crates.io classifies it as `no-std::no-alloc` |

The crate root unconditionally declares `#![no_std]`. Its default feature set is empty, and the `std` feature only adds `std::io::Write` implementations. With defaults disabled and only `xxh3` enabled, the production closure contains no normal transitive crates. The crate's `getrandom` and `xxhash-c-sys` entries are upstream development dependencies and do not enter a consumer's normal or test dependency closure unless this repository adds them separately.

The same exact dependency may be used by `spec-provenance-core` and the independent reference crate. Workspace feature unification must resolve exactly `xxh3`; enabling `std`, another xxHash family, const variants, a custom secret path, or another version is a policy failure.

## Algorithm identity

The repository profile is exactly the upstream xxHash **XXH3 128-bit** operation:

- upstream one-shot identity: `XXH3_128bits(input, len)`;
- equivalently: `XXH3_128bits_withSeed(input, len, 0)`;
- Rust one-shot API: `xxhash_rust::xxh3::xxh3_128(input)`;
- Rust incremental API: `xxhash_rust::xxh3::Xxh3::new()`, repeated `update`, then `digest128`;
- seed: `0`;
- secret: the upstream standard XXH3 default secret; no custom secret;
- output: all 128 bits.

`xxhash-rust` states that its `0.8.*` line corresponds to upstream C `0.8.*`. The official algorithm/vector anchor for this repository is upstream `Cyan4973/xxHash` release `v0.8.3`, commit `e626a72bc2321cd320e953a0ccf1584cad60f363`. Upstream library code is BSD-2-Clause licensed. The Rust implementation is BSL-1.0 licensed.

## Official vector source

The authoritative upstream sources for task 2.1 are pinned to:

- `tests/sanity_test_vectors.h` at `Cyan4973/xxHash@e626a72bc2321cd320e953a0ccf1584cad60f363`, blob `a5025a2d34be5e7a81c2e01802a5f3588399b58f`;
- `tests/sanity_test.c` at the same revision, blob `3e3d4f16130b1a226ccd59ceec5b10f0767e8dd9`;
- `tests/sanity_test_vectors_generator.c` at the same revision for vector-generation semantics;
- `xxhash.h` at the same revision for `XXH128_canonicalFromHash` and the algorithm definition.

The upstream sanity input is deterministic: `fillTestBuffer` starts `byteGen` at `PRIME32 = 2654435761`, emits `byteGen >> 56`, and multiplies by `PRIME64 = 11400714785074694797` for every next byte. Task 2.1 will extract a reviewed finite subset covering empty, short, 64-byte stripe boundaries, mid-size boundaries, long inputs, and incremental splits. It will retain the exact upstream length, seed, low half, high half, source revision, and source-array index for every copied case.

Upstream `sanity_test.c` is marked `GPL-2.0-only`; the generated vector header itself has no displayed SPDX header. Any copied vector subset must therefore carry source attribution and a license review instead of assuming the Rust crate's BSL-1.0 applies to upstream test material.

## Canonical byte conversion

`xxhash-rust::xxh3::xxh3_128` returns a `u128` whose upper 64 bits are upstream `high64` and lower 64 bits are upstream `low64`. Canonical repository bytes are produced only as:

```text
u128.to_be_bytes()
```

This is equivalent to upstream `XXH128_canonicalFromHash`: high 64-bit half in big-endian order followed by low 64-bit half in big-endian order. Native-endian bytes, low-half-first concatenation, per-half little-endian encoding, truncation, or hexadecimal formatting before byte conversion are prohibited.

## Independent-reference limits

Using the same `xxhash-rust` pin in production and reference code does **not** provide an independent low-level XXH3 implementation. The evidence split is therefore:

1. official upstream vectors anchor the XXH3 result, default-secret behavior, seed profile, 128-bit half order, and canonical bytes;
2. independently implemented production/reference envelopes and canonical encoders provide differential evidence for schema/domain separation and integration;
3. one-shot/incremental parity exercises separate crate paths but does not establish algorithm correctness or collision properties;
4. Hegel properties, finite models, Verus, and Aeneas work do not prove XXH3 internals, collision resistance, preimage resistance, compiler correctness, or machine code.

A claim of fully independent hashing implementations is forbidden unless a second reviewed implementation with a disjoint source/dependency lineage is introduced and bound explicitly.

## Sources checked

- crates.io API record for `xxhash-rust 0.8.17`, including checksum, features, license, and dependency kinds;
- `DoumanAsh/xxhash-rust@6c4963aa0b63161afc789b8e85860222447d7f0e` `Cargo.toml`, `src/lib.rs`, and `src/xxh3.rs`;
- `Cyan4973/xxHash@e626a72bc2321cd320e953a0ccf1584cad60f363` release, license, sanity tests, vector header, vector generator, and canonical conversion API.
