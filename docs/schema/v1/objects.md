# Version 1 core object schemas

Status: frozen schema decision for task 2.5. This document defines the validated values consumed and returned by the deterministic core. `canonical-encoding.md` is authoritative for binary field tags and digest preimages; this document is authoritative for value constraints, enum tags, reference kinds, and cross-object validity.

## Common scalar rules

Unless a type says otherwise:

- stable object IDs use the uppercase atom-ID grammar from `atoms.md`;
- authority, owner, adapter, schema, relation, and namespace IDs use the lowercase scope-ID grammar from `atoms.md`;
- human statements, rationales, remediation text, and exclusions are nonempty canonical text;
- sets are duplicate-free and canonically sorted;
- every digest is full and algorithm-qualified;
- every repository path and source span follows `repository-inputs.md`;
- unknown fields and enum variants are unsupported errors; and
- an object is valid only after all of its internal references close to the expected object kind and digest.

An empty string is never a substitute for an absent value. Optional values use explicit `none`/`some` encoding.

## Closed enum tags

Tags are serialization identities, not severity or assurance rankings.

### Object kind

| tag | variant |
|---:|---|
| 1 | `atom` |
| 2 | `tombstone` |
| 3 | `decision-surface` |
| 4 | `evidence-carrier` |
| 5 | `implementation-link` |
| 6 | `derivation` |
| 7 | `coverage-policy` |
| 8 | `debt-record` |
| 9 | `waiver` |
| 10 | `oracle` |
| 11 | `finite-model` |
| 12 | `formal-property` |
| 13 | `assumption` |
| 14 | `proof-record` |
| 15 | `parsed-item` |
| 16 | `reaffirmation` |
| 17 | `change-manifest` |
| 18 | `change-disposition` |

Repository configuration, snapshots, source maps, reports, facts, tool identities, and raw artifacts are domain values rather than `SpecGraph` object kinds.

### Other enums

| enum | tag assignments |
|---|---|
| atom kind | `behavioral=1`, `structural=2`, `constraint=3` |
| ID allocation class | `live=1`, `terminal=2` |
| live lifecycle | `proposed=1`, `ratified=2` |
| terminal lifecycle | `superseded=3`, `withdrawn=4` |
| evidence role | `positive=1`, `negative=2`, `property=3`, `exhaustive-model=4`, `formal-property=5`, `structural=6` |
| enforcement mode | `advisory=1`, `blocking=2` |
| policy lifecycle | `active=1`, `superseded=2`, `retired=3` |
| debt lifecycle | `active=1`, `discharged=2`, `superseded=3` |
| waiver lifecycle | `active=1`, `discharged=2`, `revoked=3`, `superseded=4` |
| surface disposition | `normative=1`, `prohibited=2`, `deferred=3`, `not-applicable=4` |
| navigation kind | `implements=1`, `owns=2`, `consumes=3`, `configures=4` |
| artifact binding | `object=1`, `repository-artifact=2` |
| derivation relation | `byte-equal=1`, `generated-from=2`, `structurally-equivalent=3`, `host-consistency=4` |
| oracle kind | `test=1`, `property=2`, `exhaustive-model=3`, `structural=4`, `formal=5`, `derivation-consistency=6`, `host=7` |
| verdict | `not-run=1`, `pass=2`, `fail=3`, `error=4`, `timeout=5` |
| source role | `authority=1`, `carrier=2`, `implementation=3`, `surface=4`, `policy=5`, `derivation=6`, `model=7`, `property=8`, `assumption=9`, `proof=10`, `debt=11`, `waiver=12` |
| change disposition | `reaffirmed=1`, `revised=2`, `retired=3`, `waived=4` |
| obligation kind | `authority=1`, `surface-completeness=2`, `surface-authority=3`, `required-role=4`, `freshness=5`, `oracle-selection=6`, `execution=7`, `derivation-consistency=8`, `proof=9`, `change-impact=10` |
| assurance class | `linked=1`, `tested=2`, `exhaustively-checked=3`, `formally-proved=4`, `structurally-enforced=5`, `review-only=6` |

## Common nested values

### `ObjectRef`

Contains object kind, stable ID, and expected full semantic object digest. The reference is valid only when exactly one object of that kind and ID exists and has that digest. Atom semantic correspondence uses `AtomRevisionRef`, not `ObjectRef`.

