## Context

Commit `763d7f084aac` establishes the first complete repository-independent Buoy system and passes `devenv test`. A source-bound audit of that commit measured 75.29% line, 71.43% function, and 50.68% branch coverage across Cargo-instrumented production files, with the standalone fixture/model checker and Dylint implementation outside the measurement. It also established that the 112 fixture cases are checked as manifest records rather than executed operations, sampled Hegel and exhaustive-model suites do not compare their results with production, handcrafted mutants do not establish a production mutation score, and proof-source digests do not prove representation correspondence. The green oracle therefore establishes only its currently registered checks and does not erase known narrowing in the implementation: some v1 objects remain generic/minimal wrappers, report identity is not yet the frozen canonical binary format, some impact and policy facts are accepted from manifests rather than derived, repository artifact freshness is incomplete, glob/case-fold behavior is narrower than the frozen policy, process supervision is not production-grade, and clean-machine Cargo closure is not yet fully vendored or Nix-fixed.

This change is a hardening and schema-conformance migration, not a revision of v1 meaning. The completed `establish-spec-provenance-system` capability deltas are reconciled under `openspec/specs/` as the frozen v1 baseline. The seven hardening capabilities remain `ADDED` because they are orthogonal acceptance surfaces rather than reinterpretations of baseline requirements.

## Goals / Non-Goals

**Goals:**

- Implement every frozen v1 field, constructor constraint, reference kind, lifecycle rule, source-map value, report fact, and digest preimage exactly.
- Make canonical binary static, execution, and impact reports authoritative.
- Derive impacts, policy ratchets, disposition protocols, and invalidation from explicit graph pairs.
- Make adapters total over full v1 objects and selectors conform to pinned syntax/glob/Unicode rules.
- Make snapshot and oracle effects resistant to races, resource exhaustion, process-tree escape, deadlock, and unbounded output.
- Make fixture, property, exhaustive-model, mutation, and formal evidence behaviorally connected to the exact production implementation it supports.
- Enforce source-bound 100% line, region, function, and branch coverage over all in-scope production crates, standalone validators, and Dylint detectors without reporting structural coverage as proof.
- Maintain a complete assurance-obligation ledger with explicit point, property, finite-model, mutation, formal, platform, and exclusion disposition.
- Establish independent, adversarial, current-host, mutation, and supply-chain evidence through local commands.
- Preserve deterministic offline semantics, one-way `no_std` dependency, provider neutrality, exact evidence scopes, and no laundering.

**Non-Goals:**

- Changing v1 semantics to preserve existing shortcuts.
- Adding host-specific or Chelis-specific semantics.
- Formally proving parsers, operating systems, rustc, LLVM, native code, or external services without separately bound evidence.
- Treating fuzzing, test counts, mutation scores, dependency scans, or reproducible builds as mathematical proof.
- Making effectful logs or timing canonical.

## Decisions

### 0. Assurance closes through four gated phases

The hardening migration is organized into four monotonic acceptance phases:

- **P0 — Stop false greens:** freeze the source-bound red baseline and make every fixture operation and mutation executable. Compare Hegel properties and finite models with production. Inventory all coverable source and close all four coverage metrics to 100% on the current local host.
- **P1 — Complete pure-core assurance:** complete validated v1 values, canonical encode/decode, graph-derived governance, independent authorities, exhaustive bounded comparisons, automated mutation resistance, and actual production proof representations for the safe deterministic kernel.
- **P2 — Adversarial systems assurance:** complete adapters, path/glob/Unicode behavior, race-resistant snapshots, resource budgets, process supervision, fuzzing, fault injection, sanitizers, platform matrices, and clean offline supply-chain realization.
- **P3 — Close proof and obligation accounting:** bind every claimed theorem to exact implementation representations and assumptions, validate all evidence/campaign freshness, disposition every assurance obligation, and run the exact final oracle.

The focused `add-cargo-llvm-cov-hegel-rs-testing-requirements` change owns the detailed `LCOV-*` and `HEGEL-*` contracts for P0. Its completed coverage/property tasks map to hardening tasks 2.10, 2.19, 2.21, and 2.22 and their assurance-ledger entries exactly once; the mapping does not mark unfinished work complete and does not let one evidence result satisfy multiple ledger entries.

