# Pinned Buoy Distribution

## Purpose

Define the versioned, deterministic, offline package interface that consumers use to realize and identify the Buoy executable.

## Requirements

### Requirement: PBD-001 Buoy-owned Nix consumer package

The Buoy repository MUST expose `nix/consumer-package-v1.nix` as the version 1 package interface for the `buoy` executable.

The function MUST require source, source revision, source-content digest, Rust toolchain, system, and digest-feature inputs.

The package implementation MUST remain Buoy-owned. A consumer MUST need only pinned inputs, verified lock data, and its repository configuration.

The package MUST build from the committed Cargo lock with an explicit digest-algorithm feature set.

The package MUST install exactly `buoy`, `buoy-migrate`, and `chelis-provenance` in its `bin` directory.
Repository checkers, generators, and policy tools MUST remain source-development commands, not installed consumer commands.
This package selection MUST preserve the separate compatibility interfaces through their documented release window.

The package MUST map compiler source paths to `/rustc` during compilation.
The installed output MUST NOT reference the pinned compiler store path.
The package MUST retain runtime library references and compiler identity metadata.

It MUST NOT resolve an ambient compiler, registry version, or host tool.

#### Scenario: Positive - consumer realizes pinned Buoy package

- **GIVEN** one consumer lock with an exact Buoy source revision, verified content, and realized fixed-output closure
- **WHEN** Nix realizes the Buoy package with empty user build state
- **THEN** the consumer receives the declared executable without a consumer-owned fetch or build script

#### Scenario: Negative - consumer wrapper fetches a live branch

- **GIVEN** one consumer command that clones a live Buoy branch or resolves a latest package version
- **WHEN** package conformance runs
- **THEN** the command cannot satisfy the pinned-distribution requirement

### Requirement: PBD-002 Explicit package variants and feature identity

Each package variant MUST declare its enabled digest-algorithm features and disabled default features.

The initial interface MUST provide `xxh3-128`, `blake3-256`, and `sha-256` variants.

Variant names and feature sets MUST remain stable within the interface version.

A package built for one active member MUST fail clearly when repository configuration requires an unavailable member.

#### Scenario: Positive - collision-resistant consumer selects BLAKE3

- **GIVEN** one package variant with only `blake3-256` enabled and one matching repository configuration
- **WHEN** Buoy materializes the repository
- **THEN** the configured member is available and no undeclared digest member supplies an identity

#### Scenario: Negative - package feature set differs from configuration

- **GIVEN** one package without the repository's configured active digest member
- **WHEN** configuration parsing runs
- **THEN** parsing fails with a deterministic unavailable-member diagnostic before graph construction

### Requirement: PBD-003 Machine-readable package identity

The packaged executable MUST expose machine schema `buoy/package-identity/v1` through read-only command `buoy package-identity`.

Package identity MUST contain the Buoy package version, explicit source revision or source-tree identity, Rust toolchain identity, enabled features, and supported digest members.

Package identity MUST remain operational provenance. Nix and Git identities MUST remain opaque ecosystem values.

Package identity MUST NOT enter canonical graph identity, satisfy authority, or establish correctness.

#### Scenario: Positive - consumer checks the approved package

- **GIVEN** one packaged executable and one reviewed consumer lock
- **WHEN** the consumer requests machine package identity
- **THEN** every locked identity and enabled feature agrees with the package report

#### Scenario: Negative - package report omits its feature set

- **GIVEN** one packaged executable whose identity report omits an enabled digest member
- **WHEN** package-identity conformance runs
- **THEN** conformance fails with a deterministic incomplete-package-identity diagnostic

### Requirement: PBD-004 Offline execution after package realization

After initial package realization, static, change, bind, trace, and publication commands MUST run without source fetches, registry queries, or package discovery.

The package MUST store no SSH key, provider token, user credential, or host-specific source path.

Private source acquisition MUST remain a Nix input concern. It MUST use noninteractive read-only SSH without repository-stored credentials.

The verified result MUST be immutable package store content.

#### Scenario: Positive - realized package runs offline

- **GIVEN** one realized Buoy package, complete mounted snapshots, and no network access
- **WHEN** each static command family runs
- **THEN** every command uses the realized executable and performs no package or source discovery

#### Scenario: Negative - packaged launcher clones Buoy at runtime

- **GIVEN** one package whose executable wrapper performs a Git fetch before each invocation
- **WHEN** the network-disabled conformance test runs
- **THEN** the package fails and cannot satisfy the distribution contract

### Requirement: PBD-005 Deterministic package and consumer acceptance

Equal source, lock, package parameters, feature sets, and supported systems MUST produce equal package outputs under the declared Nix reproducibility boundary.

Buoy final acceptance MUST include one blocking local package-contract task in the resolved `devenv test` graph.

That task MUST realize local-source variants offline from the declared closure with empty user Cargo and Rustup homes.

A separate explicit advisory task MUST test pinned private `git+ssh` acquisition when test credentials are available.

The package interface MUST document supported systems, update procedure, rollback, private-source requirements, and consumer migration from local wrappers.

#### Scenario: Positive - two clean package realizations agree

- **GIVEN** equal source and package inputs on one supported system with the declared closure
- **WHEN** two isolated package realizations complete with empty user build state
- **THEN** their verified package outputs and machine package identities agree

#### Scenario: Positive - private source integration remains advisory

- **GIVEN** one exact private `git+ssh` pin and test credentials supplied outside the repository
- **WHEN** the maintainer selects the private-source integration task
- **THEN** realization verifies pinned content, stores no credentials, and preserves the final acceptance graph

#### Scenario: Negative - package contract is absent from final acceptance

- **GIVEN** one local package-contract task with no dependency path to `devenv:enterTest`
- **WHEN** Buoy gate-omission validation runs
- **THEN** the blocking package claim fails with a deterministic omitted-package-contract diagnostic

#### Scenario: Negative - package succeeds only with developer Cargo state

- **GIVEN** one package build that requires a developer registry cache or ambient Rust toolchain
- **WHEN** isolated acceptance runs
- **THEN** realization fails and the developer result supplies no package evidence
