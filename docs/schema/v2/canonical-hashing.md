# Version 2 canonical hashing profile

Status: frozen historical contract.

## Algorithm and dependency

Schema v2 used unseeded XXH3-128 from `xxhash-rust =0.8.17`. The v2 implementation called it only through the private canonical-envelope hasher.

XXH3 is a fast non-cryptographic hash. Digest equality is not evidence of authenticity, authorization, natural-language truth, tamper resistance, collision resistance, preimage resistance, or cryptographic integrity. Security-sensitive and ecosystem integrity decisions remain outside semantic identity.

## Digest representation

Canonical binary digest values retain canonical value type tag `12` and have this payload:

```text
algorithm-tag:u16-be = 2
digest-length:u16-be = 16
digest-bytes[16]
```

`digest-bytes` is the full `u128::to_be_bytes()` result: the high 64-bit lane in big-endian order followed by the low 64-bit lane in big-endian order. Truncation, padding, lane reversal, native-endian encoding, and 32-byte storage are invalid.

Canonical text is exactly:

```text
xxh3-128:<32 lowercase hexadecimal digits>
```

The v2 API rejected uppercase text, alternate prefixes, omitted leading zeroes, whitespace, obsolete prefixes, tag `1`, and non-16-byte payloads.

## Semantic preimage envelope

Each v2 semantic digest used unseeded XXH3-128 over this exact concatenation:

```text
ASCII "SPEC-PROVENANCE" | 0x00
u16-be(2)
canonical-value(text("spec-provenance/v2"))
canonical-value(enum(2, option-none))
canonical-value(text(<domain>))
canonical-value(record(<domain fields>))
```

Canonical values use the length-delimited algebra and field/collection rules frozen by `schema/v2/inventory.tsv`: `u8` type tags; `u64` big-endian payload lengths and collection counts; ascending unique `u16` record tags; semantic sequence order; and complete encoded-byte ordering with duplicate rejection for sets. Text is NFC UTF-8 under the pinned Unicode 16.0.0 table.

Raw artifacts are not hashed directly. Their semantic preimage uses domain `raw-artifact` with field 1 media-type text and field 2 authoritative bytes. Cache keys, proof bindings, reports, graphs, configurations, and all other repository-owned identities use their declared domain and complete canonical payload.

## Observed collisions and duplicates

The implementation makes no claim that unequal preimages cannot share an XXH3-128 value. When both complete canonical preimages are observed in one operation:

- equal digest and unequal preimage is rejected as `DET-DIGEST-COLLISION`;
- repeated identical `(domain, preimage, digest)` observations are not a collision;
- object/set duplicate rules still apply independently; and
- deterministic diagnostics and ordering use complete canonical tie-breakers where the schema requires them.

The implementation cannot detect a collision when one preimage is unavailable and cannot elevate digest equality into a security claim.

## External integrity boundary

Cargo registry `checksum`, Nix fixed-output `hash`, exact Git `rev`, and declared upstream release checksum/signature fields remain opaque strings owned by those ecosystems. Shell code may preserve and pass only those exact fields to their ecosystem verifier. It must not parse them as `SemanticDigest`, feed their checksum bytes into a semantic identity, or grant a path-wide or substring-wide hashing exception.
