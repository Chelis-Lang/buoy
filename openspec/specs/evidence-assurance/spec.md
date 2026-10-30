# evidence-assurance Specification

## Purpose

Define orthogonal, role-explicit, freshness-bound, fail-closed evidence and governance semantics without assurance laundering.

## Requirements

### Requirement: EVD-001 Orthogonal evidence facts
The system MUST represent authority validity, decision-surface completeness and disposition authority, carrier registration, declared evidence scope, carrier freshness, coverage-policy applicability and enforcement, required-role coverage, debt, waivers, derivation freshness, consistency-oracle scope, oracle selection, execution verdict, finite-model scope, proof freshness, change-impact disposition, review acknowledgement, assurance class, assumptions, and exclusions as independent facts.

#### Scenario: Positive - fresh test is not run
- **GIVEN** a valid fresh test carrier whose oracle has not run
- **WHEN** evidence is reported
- **THEN** the carrier is `linked` and its execution verdict is `not-run`

#### Scenario: Negative - linked is reported as tested
- **GIVEN** a discovered fresh carrier without a passing execution verdict
- **WHEN** assurance is aggregated
- **THEN** the system MUST NOT report the atom as `tested`

### Requirement: EVD-002 Explicit evidence roles
Every carrier MUST declare a supported role such as positive, negative, property, exhaustive-model, formal-property, or structural. Coverage policy MUST identify required roles rather than infer polarity from names or prose.

#### Scenario: Positive - positive and negative roles are present
- **GIVEN** an atom policy requiring positive and negative executable evidence
- **WHEN** one fresh carrier of each role is registered
- **THEN** static required-role coverage succeeds

#### Scenario: Negative - test name implies missing role
- **GIVEN** a carrier named like a negative test but declared only with the positive role
- **WHEN** required-role coverage is evaluated
- **THEN** the negative role remains missing

### Requirement: EVD-003 Digest-bound freshness
A carrier MUST be fresh only when its expected atom revision digest equals the current atom revision and all referenced graph objects satisfy their own validity and freshness rules.

#### Scenario: Positive - every carrier binding matches
- **GIVEN** a valid carrier whose atom digest and referenced oracle identity match the current graph
- **WHEN** freshness is evaluated
- **THEN** the carrier is fresh

#### Scenario: Negative - atom revision changes
- **GIVEN** a previously fresh carrier
- **WHEN** the atom's semantic revision changes without updating the carrier
- **THEN** the carrier becomes stale and cannot satisfy required coverage

### Requirement: EVD-004 Explicit oracle execution states
Oracle execution MUST distinguish `not-run`, `pass`, `fail`, `error`, and `timeout`. Discovery and selection MUST NOT synthesize `pass`.

#### Scenario: Positive - selected oracle passes
- **GIVEN** a selected oracle that executes and returns its schema-defined successful result
- **WHEN** the result is normalized
- **THEN** its verdict is `pass`

#### Scenario: Negative - runner crashes
- **GIVEN** a selected oracle whose runner terminates without a valid result
- **WHEN** the result is normalized
- **THEN** its verdict is `error`, not `pass` or `not-run`

### Requirement: EVD-005 Fail-closed assurance aggregation
A required stale, absent, not-run, failed, errored, or timed-out input MUST prevent the corresponding stronger assurance claim even when sibling evidence passes.

#### Scenario: Positive - every required test passes
- **GIVEN** all required fresh test carriers selected by their oracles with `pass` verdicts
- **WHEN** test assurance is aggregated
- **THEN** the scoped atom claim may be `tested`

#### Scenario: Negative - one required test times out
- **GIVEN** several passing required tests and one required test with `timeout`
- **WHEN** test assurance is aggregated
- **THEN** the scoped atom claim is not `tested` and the timeout remains visible

### Requirement: EVD-006 Narrow assurance vocabulary
Reports MUST use only scoped claims including `linked`, `tested`, `exhaustively-checked`, `formally-proved`, `structurally-enforced`, and `review-only`. The system MUST NOT treat these evidence classes as interchangeable or silently upgrade one into another.

#### Scenario: Positive - structural invariant is enforced
- **GIVEN** a syntax invariant accepted by its current structural checker
- **WHEN** assurance is reported
- **THEN** the claim is `structurally-enforced` with checker scope and identity

#### Scenario: Negative - exhaustive model is labeled formal proof
- **GIVEN** every state in a declared finite model has been checked
- **WHEN** assurance is reported
- **THEN** the claim is not `formally-proved` unless a separate fresh proof record establishes that property

### Requirement: EVD-007 Bounded exhaustive model claims
An exhaustive claim MUST identify the finite model ID, revision digest, state space, event alphabet, transition relation, bounds, and checked invariants. The runner MUST explore every reachable state in the declared model until no new state remains.

#### Scenario: Positive - complete reachable model exploration
- **GIVEN** a finite lifecycle model with a declared transition relation and invariants
- **WHEN** the exhaustive runner reaches a fixed point with every reachable transition checked
- **THEN** the report states `exhaustively-checked` within that exact model and digest

