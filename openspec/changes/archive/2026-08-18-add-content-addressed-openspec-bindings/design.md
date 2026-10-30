## Context

Buoy already calculates content identities for OpenSpec requirements, scenarios, and Rust items. `ParsedItemIdentity` binds parser-canonical bytes, a source-item kind, a repository path, and an optional enclosing identity.

The current Rust attribute contains a full TOML implementation record. The Rust adapter supplies the current item digest during each run. Therefore, a Rust source edit changes the implementation-link object but does not create a static stale-link diagnostic.

The OpenSpec adapter retains requirement and scenario items inside each adapter product. The repository check currently discards source mappings before aggregate graph validation. Repository-level source traversal is therefore unavailable to the CLI.

The proposed model treats parsed source data as a content-addressed directed acyclic graph. A compact Rust attribute references one immutable implementation-binding node by its digest.

```text
current source roots
│
├── Rust item node ◄──────────────┐
│                                 │ subject
└── OpenSpec document             │
    └── requirement node ◄──── binding node ◄── Rust attribute hash
        ├── scenario node          │ authority
        │   ├── GIVEN step         ▼
        │   ├── WHEN step         atom revision
        │   └── THEN step
        └── scenario node
```

## Goals / Non-Goals

**Goals:**

- Reduce an inline Rust implementation link to one digest.
- Bind one exact Rust source node to one exact OpenSpec requirement node and atom revision.
- Detect a change on either side through static current-graph closure.
- Permit canonical traversal from the binding to Rust, OpenSpec descendants, and authority.
- Preserve deterministic offline behavior across absolute roots and traversal orders.
- Keep generic graph values in the `no_std` core.

**Non-Goals:**

- Do not infer requirement satisfaction from a fresh link.
- Do not add behavioral evidence, execution results, or proof claims.
- Do not read sibling repositories from the Buoy CLI.
- Do not add Nix parsing or Nix subtree identities.
- Do not establish authenticity from a digest.
- Do not preserve the verbose attribute as a second active writer after migration.

## Decisions

### 1. Use one immutable binding node behind the source hash

The attribute contains only a canonical binding digest:

```rust
#[buoy::implements("blake3-256:<payload>")]
pub fn render_catalog(catalog: &Catalog) -> String {
    // ...
}
```

The digest identifies an immutable `ImplementationBinding` record. The record contains:

- a stable relation ID
- a navigation relation
- the exact Rust subject-node digest
- the exact OpenSpec requirement-node digest
- the exact atom ID and revision

The record digest binds all fields under `buoy/implementation-binding/v1`. The stable relation ID preserves base-to-head impact identity when record digests change.

**Alternatives considered:**

- Put only the requirement digest in Rust. Rejected because a Rust edit leaves the source reference fresh.
- Recalculate a binding candidate from every requirement. Rejected because stale records lose their preimage and produce weak diagnostics.
- Keep all fields in the attribute. Rejected because it preserves manual digest copies and the current verbosity.

### 2. Store binding records in an immutable repository object store

Each generated record uses this path:

```text
buoy/objects/v1/<algorithm>/<hex-payload>.toml
```

The path does not contain a colon. The adapter reconstructs the canonical digest from the algorithm directory and payload filename.

The file contains the canonical record preimage, but it does not contain a mutable digest field. The adapter calculates the record digest and compares it with the path.

The `buoy bind` command creates a new file and updates one selected attribute. It never changes an existing record. Old records can remain for history and diagnostics, but only current roots can satisfy closure.

Only configured standing-spec selectors can create current OpenSpec document roots. Active and archived change artifacts cannot create current roots.

**Alternatives considered:**

- Use one mutable lock file. Rejected because record replacement obscures immutable object identity and complicates concurrent updates.
- Put the full record beside each Rust item. Rejected because the source returns to the verbose form.

### 3. Add generic source nodes and typed edges to the core

The shell parses source syntax. It passes validated value types into `buoy-core`.

The core adds these generic values:

- `SourceNode`
- `SourceNodeRef`
- `SourceEdge`
- `SourceEdgeKind`
- `SourceGraph`
- `SourceTraversal`

A `SourceNode` wraps one parsed item identity and its ordered child references. The node digest binds the item identity, child relation, position, and child digest.

The initial edge vocabulary contains:

- `contains`
- `implements`
- `authority`

The core owns canonical ordering, uniqueness, closure, acyclicity, reachability, and bounded traversal. It does not parse Markdown, Rust, TOML, or repository paths.

The root shell depends on `buoy-core`. The core does not depend on the root shell, parsers, filesystem APIs, or process APIs.

**Alternatives considered:**

- Add OpenSpec-specific core node variants. Rejected because host syntax belongs in adapters.
- Keep source mappings outside aggregate validation. Rejected because cross-adapter closure then remains impossible.

### 4. Materialize OpenSpec as a Merkle-style source tree

The OpenSpec adapter adds these source-item kinds:

- `openspec/document/v1`
- `openspec/requirement/v1`
- `openspec/scenario/v1`
- `openspec/step/v1`

The adapter constructs nodes from leaves to root. A scenario node references its ordered steps. A requirement node references its ordered scenarios. A document node references its ordered requirements.

