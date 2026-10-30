# Version 1 governed decision surfaces

Status: frozen schema decision for task 2.6. This document defines finite domain construction, member identity, evolution, completeness, authority citation, and host-adapter extension without embedding host semantics in the core.

## Purpose and boundary

An atom-only graph can detect stale or uncovered requirements but cannot detect a requirement that was never authored. A governed decision surface supplies the finite domain against which absence becomes observable.

Examples include operation × type × execution-lane matrices, protocol message vocabularies, syntax-tag dispositions, supported configuration keys, and diagnostic families. These examples are not built into the core.

A surface disposition says that a normative decision exists. It does not say an implementation exists, a test ran, or behavior is correct.

## Surface identity

A surface has one stable uppercase object ID. A domain revision has:

- the same surface ID;
- a positive `generation` beginning at 1 and increasing by exactly one per accepted domain evolution;
- a member-key schema and disposition vocabulary identified by pinned host schema IDs;
- a complete current member set; and
- evolution history inside the member-key-schema payload.

The `decision-surface-domain` digest identifies the exact generation. A `DecisionSurface` binds exactly one current domain digest. Two current domains for the same surface ID are duplicate authority and neither wins.

## Domain construction modes

The member-key-schema payload declares exactly one construction mode.

### Enumerated

`enumerated` contains an explicit nonempty set of canonical member keys. It is appropriate when there is no meaningful independent axis decomposition, such as a closed syntax-tag vocabulary.

The domain approval attests that the enumeration itself is complete. The core can detect drift from that approved set but cannot infer omitted real-world concepts.

### Cartesian product

`product` contains an ordered nonempty list of axes. Each axis has:

- a stable lowercase axis ID;
- a pinned value schema;
- a nonempty unique canonical value set; and
- an approval binding or atom authority for the value vocabulary.

The current member set MUST equal the complete Cartesian product of every axis value set. The product is generated conceptually by the core's independent reference model and compared as a set; adapters cannot provide only selected rows. Unsupported, rejected, deferred, and not-applicable combinations remain members with explicit dispositions rather than being filtered out of the domain.

For a one-axis vocabulary, `product` is equivalent to the axis value set and is preferred when future axes are plausible.

### Prohibited implicit filtering

V1 has no filtered-product mode. Predicates such as “only valid combinations” are dispositions over the complete product, not domain constructors. This prevents a host adapter from making difficult or unsupported cells disappear before completeness checking.

A future schema may define a reviewed filter language, but it cannot reinterpret a v1 product.

## Member-key schema

A product member key is a record containing one field per axis in declared axis order. The field tag is the one-based axis position and the value is the axis's canonical typed value. The key's identity is its complete encoded bytes, not a display string.

An enumerated key is one `TypedPayload` under the declared key schema.

For all keys:

- display labels, source paths, row order, comments, and table formatting are nonidentity metadata;
- two input keys with equal canonical bytes are duplicates;
- two display labels may not map to one key or one label to multiple keys within a surface;
- unknown axis IDs, missing axis fields, extra fields, and wrong typed values are invalid; and
- the host adapter supplies deterministic display rendering separately from identity.

Member keys never contain implementation references, issue status, timestamps, or provider metadata.

## Core disposition meanings

Every current member has exactly one disposition.

### `normative`

The host payload records the positive semantic contract selected for the member. It requires one controlling current atom revision.

### `prohibited`

The host payload records the normative rejection and stable reason data. It requires one controlling current atom revision. A prohibited member is still a complete authored decision.

### `deferred`

The host payload records why a decision or support commitment is deferred and names one repository-owned `DebtRecord`. It requires one controlling atom revision that explicitly states the deferral. A deferred member is visible debt and is not considered supported, tested, or implemented. A blocking policy may forbid `deferred` while an adoption policy may temporarily permit it.

### `not-applicable`

The host payload is absent unless the host vocabulary requires structured reason data. Atom authority is required whenever the applicable coverage policy requires every domain decision to be authored. `not-applicable` cannot be inferred from a missing row.

The host disposition vocabulary may narrow allowed core variants and define payload schemas. It cannot add a core variant, change these meanings, or make absence a disposition.

## Completeness algorithm

For each surface, the deterministic core:

1. validates and hashes the domain declaration;
2. independently materializes the expected member set from `enumerated` or `product` construction;
3. compares the declared current member set to that expected set;
4. indexes dispositions by canonical member-key bytes while rejecting duplicates;
5. reports missing expected keys and unknown actual keys independently;
6. validates the one disposition and host payload for every expected key;
7. validates authority and policy references; and
8. emits `complete` only if every preceding step succeeds.