#### Scenario: Negative - exploration uses an undeclared depth cutoff
- **GIVEN** exploration stops at a depth cutoff before reachability reaches a fixed point
- **WHEN** results are reported
- **THEN** the system MUST NOT report the model as exhaustively checked

### Requirement: EVD-008 Authoritative command and shared semantics
One offline repository-wide CLI command MUST be the authoritative CI check. Lint and future LSP surfaces MUST consume the same semantic library and MUST NOT define independent authority, freshness, or aggregation rules.

#### Scenario: Positive - CLI and LSP inspect the same saved snapshot
- **GIVEN** identical saved repository inputs
- **WHEN** the CLI and LSP adapter request validation
- **THEN** their structured semantic diagnostics are equivalent before presentation formatting

#### Scenario: Negative - unsaved editor buffer is green
- **GIVEN** an LSP result over an unsaved buffer that differs from the committed repository
- **WHEN** CI acceptance is evaluated
- **THEN** the editor result does not satisfy the authoritative repository check

### Requirement: EVD-009 Versioned coverage policies and adoption ratchets
Every `CoveragePolicy` MUST declare its stable identity, revision digest, atom and decision-surface scope, required evidence roles, blocking or advisory enforcement mode, and adoption-ratchet identity. Policy weakening or scope removal MUST remain an explicit change impact and MUST NOT disappear from reports through replacement or omission.

#### Scenario: Positive - blocking scope grows
- **GIVEN** a blocking coverage policy whose new revision adds a governed surface without removing existing scope or required roles
- **WHEN** policy adoption is compared
- **THEN** the growth is reported and the newly scoped obligations are evaluated under the blocking policy

#### Scenario: Negative - required role is silently removed
- **GIVEN** a policy revision that removes a required negative-evidence role without a repository-owned change disposition
- **WHEN** policy adoption is compared
- **THEN** the change gate fails with a deterministic policy-ratchet diagnostic

### Requirement: EVD-010 Exact ordinary evidence scope
An ordinary executable carrier MUST bind its exact parsed item, oracle, configuration and fixture or input-corpus digests, declared claim scope, assumptions, and exclusions. A passing verdict MUST support `tested` only within that declared scope and MUST NOT be generalized into property, exhaustive, or formal assurance.

#### Scenario: Positive - point test passes in its declared scope
- **GIVEN** a fresh carrier declaring one exact input corpus and configuration whose selected oracle passes
- **WHEN** assurance is aggregated
- **THEN** the atom may be reported `tested` only for that declared corpus and configuration

#### Scenario: Negative - point examples are presented as a property
- **GIVEN** passing carriers over several concrete examples without a property carrier or quantified domain
- **WHEN** assurance is aggregated
- **THEN** the report MUST NOT claim property coverage or generalize beyond the declared examples

### Requirement: EVD-011 Repository-owned debt and waivers do not satisfy obligations
Every `DebtRecord` or `Waiver` MUST have a stable local identity and irreversible repository-owned lifecycle and MUST bind the exact obligation and affected object revisions, rationale, ownership, remediation or discharge condition, and any terminal evidence and approval. External issue or provider metadata MAY be informational but MUST NOT determine canonical status. Active and terminal debt and waivers MUST remain visible, their IDs MUST NOT be reused, and they MUST NOT satisfy coverage, freshness, execution, structural-consistency, or proof obligations.

#### Scenario: Positive - known failing carrier has explicit debt
- **GIVEN** a fresh carrier linked to a current atom, a repository-owned debt record, and an oracle verdict of `fail` or `not-run`
- **WHEN** evidence is reported
- **THEN** the link, actual verdict, and debt are visible while the atom is not reported `tested`

#### Scenario: Negative - waiver is aggregated as passing evidence
- **GIVEN** a waiver authorizing a process gate to proceed without one required passing carrier
- **WHEN** assurance is aggregated
- **THEN** the required obligation remains unsatisfied and the waiver MUST NOT synthesize a passing verdict or stronger assurance

### Requirement: EVD-012 Reaffirmation is review-only evidence
A reaffirmation MUST bind exact base and head object digests and a repository-owned review artifact stating that an affected claim was reconsidered. Reaffirmation MUST be reported only as `review-only`; it MUST NOT make a stale carrier fresh, establish behavioral correspondence, or satisfy an execution or proof obligation.

#### Scenario: Positive - current carrier is deliberately reaffirmed
- **GIVEN** a carrier updated to the current atom revision and a valid reaffirmation binding the old and new revisions
- **WHEN** change evidence is reported
- **THEN** the carrier freshness and the review-only reaffirmation are reported as independent facts

#### Scenario: Negative - reaffirmation launders a stale carrier
- **GIVEN** a reaffirmation record but a carrier that still binds the prior atom revision
- **WHEN** assurance is aggregated
- **THEN** the carrier remains stale and the reaffirmation cannot satisfy coverage
