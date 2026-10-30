# chelis-provenance-adapter Specification

## Purpose

Integrate Chelis-owned requirement authorities, governed capability surfaces, evidence carriers, derivations, policies, and neutral change manifests with the repository-independent Buoy interfaces through a shell-side adapter that adds no host semantics to the generic core.

## Requirements
### Requirement: CHELIS-PROV-001 Shell-side dependency boundary
The Chelis integration MUST reside outside `buoy-core`, MUST depend inward on the repository-independent interfaces, and MUST NOT add Chelis, parser, filesystem, process, provider, clock, locale, or network dependencies to the `no_std` core.

#### Scenario: Positive - adapter depends inward
- **GIVEN** a Chelis shell-side adapter using explicit core values
- **WHEN** dependency and bare-metal checks run
- **THEN** the adapter may depend on the core while the core closure remains Chelis-free and `no_std`

#### Scenario: Negative - core imports Chelis semantics
- **GIVEN** a change adding a Chelis capability type or dependency to the generic core
- **WHEN** architecture checks run
- **THEN** the integration gate fails with a deterministic boundary diagnostic

### Requirement: CHELIS-PROV-002 Explicit authority and complete finite surfaces
The integration MUST use schema-owned explicit Chelis authorities and MUST materialize every member of each declared finite Chelis surface without filtering unsupported, prohibited, deferred, or not-applicable members before completeness validation.

#### Scenario: Positive - complete capability product
- **GIVEN** a declared Chelis capability product with every member explicitly dispositioned and authority-bound as required
- **WHEN** the adapter constructs the surface
- **THEN** completeness succeeds under the exact domain digest

#### Scenario: Negative - unsupported capability row is omitted
- **GIVEN** a declared product whose adapter filters one unsupported combination
- **WHEN** completeness is validated
- **THEN** the gate fails with the missing member and MUST NOT infer a disposition from implementation absence

### Requirement: CHELIS-PROV-003 Total adapter disposition
Every selected Chelis source input MUST produce supported semantic objects, an explicit nonnormative disposition, or a deterministic malformed/unsupported diagnostic. OpenSpec documents, raw text matches, provider metadata, comments, strings, and unattached annotations MUST NOT become product authority or carriers.

#### Scenario: Positive - selected input is fully dispositioned
- **GIVEN** a selected saved Chelis source file containing supported declarations and ordinary text
- **WHEN** the configured adapter runs
- **THEN** every selected syntax node receives semantic, nonnormative, or deterministic diagnostic treatment

#### Scenario: Negative - unknown schema version is skipped
- **GIVEN** a selected Chelis declaration using an unsupported schema version
- **WHEN** extraction runs
- **THEN** extraction fails loudly and MUST NOT return an empty successful graph

### Requirement: CHELIS-PROV-004 Deterministic provider-neutral static and change checks
The integration MUST produce byte-identical graph, static report, and base-to-head impact report bytes from identical explicit snapshots, configuration, schemas, tools, and neutral change manifests regardless of absolute root, traversal order, locale, timezone, or provider metadata.

#### Scenario: Positive - separate roots produce identical reports
- **GIVEN** identical saved Chelis inputs under distinct absolute roots
- **WHEN** static and change checks run offline
- **THEN** canonical graph and report bytes are identical

#### Scenario: Negative - pull-request metadata differs
- **GIVEN** identical canonical base/head inputs with different labels, users, review timestamps, or issue state
- **WHEN** impact checking runs
- **THEN** provider differences do not alter canonical output

### Requirement: CHELIS-PROV-005 Exact carrier scope and conservative aggregation
Every Chelis carrier MUST bind an eligible parsed item, explicit role, current atom revision, oracle, configuration, corpus, exact claim scope, assumptions, and exclusions. Missing, stale, unselected, failing, errored, or timed-out required evidence MUST prevent the corresponding assurance class, and debt, waivers, or reaffirmations MUST NOT synthesize satisfaction.

#### Scenario: Positive - exact required carriers pass
- **GIVEN** every required current Chelis carrier is selected and passes within its declared scope
- **WHEN** evidence is aggregated
- **THEN** the exact scoped claim may be reported `tested`

#### Scenario: Negative - waiver covers a timed-out carrier
- **GIVEN** a required carrier with `timeout` and an active process waiver
- **WHEN** evidence is aggregated
- **THEN** the execution obligation remains unsatisfied and the waiver remains independently visible

