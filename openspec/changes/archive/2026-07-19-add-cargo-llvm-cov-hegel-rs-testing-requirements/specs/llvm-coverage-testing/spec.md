## ADDED Requirements

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
A machine-readable inventory MUST enumerate every in-scope production library, binary, platform backend, maintainer generator, independent test authority, standalone validator, local Dylint detector, and generated executable Rust source with a stable target ID, normalized repository-relative path, platform scope, instrumentation class, and disposition. Every executable source MUST be instrumented under the pinned Rust/LLVM closure; only a source with no executable regions MAY use a machine-validated `not-applicable` disposition. Duplicate IDs/paths, unknown classes, broad path/module exclusions, and uninstrumented executable sources MUST fail closed with deterministic diagnostics sorted by target ID and path.

#### Scenario: Positive - inventory closes every executable source class
- **GIVEN** each in-scope source has one unique inventory row and each executable row appears in a compatible current LLVM export
- **WHEN** source-scope completeness is checked
- **THEN** every row is accounted for by instrumentation or a validated no-executable-region disposition

#### Scenario: Negative - standalone validator is absent from exports
- **GIVEN** a standalone validator contains executable functions but no required platform export contains its normalized path
- **WHEN** source-scope completeness is checked
- **THEN** the lane fails with stable diagnostic `LCOV-SOURCE-MISSING` and MUST NOT treat the validator as implicitly excluded

#### Scenario: Negative - executable module claims not applicable
- **GIVEN** an inventory row is marked `not-applicable` while its source contains an executable function, implementation, constant, static, or macro region
- **WHEN** the no-executable disposition is validated
- **THEN** the lane fails with stable diagnostic `LCOV-INVALID-DISPOSITION`

### Requirement: LCOV-003 Exact line, region, function, and branch closure
The normalized aggregate over the complete declared supported-platform scope MUST preserve integer LLVM counts and require `covered == count` and `count > 0` for lines, regions, functions, and branches. Aggregation MUST bind each input to source/tool/configuration/platform identity, normalize repository paths, merge duplicate regions without double counting, and reject absent metrics, missing required platform artifacts, unsupported merge formats, zero denominators, rounded percentages, exclusions, or any uncovered item.

#### Scenario: Positive - aggregate platform counts are exact
- **GIVEN** current compatible exports cover the declared platform union and every normalized metric has a positive denominator
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

#### Scenario: Negative - required platform artifact is missing
- **GIVEN** the inventory declares executable platform-specific regions for a required supported platform but no current compatible export exists for that platform
- **WHEN** aggregate coverage is evaluated
- **THEN** the lane fails with `LCOV-PLATFORM-MISSING` rather than excluding those regions

### Requirement: LCOV-004 Deterministic and fresh coverage evidence
A normalized coverage result MUST bind the source inventory digest, every covered source digest, toolchain and `cargo-llvm-cov` identities, command/features/targets configuration, supported-platform policy, input export digests, and exact integer counts. Entries MUST use stable IDs and deterministic repository-relative ordering and MUST omit timestamps, durations, locale text, host/store paths, traversal order, and raw terminal formatting. Any bound input change MUST make the prior result stale; only a new exact run under current inputs MAY restore a current passing verdict.

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

### Requirement: LCOV-006 Effectful gate and assurance boundary
`devenv tasks run buoy:coverage` MUST execute the focused effectful coverage lane, and the exact aggregate oracle `devenv test` MUST invoke that lane under the pinned offline closure. Acceptance requires a present, current, passing result for every coverage obligation and passing negative controls. Buoy MUST be treated as the dogfood target for this coverage capability: its core, imperative shell, validators, local Dylint detector implementation, and executable positive/negative FCIS fixtures MUST remain in the applicable source inventory and current execution scope. The separately owned Dylint FCIS structural verdict MUST remain distinct from LLVM coverage evidence; neither verdict MAY substitute for the other. Coverage MUST be reported only as source-execution/structural evidence and MUST NOT be labeled behavioral completeness, exhaustive evidence, formal proof, compiler correctness, or absence of defects.

#### Scenario: Positive - exact aggregate oracle includes coverage
- **GIVEN** all static bindings are current and the focused coverage lane can produce exact complete counts
- **WHEN** `devenv test` runs offline
- **THEN** it executes the focused lane and exits successfully only after all coverage obligations and negative controls pass

#### Scenario: Positive - dogfood target covers its own tooling and boundary fixtures
- **GIVEN** the reviewed inventory includes Buoy's core, imperative shell, validators, local Dylint detector implementation, and executable FCIS positive and negative fixtures
- **WHEN** the focused coverage lane and aggregate evidence are evaluated
- **THEN** those applicable sources execute within the pinned coverage scope while the LLVM coverage verdict and separately owned Dylint FCIS structural verdict remain independently visible

#### Scenario: Negative - aggregate command omits the effectful lane
- **GIVEN** an aggregate command validates inventories and historical records but does not execute current LLVM instrumentation
- **WHEN** repository acceptance is evaluated
- **THEN** coverage remains unexecuted or stale and acceptance fails with `LCOV-GATE-OMITTED`

#### Scenario: Negative - full coverage is presented as proof
- **GIVEN** all four source-coverage metrics equal 100%
- **WHEN** an assurance report attempts to classify the result as exhaustive or formally proved
- **THEN** the report is rejected with `LCOV-ASSURANCE-INFLATION` while the structural coverage fact remains visible
