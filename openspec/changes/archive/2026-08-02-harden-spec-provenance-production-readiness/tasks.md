## 1. P0 — Reconcile baseline and freeze truthful assurance contracts

- [x] 1.1 Archive or reconcile `establish-spec-provenance-system` into baseline capabilities and update this change's delta operation classification without changing frozen v1 meanings.
- [x] 1.2 Record the reviewed red baseline tied to commit `763d7f084aac`, including 75.29% line, 71.43% function, and 50.68% branch coverage for Cargo-instrumented production files; enumerate the standalone fixture/model checker and Dylint implementation as unmeasured rather than silently excluded.
- [x] 1.3 Define the machine-readable frozen-v1 inventory for every object, nested value, enum/field tag, collection rule, domain, reference, lifecycle, report fact, and diagnostic family.
- [x] 1.4 Implement an inventory self-check that rejects duplicate, missing, unknown, and internally inconsistent schema entries before reading production code.
- [x] 1.5 Freeze public-API migration rules, compatibility window, report presentation versioning, and historical reader requirements.
- [x] 1.6 Freeze deterministic resource-budget fields, units, upper bounds, diagnostics, and whether each budget affects semantic configuration identity.
- [x] 1.7 Select and pin full Unicode normalization/case-fold data, versioned glob grammar implementation strategy, descriptor/handle APIs, process supervisor dependencies, coverage/fuzz/mutation tools, and supply-chain policy tools.
- [x] 1.8 Define deterministic focused commands and blocking handoffs for P0 false-green closure, P1 pure-core assurance, P2 adversarial systems assurance, P3 proof/obligation closure, and exact final acceptance.
- [x] 1.9 Freeze a source inventory covering every production library, binary, supported-platform backend, standalone validator, local Dylint detector, and generated executable source target, with exact inclusion/exclusion policy.
- [x] 1.10 Freeze a machine-readable assurance-obligation ledger schema spanning requirements/scenarios, schema rows, source targets, fixture operations/mutations, properties, finite models, mutation classes, platforms, proofs, and scoped exclusions/blockers.

## 2. P0 — Make fixtures, properties, models, and coverage behaviorally truthful

Focused ownership: `add-cargo-llvm-cov-hegel-rs-testing-requirements` supplies the detailed `LCOV-*`/`HEGEL-*` work for tasks 2.10, 2.19, 2.21, and 2.22. Each completed focused obligation maps to one P0 ledger entry; this note does not mark any task complete or permit duplicate evidence accounting.

