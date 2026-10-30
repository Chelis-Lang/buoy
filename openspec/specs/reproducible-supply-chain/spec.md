# Reproducible Supply Chain

## Purpose
This specification defines controls for reproducible dependency and tool supply chains.
## Requirements
### Requirement: RSC-001 Complete immutable dependency closure

Every input that can affect a blocking claim MUST resolve through an immutable version or revision and a verified content digest.

This rule includes Rust, Nix, Devenv, Git hooks, parsers, Unicode data, test tools, formal tools, templates, wrappers, and generated tools.

The repository MUST bind the Devenv CLI to the pinned module version. Each specialized script MUST bind its runtime packages in Devenv configuration.

A command from an ambient `PATH`, live branch, or unverified registry response MUST NOT support acceptance.

#### Scenario: Positive - declared closure is realized
- **GIVEN** reviewed locks, vendor data, fixed-output metadata, runtime package declarations, and an empty build cache
- **WHEN** dependencies are realized with permitted initial fetch access
- **THEN** every artifact is content checked and later semantic commands require no version discovery

#### Scenario: Negative - mutation tool resolves latest
- **GIVEN** a blocking mutation command installs or resolves an unpinned latest release
- **WHEN** supply-chain policy is checked
- **THEN** the command is nonreproducible and cannot satisfy the mutation obligation

#### Scenario: Negative - script resolves an undeclared host tool
- **GIVEN** a blocking script omits one specialized runtime package and the host supplies that command
- **WHEN** supply-chain policy or final acceptance runs
- **THEN** the script MUST fail and the host command MUST NOT satisfy the obligation

### Requirement: RSC-002 Clean-machine network-disabled acceptance

After closure realization, the project MUST build and run the exact final oracle in a clean sandbox with the network disabled.

The sandbox MUST use empty user Cargo, Rustup, and Elan homes. It MUST contain no undeclared global tools or provider credentials.

`devenv test` MUST realize the complete task graph and every script runtime package from the declared closure. Cache hits MUST NOT supply undeclared inputs.

#### Scenario: Positive - empty user state passes offline
- **GIVEN** only the declared closure, resolved task graph, runtime package sets, and repository bytes
- **WHEN** `devenv test` runs with the network disabled and empty user tool homes
- **THEN** the complete final oracle passes without a metadata fetch or version lookup

#### Scenario: Negative - Cargo registry cache is required
- **GIVEN** a dependency is absent from the declared closure but present in a developer Cargo cache
- **WHEN** the clean offline sandbox runs
- **THEN** acceptance MUST fail and MUST NOT use the developer result as reproducibility evidence

#### Scenario: Negative - ambient package hides an incomplete runtime set
- **GIVEN** a required tool is absent from its script runtime package set but present in the developer environment
- **WHEN** the clean offline sandbox runs
- **THEN** acceptance MUST fail and MUST NOT use the developer result as reproducibility evidence

### Requirement: RSC-003 Advisory, license, source, and duplicate policy
A pinned dependency-policy tool MUST fail closed on disallowed advisories, licenses, source origins, yanked/unreviewed packages, forbidden Git/registry URLs, and unjustified duplicate versions according to repository-owned policy. Exceptions MUST be exact, reviewed, repository-owned, bounded to a package/version/advisory, and visible without claiming technical safety.

#### Scenario: Positive - reviewed dependency set passes
- **GIVEN** every dependency source, checksum, license, advisory state, and duplicate-version rationale satisfies policy
- **WHEN** dependency policy runs offline from pinned advisory/review data
- **THEN** the gate passes with a deterministic policy report

#### Scenario: Negative - wildcard advisory exemption
- **GIVEN** a wildcard exemption suppressing all advisories for a package lineage
- **WHEN** policy validation runs
- **THEN** the exemption is rejected and the affected advisories remain visible

### Requirement: RSC-005 Controlled dependency and tool updates
Updating any semantic dependency or formal tool MUST regenerate its verified lock pins, affected fixtures/goldens, parser/Unicode/path conformance evidence, proof bindings, campaign artifacts, and change impacts before acceptance. `devenv.lock` and `Cargo.lock` hold the verified pins. Historical identities MUST remain addressable and MUST NOT be rewritten in place.

#### Scenario: Positive - parser update is reviewed
- **GIVEN** a new pinned parser revision with changed syntax boundaries
- **WHEN** the update protocol runs
- **THEN** attachment/span fixtures, adapter parity, tool lock, impacts, and compatibility records are updated together

#### Scenario: Negative - lockfile-only verifier update
- **GIVEN** a verifier/solver lock update without refreshed proof records and negative controls
- **WHEN** the final gate runs
- **THEN** formal bindings are stale and `formally-proved` MUST NOT be emitted

### Requirement: RSC-006 Closure size and build-cost visibility
The system MUST report deterministic closure component sizes and measured noncanonical realization/build costs separately. Size/cost budgets MAY block release under repository policy but MUST NOT alter semantic graph identity or be presented as correctness evidence.

#### Scenario: Positive - closure growth is reviewed
- **GIVEN** a dependency update increasing the declared closure beyond its policy threshold
- **WHEN** supply-chain reporting runs
- **THEN** it emits a blocking size-policy impact while semantic object identities remain based only on schema inputs

#### Scenario: Negative - faster build upgrades assurance
- **GIVEN** an optimized cache reducing final-gate duration
- **WHEN** assurance is rendered
- **THEN** no evidence class changes because performance is operational metadata, not correctness evidence