A phase handoff is valid only when its obligations are present, current, selected, and green. The local final command executes P0 through P3 directly. Separate provider records are optional exported attestations and are not acceptance prerequisites. Later evidence cannot satisfy an earlier missing obligation.

### 1. Schema-to-API parity is a generated blocking artifact

A reviewed machine-readable v1 schema inventory will enumerate every object, nested value, enum, required/optional field, set/sequence rule, digest domain/tag, reference kind, lifecycle state, and diagnostic family from `docs/schema/v1/`. A parity checker will compare that inventory with public Rust constructors, encoder registrations, fixture coverage, and report fields.

Every semantic type will use private fields and validated constructors. Construction either returns one complete valid value or deterministic structured errors; no public constructor may accept an arbitrary `ObjectMeta` with a mismatched kind. The core still validates cross-object facts after construction.

Alternative: retain generic wrappers and rely on graph validation. Rejected because incomplete values can cross API boundaries and make parity impossible to audit.

### 2. Canonical binary values gain a strict decoder and independent implementation

The existing encoder will be completed for every domain. A strict decoder will reject unknown tags, duplicate/out-of-order fields, omitted explicit options/empties, nonminimal or incorrect lengths, non-NFC text, invalid digests, duplicate set elements, and trailing bytes. Decode-encode round trips preserve exact bytes only for valid canonical inputs.

A second implementation with no production encoder/decoder dependencies will generate and verify golden vectors. JSON and human output are derived presentations bound to the canonical report digest; neither establishes identity.

Alternative: keep deterministic JSON authoritative. Rejected because it conflicts with the frozen v1 binary identity and expands map/number/escaping ambiguity.

### 3. Governance is derived from base/head graphs

Impact analysis will index every schema-declared edge: atoms, surfaces/domains/dispositions, carriers, implementation links, policies/obligations, debt, waivers, reaffirmations, assumptions, tools, oracles, derivations/artifacts, models, properties, proofs, and source maps. Direct change and transitive closure continue to a fixed point in canonical key order.

Policy weakening is computed by materializing base/head obligation sets. Manifests contain dispositions and repository-owned records, not self-asserted ratchet classifications. Reaffirmed/waived/retired/revised shapes validate exact records, approvals, replacements, and current digests.

Alternative: trust manifest policy summaries. Rejected because a producer could omit the very obligation being weakened.

### 4. Repository artifacts have explicit current snapshot bindings

Artifact freshness receives an explicit canonical snapshot/artifact index. Repository artifacts are never assumed current. Derivations and proofs compare every path/digest/representation binding, and unknown or unselected paths fail according to the containing schema.

Formal records additionally bind actual Verus, bundled Z3, Charon, Aeneas, Lean, wrapper/template, configuration, external-model, generated-artifact, proof-source, and negative-control identities as applicable. Manual representation correspondence remains an explicit reviewed assumption unless a generated shared representation proves stronger correspondence.

### 5. Adapters materialize complete semantic objects

Markdown fences for surfaces, policies, debt, waivers, derivations, assumptions, models, properties, reaffirmations, and proof records parse into complete core values or deterministic diagnostics. Rust carrier/link/surface/derivation metadata includes exact parsed-item identity, corpus/configuration/scope, assumptions, exclusions, and role-compatible oracle bindings.

The adapter support matrix is closed and versioned. Unknown fields, duplicate keys, unsupported item classes, file-level/unattached attributes, disabled code, macros, and malformed authority-like constructs remain fail-loud.

### 5a. Adapter SDK stays shell-side; consumer UX stays downstream

The imperative shell, or a separate `std` shell-side adapter SDK crate, may expose a versioned adapter contract over canonical saved-snapshot bytes and explicit configuration. Every adapter declares its parser/schema identity and supported syntax capabilities, and it emits complete validated core values with parser-backed source maps, explicit schema-defined nonnormative dispositions, or deterministic diagnostics. Adapter discovery, registration order, daemon state, editor state, unsaved buffers, dashboards, LSP, MCP, annotations, and watchers do not enter canonical identity; only the normalized materialized values produced from the saved snapshot may reach `buoy-core`.

The frozen parsed-item protocol currently closes `ItemKind` over `MarkdownFence` and `RustItem`. A host adapter MUST NOT encode Chelis or another language as one of those kinds merely to cross the boundary. Supporting a new canonical host item kind requires a separately approved schema version, exact reader/migration behavior, independent fixtures, and compatibility mapping; it MUST NOT be introduced as an unversioned core-enum extension. The production-hardening change stabilizes the shell-side contract and its totality rules without importing host syntax or developer-interface behavior into the core.

