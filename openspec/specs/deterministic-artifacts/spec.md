# deterministic-artifacts Specification

## Purpose

Define deterministic inputs, canonical paths and encoding, stable diagnostics, static/execution separation, functional-core isolation, and provider-neutral change-impact behavior for Buoy.

## Requirements

### Requirement: DET-001 Explicit deterministic inputs
The deterministic core MUST accept repository paths and bytes, project configuration, schema identities, pinned toolchain data, optional normalized oracle verdicts, and, when computing change impact, canonical base and head graphs plus a provider-neutral change manifest as explicit immutable inputs. It MUST NOT read ambient filesystem, clock, environment, network, locale, process, branch, issue, label, pull-request, or review state.

#### Scenario: Positive - identical explicit inputs
- **GIVEN** two invocations with byte-identical explicit inputs
- **WHEN** the deterministic core constructs and renders the static graph
- **THEN** both invocations produce byte-identical canonical graph and static report bytes

#### Scenario: Negative - ambient state differs
- **GIVEN** identical explicit inputs under different timezones, locales, hostnames, absolute roots, and issue-label state
- **WHEN** the deterministic core runs
- **THEN** ambient differences do not alter canonical output

### Requirement: DET-002 Normalized repository paths
The system MUST use versioned repository-relative path normalization and MUST reject absolute paths, escaping `..` segments, duplicate normalized paths, and platform-ambiguous forms forbidden by the active schema.

#### Scenario: Positive - valid relative path
- **GIVEN** a unique repository-relative path accepted by the active schema
- **WHEN** the snapshot is normalized
- **THEN** the path receives one canonical byte representation

#### Scenario: Negative - normalized path collision
- **GIVEN** two input entries that normalize to the same canonical path
- **WHEN** the snapshot is normalized
- **THEN** normalization fails and MUST NOT select a first or last entry

### Requirement: DET-003 Total collection and diagnostic ordering
Every canonical collection and diagnostic set MUST use a schema-defined total order independent of map insertion, filesystem discovery, locale collation, and process completion order.

#### Scenario: Positive - input order is permuted
- **GIVEN** the same semantic objects inserted and discovered in different orders
- **WHEN** canonical graph and diagnostics are serialized
- **THEN** the output bytes are identical

#### Scenario: Negative - duplicate canonical key
- **GIVEN** two distinct objects with the same canonical key
- **WHEN** the collection is constructed
- **THEN** construction fails with a deterministically ordered duplicate-key diagnostic

### Requirement: DET-004 Versioned unambiguous encoding
Every canonical digest MUST identify its algorithm through the active semantic schema, and every digest preimage MUST include a domain tag, schema version, algorithm identity, and unambiguous field encoding. The active version 3 schema MUST take the algorithm name, numeric tag, output length, and complete parameters of every repository-owned identity from the closed digest-algorithm registry, and MUST use the one declared active member for computation. The canonical text form MUST be `<canonical-name>:<lowercase hexadecimal payload>`, and the binary form MUST carry the member's declared tag and declared byte count in that member's official canonical order. Truncated forms MUST be display-only. A digest under an obsolete tag or an unregistered algorithm MUST NOT be accepted by the active schema, and an obsolete digest MUST be regenerated from the authoritative canonical source under the version 3 schema and domain rather than reinterpreted in place.

#### Scenario: Positive - semantic field changes
- **GIVEN** a canonical version 3 object and one mutation to a digest-bound semantic field
- **WHEN** both domain-separated preimages are encoded and hashed with the declared active member
- **THEN** the encoded preimages and full version 3 digests differ for the exercised mutation

#### Scenario: Negative - ambiguous concatenation boundary
- **GIVEN** two different sequences of field names or values that would collide under raw concatenation
- **WHEN** the canonical encoder processes them
- **THEN** length delimiting or an equivalent encoding produces distinct preimages before hashing