- [x] 2.1 Add positive and negative schema-construction fixtures for every inventory field, explicit empty/option form, enum, reference kind, metadata-kind mismatch, and lifecycle terminal record.
- [x] 2.2 Add ID-registry, namespace allocation, parsed-item, source-map, approval, terminal policy/debt/waiver, and artifact-index base/head fixtures.
- [x] 2.3 Add canonical binary positive vectors for every type tag, nested value, semantic domain, and complete static/execution/impact report.
- [x] 2.4 Add malformed binary vectors for unknown tags, lengths/overflow, field order/duplicates/omissions, set order/duplicates, NFC, options, digests, trailing bytes, and every budget boundary.
- [x] 2.5 Implement an independent canonical encoder/decoder with a dependency check proving it cannot import production canonical modules.
- [x] 2.6 Add graph-pair fixtures for every schema-declared impact edge and dependency chains longer than one hop.
- [x] 2.7 Add policy materialization/ratchet fixtures for scope, role, mode, authority, derivation, surface evolution, waiver-class, lineage, supersession, and retirement changes.
- [x] 2.8 Add exact reaffirmed/revised/retired/waived disposition fixtures mutating approvals, records, replacements, base/head digests, applicability, debt, and waivers independently.
- [x] 2.9 Extend independent finite models for complete impact closure, policy weakening protocol, governance no laundering, terminal history, artifact freshness, and proof invalidation to fixed points.
- [x] 2.10 Add deterministic Hegel-rs properties comparing production-independent reference values for schema construction, canonical round trips, impact/ratchet closure, and no laundering under committed seeds/settings.
- [x] 2.11 Add complete Markdown object fixtures and Rust attachment fixtures for every supported v1 object/item/field, including omissions, duplicates, unknowns, versions, macros, disabled code, Unicode, and CRLF.
- [x] 2.12 Add portable glob, selector-overlap, Unicode conformance, Windows reserved/reparse, Unix link-swap, resource-budget, partial-I/O, and deep-tree fixtures.
- [x] 2.13 Add oracle fixtures for valid pass/fail protocols, malformed zero-exit results, abnormal exits, saturated pipes, output overflow, timeout descendants, cancellation, and cleanup.
- [x] 2.14 Build a fail-closed hardening manifest mapping every new requirement scenario to fixture polarity, independent oracle, stable diagnostic/verdict, focused command, and final oracle.
- [x] 2.15 Implement every closed fixture operation as an executable dispatch over concrete baselines, emit one exact source/tool/configuration/result-bound record per operation identity, and reject an operation that has no behavioral implementation or separate result.
- [x] 2.16 Implement every declared fixture mutation with ordered application to exactly one reviewed target, emit one target-cardinality/result-bound witness per mutation identity, and reject zero-target, multiple-target, no-op, or aggregated-only mutations.
- [x] 2.17 Compare each fixture's actual acceptance, stable diagnostics, facts, and canonical bytes with its frozen expected result across required production and independent authorities; materialize separate scenario and requirement results bound to their exact constituent case results.
- [x] 2.18 Add fixture-runner and ledger negative self-tests for unknown/no-op dispatch, ignored expected fields, mutation target cardinality, production/reference substitution, stale source/tool/configuration/result bindings, duplicate/orphan evidence, and stale manifest acceptance.
- [x] 2.19 Build the complete pure-production property inventory and add seeded Hegel-rs differential/metamorphic tests that call both production and dependency-independent reference authorities; retain one separately materialized expected-failure result for the intentionally-false control.
- [x] 2.20 Extend each exhaustive finite-model runner to evaluate the corresponding production function at every reachable declared state/transition, compare exact results, and emit one separately materialized production-conformance record per model.
- [x] 2.21 Instrument the complete local source inventory, including standalone validators and Dylint fixtures and detectors, without threshold rounding or broad exclusions.
- [x] 2.22 Add assertion-quality checks and close source-bound line, region, function, and branch coverage to exactly 100%, removing unreachable code or recording exact proof-backed impossibility dispositions.

## 3. P1 — Complete schema-conformant pure-core values

- [x] 3.1 Replace caller-supplied generic metadata construction with private fields and validated kind-specific constructors.
- [x] 3.2 Implement complete v1 ID registry, namespace declarations, allocation ordinals/classes, and permanent live/terminal reservation values.
- [x] 3.3 Implement complete atom, tombstone, decision-surface domain/surface/disposition, parsed-item, and source-map value surfaces.
- [x] 3.4 Implement complete carrier, implementation-link, artifact-binding, derivation, oracle, normalized-verdict, assumption, finite-model, formal-property, and proof-record values.
- [x] 3.5 Implement complete coverage-policy, obligation, debt, waiver, reaffirmation, change-manifest, and change-disposition values including irreversible terminal evidence/replacements/approvals.
- [x] 3.6 Implement constructor-level grammar, NFC, duplicate-set, role/scope, option/default, kind, lifecycle-shape, and local invariant diagnostics.
- [x] 3.7 Implement graph-level registry, authority, source-map, reference closure, lifecycle history, role compatibility, and host-schema validation against the independent fixtures.
- [x] 3.8 Implement explicit snapshot artifact indexes and replace every assumed-current repository-artifact path with exact freshness comparison.
- [x] 3.9 Add compile-fail tests preventing public direct-field construction, metadata-kind substitution, and forbidden callable/effectful core interfaces.
- [x] 3.10 Make the schema-to-API parity checker block on every omitted/mismatched type, field, encoder, validator, report fact, or fixture row.

## 4. P1 — Implement canonical binary reports and derived governance

- [x] 4.1 Complete canonical encoding for every v1 nested value, object domain, source map, graph, verdict set, change value, and report fact.
- [x] 4.2 Implement the strict budgeted canonical decoder and pass every valid/malformed independent vector.
- [x] 4.3 Implement decode-encode exact round trips and production-versus-independent differential generation across bounded randomized values.
- [x] 4.4 Implement complete orthogonal static, execution, and impact fact values and authoritative v1 binary report bytes.
- [x] 4.5 Version JSON/human rendering as derived presentation bound to the authoritative binary report digest and add deterministic goldens.
- [x] 4.6 Implement complete schema-edge indexing and direct/transitive base-to-head impact fixed-point analysis.
- [x] 4.7 Derive policy obligation sets and ratchet classification from base/head graphs, removing authoritative manifest-supplied ratchet summaries.
- [x] 4.8 Implement complete disposition record/approval/replacement/applicability validation and weakening protocol closure.
- [x] 4.9 Implement terminal policy/debt/waiver history, artifact/proof invalidation, and reaffirmation/waiver no-laundering report facts.
- [x] 4.10 Rebind selected Verus proof records to exact implementation, verifier, bundled solver, wrapper/configuration, assumptions, negative controls, scope, and exclusions; add an explicit derivation for any duplicated proof representation.

