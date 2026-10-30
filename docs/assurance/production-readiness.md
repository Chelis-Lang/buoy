# Production-readiness assurance

Status: P3 obligation-accounting contract. This document records scope and current blockers; it is not a production-readiness verdict.

## Exact accounting authority

`assurance/v1/obligation-ledger.tsv` is the materialized fail-closed ledger. Its stable identity is `(category, id)`, ordered by category then ID. `devenv tasks run buoy:obligations` discovers every frozen baseline/hardening requirement and scenario, schema row, source target, fixture operation and mutation, property, finite model, mutation class, platform backend, selected theorem, supply-chain input, and phase handoff. The checker rejects omissions, duplicates, stale source/tool/configuration/result bindings, missing evidence paths, invalid dispositions, cross-class promotion, and aggregate-command substitution.

Locally actionable fixture operations, fixture mutations, scenarios, requirements, finite-model production-conformance cases, and the intentionally-false Hegel control use the closed `buoy-local-execution-evidence/v1` record. Every `(category, id)` has one normalized result and exact source, tool, configuration, and result digests plus `execution = pass`. The ledger recomputes all four bindings and rejects stale records, duplicate identities, missing source rows, and orphan records. Manifest presence, one aggregate green command, a shared case observation, or one observation containing multiple mutation names cannot satisfy any member identity. Regeneration is explicit through each owning authority's reviewed write flag; ordinary validation only compares committed bytes.

A row is one of `satisfied`, `not-applicable`, `unsupported`, or `blocked`. Nonpassing rows require an exact blocker and repository approval authority. Ledger closure means every identity is dispositioned; it does **not** mean every row is green. `--ledger-green` separately rejects every `blocked` or `unsupported` row and is used only by P3/final acceptance.

## Trust, assumptions, and exclusions

- Repository-owned inventories, policies, approval records, and canonical saved inputs are trusted as the configured authorities, not as proof that their prose is true.
- OpenSpec remains planning/reference-only. Strict validation cannot satisfy product, fixture, coverage, campaign, platform, or proof obligations.
- Evidence classes are orthogonal. Point fixtures, sampled properties, finite models, coverage, mutation, fuzzing, fault injection, platform runs, formal proof, supply-chain policy, and review cannot fill one another's ledger rows.
- Digests bind exact observed content and detect drift. Each result cites the assurance class of its active member.
- A collision-resistant member does not establish authenticity, authorization, tamper evidence, or proof.
- The correspondence index is derived evidence. Canonical identity, reports, and freshness checks do not read it.
- Raw logs, clocks, durations, host paths, PIDs, process order, provider state, and unsaved editor state remain noncanonical.

## Schema and report compatibility

Frozen v1 and v2 meanings remain immutable. Active v3 rows match constructors, encoders, validators, fixtures, and report facts.

Canonical binary static, execution, and impact reports remain authoritative. JSON and text outputs are derived presentations that bind the active member.

The v3 registry contains `xxh3-128`, `blake3-256`, and `sha-256`. Configuration selects exactly one available member. A selection change regenerates all identities from authoritative source bytes.

## Resource and process boundaries

Versioned semantic budgets cover path depth, directory entries, selected files, per-file and total bytes, object/reference counts, canonical nesting/counts, diagnostics, oracle output, concurrency, and timeout. Budget failures occur before unbounded retention and use stable diagnostics.

Unix snapshotting uses descriptor-relative no-follow operations. Oracle execution uses bounded concurrent stdout/stderr drainage, process-group termination/reaping, default-denied environment, fixed roots, explicit result normalization, and deterministic timeout/error categories. Windows is outside the supported-platform scope declared in `assurance/v1/platform-blockers.toml`. The handle-relative snapshot and Job Object backends do not exist. Production code fails closed on Windows, and Unix success grants no Windows evidence.

## Coverage, properties, and finite models

