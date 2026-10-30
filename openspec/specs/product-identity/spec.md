# Product Identity Specification

## Purpose

Define Buoy's primary product identity, bounded compatibility aliases, frozen protocol-name invariants, rename evidence freshness, and integration-planning boundaries.

## Requirements

### Requirement: PID-001 Buoy is the primary product identity
The project SHALL use **Buoy** as its current human-facing product name, `buoy` as its primary CLI and root Rust package/crate identity, `buoy-core` (`buoy_core`) as its pure core package/crate identity, and `buoy-*` for repository-owned operational commands. User-facing help, maintained examples, current documentation, package metadata, and integration guidance MUST use the Buoy identity unless they are explicitly describing migration or a frozen compatibility identifier.

#### Scenario: Positive - primary interfaces use Buoy
- **GIVEN** a clean checkout after the rename
- **WHEN** package metadata, CLI help, maintained documentation, examples, and operational commands are inspected
- **THEN** each primary interface identifies the project as Buoy and uses its assigned `buoy` name

#### Scenario: Negative - current interface retains unclassified old branding
- **GIVEN** a maintained user-facing interface containing “Spec Provenance” or a `spec-provenance` operational name without an allowed migration or compatibility disposition
- **WHEN** the naming conformance gate runs
- **THEN** it fails with a stable stale-product-identity diagnostic

### Requirement: PID-002 Frozen v1 protocol identity is rename-invariant
The rename MUST NOT change frozen `spec-provenance-*/v1` schema IDs, semantic-digest domain separators, result protocol IDs, stable diagnostic codes, historical-reader inputs, or authoritative v1 graph/report bytes. Such strings SHALL be treated as immutable protocol identifiers rather than current branding. A future Buoy protocol namespace MUST use a separately approved schema version and migration.

#### Scenario: Positive - canonical corpus is byte-identical after rename
- **GIVEN** reviewed pre-rename canonical value, graph, static report, execution report, and impact report vectors
- **WHEN** the renamed Buoy implementation regenerates and reads the corpus with the same semantic inputs
- **THEN** every authoritative byte sequence and full digest matches exactly

#### Scenario: Negative - global replacement changes a frozen identifier
- **GIVEN** a mutation replacing a frozen `spec-provenance-*/v1` identifier or digest domain with `buoy-*`
- **WHEN** canonical differential and historical-reader checks run
- **THEN** the mutation is rejected with a stable frozen-identity or canonical-parity diagnostic

### Requirement: PID-003 Compatibility aliases are bounded and nonauthoritative
Where technically feasible, the repository SHALL provide thin compatibility aliases for the former CLI, package, crate, and operational-command names for one explicitly recorded compatibility window. An alias MUST delegate to or re-export the Buoy implementation, preserve authoritative bytes and exit classes, emit only deterministic noncanonical deprecation information, and MUST NOT become a second semantic implementation, canonical identity, or assurance source. Alias expiry and removal criteria MUST be machine-readable.

#### Scenario: Positive - former CLI delegates with canonical parity
- **GIVEN** equivalent invocations through `buoy` and the compatibility `spec-provenance` command during the declared window
- **WHEN** static checking or effectful oracle execution completes
- **THEN** exit classes and authoritative binary report bytes are identical while any deprecation message remains outside canonical output

#### Scenario: Negative - compatibility alias changes semantics or outlives its window
- **GIVEN** an alias that changes report bytes, exit classification, evidence meaning, or has passed its recorded expiry
- **WHEN** compatibility conformance runs
- **THEN** acceptance fails closed with a stable compatibility-alias diagnostic

### Requirement: PID-004 Old-name occurrences have total disposition
Every tracked occurrence of the old product/package/command name MUST be present in a machine-readable naming inventory with exactly one closed disposition: `rename-product`, `compatibility-alias`, `frozen-v1-identifier`, `historical-reference`, or `generated-or-external`. The inventory MUST bind the exact path, occurrence class, rationale, owner, and removal or retention rule. Broad directory, extension, generated-file, or substring exclusions MUST NOT satisfy this requirement.

