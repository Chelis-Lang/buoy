## 1. Project scaffolding

**Phase handoff:** Inherits the approved project intent; delivers the repository, pinned environment, OpenSpec planning workflow, and enforced FCIS boundary; freezes the crate dependency direction and scaffold oracle; explicitly excludes production provenance behavior; oracle: `devenv test` over the scaffold and architecture fixtures.

- [x] 1.1 Establish the root `spec-provenance` Cargo library without production provenance behavior.
- [x] 1.2 Add the pinned Rust Devenv shell, OpenSpec package, local tasks, and `devenv test` scaffold oracle.
- [x] 1.3 Initialize OpenSpec for Pi and migrate the planning Markdown into this proposal, design, capability specs, and task list.
- [x] 1.4 Generate lock files, initialize standalone version control, and prove the clean scaffold passes `devenv test`.
- [x] 1.5 Split the pure `no_std` functional core from the imperative shell and add pinned Dylint plus positive and negative FCIS boundary checks to `devenv test`.
- [x] 1.6 Add no-ambient-state and no-callable-public-interface Dylint rules, allowed-positive and violating-negative detector fixtures, `NoFix` behavior, and focused gate coverage before core APIs are implemented.

## 2. Resolve versioned schema decisions

**Phase handoff:** Inherits the frozen FCIS boundary and approved capability specs; delivers every semantic schema and decision required to write fixtures; freezes v1 identities, domains, dispositions, policy and change semantics; explicitly excludes parser and production-core implementation; oracle: all schema decisions are closed, independently reviewable, and `devenv test` remains green.

- [x] 2.1 Decide and document v1 atom syntax, authority boundaries, stable ID allocation, semantic metadata, approval authority, and the explicit boundary that OpenSpec is planning/reference only before parser fixtures.
- [x] 2.2 Decide and document v1 path, symlink, submodule, Unicode, line-ending, terminal-newline, Markdown-trivia, selected-root, and expected-authority policies.
- [x] 2.3 Specify the versioned algorithm-identified domain-separated canonical encoding, SHA-256 rationale and migration rule, total object and diagnostic ordering, duplicate rejection, and every digest preimage.
- [x] 2.4 Resolve the exact Hegel-rs upstream repository, immutable revision, license, API, and integrity digest without silently substituting another framework.
- [x] 2.5 Define versioned schemas for atoms, tombstones, decision surfaces and dispositions, carriers and exact evidence scopes, implementation links, derivations, coverage policies, debt records, waivers, properties, finite models, assumptions, oracles, proof records, change manifests, change dispositions, and reports.
- [x] 2.6 Define decision-surface domain evolution, completeness, member-key, authority-citation, and host-adapter extension rules without embedding Chelis-specific semantics in the core.
- [x] 2.7 Define coverage-policy adoption ratchets, policy weakening, debt and waiver discharge, and reaffirmation approval rules without using provider state as authority.
- [x] 2.8 Freeze the inherit, deliver, frozen-at-exit, explicitly-not-owned, and exact oracle contract for each remaining implementation phase before that phase starts.

## 3. Write fixtures and independent models before implementation

**Phase handoff:** Inherits frozen v1 schemas; delivers executable positive and negative examples, independent models, and a closed fixture manifest; freezes expected acceptance and diagnostic behavior before algorithms exist; explicitly excludes production implementation and fixture expectation updates that merely match later code; oracle: every manifest entry proves its intended polarity through the focused fixture runner.

- [x] 3.1 Create positive and negative schema fixtures for every SPG requirement, including duplicate authority, retired-ID reuse, dangling references, fenced examples, noncarrier mentions, missing/duplicate/unknown decision-surface members, absent disposition authority, stale derivations, unsupported syntax versions, and vacuous expected-authority roots.
- [x] 3.2 Create positive and negative deterministic fixtures for every DET requirement, including traversal permutations, normalized-path collisions, digest and algorithm boundaries, Unicode, CRLF, locale, timezone, separate absolute roots, base-to-head impact permutations, provider-metadata variation, and missing/duplicate/stale change dispositions.
- [x] 3.3 Create positive and negative evidence fixtures for every EVD requirement, including every verdict, stale and absent roles, partial execution, point-scope overclaiming, policy weakening, debt and waiver no-laundering, reaffirmation no-laundering, and known-failing carriers.
- [x] 3.4 Create positive and negative proof fixtures for every FPR requirement, mutating each proof binding independently and checking intended verifier failures.
- [x] 3.5 Define independent finite reference models for atom lifecycle, decision-surface completeness, carrier and derivation freshness, coverage-policy adoption, required-role coverage, change-impact disposition closure, debt/waiver/reaffirmation no-laundering, verdict aggregation, and proof invalidation.
- [x] 3.6 Build a fail-closed fixture manifest that maps each requirement and scenario to polarity, fixture, focused slice, and final oracle.