Alternative: expose parser callbacks or a dynamically extensible item-kind enum from `buoy-core`. Rejected because callable behavior violates the functional-core boundary, while host variants would couple canonical core evolution to downstream languages and tools.

### 6. Path selection uses pinned full Unicode and glob semantics

The shell will use a pinned full Unicode default case-fold implementation and conformance corpus rather than a handcrafted subset. The v1 glob grammar will be implemented as a closed parser and matcher with overlap detection. Selector identity includes the complete selector, not only root text.

Local acceptance checks portable behavior on the current supported host. Canonical outputs remain repository-relative. Noncurrent hosts receive explicit non-applicable dispositions and no conformance claim. Platform-specific inability to provide no-follow semantics fails closed.

### 7. Snapshot reads are descriptor-relative and budgeted

On supported Unix platforms the shell opens an explicit root descriptor and traverses with descriptor-relative no-follow operations. It validates the opened object rather than classifying a path and reopening it. Equivalent Windows APIs use reparse-point rejection and handle-relative checks where available.

Configuration supplies versioned semantic budgets for selected files, total bytes, individual file bytes, path depth, directory entries, object/reference counts, canonical nesting, diagnostics, oracle output, and execution concurrency. Exceeding a budget yields stable deterministic diagnostics before unbounded allocation.

Alternative: rely on test-repository size. Rejected because hostile or accidentally enormous repositories can exhaust memory or hang CI.

### 8. Oracle execution uses a process supervisor

The supervisor launches an explicit executable with empty/default-denied environment, fixed working root, bounded stdin/stdout/stderr, and configured semantic-result normalizer. It drains pipes concurrently, maps only schema-defined results to `pass`/`fail`, maps malformed results and abnormal exits to `error`, and enforces timeout.

Timeout/error cleanup terminates the complete process group/job object, waits for descendants, and records only normalized canonical receipt fields. Raw bounded logs remain noncanonical. Platform-specific process isolation is tested.

### 9. Adversarial evidence has independent authorities

The fixture runner executes each closed operation against its concrete baseline, applies every declared mutation to exactly one reviewed target, invokes the appropriate production and independent authorities, and compares actual outcome, diagnostic, fact, and canonical-byte results. Manifest shape, vocabulary membership, or baseline existence alone never satisfies a behavioral fixture obligation. Negative self-tests deliberately break each runner dispatch, mutation, expected-result, and independence path.

A reviewed source inventory covers the root and core crates, binaries, standalone validators, the local Dylint implementation, and current-host executable source. Pinned local coverage must report 100% lines, regions, functions, and branches. Unreachable source is removed or carries an exact proof-backed impossibility disposition. Broad exclusions and threshold rounding are forbidden. Coverage remains structural test evidence.

Every inventoried pure semantic production obligation has at least one Hegel differential property comparing production with a dependency-independent reference authority. Every exhaustive finite model compares production behavior at every reachable declared state/transition rather than only checking model-internal invariants. Coverage-guided fuzzers exercise canonical decoding/encoding, paths/globs, Markdown/TOML/Rust adapters, configuration/manifests, graph closure, and report rendering. Seed corpora are committed; minimized regressions become ordinary positive/negative fixtures.

Maintainers run `cargo-mutants` locally with an explicit baseline, exclusions, timeout, and minimum score. Named handcrafted mutants remain mandatory in the local final command. Current campaign artifacts bind the exact source, tool, and configuration. Independent models cannot call corresponding production code.

Miri and sanitizers are scoped to supported targets. Fault injection covers I/O failure, partial reads, permissions, link swaps, process crashes, full pipes, runaway descendants, malformed results, and budget boundaries.

### 10. Offline closure is realized, not assumed

Rust dependencies are vendored with checksums or built from Nix fixed-output sources; formal archives and every backend/model/template input remain hash pinned. `cargo-deny` and/or `cargo-vet` enforce advisory, license, duplicate-version, source-origin, and review policy.

A local clean sandbox test disables the network and uses empty tool homes. It realizes the declared closure and runs the exact acceptance oracle.

### 11. Formal proof claims remain narrow and obligation-accounted

