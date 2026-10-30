# llvm-coverage-testing Specification

## Purpose

Define pinned, inventory-complete, exact LLVM source coverage and its structural-evidence assurance boundary.

## Requirements
### Requirement: LCOV-001 Pinned coverage authority
All blocking LLVM source-coverage runs MUST use `cargo-llvm-cov` 0.8.5 from the repository's immutable devenv closure together with the pinned Rust/LLVM toolchain and committed Cargo/Nix locks. The lane MUST run offline without resolving an ambient executable, registry version, user Cargo home, or host LLVM tool, and a tool identity mismatch MUST emit stable diagnostic `LCOV-TOOL-IDENTITY`.

#### Scenario: Positive - pinned coverage tool runs offline
- **GIVEN** the declared devenv closure, lockfiles, Rust toolchain, and `cargo-llvm-cov` identity are realized
- **WHEN** the focused coverage lane runs with network access disabled and empty user tool homes
- **THEN** it uses the declared tool identities and proceeds without version discovery or fetch access

#### Scenario: Negative - ambient cargo subcommand shadows the pin
- **GIVEN** an executable named `cargo-llvm-cov` on ambient `PATH` has a different version or content identity
- **WHEN** the coverage preflight resolves the command closure
- **THEN** the lane fails with `LCOV-TOOL-IDENTITY` and MUST NOT record coverage acceptance

### Requirement: LCOV-002 Complete reviewed source inventory
A machine-readable inventory MUST enumerate each in-scope library, binary, current-host backend, generator, independent authority, validator, local Dylint detector, and executable source. Each row MUST have a stable target ID, normalized repository-relative path, host scope, instrumentation class, and disposition. Every current-host executable source MUST be instrumented under the pinned Rust/LLVM closure. Noncurrent hosts MUST use explicit `not-applicable` dispositions and receive no coverage claim. Duplicate IDs or paths, unknown classes, broad exclusions, and uninstrumented current-host sources MUST fail with deterministic diagnostics.

#### Scenario: Positive - inventory closes every executable source class
- **GIVEN** each in-scope source has one unique inventory row and each executable row appears in a compatible current LLVM export
- **WHEN** source-scope completeness is checked
- **THEN** every row is accounted for by instrumentation or a validated no-executable-region disposition

#### Scenario: Negative - standalone validator is absent from exports
- **GIVEN** a standalone validator contains executable functions but no current-host export contains its normalized path
- **WHEN** source-scope completeness is checked
- **THEN** the lane fails with stable diagnostic `LCOV-SOURCE-MISSING` and MUST NOT treat the validator as implicitly excluded

#### Scenario: Negative - executable module claims not applicable
- **GIVEN** an inventory row is marked `not-applicable` while its source contains an executable function, implementation, constant, static, or macro region
- **WHEN** the no-executable disposition is validated
- **THEN** the lane fails with stable diagnostic `LCOV-INVALID-DISPOSITION`

### Requirement: LCOV-003 Exact line, region, function, and branch closure
The normalized local aggregate for the current host MUST preserve integer LLVM counts. Lines, regions, functions, and branches MUST satisfy `covered == count` and `count > 0`. Aggregation MUST bind each input to source, tool, configuration, and current-host identity. It MUST normalize repository paths and merge duplicate regions without double counting. The lane MUST reject absent metrics, missing current-host artifacts, unsupported formats, zero denominators, rounded percentages, exclusions, or any uncovered item.

#### Scenario: Positive - current-host aggregate counts are exact
- **GIVEN** current compatible exports cover the current-host inventory and every normalized metric has a positive denominator
- **WHEN** the exports are merged and exact closure is evaluated
- **THEN** the lane passes only when covered and total integer counts are equal for all four metrics

#### Scenario: Negative - displayed branch percentage rounds to 100
- **GIVEN** branch coverage has one uncovered branch while a rendered percentage rounds to 100%
- **WHEN** exact closure is evaluated
- **THEN** the lane compares integer counts, fails with `LCOV-METRIC-NOT-EXACT`, and MUST NOT accept the rendered percentage

#### Scenario: Negative - metric denominator is zero or absent
- **GIVEN** an LLVM export omits regions or reports zero functions for the aggregate
- **WHEN** exact closure is evaluated
- **THEN** the lane fails with `LCOV-ZERO-DENOMINATOR` or `LCOV-METRIC-MISSING` and MUST NOT interpret the result as complete

#### Scenario: Negative - required current-host artifact is missing
- **GIVEN** the inventory declares current-host executable regions but no current compatible export exists for that host
- **WHEN** aggregate coverage is evaluated
- **THEN** the lane fails with `LCOV-PLATFORM-MISSING` rather than excluding those regions

### Requirement: LCOV-004 Deterministic and fresh coverage evidence
A normalized coverage result MUST bind the source inventory digest, each source digest, tool identities, command configuration, current-host policy, export digests, and exact counts. Entries MUST use stable IDs and deterministic repository-relative ordering and MUST omit timestamps, durations, locale text, host/store paths, traversal order, and raw terminal formatting. Any bound input change MUST make the prior result stale; only a new exact run under current inputs MAY restore a current passing verdict.

#### Scenario: Positive - identical authoritative inputs normalize identically
- **GIVEN** two coverage runs use byte-identical authoritative sources, tools, configuration, and platform inputs but different temporary directories and traversal order
- **WHEN** normalized coverage evidence is emitted
- **THEN** the deterministic records are byte-identical and independently report a current passing execution verdict

#### Scenario: Negative - source changes after a passing run
- **GIVEN** a source file bound by the latest passing coverage result changes without a new exact run
- **WHEN** coverage freshness is checked
- **THEN** the result is stale with `LCOV-EVIDENCE-STALE` and MUST NOT satisfy acceptance

#### Scenario: Negative - static registration is mistaken for execution
- **GIVEN** the inventory and tool bindings are valid but no current coverage process has executed
- **WHEN** the coverage obligation is evaluated
- **THEN** registration is present, execution remains unexecuted, and the obligation fails closed

### Requirement: LCOV-005 Coverage negative controls and deterministic diagnostics
The coverage lane MUST include focused negative controls for a missing source, invalid no-executable disposition, one uncovered line/region/function/branch, each zero denominator, incompatible tool/configuration/platform input, and stale evidence. Every control MUST be rejected with its expected stable diagnostic and exact expected cardinality; accepting a control, emitting only a rounded threshold message, or producing an unexpected/duplicate diagnostic MUST fail the lane.

#### Scenario: Positive - all focused invalid exports are rejected
- **GIVEN** committed negative fixtures independently mutate each required coverage invariant
- **WHEN** the coverage validator self-test runs
- **THEN** every fixture fails for its expected stable diagnostic and cardinality while the positive fixture passes

#### Scenario: Negative - missed branch fixture passes
- **GIVEN** a negative fixture changes one branch count from covered to uncovered
- **WHEN** the validator self-test observes a passing result or the wrong diagnostic
- **THEN** the self-test fails with `LCOV-NEGATIVE-CONTROL` and MUST NOT permit aggregate acceptance

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