## 4. Establish property and proof spikes

**Phase handoff:** Inherits frozen fixtures and independent models; delivers property suites, exhaustive runners, and comparable formal spikes; freezes the evidence used to choose the primary formal lane; explicitly excludes production provenance algorithms and preference-only verifier selection; oracle: all property suites and fixed-point model runners pass, and each formal spike has a reproducible reviewed result.

- [x] 4.1 Add Hegel-rs properties for canonicalization idempotence, permutation invariance, digest and algorithm sensitivity, lifecycle terminality, decision-surface completeness, carrier and derivation freshness, coverage-policy monotonicity, change-impact closure, and debt/waiver/reaffirmation no-laundering aggregation.
- [x] 4.2 Implement exhaustive reference-model runners that explore all reachable declared lifecycle, surface, policy, change-disposition, freshness, aggregation, and proof-invalidation states to a fixed point and emit model-scoped results.
- [x] 4.3 Pin offline-resolvable Verus, Z3, Charon, Aeneas, and candidate proof-backend toolchains with integrity digests.
- [x] 4.4 Prove equivalent nonproduction lifecycle, decision-surface-completeness, carrier-and-derivation-freshness, policy/debt/waiver/reaffirmation-no-laundering, change-impact-closure, and canonical-field-encoding spikes in Verus and AeneasVerif, or record a reviewed infeasibility blocker.
- [x] 4.5 Record an architecture decision selecting at least one primary formal lane and the Aeneas backend, with trust, assumptions, subset support, maintenance cost, and CI cost.

## 5. Implement the deterministic core

**Phase handoff:** Inherits frozen schemas, fixtures, models, and the selected formal lane; delivers the pure value model and deterministic integrity algorithms; freezes public core value and function contracts; explicitly excludes parsing, filesystem access, provider APIs, process execution, and human presentation; oracle: core unit, property, exhaustive-model, golden, mutation, bare-metal, and selected formal suites all pass.

- [x] 5.1 Implement atom, tombstone, decision-surface, carrier, implementation-link, derivation, coverage-policy, debt, waiver, oracle, property, model, assumption, proof-record, change-manifest, change-disposition, report, and `SpecGraph` value types against existing fixtures.
- [x] 5.2 Implement legal lifecycle transitions and permanent terminal ID reservation against the exhaustive reference model.
- [x] 5.3 Implement canonical encoding, algorithm-identified domain-separated SHA-256 digests, total ordering, path normalization, and duplicate rejection against golden and property tests.
- [x] 5.4 Implement graph closure, authority, decision-surface completeness, disposition authority, derivation binding, role, and source-map validation with deterministic structured diagnostics.
- [x] 5.5 Implement carrier, derivation, and proof freshness predicates; coverage-policy and adoption-ratchet evaluation; debt, waiver, and reaffirmation reporting; and conservative assurance aggregation against exhaustive and mutation tests.
- [x] 5.6 Implement deterministic base-to-head transitive impact analysis and complete reaffirmed/revised/retired/waived disposition closure without provider inputs.
- [x] 5.7 Verify the selected integrity-kernel functions and bind proof records to their exact executable implementations and assumptions.

## 6. Implement syntax adapters

**Phase handoff:** Inherits the frozen core API and parser fixtures; delivers schema-owned total Markdown and Rust adapters; freezes syntax attachment and disposition behavior; explicitly excludes OpenSpec parsing, raw-text authority, provider metadata, and independent semantic rules; oracle: adapter fixture, mutation, cross-order, unsupported-version, and root-independence suites all pass.

