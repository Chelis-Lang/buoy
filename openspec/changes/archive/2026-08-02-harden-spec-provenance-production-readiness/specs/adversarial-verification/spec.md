## ADDED Requirements

### Requirement: ADV-001 Coverage-guided fuzz targets and committed regressions
Pinned coverage-guided fuzz targets MUST exercise canonical decoding/encoding, path/glob normalization, Markdown/TOML and Rust adapters, repository configuration, neutral change manifests, graph closure, impact/ratchet/disposition analysis, and report rendering. Each target MUST have bounded input size, committed seed corpus, no network/effects beyond its declared harness, and deterministic minimized regression promotion.

#### Scenario: Positive - minimized crash becomes a fixture
- **GIVEN** a fuzz campaign finds a reproducible malformed canonical value that triggers a crash
- **WHEN** the case is minimized and reviewed
- **THEN** it becomes a committed negative fixture with stable expected diagnostics before the production fix is accepted

#### Scenario: Negative - fuzz target reads live repository state
- **GIVEN** a fuzz harness whose result depends on current directory files or environment
- **WHEN** harness policy is checked
- **THEN** the target is rejected and MUST NOT contribute fuzz evidence

### Requirement: ADV-002 Independent differential authorities
Independent canonical, path/glob, lifecycle, surface, freshness, policy, impact, proof-invalidation, and aggregation models MUST NOT import or call the corresponding production modules. Their dependency closures and algorithms MUST be reviewably distinct, and randomized bounded values MUST compare exact outputs or stable diagnostic families.

#### Scenario: Positive - independent encoders agree
- **GIVEN** a generated complete valid v1 value
- **WHEN** production and independent encoders run
- **THEN** their exact canonical bytes and domain-separated digests match

#### Scenario: Negative - reference model reexports production function
- **GIVEN** a purported independent impact model that calls production `change_impacts`
- **WHEN** independence enforcement inspects its dependency/API use
- **THEN** the evidence is rejected and the differential gate fails

### Requirement: ADV-003 Local automated mutation threshold plus named tripwires
Maintainers MUST run the pinned mutation tool locally against the deterministic core and selected shell validators. The local campaign MUST use the reviewed baseline, timeout, exclusions, and minimum score. Named handcrafted mutants MUST remain blocking in `devenv test` regardless of aggregate score. CI MUST NOT run the full mutation campaign.

#### Scenario: Positive - campaign meets reviewed threshold
- **GIVEN** the pinned mutation configuration and current production sources
- **WHEN** the campaign runs or its current policy-valid recorded artifact is replayed
- **THEN** every named mutant is killed and the accepted nonexcluded score meets the threshold

#### Scenario: Negative - hard mutant is excluded to raise score
- **GIVEN** a surviving no-laundering mutant added to the exclusion list without an approved rationale and replacement tripwire
- **WHEN** mutation policy is checked
- **THEN** the gate fails and MUST NOT report the threshold satisfied

### Requirement: ADV-004 Current-host canonical reproducibility
Local acceptance MUST execute the portable corpus twice on the current supported host. Identical repository bytes, configuration, schemas, and tools MUST produce identical canonical bytes across roots and orders. Noncurrent hosts MUST receive explicit non-applicable ledger dispositions and no conformance claim. CI MUST NOT run this assurance gate.

#### Scenario: Positive - canonical corpus repeats on the current host
- **GIVEN** one portable conformance repository and two local temporary roots
- **WHEN** static and impact reports are generated on the current host
- **THEN** authoritative binary bytes and full digests match exactly

#### Scenario: Negative - Windows case alias is accepted only on Linux
- **GIVEN** two paths sharing a portable case-fold collision key
- **WHEN** the corpus runs on a case-sensitive host
- **THEN** it is rejected identically rather than producing a host-specific green graph

### Requirement: ADV-005 Deterministic fault-injection suites
Tests MUST inject read/open/permission/partial-I/O failures, symlink/reparse swaps, deep trees, oversized files/objects, malformed parser recovery, duplicate selectors, pipe saturation, malformed semantic results, abnormal exits, process descendants, cancellation, and timeout. Each injected fault MUST terminate within a bound and map to stable canonical diagnostics or normalized verdicts.

#### Scenario: Positive - partial read fault is normalized
- **GIVEN** a selected file backend that returns a deterministic partial-read failure
- **WHEN** snapshotting runs
- **THEN** it stops within budget and emits the stable read-failure category without canonical raw OS text