### `AtomRevisionRef`

Contains atom ID and expected `atom-revision` digest. It resolves to either one live atom with that revision or, where the containing schema permits historical references, one tombstone with that final revision. A live-coverage reference cannot resolve only to a tombstone.

### `SourceRef`

Contains canonical path plus inclusive original-byte start and exclusive original-byte end. `start <= end`, and `end` MUST NOT exceed the snapshotted file length. Empty spans are allowed only for whole-file or missing-token diagnostics explicitly identified by their diagnostic schema.

### `ApprovalBinding`

Contains approval-authority ID, review-record path, and the review record's `raw-artifact` digest. The authority ID must be configured for the approved object's namespace and action. Approval is `review-only` evidence.

### `TypedPayload`

Contains a host schema ID and canonical payload bytes. The host schema ID must be declared in the schema/tool lock. Opaque payload bytes are compared exactly; the core does not guess semantics for unknown schemas.

### `ToolIdentity`

Contains tool ID, version, executable raw-artifact digest, and configuration digest. Script wrappers and templates that affect semantics are included in the configuration artifact or modeled as separate tools.

### `ArtifactBinding`

For `object`, contains exactly one `ObjectRef`, no path, and the referenced semantic digest. For `repository-artifact`, contains no object reference, one canonical path, and that file's `raw-artifact` digest. Mixed or empty bindings are invalid.

### `ApprovalAuthority`

Repository configuration maps an authority ID to:

- allowed object-kind and action pairs;
- allowed stable-ID namespaces;
- review-record selector;
- optional separation-of-duty rules represented as a pinned policy payload; and
- the authority declaration's own approval binding.

V1 validates repository-owned records and configured authority IDs. It does not authenticate provider identities or infer who authored a file.

## Identity and lifecycle objects

### `Atom`

Fields and digest boundaries are fixed by `atoms.md` and `canonical-encoding.md`.

Validity requires:

- one registered, permanently reserved ID;
- one authority source in selected authority roots;
- a computed revision matching the semantic fields;
- a live lifecycle state;
- no approval for `proposed` and one valid approval for `ratified`; and
- no tombstone using the same ID.

A proposed atom may be linked but cannot satisfy policy requiring ratified authority.

### `Tombstone`

Contains atom ID, terminal state, final revision, replacement atom references, and approval.

Rules:

- the ID exists in the permanent registry and has no live atom;
- `superseded` requires at least one replacement; `withdrawn` permits an empty replacement set;
- every replacement is a distinct live atom and cannot reference the retired ID;
- the final revision matches the last accepted live revision; and
- terminal state cannot transition or be removed in a later graph.

### `IdRegistry`

Contains namespace declarations and allocated-ID entries. A namespace declaration records its lowercase namespace ID, uppercase ID prefix, pinned allocation-policy payload, and complete approval-authority set. Each allocated entry records namespace ID, stable ID, repository-owned allocation-record digest, allocation ordinal, and current live or terminal class.

Ordinals are positive and unique within a namespace. They order allocation history but do not derive IDs. A live allocation may become terminal; terminal is a permanent reservation and can never become live again. Registry removal, duplicate allocation, namespace reassignment, changed allocation records or ordinals, and terminal-to-live change are invalid.

## Decision surfaces

### `DecisionSurfaceDomain`

Contains stable surface ID, member-key schema, disposition-vocabulary schema, complete member-key set, and positive evolution generation.

The member-key schema and disposition vocabulary are `TypedPayload`s under declared host schemas. Member keys are opaque canonical bytes to the core but unique by complete encoding. The domain has at least one member.

### `SurfaceDisposition`

Contains member key, core disposition, optional host disposition payload, and optional atom authority.

Rules:

- the key occurs exactly once in the bound domain;
- `normative` and `prohibited` require host payload and atom authority;
- `deferred` requires host payload naming a repository-owned debt record and atom authority stating the deferral;
- `not-applicable` forbids host payload unless the domain schema requires a reason and requires atom authority whenever policy requires one;
- the host payload conforms to the domain's disposition schema; and
- authority records a decision only, never implementation or evidence status.

### `DecisionSurface`