## 5. P2 — Complete adapters, selectors, and adversarial repository snapshots

- [x] 5.1 Implement complete strict Markdown materialization for every supported v1 fenced object with parser-backed source maps.
- [x] 5.2 Implement complete Rust carrier/link/surface/derivation/verifier materialization with exact parsed-item identities, scopes, corpora, assumptions, and exclusions.
- [x] 5.3 Enforce the closed adapter support matrix and total semantic/nonnormative/diagnostic disposition for every selected syntax node/class.
- [x] 5.4 Implement the versioned portable glob parser/matcher and complete selector identity/overlap checks without host glob or ignore semantics.
- [x] 5.5 Implement pinned full Unicode default case-fold collision keys and pass the reviewed conformance corpus.
- [x] 5.6 Replace recursive path traversal with budgeted iterative traversal and content-digest duplicate indexing.
- [x] 5.7 Implement Unix descriptor-relative no-follow snapshotting and link-swap/partial-I/O fault tests.
- [x] 5.8 Implement the Windows handle-relative/reparse-safe backend or record a reviewed production-platform blocker; run reserved/collision/reparse fixtures.
- [x] 5.9 Implement all snapshot/parser/graph/encoding diagnostic and object/reference budgets before unbounded work.
- [x] 5.10 Add cross-adapter, cross-order, cross-root, locale/timezone, and supported-platform canonical binary graph/report equality tests.
- [x] 5.11 Implement the versioned shell-side adapter SDK/registry with immutable adapter and parser/schema identities, supported syntax capabilities, exact configuration bindings, canonical saved-snapshot inputs, and total materialized outputs without adding parser callbacks or host dependencies to `buoy-core`.
- [x] 5.12 Add positive and negative adapter-SDK controls for cross-root and registration-order equality, ambient daemon/editor/unsaved-state exclusion, unsupported capabilities, duplicate identities, incomplete disposition, and attempted core coupling.
- [x] 5.13 Add a blocking parsed-item compatibility control that rejects a downstream host item encoded as frozen `MarkdownFence` or `RustItem` without a separately approved canonical schema version, reader/migration behavior, and compatibility mapping.

## 6. P2 — Harden effectful oracle execution

- [x] 6.1 Implement explicit process protocol/configuration values for executable, arguments, environment allowlist, working root, input, result normalizer, budgets, concurrency, and timeout.
- [x] 6.2 Implement concurrent bounded stdout/stderr draining and deterministic overflow normalization without pipe deadlock.
- [x] 6.3 Implement Unix process-group creation, complete descendant termination/reaping, timeout, cancellation, and saturated-pipe tests.
- [x] 6.4 Implement Windows job-object lifecycle with complete descendant termination/reaping or record a reviewed production-platform blocker.
- [x] 6.5 Implement schema-bound semantic result normalization so zero exit with malformed/missing results becomes `error` and valid domain failure becomes `fail`.
- [x] 6.6 Ensure raw bounded logs, elapsed time, PIDs, temporary paths, scheduling, and completion order remain outside canonical receipts/reports.
- [x] 6.7 Add deterministic execution report permutation tests and prove every execution outcome leaves static graph/report identity unchanged.

## 7. P2 — Add fuzz, mutation, fault, sanitizer, and platform gates