#### Scenario: Negative - full stderr pipe hangs CI
- **GIVEN** an oracle filling stderr while never reading/writing its result channel
- **WHEN** the fault suite runs
- **THEN** the supervisor drains/terminates within timeout and the test MUST fail if it deadlocks

### Requirement: ADV-006 Miri, sanitizer, and compile-fail coverage
Supported pure/shell code MUST run under pinned Miri and applicable sanitizers in their declared scopes, and public API invariants MUST have compile-fail tests preventing invalid direct field construction or forbidden callable/effect dependencies. Results MUST be reported only within tool scope.

#### Scenario: Positive - invalid value construction does not compile
- **GIVEN** a compile-fail fixture attempting to construct a policy with private fields or mismatched metadata kind
- **WHEN** the API tripwire runs
- **THEN** compilation fails for the intended stable API reason

#### Scenario: Negative - sanitizer success is called formal proof
- **GIVEN** all sanitizer runs passing
- **WHEN** assurance is rendered
- **THEN** the result remains testing/structural evidence and MUST NOT be labeled `formally-proved`

### Requirement: ADV-007 Formal representation drift and negative controls
Every selected production formal lane MUST have exact source/tool/model/configuration digest checks, representation-correspondence review, and intended negative controls. Where production and proof representations are duplicated, an explicit derivation/consistency oracle MUST bind them; test equality MUST NOT be called proof of correspondence.

#### Scenario: Positive - proof representation derivation is current
- **GIVEN** exact implementation and proof representations, current derivation/checker bindings, accepted proof, and intended negative-control rejection
- **WHEN** formal freshness is evaluated
- **THEN** the exact scoped proof may remain current under its stated correspondence assumption

#### Scenario: Negative - production logic changes without proof binding
- **GIVEN** a semantic change to a verified kernel function while proof sources and proof record remain unchanged
- **WHEN** binding and derivation checks run
- **THEN** the proof becomes stale even if the old proof command still passes

### Requirement: ADV-008 Bounded deterministic final versus local campaigns
The exact final oracle MUST run deterministic fuzz replay, named mutation tripwires, and current campaign-artifact validation. Maintainers MUST run longer fuzz and full mutation campaigns locally. Each artifact MUST bind its tool version, source digest, bounds, results, and expiry rules. Campaign artifacts MUST NOT hide current deterministic failures.

#### Scenario: Positive - current local artifact is accepted
- **GIVEN** a local campaign artifact matching current source, tool, configuration, and policy bounds with no unresolved failures
- **WHEN** the final gate validates it and replays deterministic corpora
- **THEN** campaign freshness is reported in its exact scope

#### Scenario: Negative - stale campaign artifact masks regression
- **GIVEN** a prior mutation/fuzz artifact whose source digest differs from head
- **WHEN** the final gate runs
- **THEN** the artifact is stale and MUST NOT satisfy the current campaign obligation

### Requirement: ADV-009 Executable fixture and mutation truthfulness
Every declared fixture case MUST dispatch its closed operation against the concrete baseline, apply each declared mutation in order to exactly one expected target, invoke the specified production and independent authorities, and compare actual acceptance, deterministic diagnostic codes, facts, and canonical bytes with the frozen expected result. Every locally actionable operation, mutation, scenario, requirement, bounded production-conformance model, and intentionally-false control MUST own one separately materialized normalized execution result bound to exact source, tool, configuration, and result digests. Manifest shape, vocabulary membership, baseline existence, aggregate command success, shared case observations, aggregated mutation lists, or an unexecuted expected-result record MUST NOT satisfy behavioral fixture evidence. Validators MUST fail closed on stale bindings, duplicates, omissions, and orphan execution records. The runner MUST have negative self-tests for unknown/no-op operations, zero/multiple mutation targets, ignored expected fields, production/reference dispatch substitution, stale results, and orphan evidence.

#### Scenario: Positive - fixture mutation is executed and detected
- **GIVEN** a negative fixture that mutates one current repository-artifact digest and expects a stale-freshness diagnostic and unsatisfied fact
- **WHEN** the focused fixture runner executes the operation and mutation
- **THEN** exactly one target changes, production and independent results match the frozen diagnostic/fact contract, and the case contributes behavioral evidence

#### Scenario: Negative - manifest-only fixture passes without execution
- **GIVEN** a runner that validates operation and mutation names but never applies them or compares actual results
- **WHEN** fixture-runner self-tests and final acceptance run
- **THEN** the runner fails and none of its cases satisfy fixture, mutation, property, model, or production-conformance obligations

