# Version 1 canonical encoding and digest domains

Status: frozen schema decision for task 2.3. This document defines the v1 canonical binary value encoding, SHA-256 preimage envelope, object and diagnostic ordering, duplicate rejection, digest display, and algorithm migration. Object-schema documents may refine validation constraints but MUST NOT reorder, add, remove, or reinterpret the digest-bound fields listed here without creating a later schema version.

## Design choice

V1 uses a small purpose-built, length-delimited binary encoding rather than canonical JSON, CBOR, protobuf, or concatenated text. The format has no maps with implicit iteration order, no floating-point values, no platform integers, no omitted defaults, and no parser-dependent whitespace. It is intended to be independently implemented and small enough to model formally.

All multibyte integers are unsigned big-endian. All lengths and counts are `u64`; a producer MUST reject a value that cannot be represented. Text is valid NFC UTF-8 under the pinned Unicode data version.

## Canonical value encoding

Every value is encoded as:

```text
type-tag:u8 | payload-length:u64 | payload:bytes[payload-length]
```

The closed v1 type tags are:

| tag | name | payload |
|---:|---|---|
| 0 | `false` | empty |
| 1 | `true` | empty |
| 2 | `u64` | exactly 8 bytes, unsigned big-endian |
| 3 | `i64` | exactly 8 bytes, two's-complement big-endian |
| 4 | `bytes` | arbitrary bytes |
| 5 | `text` | NFC UTF-8 bytes |
| 6 | `record` | field count followed by fields |
| 7 | `sequence` | element count followed by values |
| 8 | `set` | element count followed by unique sorted values |
| 9 | `option-none` | empty |
| 10 | `option-some` | exactly one encoded value |
| 11 | `enum` | variant tag and optional payload |
| 12 | `digest` | algorithm tag and digest bytes |

Boolean payload length MUST be zero. Numeric payload length MUST be eight. `option-none` has zero payload; `option-some` contains one complete value.

### Records

A record payload is:

```text
field-count:u64 |
  (field-tag:u16 | encoded-value) * field-count
```

Fields are always emitted in ascending field-tag order. Every field declared by the active object schema is present, including explicit `option-none`, empty sequence, or empty set values. Unknown, duplicate, omitted, or out-of-order field tags are invalid. Defaults are validation conveniences only and never omitted from canonical bytes.

### Sequences and sets

A sequence payload is:

```text
element-count:u64 | encoded-value * element-count
```

Sequence order is semantic.

A set uses the same physical shape, but elements are sorted by their complete encoded-value bytes using unsigned lexicographic byte order. Duplicate encoded values are invalid rather than deduplicated. Input order is nonsemantic.

Schemas use a sequence only when order carries meaning. All collections described as scopes, references, roles, assumptions, exclusions, members, bindings, or object collections are sets unless explicitly declared otherwise.

### Enums

An enum payload is:

```text
variant-tag:u16 | option-none-or-some-payload
```

Variant tags are fixed by the object schema. Unknown tags are invalid. A unit variant uses `option-none`; a data variant uses `option-some` containing one record.

### Digests

A digest payload is:

```text
algorithm-tag:u16 | digest-length:u16 | digest-bytes
```

V1 defines algorithm tag `1` as SHA-256 and requires a length of 32. Unknown algorithms are unsupported, not guessed. Canonical text display is lowercase:

```text
sha256:<64 lowercase hexadecimal digits>
```

Text display is never reparsed without validating the algorithm, exact length, and lowercase canonical spelling.

## Digest preimage envelope

Every v1 semantic digest is SHA-256 over exactly:

```text
magic                    = ASCII "SPEC-PROVENANCE" | 0x00
encoding-version         = u16(1)
schema-id                = value(text("spec-provenance/v1"))
digest-algorithm         = value(enum(1, none))
domain                   = value(text(<domain from the table below>))
payload                  = value(record(<domain fields below>))
```

The NUL after the magic is required. The value encodings include their type tags and lengths. A digest from one domain can never be reused as an identity in another domain without failing the expected-domain check.

Raw byte artifacts use the same envelope with domain `raw-artifact` and a record containing media type and bytes. SHA-256 directly over a file is not a v1 semantic digest.

## Common nested records

The following nested records have fixed field tags wherever used:

### Object reference

1. object-kind enum
2. stable object ID text
3. expected object digest

### Atom revision reference

1. atom ID text
2. expected atom revision digest

### Source reference

1. canonical repository-relative path text
2. original-byte start offset `u64`, inclusive
3. original-byte end offset `u64`, exclusive

Source references are included only in source-map and complete-graph domains, never in semantic object revisions unless a table below explicitly says otherwise.

### Approval binding

1. approval-authority ID text
2. normalized repository-relative review-record path text
3. review-record raw-artifact digest

### Tool identity

1. tool ID text
2. version text
3. executable raw-artifact digest
4. configuration digest

### Typed payload

1. host schema ID text
2. canonical payload bytes

Host payload bytes are opaque to the core only when the host schema is declared and pinned. An unknown host schema is unsupported.

### ID namespace declaration

1. namespace ID text
2. uppercase stable-ID prefix text
3. allocation-policy typed payload
4. approval-authority ID set

### ID allocation

1. namespace ID text
2. stable ID text
3. repository-owned allocation-record raw-artifact digest
4. positive namespace-local allocation ordinal `u64`
5. allocation-class enum (`live=1`, `terminal=2`)

## Domain and field table

Each row names the exact ordered digest-bound payload fields. Field numbers are the record tags.

### Repository and configuration domains

- `raw-artifact`: 1 media-type text; 2 raw bytes.
- `repository-config`: 1 configuration schema ID; 2 selectors set; 3 ID namespaces set; 4 approval authorities set; 5 policy references set; 6 pinned semantic tool identities set.
- `repository-snapshot`: 1 repository-config digest; 2 schema/tool-lock digest; 3 snapshot-file set; 4 rejected-entry observations set.
- `snapshot-file`: 1 canonical path; 2 entry-kind enum; 3 raw-artifact digest.
- `schema-tool-lock`: 1 encoding schema ID; 2 Unicode data identity; 3 CommonMark parser identity; 4 TOML parser identity; 5 Rust parser identity option; 6 additional semantic tools set.

### Atom and lifecycle domains

- `atom-revision`: 1 atom-kind enum; 2 scope-ID set; 3 canonical normative statement text.
- `atom-object`: 1 atom ID; 2 atom-revision digest; 3 lifecycle-state enum; 4 approval binding option.
- `tombstone`: 1 atom ID; 2 terminal-state enum; 3 final atom-revision digest; 4 replacement atom-revision-reference set; 5 approval binding.
- `id-registry`: 1 namespace declarations set; 2 allocated-ID entries set.

Authority source references are encoded in `source-map`, not `atom-revision` or `atom-object`.

### Decision-surface domains

- `decision-surface-domain`: 1 surface ID; 2 member-key schema typed payload; 3 disposition vocabulary typed payload; 4 canonical member-key set; 5 domain-evolution generation `u64`.
- `surface-disposition`: 1 member key typed payload; 2 disposition enum; 3 host disposition payload option; 4 authority atom-revision-reference option.
- `decision-surface`: 1 surface ID; 2 decision-surface-domain digest; 3 surface-disposition set; 4 applicable coverage-policy reference set.

### Link and evidence domains

- `evidence-carrier`: 1 carrier ID; 2 evidence-role enum; 3 atom-revision-reference set; 4 parsed-item identity digest; 5 oracle object reference; 6 execution-configuration digest; 7 fixture-or-input-corpus digest set; 8 claim-scope typed payload; 9 assumption object-reference set; 10 exclusions text set.
- `parsed-item-identity`: 1 language/schema ID; 2 canonical path; 3 item-kind enum; 4 item semantic bytes; 5 enclosing-item identity option.
- `implementation-link`: 1 link ID; 2 atom-revision-reference set; 3 parsed-item-identity digest; 4 navigation-kind enum.
- `derivation`: 1 derivation ID; 2 source binding set; 3 target binding set; 4 generator-or-checker tool identity; 5 derivation-configuration digest; 6 consistency oracle object reference; 7 declared relation enum; 8 relation scope typed payload; 9 exclusions text set.
- `artifact-binding`: 1 binding-kind enum; 2 object reference option; 3 canonical path option; 4 artifact digest; 5 raw-domain text option; 6 media-type text option; 7 representation text option. Fields 5-7 are absent for object bindings and present for repository-artifact bindings; raw-domain is exactly `raw-artifact`.