### Requirement: CHELIS-PROV-006 Explicit derivations for intentional duplicates
Every intentional Chelis generated or mirrored artifact MUST have a current derivation binding source and target digests, generator/checker identity, configuration, relation, consistency oracle, scope, and exclusions. A passing consistency result MUST NOT establish behavioral correctness.

#### Scenario: Positive - generated artifact is current and checked
- **GIVEN** a current derivation and passing selected consistency oracle
- **WHEN** Chelis assurance is reported
- **THEN** freshness and structural consistency are reported independently within the declared relation scope

#### Scenario: Negative - mirrored table changes without derivation update
- **GIVEN** a target artifact changed while its derivation retains the prior digest
- **WHEN** static checking runs
- **THEN** the derivation is stale and MUST NOT be reported consistent or behaviorally correct

### Requirement: CHELIS-PROV-007 Complete impact dispositions and policy ratchets
Every changed or transitively affected Chelis claim MUST receive exactly one current `reaffirmed`, `revised`, `retired`, or `waived` disposition. Coverage weakening MUST remain visible and MUST require the full repository-owned migration, approval, debt, and applicable waiver protocol.

#### Scenario: Positive - every transitive impact is dispositioned
- **GIVEN** an atom change affecting Chelis carriers, surfaces, derivations, policies, assumptions, and proofs
- **WHEN** one valid exact disposition exists for every impact
- **THEN** impact closure succeeds while review, waiver, freshness, and assurance facts remain separate

#### Scenario: Negative - policy role is silently removed
- **GIVEN** a blocking Chelis policy revision that removes a required negative role without an accepted weakening protocol
- **WHEN** change checking runs
- **THEN** the gate fails with deterministic ratchet and disposition diagnostics

### Requirement: CHELIS-PROV-008 Static, change, and execution command separation
The Chelis command surface MUST separate static repository checking, provider-neutral base-to-head change checking, and effectful oracle execution. Live execution MUST NOT mutate static graph identity, and canonical reports MUST exclude raw logs, elapsed time, temporary paths, PIDs, and process completion order.

#### Scenario: Positive - normalized verdict order varies
- **GIVEN** the same static graph and normalized Chelis verdict set arriving in different orders
- **WHEN** execution reports are rendered
- **THEN** report bytes are identical and static graph identity is unchanged

#### Scenario: Negative - runner crash is treated as success
- **GIVEN** a selected Chelis oracle that terminates without a valid semantic result
- **WHEN** execution is normalized
- **THEN** its verdict is `error` and the authoritative command fails rather than inventing `pass`

### Requirement: CHELIS-PROV-009 Bounded and formal claim bindings
Every Chelis exhaustive claim MUST name its exact finite model, digest, state/event/transition definitions, bounds, and fixed-point result. Every Chelis formal claim MUST bind approved correspondence, exact implementation representation, assumptions, translators, verifier, external models, generated artifacts, proof source, scope, exclusions, and intended negative-control failure.

#### Scenario: Positive - exact formal binding is current
- **GIVEN** a complete current Chelis proof record accepted under pinned offline tools with its intended negative control
- **WHEN** formal assurance is aggregated
- **THEN** only the exact scoped property may be reported `formally-proved`

#### Scenario: Negative - backend correctness is inferred
- **GIVEN** a source or LLBC proof without separately verified compiler/backend evidence
- **WHEN** its claim is rendered
- **THEN** emitted backend or machine code MUST NOT be reported formally proved

### Requirement: CHELIS-PROV-010 Advisory-first blocking rollout
The integration MUST expose all selected Chelis obligations under advisory policy before promotion and MUST retain the same ratchet identity when promoting to blocking. Promotion MUST occur only after fixtures, complete surfaces, current required roles, deterministic integration checks, and selected formal bindings are green.

#### Scenario: Positive - advisory scope is promoted intact
- **GIVEN** an advisory Chelis policy whose complete obligation set is current and passing
- **WHEN** a reviewed revision changes only enforcement to blocking under the same ratchet
- **THEN** promotion succeeds without erasing advisory history

#### Scenario: Negative - promotion hides uncovered scope
- **GIVEN** an advisory policy with an undecided surface member or missing required role
- **WHEN** blocking adoption is proposed with reduced scope
- **THEN** the gate reports weakening and MUST NOT accept the reduced policy silently

