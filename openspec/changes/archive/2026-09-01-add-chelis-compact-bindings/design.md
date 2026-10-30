## Context

The Rust side carries compact bindings: `#[buoy::implements("<digest>")]` resolving to an immutable `buoy/implementation-binding/v1` object whose filename is its own hash, with core validation over `graph.implementation_bindings`. Chelis links are inline ten-line records whose digest fields churn saved sources on every rebind. The core type, the object schema, and the freshness algebra are repository-independent; only anchoring and store resolution are host work.

## Goals / Non-Goals

**Goals:** the same store layout, object schema, and core semantics as the Rust compact form; a two-line in-source footprint; conversion and regeneration behind reviewed dry-run plans; no enforcement weaker than the inline form.

**Non-Goals:** compact carriers, surfaces, or oracles; authoring bindings from scratch through `bind` (conversion of existing inline links covers the current need); Deep dialect; host adoption.

## Decisions

### The record form is two lines

```
-- chelis:provenance/v1 binding
-- record = xxh3-128:<hex>
def bound_declaration(...) = ...
```

The marker keeps the existing kind grammar (`binding` is one more kind token); the single `record` field carries the address. Anchoring, unattached handling, and parse-error behavior are inherited unchanged from the total record grammar.

### The store is a configured mount prefix, bounded by selectors

The adapter configuration gains an optional `binding-store` mount prefix (for example `chelis/objects`). It names where the frozen upstream object layout is mounted, not the layout itself: one address resolves at exactly

```
<binding-store>/buoy/objects/v1/<algorithm>/<hex>.toml
```

and nowhere else. That is `buoy::adapters::extract_binding_in_mount` used verbatim, so a Chelis store holds byte-identical objects to a Rust store and inherits the canonical-form, path-digest, schema, relation, digest-class, and digest-member checks without reimplementing any of them.

Resolution reads store bytes through the snapshot's artifact map, so the store MUST lie under a selector root; configuring a store outside every selector root is a configuration error at the boundary. Lookup is exact: an object planted at any other path answers for no address. Alternative: implicit repository-root `buoy/objects/v1`. Rejected: root-level artifacts are unreachable through selectors (the recorded `reef.toml` lesson), and hosts should choose where immutable objects live.

### The compact form requires a collision-resistant active member

Upstream `parse_stored_binding` refuses any stored binding whose locator, subject, requirement, or atom revision falls outside `DigestAssuranceClass::CollisionResistant`, and every digest in one stored binding must share the member the rest of the graph uses. A build whose active member is `xxh3-128` therefore resolves no compact binding at all and says so. This is not a test-configuration detail: adopting the compact form means pinning `blake3-256` or `sha-256` as the digest feature for the whole host.

Nautilus therefore cannot adopt the compact form by converting links alone. It must first migrate its graph digests to BLAKE3-256 and re-execute its receipts under that member, because a rebind changes static identity and committed receipts are bound to it. That migration is a prerequisite of the follow-on change, not a consequence of it.

### Materialize into the core, not beside it

A resolved object constructs the core `ImplementationBinding` value. Static verification checks the address, subject, requirement, atom revision, and relation-form uniqueness. Staleness contexts carry bound and current values.

The integration exposed one missing core closure edge: `validate_graph` did not check atom revisions in `implementation_bindings`. This change adds that edge to the shared core. A stale compact binding now rejects with `SPG-REF-DANGLING-OR-STALE`, which matches the inline form without host-specific freshness algebra.

A valid object remains in the rejected graph when its subject, requirement, or atom revision is stale. This matches inline-link behavior and preserves conservative impact data. An object with an invalid address never materializes.

### The requirement digest is the authority record block

A compact binding records `requirement` as a source-node digest. The Chelis analogue of the Markdown atom block is the authority record block itself. The digest covers the authority marker through its last field line, as saved. Static verification and `trace` both recompute this digest. Therefore, a formatting-only authority edit also requires a reviewed rebind.

### Conversion is mechanical and one-way

`bind --link <ID> --write` reads one inline `implementation-link`, derives the binding object (subject digest = the link's enforced item digest, requirement digest = the authority record block digest, atom revision = the link's bound revision), writes the object at its content address, and replaces the inline record block with the two-line form. The declaration and every other byte stay untouched. No reverse command exists; reverting is a git revert.

Conversion refuses everything it cannot re-encode exactly, because narrowing a claim to fit the stored schema would launder it. The stored schema carries one `implements` relation over one atom, so any other navigation and any atom count other than one are refused. A link whose recorded item digest is stale is refused too: recomputing the subject from current bytes would turn a rejected link into an accepted binding. Every planned object is read back through the same upstream parser at the exact path the plan names before the plan can succeed, and a destination already holding different bytes refuses at planning time, so a dry run reports it.

### Rebind supersedes objects instead of editing them

Content-addressed objects are immutable: a stale binding regenerates as a successor object plus an address update in source, and the write removes the superseded object in the same reviewed plan. The dry run therefore shows three entry classes: `object-write`, `address`, and `object-remove`. Store hygiene is a property of the plan, not a separate garbage collector.

## Risks / Trade-offs

- **A bare hash reads as nothing at the code site** → `trace` expands any binding in one command with the object, subject, and requirement hops each recomputed from saved bytes, and the object file is committed TOML one hop away.
- **Two forms for links during migration** → address integrity, subject currency, atom revisions, and impact closure are enforced at equal strength; conversion refuses any inline shape the stored schema cannot represent.
- **Store files multiply per rebind** → superseded objects are removed by the same write, filtered against every successor the plan writes so one marker's predecessor is never another's current object; the plan keeps the store minimal and reviewable.

## Migration Plan

1. Record kind, configuration key, resolution, and the two diagnostics with crate tests and pack fixtures.
2. `bind` conversion with dry-run/write/idempotence tests.
3. Rebind and trace extensions with successor-object tests and goldens.
4. Standard assurance regeneration; full `devenv test`.
5. Follow on in Nautilus: configure the store, convert the three inline links, and let the gate's rebind dry-run guard the store.

## Open Questions

None.
