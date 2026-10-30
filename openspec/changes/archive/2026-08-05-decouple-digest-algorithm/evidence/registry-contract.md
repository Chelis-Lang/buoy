# Digest registry contract

## Consumer and publication review

Review date: 2026-07-29.

A manifest search across local repositories found only path dependencies inside this repository.
No other local repository consumes `buoy` or `buoy-core`.

The active Chelis work contains planning artifacts only. It does not contain a deployed Buoy dependency or a released Buoy artifact.

The workspace contains these five packages:

- `buoy`
- `buoy-core`
- `buoy-reference`
- `spec-provenance` compatibility-alias package
- `spec-provenance-core` compatibility-alias package

`[workspace.package]` sets `publish = false`. Each workspace package inherits this value through `publish.workspace = true`.

The repository also contains excluded crates and fixture crates. Their manifests set `publish = false` directly.

No manifest sets `publish = true`. This result permits complete v3 regeneration instead of a compatibility migration.

## Initial member set

The first implementation includes all three registered members:

| Canonical name | Tag | Implementation status |
|---|---:|---|
| `xxh3-128` | 2 | default member |
| `blake3-256` | 3 | optional member |
| `sha-256` | 4 | optional member |

Tags `0` and `1` stay permanently obsolete. The registry rejects both tags.

## SHA-256 implementation

The `sha-256` member uses RustCrypto `sha2` version `0.10.9`. The manifest pins `=0.10.9` and disables default features.

The adapter uses `Sha256` through the `Digest` trait. The dependency supports the `no_std` core boundary.

The retired in-tree SHA-256 implementation remains a negative fixture. Production code does not use or promote that implementation.

## Registry table

| Canonical name | Tag | Length | Assurance class | Complete parameters |
|---|---:|---:|---|---|
| `xxh3-128` | 2 | 16 bytes | `change-detection` | XXH3 128-bit mode, seed `0`, standard default secret, full output, high half then low half, big-endian bytes |
| `blake3-256` | 3 | 32 bytes | `collision-resistant` | BLAKE3 hash mode, no key, no context, 32-byte output, official output-byte order |
| `sha-256` | 4 | 32 bytes | `collision-resistant` | SHA-256 from FIPS 180-4, no key, full 32-byte digest, official digest-byte order |

Tags `0` and `1` are obsolete. The parser rejects both tags without a fallback.

## Hash adapter allowlist

The allowlist contains exactly these private core modules:

- `crate::value::hash::xxh3_128`
- `crate::value::hash::blake3_256`
- `crate::value::hash::sha_256`

Each module exposes one function to its parent module:

```rust
// crate::value::hash::xxh3_128
pub(super) fn hash(preimage: &[u8]) -> [u8; 16]

// crate::value::hash::blake3_256
pub(super) fn hash(preimage: &[u8]) -> [u8; 32]

// crate::value::hash::sha_256
pub(super) fn hash(preimage: &[u8]) -> [u8; 32]
```

An adapter accepts only the complete preimage. It returns only the complete output bytes for its own member.

An adapter does not parse values, select algorithms, truncate output, change parameters, or expose an implementation type.

## Project configuration

The configuration schema is `spec-provenance-config/v2`. The required top-level field is `digest_algorithms`.

The field is an array with exactly one value. The accepted values are:

- `xxh3-128`
- `blake3-256`
- `sha-256`

The parser returns these exact diagnostic codes and messages:

| Condition | Code | Message |
|---|---|---|
| Empty array | `DIG-ACTIVE-MISSING` | `project configuration must declare one digest algorithm` |
| More than one value | `DIG-ACTIVE-MULTIPLE` | `project configuration declares multiple digest algorithms` |
| Unknown value | `DIG-ALGORITHM-UNKNOWN` | `project configuration names an unregistered digest algorithm: <name>` |
| Missing Cargo feature | `DIG-ALGORITHM-UNAVAILABLE` | `digest algorithm <name> requires Cargo feature <feature>` |

The parser converts the one valid value into `ActiveAlgorithm`. Downstream code receives that type instead of the raw array.

## Digest artifact inventory

`assurance/v1/digest-artifact-inventory.tsv` lists every committed file with a valid repository-owned digest text value.

The final review found 116 files:

- 80 files have a generator or an execution lane that reproduces them.
- 33 active files contain hand-authored source or fixture literals.
- 3 archived files stay unchanged as historical evidence.

The expected v3 cutover scope is 113 files. This count includes 80 regenerated files and 33 reviewed source or fixture updates.

The correspondence tool excludes its index and derivation from their own source set. This rule prevents a recursive digest dependency.

External ecosystem integrity values are not in this count. `assurance/supply-chain/v1/closure-inventory.tsv` owns those opaque values under XXH-005.

## Adversarial review

The final adversarial review covered each DIG requirement and each revised XXH requirement. It also covered fixtures, bindings, the model, and gate wiring.

The review found that the change-local inventory path would break after archive. The permanent inventory now exists at `assurance/v1/digest-artifact-inventory.tsv`.

The correspondence gate now compares the inventory with every current repository digest file. The gate rejects unowned files, stale rows, and unapproved exclusions.

The review found that claim fixtures did not enforce production documentation policy. The hashing-policy gate now rejects unsupported assurance claims with a stable diagnostic.

The review found weak model controls and an unsupported production claim. External fault commands now fail with exact diagnostics and states.

The model now checks its contract digest and expected counts. It reruns accepted targets to a computed fixed point.

The model claim applies only to its 4,172 declared states. Focused production controls provide separate correspondence evidence.

A production integration control shows that index creation and deletion do not change the selected snapshot, graph, report facts, or graph digest.

The registry makes no authentication or signature claim. No valid assurance-inflation finding remains after these corrections.