### Requirement: CHELIS-PROV-011 Saved-snapshot adapter and host-item protocol prerequisite
The Chelis integration MUST use `buoy.adapter-sdk/v3` with immutable adapter, parser, schema, capability, and configuration identities. Canonical inputs MUST contain only saved repository paths, saved bytes, and explicit configuration. Chelis consumer state and provider metadata MUST NOT supply authority or alter canonical output. The adapter MUST declare `chelis/surf-declaration/v1` and `chelis/deep-node/v1` through the schema-v3 `SourceItemKind` protocol. It MUST NOT use a legacy reader, migration mapping, opaque payload, cross-adapter kind, or host-specific core enum.

#### Scenario: Positive - approved Chelis item schema is root-independent
- **GIVEN** `buoy.adapter-sdk/v3` and descriptor-declared Chelis source-item kinds for saved repository bytes
- **WHEN** identical Chelis declarations are materialized under different absolute roots and adapter registration orders
- **THEN** their parsed-item identities, source maps, core values, canonical graph, and reports are byte-identical while downstream UX state remains excluded

#### Scenario: Negative - Chelis declaration uses an undeclared kind
- **GIVEN** an adapter that labels a Chelis declaration with `rust/item/v1`, `markdown/fence/v1`, raw kind text, or an opaque declaration
- **WHEN** adapter protocol validation runs
- **THEN** validation fails with `SPG-ADAPTER-UNDECLARED-SOURCE-ITEM-KIND` before graph construction and MUST NOT establish a false canonical identity

### Requirement: CHELIS-PROV-012 Parser-canonical Surf identity
Surf extraction MUST parse saved sources with the pinned vendored `tree-sitter-chelis-surf` grammar. Every record MUST anchor to a parsed top-level declaration node, and every parsed-item identity MUST derive from that node's canonical bytes and exact byte span. A parse `ERROR` node in a selected source MUST fail extraction loudly. The vendored grammar artifacts MUST carry a current registered derivation from the pinned Chelis revision with a fail-closed drift check.

#### Scenario: Positive - identity survives reformatting around the declaration
- **GIVEN** one Surf declaration whose surrounding lines move without changing the declaration node's canonical bytes
- **WHEN** extraction runs before and after the move
- **THEN** the parsed-item identity digest is unchanged while the span tracks the new location

#### Scenario: Negative - parse error cannot pass as nonnormative text
- **GIVEN** a selected Surf source that the pinned grammar cannot parse
- **WHEN** extraction runs
- **THEN** the check fails with a stable parse diagnostic and MUST NOT emit an empty successful graph

#### Scenario: Negative - vendored grammar drift is rejected
- **GIVEN** a vendored grammar file whose bytes differ from the registered derivation digests
- **WHEN** the drift check runs
- **THEN** the gate fails with a deterministic derivation diagnostic

### Requirement: CHELIS-PROV-013 Revision-bearing canonical reports
The canonical static report MUST expose every atom's revision digest and every graph object's identity digest, deterministically ordered, so a host repository can bind carriers, links, and surface rows without building out-of-band tooling. Report bytes MUST stay byte-identical for identical canonical inputs.

#### Scenario: Positive - host binds from the report alone
- **GIVEN** a static report for a repository with authority records
- **WHEN** a host reads the report
- **THEN** it obtains each atom's exact revision digest and each object's identity digest without any helper build

#### Scenario: Negative - report bytes stay canonical
- **GIVEN** identical saved inputs under two distinct absolute roots
- **WHEN** the revision-bearing report renders in both
- **THEN** the report bytes are identical

### Requirement: CHELIS-PROV-014 Complete command exposure
The consumer package MUST expose the `chelis-provenance` binary alongside `buoy`, and the `chelis-provenance` CLI MUST expose the advisory report as a deterministic command. Both surfaces MUST preserve the core exit classes.

#### Scenario: Positive - pinned package exposes the command
- **GIVEN** the realized consumer package
- **WHEN** a consumer lists its binaries
- **THEN** `chelis-provenance` is present and answers with the core usage exit class

#### Scenario: Positive - advisory report through the CLI
- **GIVEN** a repository with registered records and no receipts
- **WHEN** the advisory command runs twice
- **THEN** both runs emit byte-identical advisory bytes that keep uncovered atoms and `not-run` carriers visible without changing the exit class

