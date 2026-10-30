## ADDED Requirements

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