### Policy and review domains

- `coverage-policy-revision`: 1 policy scope typed payload; 2 required evidence-role set; 3 enforcement-mode enum; 4 adoption-ratchet ID; 5 expected-authority selectors set; 6 derivation requirements typed payload option.
- `coverage-policy-object`: 1 policy ID; 2 coverage-policy-revision digest; 3 active approval binding; 4 policy lifecycle enum; 5 replacement policy object-reference set; 6 terminal approval binding option.
- `debt-record`: 1 debt ID; 2 affected obligation-reference set; 3 affected object-reference set; 4 rationale text; 5 owner ID; 6 remediation text; 7 discharge condition typed payload; 8 informational external reference set; 9 debt lifecycle enum; 10 discharge-evidence object-reference set; 11 replacement debt object-reference set; 12 terminal approval binding option.
- `waiver`: 1 waiver ID; 2 affected obligation-reference set; 3 affected object-reference set; 4 rationale text; 5 owner ID; 6 discharge condition typed payload; 7 active approval binding; 8 informational external reference set; 9 waiver lifecycle enum; 10 discharge-evidence object-reference set; 11 replacement waiver object-reference set; 12 terminal approval binding option.
- `reaffirmation`: 1 reaffirmation ID; 2 affected object kind; 3 base object digest; 4 head object digest; 5 review approval binding; 6 review statement text.
- `obligation-reference`: 1 policy ID; 2 obligation-kind enum; 3 subject object reference; 4 required role option.

Informational external references are canonical text carried for navigation, but their live state is never fetched or interpreted by the core.

### Oracle and execution domains

- `oracle`: 1 oracle ID; 2 oracle-kind enum; 3 runner tool identity; 4 selection semantics typed payload; 5 verdict-normalization typed payload; 6 configuration digest; 7 scope typed payload; 8 exclusions text set.
- `normalized-verdict`: 1 oracle object reference; 2 selected-subject object-reference set; 3 verdict enum; 4 semantic result typed payload option.
- `verdict-set`: 1 normalized-verdict set.
- `execution-report`: 1 static-graph digest; 2 verdict-set digest; 3 per-obligation evidence-fact set; 4 canonical diagnostic set.

Raw logs, timing, PIDs, process completion order, and host error strings are excluded.

### Model and formal domains

- `assumption`: 1 assumption ID; 2 statement text; 3 scope typed payload; 4 exclusions text set; 5 approval binding.
- `finite-model`: 1 model ID; 2 state-space typed payload; 3 event-alphabet typed payload; 4 transition-relation typed payload; 5 initial-state set; 6 bound typed payload; 7 checked-invariant atom-revision-reference set; 8 assumptions object-reference set; 9 exclusions text set.
- `formal-property`: 1 property ID; 2 atom-revision-reference set; 3 property payload typed payload; 4 implementation-abstraction typed payload; 5 assumptions object-reference set; 6 exclusions text set; 7 correspondence approval binding.
- `proof-record`: 1 proof ID; 2 atom-revision-reference set; 3 formal-property object reference; 4 implementation-representation binding; 5 assumption object-reference set; 6 translator tool-identity set; 7 verifier tool identity; 8 verifier configuration digest; 9 external-model artifact-binding set; 10 generated-artifact-binding set; 11 proof-source artifact-binding set; 12 normalized verdict enum; 13 scope typed payload; 14 exclusions text set.

### Static graph and source domains

- `source-map-entry`: 1 object reference; 2 source-reference set; 3 source-role enum.
- `source-map`: 1 source-map-entry set.
- `spec-graph-semantic`: 1 atom-object set; 2 tombstone set; 3 decision-surface set; 4 evidence-carrier set; 5 implementation-link set; 6 derivation set; 7 coverage-policy-object set; 8 debt-record set; 9 waiver set; 10 oracle set; 11 finite-model set; 12 formal-property set; 13 assumption set; 14 proof-record set.
- `spec-graph-complete`: 1 semantic-graph digest; 2 source-map digest; 3 repository-config digest; 4 schema-tool-lock digest.
- `static-report`: 1 complete-graph digest; 2 authority facts set; 3 surface facts set; 4 evidence-registration facts set; 5 policy facts set; 6 freshness facts set; 7 debt-and-waiver facts set; 8 derivation facts set; 9 proof facts set; 10 canonical diagnostic set.