#### Scenario: Positive - only reviewed retained identifiers remain
- **GIVEN** the completed rename and its naming inventory
- **WHEN** all tracked text, metadata, generated sources, fixtures, and planning artifacts are scanned deterministically
- **THEN** every old-name occurrence has exactly one valid disposition and every retained occurrence matches its rule

#### Scenario: Negative - hidden or multiply classified stale occurrence
- **GIVEN** an old-name occurrence that is absent from the inventory, matches multiple rows, or is covered only by a broad exclusion
- **WHEN** the stale-name checker runs
- **THEN** it fails with a stable missing, duplicate, or invalid-disposition diagnostic

### Requirement: PID-005 Rename preserves the functional-core boundary
`buoy-core` MUST remain safe, sequential, pure `no_std` code and MUST NOT acquire filesystem, environment, clock, locale, network, parser, process, provider, or repository dependencies. The Buoy shell, adapters, compatibility wrappers, and integrations MAY depend inward on `buoy-core`; `buoy-core` MUST NOT depend on them. Static registration, freshness, execution verdict, and assurance class SHALL remain separate facts across both primary and compatibility entry points.

#### Scenario: Positive - renamed dependency graph remains one-way
- **GIVEN** the renamed Cargo workspace and structural boundary checker
- **WHEN** every production crate, feature, and public interface is inspected
- **THEN** `buoy-core` is `no_std` and effect-free and all shell or compatibility dependencies point inward

#### Scenario: Negative - rename wrapper enters the core or launders assurance
- **GIVEN** a mutation adding an effectful compatibility API to `buoy-core` or treating alias parity, renamed tests, or aggregate success as proof
- **WHEN** compile-fail, boundary, and evidence aggregation checks run
- **THEN** the mutation is rejected with stable architecture or no-laundering diagnostics

### Requirement: PID-006 Static and effectful Buoy commands retain orthogonal semantics
The `buoy` command SHALL expose the existing deterministic static, change-impact, and effectful execution modes under Buoy naming without changing their authoritative inputs or fact boundaries. Process logs, elapsed time, PIDs, scheduling, host paths, provider state, and deprecation messages MUST remain noncanonical. Completion order and selection through a compatibility alias MUST NOT change static graph or report identity.

#### Scenario: Positive - renamed static and execution modes remain deterministic
- **GIVEN** the same repository bytes, configuration, schema versions, pinned tools, and normalized oracle results
- **WHEN** Buoy static and execution commands run under permuted traversal and completion order
- **THEN** static identity and the applicable authoritative report bytes remain exactly equal

#### Scenario: Negative - branding or deprecation text enters canonical receipt
- **GIVEN** a mutation encoding the current product label, alias name, warning text, elapsed time, PID, or host path into an authoritative report
- **WHEN** canonical parity tests run
- **THEN** they reject the mutation with a stable noncanonical-input or byte-parity diagnostic

### Requirement: PID-007 Rename-induced evidence drift is explicit staleness
Any source path, Cargo package identity, implementation representation, proof binding, derivation, coverage object, fuzz or mutation campaign, supply-chain closure record, source inventory, or generated presentation affected by the rename MUST be rebound and replayed against its exact post-rename inputs. Pre-rename evidence MUST NOT satisfy post-rename obligations through source hashes alone, aggregate status, annotations, or cross-evidence-class inference. A current replay MAY retain an explicit nonpassing disposition when the underlying behavioral obligation was already unfinished; rename acceptance MUST preserve that result and ownership rather than mark the obligation complete.

#### Scenario: Positive - exact evidence is replayed after rename
- **GIVEN** a proof, coverage result, campaign, or closure record whose bound path or package identity changed
- **WHEN** the rename migration completes
- **THEN** a current post-rename record names the exact implementation, tools, configuration, assumptions, scope, exclusions, and replay result

#### Scenario: Negative - pre-rename green evidence is copied forward
- **GIVEN** a post-rename obligation backed only by an old path-bound record, aggregate green status, test annotation, or unchanged source hash
- **WHEN** assurance freshness validation runs
- **THEN** the obligation remains stale or unsatisfied with a stable evidence-class-specific diagnostic

