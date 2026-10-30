## MODIFIED Requirements

### Requirement: LCOV-006 Manual effectful task and blocking evidence boundary

`devenv tasks run buoy:coverage` MUST execute the focused local coverage lane. The task MUST require explicit selection and MUST NOT be a dependency of `devenv test`, a phase task, or another automatic task.

Automatic acceptance MUST validate the recorded coverage result against current sources and configuration. It MUST fail when the result is absent, stale, malformed, or not exact. This validation MUST NOT execute LLVM instrumentation or create a new execution verdict.

CI MUST NOT execute the coverage lane. Buoy MUST remain the dogfood target for this capability. The source inventory MUST include its core, shell, validators, local Dylint detector, and executable FCIS fixtures.

The Dylint FCIS verdict MUST remain distinct from LLVM coverage evidence. Coverage MUST remain structural source-execution evidence. It MUST NOT supply behavioral completeness, exhaustive evidence, formal proof, compiler correctness, or defect-absence claims.

#### Scenario: Positive - maintainer selects exact coverage
- **GIVEN** the pinned coverage closure and reviewed source inventory
- **WHEN** a maintainer explicitly runs `devenv tasks run buoy:coverage`
- **THEN** the task executes LLVM instrumentation and records a current result only after all exact metrics pass

#### Scenario: Positive - automatic acceptance checks current evidence
- **GIVEN** the recorded coverage result binds all current sources and configuration
- **WHEN** `devenv test` runs offline
- **THEN** it validates the recorded evidence without LLVM instrumentation and continues when the evidence is exact

#### Scenario: Negative - automatic acceptance starts coverage
- **GIVEN** the coverage result is current
- **WHEN** the resolved acceptance graph is inspected
- **THEN** any dependency path from `devenv test` to `buoy:coverage` fails with a deterministic task-graph diagnostic

#### Scenario: Negative - recorded coverage is stale
- **GIVEN** one inventoried source changes after the last manual coverage run
- **WHEN** automatic acceptance validates recorded coverage evidence
- **THEN** acceptance fails with a stable stale-coverage diagnostic and MUST NOT create a new coverage verdict

#### Scenario: Negative - full coverage is presented as proof
- **GIVEN** all four source-coverage metrics equal 100 percent
- **WHEN** an assurance report classifies the result as exhaustive or formally proved
- **THEN** the report fails with `LCOV-ASSURANCE-INFLATION` while the structural fact remains visible