Equal row counts do not establish completeness. An unknown key paired with a missing key reports both. No source, insertion, or traversal order chooses a winner.

Completeness precedes coverage. Tests attached to present rows cannot make a surface with a missing row complete.

## Authority citation

A disposition contains at most one controlling `AtomRevisionRef` because normative authority must not be ambiguous. If several requirements jointly control one member, the repository authors one controlling atom that cites or composes those decisions; implementation and evidence can link to all relevant atoms separately.

A controlling citation is valid only when:

- it resolves to one live atom and exact revision;
- the atom's scope includes the host scope required by the surface schema;
- the atom kind is compatible with the disposition vocabulary;
- the atom lifecycle meets the applicable policy, with blocking policy normally requiring `ratified`; and
- the disposition payload does not contradict a schema-checkable field of the atom metadata.

The core validates schema-level compatibility only. It does not prove that natural-language text entails the host payload. That correspondence is review evidence under the atom's approval binding.

An implementation link, passing test, issue, code comment, or existing runtime behavior cannot replace the controlling atom.

## Domain evolution

The member-key-schema payload contains an append-only `evolution` sequence. Each record binds:

1. prior generation and domain digest;
2. new generation;
3. added member or axis-value set;
4. retired member or axis-value set;
5. replacement mapping from each retired key/value to zero or more new keys/values;
6. rationale;
7. approval binding; and
8. migration disposition reference.

Generation 1 has no predecessor and an empty evolution sequence.

### Additions

Adding an axis value to a product adds every new Cartesian-product member induced by that value. All new members require dispositions in the same head graph. A new surface axis is a breaking domain evolution because it changes every key shape; it requires a new key schema ID, explicit mapping, and complete member migration.

### Retirement and replacement

A member or axis value cannot silently disappear. Retirement requires an approved evolution record, explicit base-to-head change impact, and a replacement mapping. An empty replacement set means deliberate withdrawal, not forgotten work.

Historical domain digests and evolution records remain addressable. Existing proof, evidence, or change records retain their exact historical binding. A retired member key cannot be reused for a different meaning.

### Rename

Rename is retirement plus addition with a one-to-one replacement mapping. Display-label-only edits that preserve canonical key bytes are not renames and do not change domain identity.

### Vocabulary and payload changes

Changing key interpretation, disposition meaning, or payload field semantics requires a new host schema ID and explicit migration. A schema ID cannot change meaning in place.

### Ratchet interaction

Any domain contraction, `normative`/`prohibited` to `deferred`, authority removal, blocking-to-advisory policy change, or reduced required evidence role is a ratchet impact. It remains visible even when an approved waiver permits a process gate to continue.

## Host-adapter extension contract

A host extension is shell-side and registers:

- immutable host schema IDs for axis values, member keys, disposition payloads, and display rendering;
- canonical encode/decode and validation functions for each typed payload;
- a complete construction declaration using core `enumerated` or `product` mode;
- positive and negative fixtures for every payload variant and malformed form;
- mutation fixtures for missing, duplicate, and unknown members;
- an exact tool identity and configuration digest; and
- explicit scope compatibility rules for controlling atoms.

The deterministic core owns set comparison, product expansion, duplicate rejection, authority closure, policy facts, and canonical reporting. The host extension owns only interpretation of its typed payloads.

Unknown host schema IDs fail loudly. A host adapter cannot:

- define an implicit filter;
- infer a disposition from implementation behavior;
- synthesize missing members;
- downgrade a diagnostic to an empty result;
- mark a row tested or implemented through disposition payload;
- change core enum meanings or assurance classes; or
- read network/provider state as semantic input.

## Generic example

A host may declare axes `operation={read,write}` and `transport={local,remote}`. The expected domain is exactly four members. If `remote/write` is unsupported, the member remains present with a `prohibited` disposition and atom authority. If the implementation has no remote backend yet but the behavior is intended, the member remains `normative`; implementation/evidence reports show the missing support separately.

Removing `remote` from the axis to make the table green is a domain contraction and ratchet impact, not a fix.

## Rejected alternatives

- **Infer the surface from authored atoms:** cannot detect absent atoms.
- **Infer rows from implementations or tests:** turns current behavior into authority and hides unsupported cells.
- **Allow product filters:** makes omission easy to encode as “invalid combination.”
- **Use display labels as IDs:** renaming or localization would destroy stable identity.
- **Allow multiple controlling atoms per member:** creates ambiguous primary authority and difficult supersession semantics.
- **Treat deferred as complete assurance:** records debt but does not satisfy support or evidence obligations.
