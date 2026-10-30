## MODIFIED Requirements

### Requirement: DET-009 Deterministic base-to-head change impact
The core MUST compare explicit base and head graphs with one provider-neutral change manifest. Equal inputs MUST produce byte-identical impact reports.

The report MUST include all changed, retired, and affected governed objects. This set includes source items, external trace links, carriers, and implementation links.

The report MUST also include affected surfaces, policies, derivations, properties, assumptions, models, proofs, debt, and waivers.

The core MUST NOT consult version-control hosts, issue APIs, pull-request APIs, labels, users, reviews, timestamps, or network state.

#### Scenario: Positive - identical graph pair in separate repositories
- **GIVEN** identical canonical base and head graphs and neutral change manifests in two different absolute roots
- **WHEN** change impact is computed
- **THEN** the canonical impact report bytes are identical

#### Scenario: Positive - requirement change reaches all links
- **GIVEN** an OpenSpec requirement revision with code, evidence, and external trace links
- **WHEN** that requirement changes and impact is computed
- **THEN** the impact set contains every transitively affected parsed item, link, carrier, and dependent governed object

#### Scenario: Negative - provider metadata alters impact
- **GIVEN** identical canonical base and head inputs but different pull-request labels, issue state, authors, or review timestamps
- **WHEN** change impact is computed
- **THEN** provider metadata does not alter the canonical impact report

#### Scenario: Negative - external trace link is omitted
- **GIVEN** an atom revision change that makes one external trace link stale
- **WHEN** impact computation omits that link
- **THEN** impact validation fails with a deterministic missing-impact-edge diagnostic

## ADDED Requirements

### Requirement: DET-011 Deterministic OpenSpec adapter inputs
OpenSpec adapter output MUST depend only on saved repository paths, saved bytes, explicit configuration, schema identities, and pinned parser inputs.

#### Scenario: Positive - equal saved inputs under different hosts
- **GIVEN** equal OpenSpec files and configuration on hosts with different roots, locales, and traversal orders
- **WHEN** the OpenSpec adapter runs
- **THEN** both hosts produce byte-identical graph fragments, source maps, dispositions, and diagnostics

#### Scenario: Negative - OpenSpec CLI or provider supplies ambient state
- **GIVEN** an adapter path that reads an OpenSpec process, unsaved editor buffer, or provider API
- **WHEN** deterministic boundary validation runs
- **THEN** validation fails with a stable ambient-input diagnostic