### Requirement: ADV-010 Complete local source-bound structural coverage
A reviewed source inventory MUST enumerate every in-scope production library, binary, current-host backend, standalone validator, local Dylint detector, and generated executable target. Pinned local coverage MUST report exactly 100% lines, regions, functions, and branches. Threshold rounding, broad exclusions, assertion-free calls, and uninstrumented targets MUST NOT satisfy coverage. Source that cannot execute MUST be removed or receive an exact proof-backed impossibility disposition. CI MUST NOT run this coverage gate.

#### Scenario: Positive - current-host coverage is complete
- **GIVEN** the reviewed source inventory and one current local coverage artifact
- **WHEN** coverage closure is evaluated
- **THEN** every current-host target is instrumented and all four metrics equal 100% without an unresolved source item

#### Scenario: Negative - branch threshold is rounded up
- **GIVEN** a coverage result with one missed branch or one uninstrumented standalone validator while a renderer displays 100% after rounding or exclusion
- **WHEN** coverage policy runs
- **THEN** the gate fails and MUST NOT report structural coverage complete

### Requirement: ADV-011 Production property and finite-model conformance
A machine-readable property inventory MUST map every pure public semantic production algorithm and frozen invariant to deterministic Hegel-rs generators, dependency-independent reference evaluation, production evaluation, equality/metamorphic assertions, committed seeds/settings, and minimized replay. Every exhaustive finite model used as implementation evidence MUST compare production behavior at every reachable declared state and transition with the independent model's exact result. A property or model that exercises only test-local reference helpers MUST be labeled reference-only and MUST NOT satisfy production-conformance obligations.

#### Scenario: Positive - exhaustive lifecycle model agrees with production
- **GIVEN** the complete declared lifecycle state/event model and the production transition function
- **WHEN** every reachable state/event pair is evaluated
- **THEN** production and independent outcomes agree exactly and the source/model/configuration-bound conformance artifact is current

#### Scenario: Negative - Hegel proves only its helper
- **GIVEN** a Hegel property whose closure calls only test-local sorting, Boolean, digest, or transition helpers
- **WHEN** the property inventory is validated
- **THEN** it remains reference-only and the corresponding production property obligation is unsatisfied

### Requirement: ADV-012 Complete assurance-obligation and proof accounting
The project MUST maintain a machine-readable assurance-obligation ledger mapping every frozen requirement/scenario, schema inventory row, source target, fixture operation/mutation, property, finite model, mutation class, supported platform backend, and selected theorem to exact current evidence or one explicit `not-applicable`, `unsupported`, or `blocked` disposition with scope and rationale. A production formal claim MUST bind an actual shared/generated production representation or an explicit consistency derivation plus correspondence approval; a raw production-source digest alone establishes drift detection, not proof correspondence. No evidence class MAY satisfy another class's missing ledger entry.

#### Scenario: Positive - exact theorem and implementation derivation are current
- **GIVEN** an approved property, exact production representation, current consistency derivation, proof source, tool/model/configuration bindings, accepted positive proof, and intended negative-control rejection
- **WHEN** proof and ledger closure are evaluated
- **THEN** the theorem's exact scope is `formally-proved` while compiler, operating-system, parser, and prose-correspondence exclusions remain explicit

#### Scenario: Negative - source hash is treated as correspondence proof
- **GIVEN** a detached Boolean proof model and a test that only checks a digest of the production source file
- **WHEN** formal assurance is requested
- **THEN** drift detection may be current but proof correspondence remains unsatisfied and `formally-proved` MUST NOT be emitted for production

### Requirement: ADV-013 Monotonic local P0-P3 phase handoffs
Local production-readiness acceptance MUST execute P0, P1, P2, and P3 once each through `devenv test`. Each handoff MUST name its artifacts, identities, focused command, diagnostics, and success condition. Separate external phase records MUST NOT be acceptance prerequisites. A later phase MUST NOT hide an earlier absent, stale, or failing obligation.

#### Scenario: Positive - every local phase handoff is current
- **GIVEN** current P0 evidence, P1 evidence, P2 current-host evidence, and a closed P3 ledger
- **WHEN** exact local final acceptance runs
- **THEN** every phase executes in order and passes before production readiness is accepted

#### Scenario: Negative - final command omits fixture execution
- **GIVEN** a green aggregate command whose fixture slice validates only manifest shape and never executes operations
- **WHEN** phase-handoff closure is evaluated
- **THEN** P0 remains incomplete and production readiness fails regardless of later green tests, proofs, waivers, or reviews