- [x] 6.1 Add Markdown parser fixtures and mutation tests for explicit atom and decision-surface boundaries, metadata, fences, nested constructs, malformed and unsupported syntax, explicit nonnormative dispositions, expected-empty roots, and Unicode spans.
- [x] 6.2 Implement total Markdown atom, decision-surface, policy, debt, waiver, derivation, and formal-property extraction without authoritative raw-text scanning or OpenSpec lifecycle semantics.
- [x] 6.3 Add Rust parser fixtures for eligible test and verifier items, exact evidence scopes, implementation and derivation links, attributes, comments, strings, macros, disabled code, file-level annotations, unsupported forms, and malformed carriers.
- [x] 6.4 Implement Rust carrier, implementation-link, governed-surface, and derivation extraction with syntax attachment, normalized spans, and explicit semantic or diagnostic disposition.
- [x] 6.5 Add cross-adapter graph tests proving extraction order, semantic parity, total disposition, and source-root independence.

## 7. Implement repository and oracle shells

**Phase handoff:** Inherits total adapters and the frozen core API; delivers snapshotting, provider-neutral static/change checks, reports, and effectful oracle orchestration; freezes CLI exit classes and machine-report schemas; explicitly excludes network-required canonical behavior and provider-specific semantics in the core; oracle: isolated repository and CLI integration suites prove offline, cross-root, change-impact, execution, and presentation behavior.

- [x] 7.1 Add isolated temporary-repository tests for explicit snapshot construction, path rejection, expected-authority roots, offline operation, and identical outputs from distinct roots.
- [x] 7.2 Implement filesystem snapshotting and configuration loading outside the deterministic core.
- [x] 7.3 Add CLI tests separating static checking, base-to-head change checking, and oracle execution, including exact exit classes, incomplete impact dispositions, policy ratchet failures, partial selection, runner crashes, timeouts, and noncanonical logs.
- [x] 7.4 Implement the authoritative repository-wide static and provider-neutral change CLI checks with deterministic machine and human reports for surfaces, policy, debt, waivers, derivations, and impacts.
- [x] 7.5 Implement explicit oracle and derivation-consistency orchestration with normalized execution receipts without allowing live results to mutate static graph identity.
- [x] 7.6 Add advisory migration reports for normative-looking unatomized text, uncovered atoms, undecided surface members, untracked intentional copies, active debt and waivers, and carrier counts before any policy scope becomes blocking.
- [x] 7.7 Add lint and LSP adapter parity tests before exposing either surface.

## 8. Complete assurance and release gates

**Phase handoff:** Inherits the complete repository-wide product and every prior frozen oracle; delivers release documentation, mutation resistance, adversarial review, and the single final gate; freezes published schema and report compatibility; explicitly excludes Chelis-specific semantics or integration code; oracle: the exact final `devenv test` invocation is green with every registered blocking obligation present, current, selected, and passing.

- [x] 8.1 Add deterministic golden regeneration policy, mutation testing, and deliberate faulty-implementation tests for lifecycle transition, surface completeness, carrier and derivation freshness, encoding, policy ratchets, impact closure, waiver/reaffirmation no-laundering, and aggregation logic.
- [x] 8.2 Document every accepted trust boundary, assumption, exclusion, manual prerequisite, evidence-class scope, hash-algorithm rationale, and OpenSpec reference-only boundary without claiming downstream backend proof.
- [x] 8.3 Document the engineering enforcement order, no-intentional-duplicate-without-a-derivation rule, phase-handoff protocol, and deferred-evidence ledger; push each enforceable rule to a type, lint, gate, or mutation-tested tripwire where practical.
- [x] 8.4 Extend `devenv test` to run OpenSpec planning validation, formatting, Clippy, Dylint architecture checks, the bare-metal `no_std` core build, all positive and negative suites, Hegel-rs properties, exhaustive models, decision-surface and adapter-totality checks, policy/debt/waiver and change-impact no-laundering tests, derivation checks, deterministic integration tests, and selected formal gates offline.
- [x] 8.5 Run an adversarial review against the specs, fixtures, implementation, reports, and exact final oracle; resolve every assurance-inflation finding.
- [x] 8.6 Run `devenv test` as the final acceptance oracle and require every registered blocking obligation to be present, current, selected, and green while advisory debt remains explicitly visible.
- [x] 8.7 Propose Chelis integration as a separate OpenSpec change only after this repository-independent system satisfies its final oracle.