#### Scenario: Negative - obsolete algorithm changes in place
- **GIVEN** an object identity established under an obsolete tag, an unregistered algorithm, or a registered member that is not the declared active member
- **WHEN** a configuration, parser, or migration attempts to relabel or reinterpret its digest bytes under the active member
- **THEN** validation fails with a deterministic digest-schema diagnostic and requires regeneration from authoritative canonical source bytes

### Requirement: DET-005 Syntax-aware text canonicalization
Text canonicalization MUST be versioned and syntax-aware. Until a schema explicitly declares a representation difference insignificant, raw bytes MUST remain semantically significant.

#### Scenario: Positive - declared line-ending equivalence
- **GIVEN** a schema that explicitly normalizes CRLF and LF for one syntax class
- **WHEN** otherwise identical inputs differ only by those line endings
- **THEN** that syntax class receives identical canonical semantic content

#### Scenario: Negative - prose edit is guessed equivalent
- **GIVEN** two normative prose bodies that differ in bytes and no schema rule declares the difference insignificant
- **WHEN** atom revisions are computed
- **THEN** the system treats them as distinct semantic revisions

### Requirement: DET-006 Static and execution artifact separation
The static graph MUST be independent of live oracle execution. An execution report MUST be a deterministic function of the static graph and normalized verdict set, while raw logs and operational metadata remain noncanonical.

#### Scenario: Positive - same normalized verdicts arrive differently
- **GIVEN** the same static graph and normalized oracle verdicts delivered in different completion orders
- **WHEN** execution reports are serialized
- **THEN** report bytes are identical

#### Scenario: Negative - timeout changes static identity
- **GIVEN** a live oracle that times out on one host
- **WHEN** its verdict is recorded
- **THEN** the execution report records `timeout` but the canonical static graph identity remains unchanged

### Requirement: DET-007 Deterministic diagnostics
Diagnostics MUST have stable machine codes, normalized locations, structured fields, and a total order. Canonical diagnostic output MUST exclude timestamps, temporary paths, PIDs, terminal width, and nondeterministic raw tool text.

#### Scenario: Positive - same invalid graph in separate roots
- **GIVEN** identical invalid repositories located under different host absolute roots
- **WHEN** static validation runs
- **THEN** canonical diagnostic bytes are identical

#### Scenario: Negative - raw verifier log contains host data
- **GIVEN** a verifier log containing a temporary path and process ID
- **WHEN** the system builds a canonical report
- **THEN** those raw values are confined to a noncanonical diagnostic channel

### Requirement: DET-008 Structurally isolated functional core
The deterministic semantic kernel MUST reside in a `no_std` core crate whose dependency closure builds without `std`. It MUST NOT contain mutable statics or immutable statics whose types permit interior mutation, and its exported function signatures MUST NOT contain function pointers or `Fn`, `FnMut`, or `FnOnce` bounds. Immutable `Freeze` statics, invocation-local mutation, and private core-owned callable helpers MAY remain. Filesystem, environment, clock, network, locale, parser, and process access MUST remain in imperative-shell or adapter crates, shell-side crates MAY depend on the core, and the core MUST NOT depend outward on them. Structural checks MUST NOT be reported as proof of semantic purity or correctness beyond their fixture-proven detector forms.

#### Scenario: Positive - core boundary is structurally enforced
- **GIVEN** a core crate that declares `#![no_std]`, has only compatible inward dependencies, uses only immutable `Freeze` statics and invocation-local mutation, exposes only concrete value-bearing function signatures, and receives all semantic inputs explicitly
- **WHEN** the FCIS architecture gate runs its Dylint rules and bare-metal target build
- **THEN** all checks pass and the result is reported only as structural enforcement

#### Scenario: Negative - core enables the standard library
- **GIVEN** a proposed core crate that omits `#![no_std]` or introduces a dependency requiring `std`
- **WHEN** the FCIS architecture gate runs
- **THEN** the `no_std` Dylint rule or bare-metal target build fails and the crate cannot be accepted as the deterministic core