### Change domains

- `change-manifest`: 1 manifest ID; 2 base complete-graph digest; 3 head complete-graph digest; 4 change-disposition set; 5 approval binding option.
- `change-disposition`: 1 disposition ID; 2 impacted object kind; 3 stable object ID; 4 base digest option; 5 head digest option; 6 disposition enum; 7 reaffirmation digest option; 8 waiver object-reference option; 9 replacement object-reference set.
- `impact-set`: 1 directly changed object-reference set; 2 transitively impacted object-reference set; 3 policy-ratchet impact set.
- `impact-report`: 1 base complete-graph digest; 2 head complete-graph digest; 3 change-manifest digest; 4 impact-set digest; 5 disposition-closure facts set; 6 canonical diagnostic set.

## Object-kind, collection, and diagnostic ordering

Object-kind order is the order in the `spec-graph-semantic` field list above, followed by source/report/change-only kinds in their domain-table order. Within a kind, objects are ordered by:

1. stable ID canonical UTF-8 bytes;
2. full semantic object digest bytes; and
3. complete encoded object bytes as a final total-order tie breaker.

Two objects with the same canonical kind and stable ID are a duplicate-key error even if their digests differ. No object wins.

Object references are ordered by kind tag, stable ID bytes, then digest bytes. Atom revision references are ordered by atom ID then revision digest. Source references are ordered by canonical path bytes, start offset, then end offset. Tool identities are ordered by tool ID, version, executable digest, then configuration digest.

Canonical diagnostics are ordered by:

1. path option, with no path before a path;
2. original-byte start offset;
3. original-byte end offset;
4. stable diagnostic code bytes;
5. primary object-kind tag;
6. primary stable ID bytes; and
7. complete encoded structured-field bytes.

Two diagnostics that compare equal under all fields are duplicates and one canonical diagnostic is emitted. This diagnostic deduplication is schema-defined and must not hide distinct sources because source coordinates and structured fields participate in the key.

Enum tags define serialization order only. They MUST NOT be interpreted as assurance strength or severity. In particular, evidence classes remain orthogonal.

## Duplicate rejection

The following are errors before canonical serialization:

- duplicate normalized repository paths or portable collision keys;
- duplicate stable object IDs within an object kind;
- duplicate set elements;
- duplicate surface member keys or missing expected members;
- duplicate field tags or object keys;
- duplicate impact dispositions for the same required impact; and
- two authority declarations for one live atom ID.

First-wins, last-wins, traversal-order selection, and silent set deduplication are forbidden.

## SHA-256 rationale and algorithm migration

V1 chooses SHA-256 because normative graphs are small, SHA-256 has ubiquitous audited implementations, its 32-byte output is straightforward to bind in proof tools and external records, and throughput is not a load-bearing constraint for repository checks. BLAKE3 is materially faster and remains a reasonable candidate for a future schema, but its speed does not offset migration and verification surface in v1.

Algorithm identity is explicit in the schema, preimage envelope, binary digest value, and text display. A future algorithm requires:

1. a new semantic schema ID and algorithm tag;
2. new domain-separated preimages and golden vectors;
3. an explicit mapping artifact that binds each retained v1 identity to its new-schema identity where continuity is claimed;
4. readers that preserve v1 tombstones and historical proof bindings; and
5. change-impact review of every digest-bound carrier, derivation, policy, and proof.

No configuration flag may reinterpret existing `sha256:` values under another algorithm. Truncated digests are display aids only and can never satisfy identity or freshness.

## Rejected alternatives

- **Canonical JSON:** approachable but requires extensive number, escaping, map-order, duplicate-key, and Unicode rules while remaining awkward for arbitrary bytes.
- **Canonical CBOR:** standardized but brings a larger decoder and canonical-profile proof surface than the small required value algebra.
- **Protobuf:** unknown-field and default-elision behavior complicate exact schema identity and independent canonical reproduction.
- **Raw concatenation:** compact but vulnerable to field-boundary and domain confusion.
- **Hashing source text directly:** binds editorial syntax, locations, and parser trivia rather than semantic objects.