Contains surface ID, domain digest, complete disposition set, and applicable policy references.

Validity requires exact surface/domain ID agreement, one disposition for every domain member, no unknown member, no duplicate key, valid disposition authority, and closed policy references. Completeness is evaluated before evidence coverage.

## Evidence and navigation

### `ParsedItemIdentity`

Contains language/schema ID, path, item kind, parser-canonical item semantic bytes, and optional enclosing item identity.

The identity excludes source offsets and formatting trivia declared nonsemantic by the language adapter. A macro invocation and its expansion are distinct item kinds unless the Rust adapter schema explicitly binds them.

### `EvidenceScope`

The `claim_scope` payload uses one of these core schema IDs:

- `spec-provenance.scope.exact-corpus/v1`: corpus digest set and exact configuration digest;
- `spec-provenance.scope.bounded-domain/v1`: declared finite or bounded input-domain payload and bound;
- `spec-provenance.scope.quantified-property/v1`: quantified generator-domain payload; or
- a pinned host scope schema.

An exact-corpus scope can support only the enumerated corpus. A bounded domain does not imply exhaustive exploration. A quantified property requires the `property` role and still reports sampled execution unless a separate exhaustive model or proof applies.

### `EvidenceCarrier`

Contains carrier ID, role, nonempty atom-revision reference set, parsed-item identity, oracle reference, execution-configuration digest, fixture/input-corpus digest set, claim scope, assumption references, and exclusions.

Rules:

- metadata is syntax-attached to one eligible parsed item;
- role is explicit and never inferred from names;
- role and scope schema are compatible;
- all atom references are live and fresh for current coverage;
- the oracle kind is compatible with the role;
- configuration and corpus bindings are complete even when empty by design; and
- registration/freshness are independent from selection and verdict.

### `ImplementationLink`

Contains link ID, nonempty atom-revision reference set, parsed-item identity, and navigation kind. It may support navigation and impact analysis only. It cannot satisfy an evidence role, execution obligation, or proof binding.

### `Derivation`

Contains derivation ID, nonempty source and target binding sets, tool identity, configuration digest, oracle reference, relation, relation scope, and exclusions.

Rules:

- source and target sets are disjoint;
- all bindings resolve and are current for static freshness;
- the oracle kind is `derivation-consistency` or a compatible pinned host kind;
- `byte-equal` requires exactly one source and one target and declares equality of raw-artifact digest payloads;
- `generated-from` requires a generator tool identity;
- `structurally-equivalent` requires a declared comparison schema;
- `host-consistency` requires a pinned relation schema; and
- a passing oracle establishes only the declared relation scope.

## Policies, debt, waivers, and review

### `CoveragePolicy`

A policy object contains stable policy ID, revision digest, active approval, lifecycle, replacement-policy set, and optional terminal approval. Its revision contains scope, required evidence-role set, enforcement mode, ratchet ID, expected-authority selectors, and optional derivation requirements.

The scope is a pinned typed predicate over atom IDs/scopes, surface IDs/member keys, or object kinds. Evaluation must yield a finite canonical obligation set for one graph. Empty required roles are allowed only for an explicitly review-only policy. Blocking policy requires ratified approval.

`active` requires no replacements and no terminal approval. `superseded` requires one or more current replacement policies and terminal approval. `retired` requires an empty replacement set and terminal approval. Terminal policies remain in the graph for identity reservation and history but create no current obligations. A terminal policy can never become active again.

### `ObligationRef`

Contains policy ID, obligation kind, subject object reference, and optional required role. The required role is present exactly for `required-role`; other kinds forbid it. An obligation remains identifiable and reportable when debt or a waiver references it.

### `DebtRecord`

Contains debt ID, nonempty obligation set, affected object set, rationale, owner, remediation, discharge condition, optional informational external references, lifecycle, discharge-evidence set, replacement-debt set, and optional terminal approval.

Debt explains unsatisfied or advisory work. It cannot authorize a blocking gate to pass unless a separate waiver exists. External references are recorded strings only; their state is never read.

`active` has no discharge evidence, replacements, or terminal approval. `discharged` requires terminal approval, an empty replacement set, and evidence that every affected obligation is satisfied, retired, or no longer applicable under an approved policy evolution. `superseded` requires terminal approval and one or more active replacement debt records covering every still-unsatisfied obligation. Terminal debt remains in the graph and cannot reactivate.

