# Derived Governance Analysis

## Purpose
This specification defines graph-derived impact, policy, disposition, and governance analysis.

## Requirements

### Requirement: DGA-001 Complete schema-declared impact graph
Base-to-head impact analysis MUST derive direct changes by canonical kind/ID/digest comparison and MUST traverse every frozen v1 dependency edge for atoms, tombstones, surface domains/dispositions, carriers, implementation links, policies/obligations, debt, waivers, reaffirmations, derivations/artifacts, oracles/tools, models, properties, assumptions, proofs, source maps, and change records until a fixed point.

#### Scenario: Positive - assumption change reaches all dependents
- **GIVEN** an assumption revision referenced by a carrier, finite model, formal property, oracle configuration, and proof
- **WHEN** base/head impact analysis reaches a fixed point
- **THEN** every dependent object appears exactly once in the canonical transitive-impact set

#### Scenario: Negative - debt edge is omitted
- **GIVEN** a changed blocking policy obligation referenced by an active debt record
- **WHEN** impact analysis runs with an implementation that ignores debt edges
- **THEN** an independent model/fixture detects the missing transitive impact and the gate fails

### Requirement: DGA-002 Graph-derived policy ratchets
Policy adoption and weakening MUST be computed from complete materialized base/head obligation sets, policy lifecycles, ratchet identities, surface-domain evolution, authority requirements, evidence roles, derivation requirements, and accepted waiver classes. A change manifest MUST NOT supply an authoritative weakening classification or omit obligations from materialization.

#### Scenario: Positive - strengthening is derived
- **GIVEN** one active policy whose head revision adds a subject and required role while retaining prior obligations and ratchet identity
- **WHEN** base/head policy comparison runs
- **THEN** the change is classified strengthening and every new obligation remains an explicit impact

#### Scenario: Negative - manifest claims no weakening
- **GIVEN** a head policy that removes a negative role while its manifest asserts a neutral change
- **WHEN** graph-derived ratchet comparison runs
- **THEN** the removal is reported as weakening regardless of the manifest assertion

### Requirement: DGA-003 Exact disposition protocol validation
Every required impact MUST have exactly one current disposition whose object kind/ID and base/head digests match the derived impact. `reaffirmed`, `revised`, `retired`, and `waived` dispositions MUST validate their complete frozen-v1 shape, repository-owned records, approvals, replacements, and applicability.

#### Scenario: Positive - exact reaffirmation closes one impact
- **GIVEN** one changed current object, one exact reaffirmation binding different base/head digests and valid approval, and one matching `reaffirmed` disposition
- **WHEN** disposition closure is validated
- **THEN** that impact is closed with an independent `review-only` fact

#### Scenario: Negative - waived disposition lacks current waiver
- **GIVEN** a `waived` disposition with matching graph digests but no current exact waiver record accepted by the affected policy authority
- **WHEN** disposition closure is validated
- **THEN** closure fails and the technical obligation remains unsatisfied

### Requirement: DGA-004 Complete weakening protocol
A weakening change MUST remain visible and MUST require the derived ratchet impact, applicable revised/superseded/retired disposition, policy-authority approval, repository-owned migration record, debt for every intentionally unowned or unsatisfied obligation, and a valid active waiver wherever blocking continuation is authorized.

#### Scenario: Positive - reviewed weakening retains unsatisfied facts
- **GIVEN** a blocking scope reduction with exact migration, approval, debt, and active waiver records
- **WHEN** weakening protocol validation runs
- **THEN** process continuation may be authorized while removed obligations and weakening remain visibly unsatisfied

#### Scenario: Negative - changed ratchet ID hides removal
- **GIVEN** a policy that removes obligations and changes its ratchet ID without the accepted weakening protocol
- **WHEN** base/head governance analysis runs
- **THEN** the gate fails and MUST NOT treat the head policy as an unrelated clean lineage

### Requirement: DGA-005 Governance records never launder assurance
Debt, waivers, reaffirmations, migration approvals, change dispositions, and terminal discharge records MUST remain orthogonal to authority, completeness, role presence, freshness, selection, verdict, derivation consistency, proof freshness, and assurance classes.

#### Scenario: Positive - waiver permits process only
- **GIVEN** a current exact waiver for one unsatisfied blocking execution obligation
- **WHEN** governance and assurance reports are aggregated
- **THEN** continuation authorization, unsatisfied execution, actual verdict, and active waiver are all independently visible

#### Scenario: Negative - reaffirmation freshens a carrier
- **GIVEN** a valid reaffirmation but a carrier still binding the base atom revision
- **WHEN** freshness and assurance are aggregated
- **THEN** the carrier remains stale and MUST NOT satisfy coverage or `tested`

### Requirement: DGA-006 Order-independent fixed-point evidence
Impact, ratchet, invalidation, and disposition algorithms MUST have independent finite reference models and permutation tests that compare exact canonical result bytes. A fixed-point claim MUST identify the model, digest, reachable states, transitions, bounds, and invariants.

#### Scenario: Positive - traversal order is permuted
- **GIVEN** one graph pair and neutral manifest traversed under multiple object/edge insertion orders
- **WHEN** the production algorithm and independent model reach fixed points
- **THEN** their canonical impact, ratchet, invalidation, and closure bytes are identical

#### Scenario: Negative - traversal stops at one hop
- **GIVEN** a dependency chain longer than one edge
- **WHEN** an implementation stops before the declared fixed point
- **THEN** the exhaustive model detects missing reachable impacts and the result MUST NOT be reported complete