#### Scenario: Negative - core owns ambient mutable state
- **GIVEN** a proposed core crate containing a mutable static or an immutable static whose type permits interior mutation
- **WHEN** the FCIS architecture gate runs
- **THEN** the ambient-state Dylint rule fails while immutable `Freeze` statics and invocation-local mutation remain accepted

#### Scenario: Negative - public core interface accepts callable behavior
- **GIVEN** an exported core function signature containing a function pointer or an `Fn`, `FnMut`, or `FnOnce` bound
- **WHEN** the FCIS architecture gate runs
- **THEN** the callable-interface Dylint rule fails while private core-owned callable helpers remain accepted

### Requirement: DET-009 Deterministic base-to-head change impact
The core MUST compare explicit base and head graphs with one provider-neutral change manifest. Equal inputs MUST produce byte-identical impact reports.

The report MUST include all changed, retired, and affected governed objects. This set includes source items, external trace links, carriers, and implementation links.

The report MUST also include affected surfaces, policies, derivations, properties, assumptions, models, proofs, debt, and waivers.

The core MUST NOT consult version-control hosts, issue APIs, pull-request APIs, labels, users, reviews, timestamps, or network state.

#### Scenario: Positive - identical graph pair in separate repositories
- **GIVEN** identical canonical base and head graphs and neutral change manifests in two different absolute roots
- **WHEN** change impact is computed
- **THEN** the canonical impact report bytes are identical

#### Scenario: Positive - requirement change reaches all links
- **GIVEN** an OpenSpec requirement revision with code, evidence, and external trace links
- **WHEN** that requirement changes and impact is computed
- **THEN** the impact set contains every transitively affected parsed item, link, carrier, and dependent governed object

#### Scenario: Negative - provider metadata alters impact
- **GIVEN** identical canonical base and head inputs but different pull-request labels, issue state, authors, or review timestamps
- **WHEN** change impact is computed
- **THEN** provider metadata does not alter the canonical impact report

#### Scenario: Negative - external trace link is omitted
- **GIVEN** an atom revision change that makes one external trace link stale
- **WHEN** impact computation omits that link
- **THEN** impact validation fails with a deterministic missing-impact-edge diagnostic

### Requirement: DET-010 Complete change-impact dispositions
Every impact required by the active change policy MUST receive exactly one provider-neutral disposition of `reaffirmed`, `revised`, `retired`, or `waived`, binding the relevant base and head digests. Missing, duplicate, stale, or inapplicable dispositions MUST fail the change gate with deterministically ordered diagnostics.

#### Scenario: Positive - every affected claim is dispositioned
- **GIVEN** an atom revision change whose affected carriers, surface members, derivations, and proofs each have one valid disposition
- **WHEN** change-impact closure is validated
- **THEN** closure succeeds while each disposition retains its independent evidence meaning

#### Scenario: Negative - one affected derivation is omitted
- **GIVEN** an atom revision change that transitively affects a derivation but the neutral change manifest has no disposition for it
- **WHEN** change-impact closure is validated
- **THEN** the change gate fails with a deterministic missing-impact-disposition diagnostic

### Requirement: DET-011 Deterministic OpenSpec adapter inputs
OpenSpec adapter output MUST depend only on saved repository paths, saved bytes, explicit configuration, schema identities, and pinned parser inputs.

#### Scenario: Positive - equal saved inputs under different hosts
- **GIVEN** equal OpenSpec files and configuration on hosts with different roots, locales, and traversal orders
- **WHEN** the OpenSpec adapter runs
- **THEN** both hosts produce byte-identical graph fragments, source maps, dispositions, and diagnostics

#### Scenario: Negative - OpenSpec CLI or provider supplies ambient state
- **GIVEN** an adapter path that reads an OpenSpec process, unsaved editor buffer, or provider API
- **WHEN** deterministic boundary validation runs
- **THEN** validation fails with a stable ambient-input diagnostic