- [x] 7.1 Add pinned coverage-guided fuzz targets and committed seed corpora for canonical values, paths/globs, adapters, config/manifests, graphs/governance, and reports.
- [x] 7.2 Run initial bounded fuzz campaigns, minimize every finding, promote regressions to frozen fixtures, and record source/tool/configuration-bound campaign artifacts.
- [x] 7.3 Configure pinned `cargo-mutants` baseline, exclusions, timeout, and minimum score without excluding named high-consequence paths.
- [x] 7.4 Add named mutants for artifact freshness, strict decoding, complete impacts, policy derivation, process cleanup, output/budget enforcement, and retain all existing tripwires.
- [x] 7.5 Run the initial mutation campaign, kill every non-equivalent mutant, give every equivalent/unsupported survivor an exact reviewed nonpassing disposition, and freeze the current source/tool/configuration-bound artifact.
- [x] 7.6 Add Miri, applicable sanitizer, and compile-fail focused commands with explicit assurance scope and supported-platform declarations.
- [x] 7.7 Add deterministic fault-injection infrastructure for filesystem, parser, allocation budget, process, pipe, result, timeout, and cancellation failures.
- [x] 7.8 Add a local current-host command that repeats the portable canonical corpus and emits stable platform-effect diagnostics.
- [x] 7.9 Add campaign-artifact freshness/expiry/discharge validation so stale fuzz/mutation evidence cannot satisfy current obligations.

## 8. P2 — Close the reproducible supply chain

- [x] 8.1 Choose reviewed Cargo vendoring or Nix fixed-output packaging and materialize every Rust source/checksum required by clean offline builds.
- [x] 8.2 Pin every fuzz, mutation, Miri, sanitizer, parser, Unicode, formal, solver, backend, model, template, and wrapper input with verified content identity.
- [x] 8.3 Add deterministic closure inventory generation distinguishing artifact role, platform, source, digest, license, trust scope, and exclusions.
- [x] 8.4 Configure pinned advisory/license/source/yank/duplicate-version policy with exact repository-owned exception records and negative fixtures.
- [x] 8.5 Add dependency/tool update protocol tests proving parser/Unicode/proof/campaign/report bindings become stale when relevant inputs change.
- [x] 8.6 Add clean sandbox acceptance with network disabled, empty user tool homes/caches, controlled environment, and no undeclared global executables.
- [x] 8.7 Add closure-size policy and noncanonical realization/build-cost reporting without changing semantic graph identity or assurance classes.

## 9. P3 — Close proof, obligation, and final production-readiness gates

- [x] 9.1 Materialize the assurance-obligation ledger and require exact current evidence or explicit `not-applicable`, `unsupported`, or `blocked` disposition for every inventoried entry without cross-class inference.
- [x] 9.2 Validate the local P0 handoff: every fixture and differential is current, every local target is instrumented, and all four coverage metrics equal 100%.
- [x] 9.3 Validate the P1 handoff: complete v1 core parity, strict canonical round trips, graph-derived governance, automated mutation resistance, and selected safe-kernel production proof bindings all pass their independent authorities.
- [x] 9.4 Validate the local P2 handoff: adapters, fault suites, fuzz artifacts, local tools, current-host observations, dependency policy, and offline closure are current and green.
- [x] 9.5 Replace detached production proof models with shared/generated production representations or explicit consistency derivations and correspondence approvals; retain exact blockers/exclusions where stronger correspondence is unavailable.
- [x] 9.6 Add proof-ledger negative controls showing that source digests establish drift detection only and cannot independently emit `formally-proved`.
- [x] 9.7 Update trust, assumption, exclusion, schema/report compatibility, resource budget, platform support, process isolation, campaign, coverage, property/model, formal, and supply-chain documentation.
- [x] 9.8 Update golden regeneration and engineering enforcement docs for executable fixture truthfulness, strict decoding, parity, complete source coverage, independent authorities, fuzz/mutation artifacts, proof accounting, and dependency updates.
- [x] 9.9 Register all focused phase-handoff, parity, fixture, property, exhaustive-model, coverage, canonical, governance, adapter, repository/process, adversarial, formal, and supply-chain commands in `devenv test` without counting duplicate nested execution as independent evidence.
- [x] 9.10 Run an adversarial review against every hardening requirement, frozen fixture/vector, independent model, production implementation, source target, report, proof binding, campaign artifact, ledger entry, and exact final command.
- [x] 9.11 Resolve every assurance-inflation, false-green, nondeterminism, uncovered-source, race, resource-exhaustion, process-leak, schema-parity, proof-correspondence, and offline-closure review finding.
- [x] 9.12 Run pinned fuzz and mutation campaigns locally and require current source-bound artifacts with no unresolved blocking findings.
- [x] 9.13 Run local `devenv test` as the exact final acceptance oracle. Require direct P0-P3 execution and a green ledger while advisory debt remains visible.
