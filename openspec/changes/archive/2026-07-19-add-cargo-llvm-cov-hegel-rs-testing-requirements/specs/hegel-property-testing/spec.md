## ADDED Requirements

### Requirement: HEGEL-001 Exact Hegel-rs identity and dependency boundary
“Hegel-rs” MUST mean `hegeltest` 0.28.0 from `https://github.com/hegeldev/hegel-rust` with the immutable source commits, Git trees, companion package versions, crates.io archive checksums, license, and approved API surface recorded in `docs/tooling/hegel-rs.md`. The package MUST remain a test-shell development dependency and MUST NOT enter `buoy-core` or its `no_std` dependency closure. A moving revision, unverified archive, ambient package, the unrelated `hegel` crate, or another property framework MUST fail with stable diagnostic `HEGEL-TOOL-IDENTITY`.

#### Scenario: Positive - exact package closure is resolved offline
- **GIVEN** the Cargo/Nix locks contain the approved Hegel package and companion package identities and their archives have been realized
- **WHEN** the property lane resolves dependencies offline
- **THEN** it uses the exact approved closure and the `no_std` core dependency closure remains Hegel-free

#### Scenario: Negative - similarly named framework is substituted
- **GIVEN** a property test imports another `hegel` package, Proptest, QuickCheck, or an unchecksummed Git revision in place of the approved package
- **WHEN** Hegel tool identity and dependency boundaries are validated
- **THEN** the lane fails with `HEGEL-TOOL-IDENTITY` and MUST NOT record sampled Hegel evidence

### Requirement: HEGEL-002 Deterministic runner settings and declared domains
Every authoritative Hegel property MUST explicitly use `Backend::Default`, exactly 256 cases, a committed property-specific seed under `0x5350_5631_0000_0000`, `derandomize(true)`, `database(None)`, and the ordered phases `Explicit`, `Reuse`, `Generate`, `Target`, and `Shrink`. Its inventory row MUST declare stable property ID, generator types and bounds, explicit edge examples, rejection/assumption limits where applicable, and scoped claim. Automatic backends, `Urandom`, environment-derived settings, local failure databases, implicit case counts/phases, undeclared domains, or duplicate seeds/IDs MUST fail deterministically.

#### Scenario: Positive - committed settings reproduce a sampled run
- **GIVEN** a registered property has the exact settings, unique seed, declared generator bounds, and explicit edge examples
- **WHEN** the focused property lane runs twice from identical repository bytes under controlled environments
- **THEN** both runs exercise the same configured sampled obligation and emit the same normalized verdict binding

#### Scenario: Negative - ambient default changes case count
- **GIVEN** a property omits the explicit case count or reads a Hegel/CI environment variable
- **WHEN** property settings are validated
- **THEN** the lane fails with stable diagnostic `HEGEL-SETTINGS-AMBIENT` and MUST NOT execute that property as authoritative evidence

#### Scenario: Negative - generator domain is undocumented
- **GIVEN** a property draws arbitrary values but its inventory omits type, size, depth, rejection, or other applicable bounds
- **WHEN** generator-domain completeness is checked
- **THEN** the lane fails with stable diagnostic `HEGEL-DOMAIN-MISSING`

### Requirement: HEGEL-003 Complete production/reference property inventory
A machine-readable property inventory MUST map every in-scope pure public semantic production algorithm and frozen invariant to one or more Hegel properties. Each production-conformance property MUST call the production authority and a dependency-independent reference authority or a justified independent metamorphic relation, compare exact acceptance/results/bytes/facts as applicable, and bind both source identities. The reference authority MUST NOT import production semantic implementations. A helper-only property MUST be labeled `reference-only` and MUST NOT satisfy a production-conformance obligation; missing, duplicate, or falsely closed obligations MUST fail with deterministic diagnostics ordered by production target and property ID.

#### Scenario: Positive - production and independent canonical encoders agree
- **GIVEN** an inventory row binds a production canonical encoder, a dependency-independent encoder, declared generated/edge domains, and exact byte equality assertions
- **WHEN** the registered Hegel property runs
- **THEN** every sampled input compares both authorities and a passing result closes only that declared production-conformance obligation

#### Scenario: Negative - property proves only a test helper
- **GIVEN** a registered property invokes test-local sorting, digest, Boolean, or transition helpers without invoking the mapped production API
- **WHEN** property-inventory closure is validated
- **THEN** the property is `reference-only`, `HEGEL-PRODUCTION-NOT-CALLED` is emitted, and the production obligation remains open

#### Scenario: Negative - reference imports production semantics
- **GIVEN** the claimed independent reference authority imports or re-exports the production canonical or semantic implementation
- **WHEN** reference independence is validated
- **THEN** the lane fails with `HEGEL-REFERENCE-NOT-INDEPENDENT` and MUST NOT accept equality between the two aliases as differential evidence

