# Shell-Side Adapter SDK v3

The public adapter contract is `buoy::adapter_sdk`. Its schema identity is `buoy.adapter-sdk/v3`.

The root `buoy` crate implements the contract in the `std` shell. `buoy-core` contains no parser, adapter registry, provider client, or host-specific item enum.

## Descriptor Boundary

Each `ShellAdapter` supplies one `AdapterDescriptor`. The descriptor contains:

- one immutable versioned adapter ID
- one exact parser ID and parser version
- one versioned syntax-schema ID
- one versioned configuration-schema ID
- one nonempty set of syntax capabilities
- one nonempty set of source-item kinds

The source-item kinds use `SourceItemKind`. This type accepts a versioned lower-case ID and stores its private validated text.

`AdapterRegistry::register` freezes the descriptor. Registration rejects a duplicate adapter ID or a duplicate source-item kind.

Registry lookup uses a `BTreeMap`. Discovery order and registration order are not canonical inputs.

## Input Boundary

`AdapterConfiguration` binds exact configuration bytes to an adapter. `SavedSnapshotInput` contains only:

1. a validated repository-relative path
2. exact saved bytes
3. the explicit configuration binding

The input does not contain an absolute root, provider state, a clock, an editor state, or an unsaved buffer.

## Output Contract

A `ShellAdapter` returns an `AdapterProduct`. The registry accepts the product only when all conditions are true:

- The graph contains validated `buoy-core` values.
- Each graph object has an exact parser-backed source mapping.
- Each source mapping uses a descriptor-declared source-item kind.
- Each product retains source nodes, declared edges, current roots, authorities, unresolved bindings, and stored records.
- Each nonnormative form has an explicit disposition.
- Each other selected form has a stable diagnostic.
- Each span belongs to the input path and saved bytes.
- Each output set is duplicate-free and canonical.

The output digest binds the descriptor, configuration, input, graph, mappings, dispositions, diagnostics, and visible debt.

The repository shell combines all retained products before reference resolution. It then builds one bounded, closed, acyclic source graph.

Only configured standing-spec selectors can declare current OpenSpec roots. An adapter must not declare active or archived change documents as current roots.

An unresolved compact binding contains the exact subject span and record locator. The repository resolves it only after every adapter product is available.

A fresh binding supplies navigation and impact information only. It does not establish coverage, execution, proof, approval, correctness, or correspondence.

The digest does not establish parser correctness, authority, freshness, execution, assurance, or formal proof.

## Built-In Adapters

The registry contains these built-in adapters:

- `artifact/v1` with `artifact/file/v1` and `artifact.opaque-bytes/v1`.
- `binding/v1` with immutable objects and artifact attachment sidecars.
- `markdown/v1` with `markdown/fence/v1` and `markdown/external-trace/v1`.
- `openspec/v1` with document, requirement, scenario, step, and `openspec/external-trace/v1` source-item kinds.
- `rust/v1` with `rust/item/v1` and compact implementation-binding attachments.

The Markdown adapter uses `pulldown-cmark` 0.13.5. The OpenSpec adapter uses the same pinned parser.

Both adapters call `Parser::new` without optional parser flags. The patched dependency replaces the version affected by RUSTSEC-2026-0414.

The parser version forms part of each adapter descriptor. A version change changes descriptor and materialization digests.

The dependency and descriptor edits also change exact source-file bindings. Semantic graph identities change only if the parser output changes.

Refresh affected source-bound assurance records after a parser update. Preserve frozen v1 and v2 protocol bytes.

The Rust adapter uses `syn` 2.0.106. Each parser-backed adapter supplies exact source ranges.

## Shared External Trace Contract

One shared parser owns the `buoy-external-trace-v1` value grammar. Every accepted host adapter calls that parser and returns equal link values for equal fence bytes.

Each host adapter keeps its own parser-backed item identity, source-item kind, and exact source mapping. No host copies another host identity.

A host that returns authority nodes and trace links uses one complete product constructor, so links, graph objects, and mappings reach the registry together.

An external trace link supplies navigation and impact data only. It reads no provider state and adds no assurance class.

## Opaque Artifact Contract

The `artifact/v1` adapter does not use a language parser. It accepts arbitrary saved bytes.

Its one source span must cover the complete input byte range. Its item identity binds fixed opaque metadata and exact saved bytes.

The SDK rejects these artifact products:

- An undeclared source-item kind.
- A partial or out-of-range source span.
- A semantic graph object.
- An authority or assurance claim.
- An unsupported syntax capability.

The adapter accepts only the forbidden authority expectation. It emits one navigational root and no semantic object.

## Extensible Kind Protocol

`validate_parsed_item_protocol` compares three validated values:

- the adapter ID
- the parser schema ID
- the source-item kind

The function accepts only a kind that the selected adapter declares. It rejects a cross-adapter substitution or an undeclared kind.

The core does not require a new enum variant for a new host adapter. The adapter must declare and fixture each new versioned kind.

Schema v3 has no runtime reader for a numeric item-kind tag. Adoption requires source reingestion and complete identity regeneration.

## Downstream Use

1. Implement `ShellAdapter` in the downstream shell.
2. Declare each syntax capability and source-item kind.
3. Register the adapter before materialization.
4. Construct an exact `AdapterConfiguration`.
5. Supply saved bytes through `SavedSnapshotInput`.
6. Return complete core values and exact source mappings.

Do not label host syntax with a kind from another adapter. The registry rejects this output.

If a language has no dedicated adapter, use a raw-artifact target or add an adapter. Do not represent unsupported syntax as Rust.