### `Waiver`

Contains waiver ID, nonempty obligation set, affected object set, rationale, owner, discharge condition, active approval, optional external references, lifecycle, discharge-evidence set, replacement-waiver set, and optional terminal approval.

A waiver may authorize a process policy to continue while preserving every unsatisfied fact. It never changes authority, freshness, role presence, selection, verdict, consistency, or proof state. Waivers must be explicitly accepted by the affected policy's approval authority.

`active` has no discharge evidence, replacements, or terminal approval. `discharged` requires terminal approval and evidence that every waived obligation is now satisfied, retired, or no longer applicable. `revoked` requires terminal approval, permits unsatisfied obligations, and immediately removes process authorization. `superseded` requires terminal approval and one or more active replacement waivers that explicitly cover any continued exception. Terminal waivers remain visible and cannot reactivate.

### `Reaffirmation`

Contains reaffirmation ID, affected object kind, base digest, head digest, review approval, and statement.

Base and head digests must differ, both objects must be available to change analysis, and the approval authority must be valid for the object namespace. A reaffirmation is valid only for a `reaffirmed` change disposition and supplies `review-only` evidence. It cannot update a stale binding itself.

## Oracle, model, property, and proof objects

### `Oracle`

Contains oracle ID, kind, runner tool identity, selection semantics, verdict-normalization semantics, configuration digest, scope, and exclusions.

Selection and normalization are pinned typed payloads. A selected invocation yields exactly one of `not-run`, `pass`, `fail`, `error`, or `timeout`. Runner crashes and invalid semantic results normalize to `error`; discovery alone remains `not-run`.

### `NormalizedVerdict`

Contains exact oracle reference, selected subject set, verdict, and optional semantic result. `not-run` forbids a semantic result. Other verdicts permit only the result schema declared by the oracle. Timing and raw logs are excluded.

### `Assumption`

Contains assumption ID, normative statement, scope, exclusions, and approval. Assumptions are explicit dependencies, not inferred environment facts. A changed assumption digest invalidates every dependent property, model, carrier, and proof according to its binding.

### `FiniteModel`

Contains model ID, state-space, event alphabet, transition relation, initial states, bound, checked atom invariants, assumptions, and exclusions.

State/event/transition payloads use pinned model schemas. The state space and initial set are nonempty. Exhaustive assurance additionally requires a fixed-point oracle result proving all reachable declared transitions were explored; the model object alone is only registration.

### `FormalProperty`

Contains property ID, nonempty atom-revision set, formal property payload, implementation abstraction, assumptions, exclusions, and correspondence approval.

The approval asserts reviewed prose-to-property correspondence only. Every atom is ratified and current. A property with no implementation abstraction, assumption closure, or approval cannot support proof execution.

### `ProofRecord`

Contains proof ID, atom references, formal property reference, implementation representation binding, assumptions, translators, verifier, verifier configuration, external models, generated artifacts, proof sources, normalized verdict, scope, and exclusions.

Rules:

- atom set equals or explicitly narrows the property's atom set under its schema;
- assumptions include every property and external-model assumption;
- tool and artifact sets are complete and nonempty where the proof lane requires them;
- verdict is the verifier's normalized verdict but is not accepted assurance by itself;
- every binding must be current;
- required negative proof fixtures must fail for their intended diagnostic; and
- downstream compiler/backend semantics remain excluded absent separate bound evidence.

## `SpecGraph`

The semantic graph contains canonical sets of live atoms, tombstones, decision surfaces, evidence carriers, implementation links, derivations, coverage policies, debt records, waivers, oracles, finite models, formal properties, assumptions, and proof records.

Graph validity requires:

1. unique kind/ID keys and authority;
2. permanent live/tombstone ID separation;
3. complete decision surfaces;
4. every reference closed to one current expected object;
5. every policy scope evaluable to a finite obligation set;
6. no unsupported object or host schema;
7. no stale value being used to satisfy an obligation; and
8. deterministic diagnostics for every failure without partial winner selection.

The complete graph adds source map, repository configuration digest, and schema/tool-lock digest. Source movement can change complete-graph identity without changing semantic-object revisions.
