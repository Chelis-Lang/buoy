# Content-addressed implementation bindings

Status: active schema v3 navigation protocol.

## Compact Rust syntax

Attach one compact locator to the exact Rust item:

```rust
#[buoy::implements("blake3-256:0e857e33e387325fbf457e426bc42286d9d99926b19f3efa3c203795cfeeb6c4")]
pub fn render_catalog() {}
```

The locator contains exactly one canonical algorithm-qualified digest: `<canonical-name>:<lowercase hexadecimal payload>`. Its member must equal the repository's active collision-resistant digest member. A raw payload with no algorithm name, or a member that disagrees with the active configuration, fails at the parse or configuration boundary with a deterministic diagnostic.

The Rust adapter removes recognized compact and legacy attributes from subject tokens. Thus, locator replacement does not change the subject identity.

Do not place compact and legacy binding attributes on one item. Buoy rejects mixed writers with `CAB-MIXED-BINDING-SYNTAX`.

## Immutable object store

The locator selects this path:

```text
buoy/objects/v1/<algorithm>/<digest>.toml
```

The record uses schema `buoy/implementation-binding/v1`. It binds these values:

1. The stable relation ID.
2. The navigation kind.
3. The exact implementation source node.
4. The exact requirement source node.
5. The exact governing atom revision.

Buoy checks the record content against its locator and path. A record update always creates a new object. The old object remains unchanged.

## Update process

Run a static machine report before the first binding:

```text
buoy static --root ROOT --config FILE --format machine
```

If the command exits with a nonzero status, correct all diagnostics.

Run the command again.

The report omits `requirement_subjects` until the static check is diagnostic-free.

Select exactly one `requirement_subjects` row by stable ID, scope, and logical path. Use its complete `subject` value as the requirement selector.

Run `buoy bind` with explicit selectors:

```text
buoy bind --root ROOT --config FILE --requirement DIGEST --implementation DIGEST --relation ID --format machine
```

The command rejects a repository that has static diagnostics.

The command creates one immutable record. It then replaces the selected compact attribute.

The command does not delete an old record. If any operation fails, it leaves the source file and object store unchanged.

## Current roots and traversal

Configured standing-spec selectors create current OpenSpec roots. Active and archived change documents do not create current roots.

OpenSpec documents contain requirements. Requirements contain scenarios. Scenarios contain steps.

Child positions use parser source order. Canonical graph order does not renumber semantic scenario identities.

A fresh binding resolves from its implementation subject to the exact requirement and governing atom revision. Traversal also exposes scenarios and ordered steps.

## Stale and invalid bindings

Buoy uses stable diagnostics for each primary fault:

| Diagnostic | Meaning |
|---|---|
| `CAB-RECORD-ABSENT` | The locator has no stored record. |
| `CAB-RECORD-DIGEST` | The stored record content does not match the locator. |
| `CAB-SUBJECT-STALE` | The exact implementation subject is not current. |
| `CAB-REQUIREMENT-STALE` | The exact requirement is not current. |
| `CAB-REQUIREMENT-KIND` | The requirement reference selects another source-node kind. |
| `CAB-AUTHORITY-MISMATCH` | The requirement authority differs from the record atom. |
| `CAB-AUTHORITY-AMBIGUOUS` | More than one authority governs the requirement. |
| `CAB-DIGEST-CLASS` | A compact identity lacks the collision-resistant class. |
| `CAB-DIGEST-MEMBER` | A record mixes digest members. |
| `CAB-SUBJECT-ATTACHMENT` | The record subject differs from the attached current item. |
| `CAB-REQUIREMENT-SUBJECT-NODE` | A retained requirement authority lacks its source node. |
| `CAB-REQUIREMENT-SUBJECT-ATOM` | A retained requirement authority lacks its live atom. |
| `CAB-REQUIREMENT-SUBJECT-AMBIGUOUS` | More than one live atom has the requirement ID. |
| `CAB-REQUIREMENT-SUBJECT-REVISION` | The source authority and live atom revisions differ. |
| `SRG-SOURCE-CYCLE` | The repository source graph contains a directed cycle. |

A retained historical node is not current. Its binding fails with the stale diagnostic for that side.

## Opaque artifact sidecars

Opaque artifacts use repository-owned sidecars instead of Rust attributes. The sidecar selects one immutable object for one stable relation.

The mount-relative sidecar path is:

```text
buoy/attachments/v1/<algorithm>/<relation-key-digest>.toml
```

The sidecar binds the relation, complete logical artifact path, current subject, and immutable record.

The selected artifact mount owns both files. Physical writes use `<mount-root>/buoy/...`.

## Opaque artifact update process

Read the exact current requirement subject from `buoy static --format machine`. `buoy trace --requirement-id` cannot bootstrap a relation that does not exist.

Run `buoy bind` with the complete mount set and one logical artifact path:

```text
buoy bind --mount ID=PATH --config FILE --requirement DIGEST --artifact LOGICAL_PATH --relation ID --format machine
```

The command stages the object before it replaces the sidecar. A failed precommit operation leaves the prior sidecar unchanged.

The command never edits the selected artifact. Old immutable objects remain stored.

See [`logical-mount-artifacts.md`](logical-mount-artifacts.md) for the complete configuration and transaction contract.

## Read one current or historical relation

Run `buoy trace` to read stored relations without a write operation:

```text
buoy trace --root ROOT --config FILE --relation-id ID --format machine
buoy trace --root ROOT --config FILE --record DIGEST --include-history --format human
```

The query accepts exactly one typed selector. A requirement ID, a relation ID, and a logical implementation path each select current relations. A complete record digest selects one exact immutable object.

The result classifies each record as `current`, `stale`, or `historical`. A record stays historical while no current source item declares its locator.

A selected stale current attachment emits its projection and then returns exit `2`. A historical row never changes a successful exit class.

The successful-bind report and one fresh trace row read the same internal facts, so the two views agree about one relation.

See [`readable-trace-projections.md`](readable-trace-projections.md) for the complete report contract.

## Assurance boundary

A fresh binding supplies navigation and impact information only. It does not establish coverage, execution, proof, approval, correctness, or prose-to-code correspondence.

A trace projection carries the same limit. It reports stored navigation facts and adds no assurance class.