Verus remains primary. Production proof drift checks bind exact executable source and all proof/tool/configuration artifacts. A theorem over a detached Boolean or mathematical helper is accepted for production only when a generated/shared representation or explicit consistency derivation establishes the declared correspondence; hashing a production file detects drift but does not prove correspondence. Aeneas/Lean remains a translation sentinel unless a separately complete production record is approved. Neither lane proves parser/shell effects, tool soundness, representation correspondence, rustc/LLVM, or native code without additional evidence.

A machine-readable assurance-obligation ledger maps every frozen requirement, scenario, schema inventory row, source target, finite model, mutation class, platform backend, and selected proof theorem to exact evidence or an explicit scoped exclusion/blocker. The ledger distinguishes `not-applicable`, `unsupported`, `blocked`, and `satisfied`; it cannot infer proof from coverage, tests from annotations, production conformance from a reference-only model, or correctness from a green aggregate command.

### 12. Exact acceptance oracle

The exact final command remains:

```text
devenv test
```

The local command is green only when P0 through P3 execute in order and every blocking ledger entry is current. Local coverage must reach exactly 100% for all four metrics. The command also checks fixtures, models, proofs, bounded fuzz replay, named mutants, current campaign artifacts, current-host behavior, and offline closure. Full fuzz and mutation campaigns run locally through focused commands. CI is not an assurance authority.

## Risks / Trade-offs

- **Public API migration breaks callers** → provide an explicit conversion layer, compile-fail migration fixtures, and one documented compatibility window; never weaken v1 constructors.
- **Independent implementations correlate through shared dependencies** → forbid production encoder/path/impact modules in reference crates and audit dependency graphs.
- **Coverage closure is gamed with exclusions or assertion-free calls** → freeze the source inventory, forbid broad exclusions/rounding, require branch/function/region/line equality, and retain mutation/property/model obligations independently.
- **Fixture metadata is mistaken for behavioral execution** → require dispatch/mutation/result witnesses and negative self-tests that fail when any operation is a no-op or expected output is ignored.
- **Descriptor-relative APIs differ by platform** → isolate shell backends, share only normalized observations, and fail closed where equivalent safety is unavailable.
- **Fuzzing and mutation use substantial resources** → run campaigns locally, pin seeds and tools, and retain minimized failures.
- **Resource budgets reject legitimate large repositories** → make budgets explicit versioned configuration inputs with safe upper bounds and deterministic migration reports.
- **Vendoring increases repository/closure size** → prefer Nix fixed-output sources where reviewability is equivalent; record closure size and update protocol.
- **Formal bindings churn after refactors** → keep the verified kernel small and require proof-record migration rather than weakening freshness.

## Migration Plan

1. **P0 — Stop false greens:** reconcile the baseline; freeze the schema/source/assurance inventories and red audit at `763d7f084aac`; implement executable fixture operations and exact mutation dispatch; add runner negative self-tests; connect Hegel and exhaustive models to production; instrument all source targets; and close line, region, function, and branch coverage to 100%.
2. **P1 — Complete pure-core assurance:** add independent canonical/reference implementations and malformed vectors before rewrites; introduce complete private-field values and strict encode/decode; derive governance from graph pairs; run bounded production/reference properties and models; establish automated mutation resistance; and rebind safe-kernel proofs to actual production representations.
3. **P2 — Adversarial systems assurance:** complete adapters and bounded effect interfaces. Activate local fault, fuzz, Miri, sanitizer, compile-fail, and current-host suites. Then prove clean network-disabled acceptance.
4. **P3 — Close proof and obligation accounting:** complete the assurance ledger, proof correspondence and negative controls, campaign freshness, documentation, and adversarial review; resolve every unsupported blocking entry; run all focused gates; then run exact `devenv test`.

Rollback before publishing hardened APIs restores the prior implementation while retaining new red fixtures and parity inventory. After publishing canonical report/schema compatibility, rollback must retain readers and migration mappings; it cannot reinterpret v1 bytes or IDs.

## Open Questions

- Which machine-readable schema description is simplest to review while remaining independent from Rust code generation?
- Which no-follow/handle-relative Windows API baseline is required for the first production platform claim?
- What initial semantic resource budgets are conservative enough for intended repositories?
- What mutation threshold and local fuzz duration become blocking without encouraging superficial exclusions?
- Should Cargo sources be committed under `vendor/` or represented exclusively as Nix fixed-output derivations?
