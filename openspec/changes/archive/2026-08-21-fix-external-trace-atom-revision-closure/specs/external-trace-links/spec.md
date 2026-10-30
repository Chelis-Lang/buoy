## MODIFIED Requirements

### Requirement: ETL-006 Link freshness and impact

An external trace link MUST bind exact semantic atom revisions. Validation MUST compare each reference with `Atom::revision()` for the same stable ID.

Validation MUST NOT reinterpret an atom revision as `Atom::meta().digest()`. A changed or retired atom MUST make the old link stale and create explicit change impact.

#### Scenario: Positive - current link remains fresh

- **GIVEN** a trace link whose references match all current semantic atom revisions
- **WHEN** link freshness is validated
- **THEN** the link is fresh and available for navigation even when each atom object digest differs

#### Scenario: Negative - atom object digest occupies the revision field

- **GIVEN** a trace link whose atom reference contains the current atom object digest instead of its semantic revision
- **WHEN** link freshness is validated
- **THEN** validation fails with `SPG-REF-DANGLING-OR-STALE` and accepts no fresh link

#### Scenario: Negative - requirement scenario changes

- **GIVEN** a trace link to an OpenSpec atom revision
- **WHEN** a scenario edit changes that semantic atom revision
- **THEN** the link becomes stale and the change report requires one explicit disposition