### Requirement: PID-008 In-flight changes target Buoy without semantic coupling or duplicate evidence
OpenSpec changes and integration guidance that refer to current package, crate, command, or product names MUST be reconciled to Buoy before implementation resumes. The focused cargo-llvm-cov/Hegel-rs change MUST target `buoy`, `buoy-core`, and `buoy-reference`, use `devenv tasks run buoy:coverage` and `devenv tasks run buoy:properties` as its primary focused commands, and retain former task names only as bounded compatibility aliases. Its `LCOV-*` and `HEGEL-*` obligations MUST remain independently visible while satisfying mapped production-hardening P0 obligations exactly once; rename bookkeeping MUST NOT duplicate evidence or mark unfinished behavioral work complete. The Chelis integration SHALL use Buoy shell-side APIs through a one-way adapter and MUST NOT alter Chelis compiler semantics or make provider, OpenSpec, network, clock, locale, or Git state canonical.

#### Scenario: Positive - focused testing planning targets Buoy and maps once to P0
- **GIVEN** the focused coverage/Hegel and production-hardening artifacts after reconciliation
- **WHEN** their package, crate, command, task, inventory, evidence-binding, and obligation mappings are inspected
- **THEN** primary operational names target Buoy, former names are only bounded compatibility aliases, unfinished work remains open, and each completed focused obligation satisfies at most one mapped hardening P0 ledger entry

#### Scenario: Negative - focused evidence is duplicated or old commands remain primary
- **GIVEN** a reconciliation that keeps `spec-provenance:coverage` or `spec-provenance:properties` as a primary command, marks an unfinished focused task complete, or counts one result twice in hardening P0
- **WHEN** naming and assurance-ledger conformance run
- **THEN** acceptance fails with a stable product-identity, stale-evidence, or duplicate-obligation diagnostic

#### Scenario: Positive - Chelis planning references Buoy shell interfaces
- **GIVEN** the deferred Chelis integration artifacts after reconciliation
- **WHEN** their package, crate, CLI, and dependency descriptions are inspected
- **THEN** they target Buoy names while preserving the one-way shell adapter and saved-snapshot semantics

#### Scenario: Negative - rename is implemented by coupling compiler semantics
- **GIVEN** an integration revision that imports Buoy into Chelis compiler-core crates or makes ambient repository/provider state canonical
- **WHEN** architecture and canonical-input checks run
- **THEN** the revision is rejected with stable dependency-boundary or noncanonical-input diagnostics

### Requirement: PID-009 Optional OpenSpec standing-spec adapter boundary
Schema v3 MUST support an optional shell adapter for saved standing specs under `openspec/specs/`.

The adapter MUST NOT parse OpenSpec change state as product authority. `buoy-core` MUST remain independent from OpenSpec, parsers, processes, and providers.

#### Scenario: Positive - v3 adapter reads standing specs
- **GIVEN** schema v3, an explicit OpenSpec selector, saved standing specs, and complete adapter configuration
- **WHEN** the shell materializes the selected requirements
- **THEN** the graph contains their atoms while `buoy-core` receives only complete parsed values

#### Scenario: Positive - v3 cutover is complete
- **GIVEN** a repository that enables schema v3
- **WHEN** all selected adapters run
- **THEN** every parsed item uses a schema-v3 source item kind

#### Scenario: Negative - archive action grants authority
- **GIVEN** an OpenSpec archive operation without an explicit Buoy lifecycle or approval update
- **WHEN** schema v3 materializes the resulting standing requirement
- **THEN** the operation MUST NOT ratify the atom or create review-only assurance

#### Scenario: Negative - core imports OpenSpec behavior
- **GIVEN** a change that adds OpenSpec parsing, lifecycle state, filesystem access, or process access to `buoy-core`
- **WHEN** architecture validation runs
- **THEN** validation fails with a stable dependency-boundary diagnostic