Maintainers must explicitly select `devenv tasks run buoy:coverage`. The task requires exact integer line, region, function, and branch equality for every in-scope source target.

Automatic acceptance checks the committed result against current sources. Threshold rounding, broad exclusions, zero denominators, missing validators, and stale artifacts fail. Coverage is structural source-execution evidence only.

`devenv tasks run buoy:properties` binds each sampled Hegel result to production/reference source, tool, settings, domain, seed, replay, assertions, and current registration. The intentionally-false control additionally owns a separate expected-failure execution record; a property-run inventory row cannot substitute for it. Reference-only helpers remain reference-only. Every production-conformance finite model owns one separately materialized result after exhaustive production/reference comparison over its declared bound. Missing or stale per-model results remain explicit ledger blockers rather than inferred success.

## Campaign and platform evidence

The deterministic final path replays committed fuzz seeds, named mutation tripwires, fault suites, local tool scopes, current-host output, and campaign freshness.

Maintainers must explicitly select full fuzz and mutation campaigns. Their artifacts bind exact source, tool, configuration, bounds, results, expiry, and survivor dispositions.

Platform ledger rows require a normalized artifact for the current local host. The artifact must match the current generator and scoped effect diagnostics. Noncurrent hosts receive explicit non-applicable dispositions. Local evidence makes no cross-host conformance claim. Windows production backends remain outside the supported scope.

The repository has no GitHub Actions workflow definitions.
A missing or stale current-host result remains blocked.
Its ledger row cites `assurance/v1/phase-handoffs.toml` as the local policy authority, not as proof of host execution.

## Formal correspondence and proof accounting

`proofs/core/derivation-record.toml` maps named production symbols to named Verus specification/theorem symbols. `proofs/core/correspondence-approval.toml` approves only lifecycle, surface-completeness, freshness, policy-monotonicity, impact-closure, and no-laundering mappings under stated assumptions. The accepted proof, intended negative rejection, implementation/proof/configuration/tool bindings, derivation, and approval are all required before those exact theorem rows may say `formal-proof`/`satisfied`.

The detached canonical field-identity theorem had no shared/generated encoder representation or approved production mapping. It was removed from the selected production theorem inventory rather than being relabeled satisfied or not-applicable; its mathematical helper remains explicitly unselected and matching source digests cannot upgrade it. Verus/Z3 soundness, rustc, LLVM, native code, parsers, filesystems, processes, operating systems, and natural-language entailment remain excluded.

## Supply-chain scope

`devenv.lock` supplies the authoritative Nix revisions and content hashes. `Cargo.lock` supplies the authoritative package versions, sources, and checksums.

Each specialized Devenv script declares its runtime packages. The workflow checker rejects an absent runtime package binding.

`buoy:deny` runs `cargo-deny` offline with the pinned advisory database, registry index, and repository policy. Wildcard advisory exemptions fail the negative control.

Cargo-deny receives configuration and metadata before its `check` subcommand. The offline flag excludes dependency and advisory fetches.

`buoy:offline-acceptance` uses a Nix build sandbox with empty Cargo and Rustup homes. The sandbox excludes developer caches, ambient tools, and network access.

`buoy:closure-size` reports deterministic `nix path-info` sizes and enforces the repository size policy. Closure size does not alter semantic or assurance identity.

## Non-duplicative P0–P3 execution

The exact final oracle is `devenv test`. The resolved task graph runs P0 through P3 once, in order.

Each phase task owns its direct children. The child tasks form one serial chain.

Phase scripts do not run children. They report completion after their dependencies succeed.

`buoy-gate-wiring` parses `DEVENV_TASK_FILE`. Its negative controls cover omitted phases, direct sink children, nested phases, and duplicate owners.

Separate phase-result records remain optional export artifacts. They do not close ledger rows.

Static registration does not establish a child result. Unresolved coverage, host evidence, campaign freshness, or ledger obligations block acceptance.
