## MODIFIED Requirements

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

### Requirement: RSC-004 Reviewable provenance and closure inventory

The repository MUST generate a deterministic inventory for dependencies and closure components. Each row MUST bind name, version, source, digest, license, role, platform, and claim scope.

Each tool row MUST state its exposure mode as common shell, script runtime, generated script, or internal dependency. A runtime-only tool MUST remain inventoried.

The inventory MUST exclude host store paths as identity. It MUST distinguish bundled Verus Z3, standalone Z3, minimal Aeneas models, and the full backend.

#### Scenario: Positive - formal tools are distinguished
- **GIVEN** Verus, bundled Z3, standalone Z3, Aeneas, Charon, Lean, and a minimal Lean model
- **WHEN** the closure inventory is generated
- **THEN** each tool has one exact identity, role, platform, trust scope, and exclusions

#### Scenario: Positive - shell and runtime tools remain visible
- **GIVEN** common development tools and specialized script runtime tools in the same pinned closure
- **WHEN** the closure inventory is generated
- **THEN** each tool has one exact identity, exposure mode, role, platform, trust scope, and exclusions

#### Scenario: Negative - two solvers collapse by executable name
- **GIVEN** bundled and standalone solvers share a command name but use different artifacts and configurations
- **WHEN** inventory identity is checked
- **THEN** the solvers MUST remain distinct and MUST NOT satisfy one another's proof bindings

#### Scenario: Negative - runtime-only formal tool disappears from inventory
- **GIVEN** a formal tool moves from the common shell to one script runtime package set
- **WHEN** inventory completeness is checked
- **THEN** the missing tool row MUST fail with a deterministic exposure-mode diagnostic