### Requirement: CHELIS-PROV-015 Complete digest enforcement
The static check MUST compare every record-carried binding digest against its current recomputed value. A carrier or link whose bound item digest differs from the current parsed declaration node digest MUST fail with `CHELIS-PROV-ITEM-STALE`. An oracle that declares `runner-config-paths` MUST fail with `CHELIS-PROV-ORACLE-RUNNER-STALE` when its recorded runner digests differ from the current saved artifact bytes. No record field may be carried without enforcement or an explicit nonnormative declaration.

#### Scenario: Positive - current bindings pass
- **GIVEN** records whose item digests and declared runner digests match the current saved bytes
- **WHEN** the static check runs
- **THEN** the verdict is accept with no staleness diagnostic

#### Scenario: Negative - body edit invalidates the bound item
- **GIVEN** a bound declaration whose body bytes changed after the item digest was recorded
- **WHEN** the static check runs
- **THEN** the check reports `CHELIS-PROV-ITEM-STALE` with the static exit class

#### Scenario: Negative - runner config drift invalidates the oracle
- **GIVEN** an oracle with `runner-config-paths` whose referenced artifact bytes changed
- **WHEN** the static check runs
- **THEN** the check reports `CHELIS-PROV-ORACLE-RUNNER-STALE` with the static exit class

### Requirement: CHELIS-PROV-016 Actionable staleness contexts
Every digest- or revision-staleness diagnostic MUST carry both the bound value and the current expected value in its deterministic context. A maintainer or tool MUST be able to correct the record from the diagnostic alone, without an out-of-band recomputation build.

#### Scenario: Positive - context names both values
- **GIVEN** any staleness diagnostic from the static check
- **WHEN** the report renders
- **THEN** the context contains the bound digest and the current expected digest in a stable format

#### Scenario: Negative - contexts stay canonical
- **GIVEN** identical stale inputs under two distinct absolute roots
- **WHEN** the static check runs in both
- **THEN** the diagnostic contexts are byte-identical

### Requirement: CHELIS-PROV-017 Deterministic trace projection
The command MUST expose a `trace` subcommand that, for one stable ID, renders the complete digest chain: the atom revision, every surface row, link, and carrier that binds it, each bound item digest with its recomputed current value, and each referenced oracle digest. Every hop MUST be recomputed from saved bytes. The output MUST be canonical machine bytes, and any hop mismatch MUST produce the static exit class.

#### Scenario: Positive - full chain verifies in one command
- **GIVEN** a repository whose bindings are current
- **WHEN** `trace` runs for a bound atom ID
- **THEN** every hop reports its matching recomputed digest and the exit class is success

#### Scenario: Negative - a tampered hop fails the trace
- **GIVEN** one changed byte in the statement or a bound declaration body
- **WHEN** `trace` runs for the affected ID
- **THEN** the mismatched hop is named with bound and current values and the exit class is static rejection

### Requirement: CHELIS-PROV-018 Sanctioned rebind regeneration
The command MUST expose a `rebind` subcommand that recomputes every record-carried digest and revision field from the current graph and prints a deterministic field-level diff. Saved sources MUST change only under an explicit `--write` flag. Without `--write` the command MUST leave every byte unchanged. The rewrite MUST touch only record comment fields, never declarations, and a second run after a write MUST report an empty diff.

#### Scenario: Positive - review-then-write converges
- **GIVEN** records left stale by a reviewed spec edit
- **WHEN** `rebind` runs without and then with `--write`, followed by the static check
- **THEN** the dry run prints the exact field diff, the write applies only those fields, and the static check accepts

#### Scenario: Negative - dry run cannot mutate
- **GIVEN** any repository state
- **WHEN** `rebind` runs without `--write`
- **THEN** every saved byte is unchanged

#### Scenario: Negative - declarations are never rewritten
- **GIVEN** a stale binding on a declaration
- **WHEN** `rebind --write` runs
- **THEN** only `--`-prefixed record fields change and the declaration bytes stay byte-identical

### Requirement: CHELIS-PROV-019 Content-addressed binding resolution
The Surf grammar MUST accept the `binding` record kind: the marker line plus exactly one `record = <digest>` field, anchored to the next top-level declaration. Any other field set MUST fail with `CHELIS-PROV-MALFORMED-RECORD` and extract no record. Extraction MUST resolve the digest against the configured `binding-store` mount prefix at exactly `<binding-store>/buoy/objects/v1/<algorithm>/<hex>.toml`, MUST verify that the stored bytes hash to the address, and MUST materialize the parsed `buoy/implementation-binding/v1` record into the core graph. A missing store object, a hash mismatch, a malformed stored record, or a build whose active member is not collision resistant MUST fail with `CHELIS-PROV-BINDING-ADDRESS` and the static exit class.

