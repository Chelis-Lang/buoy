## MODIFIED Requirements

### Requirement: ADV-003 Manual mutation campaign plus blocking replay

Maintainers MUST explicitly select the pinned mutation campaign against the deterministic core and selected shell validators. The full campaign MUST use the reviewed baseline, timeout, exclusions, and minimum score.

`buoy:mutation-campaign` MUST NOT be a dependency of `devenv test`, a phase task, or another automatic task. CI MUST NOT run the full mutation campaign.

Named handcrafted mutants, deterministic replay, artifact freshness, and policy threshold checks MUST remain blocking in `devenv test`. Recorded campaign evidence MUST bind current sources and policy. Replay MUST NOT start `cargo-mutants` or create a new campaign verdict.

#### Scenario: Positive - selected campaign meets the reviewed threshold
- **GIVEN** the pinned mutation configuration and current production sources
- **WHEN** a maintainer explicitly runs `devenv tasks run buoy:mutation-campaign`
- **THEN** every selected mutant is killed and the accepted nonexcluded score meets the threshold

#### Scenario: Positive - automatic replay checks current campaign evidence
- **GIVEN** the recorded campaign artifact binds current sources and policy
- **WHEN** `devenv test` runs
- **THEN** named mutants and artifact checks pass without a `cargo-mutants` process

#### Scenario: Negative - automatic graph starts the full campaign
- **GIVEN** a phase or automatic task depends on `buoy:mutation-campaign`
- **WHEN** the resolved task graph is validated
- **THEN** validation fails with a deterministic manual-task dependency diagnostic

#### Scenario: Negative - hard mutant is excluded to raise the score
- **GIVEN** a surviving no-laundering mutant enters the exclusion list without an approved replacement tripwire
- **WHEN** mutation policy is checked
- **THEN** the gate fails and MUST NOT report the threshold as satisfied