Child positions come from parser source order before canonical output sorting. Semantic slugs identify scenarios. A sibling edit must not renumber an unchanged scenario.

The requirement node still maps to one atom. Scenario and step nodes remain navigational in this version.

A changed step changes its scenario node and each containing requirement node. Unchanged sibling requirements retain their identities.

The source tree is Merkle-style rather than a complete CommonMark AST. It contains only the OpenSpec semantic nodes that the pinned adapter recognizes.

**Alternatives considered:**

- Retain only requirement bytes. Rejected because clients cannot follow the scenario structure.
- Expose the complete parser AST. Rejected because parser internals become a public semantic schema.

### 5. Exclude recognized Buoy attributes from Rust subject identity

The Rust adapter removes recognized `buoy` binding attributes before it creates parser-canonical subject tokens. Every other semantic token remains in the subject identity.

This rule prevents a digest cycle:

```text
attribute hash -> binding node -> Rust subject hash
```

A binding-hash replacement does not change the subject. A function signature, body, non-Buoy attribute, path, or enclosing identity change does change the subject.

The adapter keeps the original byte span for diagnostics and updates. It does not remove attributes from compiler input.

**Alternatives considered:**

- Include the compact attribute in the subject digest. Rejected because no finite self-referential digest exists in the general case.
- Exclude all attributes. Rejected because semantic Rust attributes can change implementation behavior.

### 6. Resolve all references after aggregate adapter materialization

Each adapter emits source nodes and unresolved typed references. No per-file adapter claims repository closure.

The shell aggregates all selected products. Then it constructs one `SourceGraph`, maps requirement nodes to atoms, and resolves each binding record.

A fresh binding requires all these facts:

1. The attribute references one available binding record.
2. The record digest equals the attribute digest.
3. The attached Rust subject equals the record subject.
4. The requirement node is reachable from a current OpenSpec document root.
5. The requirement node maps to the record atom ID and revision.
6. Every traversed edge has the required target kind.

The shell emits separate deterministic diagnostics for each failed fact. It does not replace several failures with one optimistic result.

### 7. Require collision-resistant digests for hash-only references

A hash-only reference removes all human-readable locator fields from the source attribute. Therefore, this feature requires `blake3-256` or `sha-256`.

The build must include the selected algorithm feature. Configuration with `xxh3-128` fails with `CAB-DIGEST-CLASS` when compact bindings are enabled.

The digest supplies collision resistance only. Repository review, signatures, and provider controls remain separate authenticity mechanisms.

### 8. Keep lifecycle and assurance facts separate

A binding record has no mutable lifecycle field. A new source or requirement revision creates a new immutable record and digest.

Current-root reachability determines freshness. Historical records remain noncurrent even when their object files remain available.

A fresh implementation binding supplies navigation and impact edges only. It does not supply:

- behavioral coverage
- an execution verdict
- formal proof
- approval authority
- implementation correctness

Effectful oracles remain under `buoy execute`. Static graph construction runs no command and reads no provider state.

### 9. Validate the design with a focused OpenSpec spike first

The first implementation slice uses one saved OpenSpec document and one Rust function. It proves these cases:

- one compact hash resolves
- traversal reaches scenarios and steps
- an OpenSpec step edit makes the binding stale
- a Rust body edit makes the binding stale
- a compact-attribute edit alone preserves the subject identity
- separate absolute roots produce equal canonical output
- a historical but noncurrent requirement remains stale

The spike must fail before production algorithms exist. Production code follows the positive and negative fixtures.

The exact acceptance oracle is `devenv test`. Success means that all tasks complete with exit status zero and no coverage regression.

## Deterministic Ordering and Budgets

Source nodes use digest order for sets. Child edges use `(relation, position, target digest)` order. Binding records use stable relation ID, then complete digest.

Repository configuration adds positive limits for source nodes, source edges, traversal depth, and visited nodes. Construction fails before partial graph acceptance when any limit is exceeded.

Absolute roots, file discovery order, locale, wall clock, network state, and provider state do not enter canonical values.

## Trust and Proof Boundaries

The CommonMark and Rust parsers remain trusted shell components. This change does not prove parser correctness or natural-language correspondence.

Verus can examine pure closure, ordering, and bounded traversal algorithms after the Rust model stabilizes. A Verus result does not prove BLAKE3, parser output, allocation behavior, or the shell.

AeneasVerif can examine a safe sequential subset of graph traversal. It does not support the parser, filesystem code, or all collection behavior without abstraction.

No formal proof claim enters this change until a proof record binds exact source, property, assumptions, toolchain, and proof output.

## Risks / Trade-offs

- The new object store adds repository files. Generated paths and an explicit update command limit manual work.
- A source move changes path-bound identities. The update command creates a new record after review.
- Retained historical objects increase repository size. Garbage collection remains a separate reviewed change.
- OpenSpec child nodes expand graph size. Explicit budgets prevent unbounded traversal.
- A digest-only attribute is not readable without tooling. Hover, traversal, and machine reports must show the stable relation ID and requirement ID.
- The feature requires a nondefault digest feature in current builds. Build and configuration diagnostics must identify the missing feature.