#### Scenario: Positive - one line resolves the full binding
- **GIVEN** a two-line binding record above a declaration and a store object whose bytes hash to the address
- **WHEN** the static check runs
- **THEN** the binding materializes into `graph.implementation_bindings` and the verdict reflects the graph

#### Scenario: Negative - a change-detection member resolves nothing
- **GIVEN** a compact binding in a build whose active digest member is `xxh3-128`
- **WHEN** the static check runs
- **THEN** the check reports `CHELIS-PROV-BINDING-ADDRESS` with `algorithm-not-collision-resistant` and materializes no binding

#### Scenario: Negative - tampered store object dangles
- **GIVEN** a store object edited after the address was recorded
- **WHEN** the static check runs
- **THEN** the check reports `CHELIS-PROV-BINDING-ADDRESS` with the address and the recomputed hash, and MUST NOT materialize the binding

### Requirement: CHELIS-PROV-020 Binding source currency
A resolved binding whose subject digest differs from the attached declaration MUST fail with `CHELIS-PROV-BINDING-SUBJECT-STALE`. A resolved binding whose requirement digest differs from the exact authority record block MUST fail with `CHELIS-PROV-BINDING-REQUIREMENT-STALE`. Both diagnostics MUST carry bound and current values. The compact form MUST NOT weaken any enforcement the inline form carries.

Shared core validation MUST check each compact binding's atom revision with the same `SPG-REF-DANGLING-OR-STALE` rule as an inline link.

#### Scenario: Positive - subject matches the declaration
- **GIVEN** a binding whose subject digest equals the attached declaration's parsed node digest
- **WHEN** the static check runs
- **THEN** no subject staleness diagnostic appears

#### Scenario: Negative - body edit invalidates the subject
- **GIVEN** an edit to the attached declaration's body after binding
- **WHEN** the static check runs
- **THEN** the check reports `CHELIS-PROV-BINDING-SUBJECT-STALE` with bound and current digests

#### Scenario: Negative - authority edit invalidates the atom revision
- **GIVEN** an edit to the bound authority after binding
- **WHEN** the static check runs
- **THEN** shared core validation reports `SPG-REF-DANGLING-OR-STALE` for the compact binding

#### Scenario: Negative - authority formatting invalidates the requirement
- **GIVEN** a formatting-only edit to the bound authority record block
- **WHEN** the static check runs
- **THEN** the check reports `CHELIS-PROV-BINDING-REQUIREMENT-STALE` with bound and current digests

### Requirement: CHELIS-PROV-021 Sanctioned conversion and regeneration
The `bind` subcommand MUST convert one inline `implementation-link` record into a compact binding: write the content-addressed object, replace the inline record with the two-line form, and change nothing else. `rebind` MUST regenerate stale bindings by writing the successor object, updating the in-source address, and removing the superseded object. `trace` MUST expand bindings through the store with every hop recomputed. All writes stay behind explicit flags with deterministic dry-run plans, and declarations stay byte-identical.

#### Scenario: Positive - convert, verify, trace
- **GIVEN** an inline implementation link
- **WHEN** `bind --write` converts it and the static check and `trace` run
- **THEN** the graph is semantically unchanged, the source carries only the two-line form, and the trace expands the stored binding with matching hops

#### Scenario: Positive - rebind supersedes the object
- **GIVEN** a binding left stale by a reviewed spec edit
- **WHEN** `rebind` runs without and then with `--write`
- **THEN** the dry run names the object and address updates, the write leaves exactly one current object with a matching in-source address, and a second run reports an empty plan

#### Scenario: Negative - dry runs cannot mutate
- **GIVEN** any repository state
- **WHEN** `bind` or `rebind` runs without `--write`
- **THEN** every saved byte, including the store, is unchanged

#### Scenario: Negative - inline and compact forms cannot coexist
- **GIVEN** one relation ID in both an inline link and a compact binding
- **WHEN** the static check runs
- **THEN** the check reports `CHELIS-PROV-BINDING-LINK-DUPLICATE`