#### Scenario: Negative - pure production API has no property
- **GIVEN** a reviewed pure public semantic production function has no mapped current production-conformance property and no explicit inapplicable disposition
- **WHEN** inventory completeness is checked
- **THEN** the lane fails with `HEGEL-PROPERTY-MISSING`

### Requirement: HEGEL-004 Source-bound results, freshness, and replay
A normalized property result MUST bind property ID, expected polarity, exact Hegel/toolchain identity, settings and generator-domain digest, property source digest, production source digest, independent reference source digest, replay-fixture digests, and execution verdict. Entries MUST use deterministic stable-ID ordering and omit timestamps, durations, host paths, terminal formatting, raw shrink traces, and local database state. Any bound input change MUST make prior evidence stale. A failure MUST retain property ID, seed/settings identity, and sufficient noncanonical replay information; a minimized counterexample becomes deterministic evidence only after review and commitment as a digest-bound replay fixture.

#### Scenario: Positive - reviewed counterexample replays
- **GIVEN** a failing sample has been minimized, reviewed, committed as an exact replay fixture, and bound to the property record
- **WHEN** the focused replay command executes under the current pinned closure
- **THEN** the fixture deterministically reproduces the expected failure before any corrected implementation may record a pass

#### Scenario: Negative - property source changes after a pass
- **GIVEN** a passing result binds an earlier property source digest and the assertion or generator subsequently changes
- **WHEN** result freshness is evaluated
- **THEN** the result is stale with `HEGEL-EVIDENCE-STALE` and MUST NOT satisfy the property obligation until rerun

#### Scenario: Negative - raw shrink log is treated as canonical evidence
- **GIVEN** a Hegel failure log contains timing, terminal formatting, host paths, or an uncommitted minimized example
- **WHEN** normalized evidence is generated
- **THEN** those fields are excluded, the uncommitted example remains noncanonical, and no replay-fixture binding is inferred

### Requirement: HEGEL-005 Positive, negative, and false-green controls
The Hegel lane MUST include focused controls proving that the exact approved API compiles, a known-valid property passes, an intentionally false property is rejected, ambient settings are rejected, helper-only production mappings remain open, reference aliasing is rejected, and stale evidence cannot pass. Expected-failure controls MUST verify stable diagnostic/replay identity and exact expected cardinality; an unexpectedly passing false property or an unexecuted property MUST fail the lane.

#### Scenario: Positive - intentionally false property is caught
- **GIVEN** a committed expected-failure property has a declared seed/domain and an assertion falsified within its explicit examples or sampled cases
- **WHEN** the negative-control harness runs
- **THEN** Hegel rejects the property and the harness observes the expected stable property/diagnostic identity exactly once

#### Scenario: Negative - false property unexpectedly passes
- **GIVEN** the expected-failure property is not executed, its assertion is weakened, or the configured domain no longer reaches its counterexample
- **WHEN** the negative-control harness observes no expected rejection
- **THEN** the lane fails with `HEGEL-NEGATIVE-CONTROL` and MUST NOT record aggregate success

#### Scenario: Negative - property panic is counted as successful evidence
- **GIVEN** a property aborts, panics outside the expected assertion protocol, or fails to emit a complete result binding
- **WHEN** verdict aggregation runs
- **THEN** the property is failing with `HEGEL-EXECUTION-INCOMPLETE` and MUST NOT be counted as a pass

### Requirement: HEGEL-006 Effectful gate and sampled-evidence boundary
`devenv tasks run buoy:properties` MUST execute every required current Hegel property and its controls under the pinned offline closure, and the exact aggregate oracle `devenv test` MUST invoke that focused lane. Acceptance requires present registration, current bindings, and passing execution for every blocking property. A passing Hegel result MUST be labeled sampled evidence scoped to its declared domain/settings and MUST NOT be labeled exhaustive, formally proved, compiler-correct, cryptographically proved, or proof of prose-to-property correspondence.

#### Scenario: Positive - aggregate oracle executes all current properties
- **GIVEN** the property inventory, exact settings, sources, references, and tool closure are current
- **WHEN** `devenv test` runs offline
- **THEN** every blocking Hegel property and control executes and aggregate success is possible only when all required normalized verdicts pass

#### Scenario: Negative - aggregate oracle trusts a historical pass
- **GIVEN** a previous property result passed but a bound production, reference, generator, setting, or tool input changed
- **WHEN** `devenv test` runs without a current property execution
- **THEN** acceptance fails with `HEGEL-EVIDENCE-STALE` or `HEGEL-GATE-OMITTED`

#### Scenario: Negative - sampled run is presented as exhaustive proof
- **GIVEN** all 256 configured cases pass for a property
- **WHEN** an assurance report attempts to classify the result as exhaustive or formally proved
- **THEN** the report is rejected with `HEGEL-ASSURANCE-INFLATION` while the scoped sampled result remains visible
