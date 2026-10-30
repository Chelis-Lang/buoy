## ADDED Requirements

### Requirement: RSC-001 Complete immutable dependency closure
Every Rust, Nix, parser, Unicode, fuzz, mutation, sanitizer, formal, solver, backend, model, template, wrapper, and generated-tool input that can affect a blocking claim MUST resolve through an immutable version/revision and verified content digest. A command found incidentally on ambient `PATH`, a live branch, or an unverified registry response MUST NOT support acceptance.

#### Scenario: Positive - declared closure is realized
- **GIVEN** the reviewed lock/vendor/fixed-output metadata and an empty build store/cache
- **WHEN** dependencies are realized with permitted initial fetch access
- **THEN** every artifact is content verified and subsequent semantic commands require no version discovery

#### Scenario: Negative - mutation tool resolves latest
- **GIVEN** a blocking mutation command that installs or resolves an unpinned latest release
- **WHEN** supply-chain policy is checked
- **THEN** the command is nonreproducible and cannot satisfy the mutation obligation

### Requirement: RSC-002 Clean-machine network-disabled acceptance
After realizing the declared content-addressed closure, the project MUST build and run the exact final oracle in a clean sandbox with network disabled, empty user Cargo/rustup/elan homes, no undeclared global tools, controlled environment, and no provider credentials. Cache hits MAY improve performance but MUST NOT supply undeclared inputs.

#### Scenario: Positive - empty user state passes offline
- **GIVEN** only the declared realized closure and repository bytes in a clean sandbox
- **WHEN** `devenv test` runs with network disabled and empty user tool homes
- **THEN** the complete final oracle passes without fetching or resolving metadata

#### Scenario: Negative - Cargo registry cache is required
- **GIVEN** a dependency absent from vendor/fixed-output closure but present in a developer's global Cargo cache
- **WHEN** the clean offline sandbox runs
- **THEN** acceptance fails and MUST NOT claim offline reproducibility from the developer-host pass

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

### Requirement: RSC-004 Reviewable provenance and closure inventory
The repository MUST generate a deterministic dependency/closure inventory binding package/tool name, version/revision, source, content digest, license, role, platform, and claim scope. The inventory MUST exclude host store paths as identity and MUST distinguish bundled Verus Z3 from standalone Z3 and minimal Aeneas models from the full backend.

#### Scenario: Positive - formal tools are distinguished
- **GIVEN** one Verus distribution with bundled Z3, standalone Z3, Aeneas/Charon, Lean, and a minimal bound Lean model
- **WHEN** the closure inventory is generated
- **THEN** each receives a distinct exact identity, role, platform, trust scope, and exclusions

#### Scenario: Negative - two solvers collapse by executable name
- **GIVEN** bundled and standalone solvers with the same program name but different artifacts/configurations
- **WHEN** inventory identity is validated
- **THEN** they remain distinct and MUST NOT satisfy one another's proof bindings

### Requirement: RSC-005 Controlled dependency and tool updates
Updating any semantic dependency or formal tool MUST regenerate its verified closure, inventory, affected fixtures/goldens, parser/Unicode/path conformance evidence, proof bindings, campaign artifacts, and change impacts before acceptance. Historical identities MUST remain addressable and MUST NOT be rewritten in place.

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
