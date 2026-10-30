## Context

The current repository pins `cargo-llvm-cov` 0.8.5 and `hegeltest` 0.28.0, exposes focused coverage and property tasks, maintains a reviewed source inventory, and has seeded Hegel tests. It deliberately serves as a dogfood target for `cargo-llvm-cov`, pinned local Dylint rules, and the functional-core/imperative-shell (FCIS) split by applying those controls to its own complete Rust source inventory and actual crate boundary. The current contracts are nevertheless split among `devenv.nix`, tooling notes, assurance records, and a broader production-hardening change. In particular, a green Cargo coverage export can omit non-workspace executable sources, Hegel can be invoked with ambient defaults, and the aggregate acceptance path can omit an expensive lane without invalidating its historical evidence.

This change gives the two lanes focused, testable contracts. It is the bounded owner of the detailed `LCOV-*` and `HEGEL-*` portion of production-hardening P0: completed obligations map once to hardening tasks 2.10, 2.19, 2.21, and 2.22 without marking unfinished hardening work complete or duplicating evidence. The baseline deterministic-artifacts capability remains the owner of the FCIS architecture contract and its Dylint enforcement; this change records their self-application and requires their executable sources to remain visible to coverage. Maintainers and CI are the direct consumers. The deterministic static checker consumes repository-owned manifests and bindings; the imperative test shell runs instrumented programs and sampled properties. Raw LLVM/Hegel logs remain noncanonical observations.

The functional boundary does not change: `buoy-core` remains safe, sequential, pure, and `no_std`; the root/test shell may depend inward on the core; the core MUST NOT depend on the root crate, Hegel, coverage tooling, operating-system APIs, or test-only reference code. The repository dogfoods that split through its existing pinned Dylint FCIS rules and bare-metal build, whose structural verdict remains distinct from coverage evidence. `hegeltest` remains a development dependency, while `cargo-llvm-cov` remains an external pinned shell tool.

## Goals / Non-Goals

**Goals:**

- Make source-scope completeness and exact LLVM metric closure explicit and fail closed.
- Keep repository dogfooding honest by rejecting self-exemptions for the core, imperative shell, local Dylint detectors, and FCIS fixtures while preserving separate coverage and structural verdicts.
- Make every authoritative Hegel run reproducible from committed settings and identifiable generator domains.
- Require production/reference conformance rather than allowing reference-only helpers to satisfy production obligations.
- Bind normalized results to source, tool, configuration, platform, and reference identities and invalidate them on change.
- Put both effectful lanes behind one exact offline acceptance oracle while retaining focused commands.

**Non-Goals:**

- No production API, schema, canonical graph, or assurance-class change.
- No claim that 100% source coverage proves behavior, requirements, or absence of defects.
- No claim that 256 sampled Hegel cases are exhaustive or formally verified.
- No replacement of independent finite models, mutation testing, Verus, or AeneasVerif.
- No use of coverage or property evidence as proof of natural-language-to-formal correspondence, compiler correctness, operating-system correctness, or cryptographic collision resistance.

## Decisions

### 1. Keep static registration separate from effectful execution

Repository-owned validators statically check source/property inventories, tool identities, configuration completeness, digest bindings, and normalized result shape. Separate shell tasks execute `cargo llvm-cov` and Hegel. A static registration check MUST NOT manufacture a passing execution verdict, and an execution log MUST NOT repair an absent or stale registration.

Each obligation has independent state:

1. registration is absent or present;
2. freshness is current or stale against authoritative input digests;
3. execution verdict is unexecuted, pass, or fail; and
4. assurance class is computed separately without promoting coverage to exhaustive/formal or sampled properties to exhaustive/formal.

Any authoritative-input change transitions a current result to stale. A stale pass remains historical but cannot satisfy acceptance. Only a new run under the exact current closure can restore a current pass; approval, waiver, or another evidence class cannot do so.

Alternative considered: infer current state directly from the latest log. Rejected because logs may contain host paths, timings, incomplete scope, or settings not represented in repository state.

### 2. Use explicit authoritative inputs and deterministic normalization

Coverage authority consists of the reviewed source inventory, supported-platform policy, pinned Rust/LLVM and `cargo-llvm-cov` identities, Cargo/Nix locks, feature/target command configuration, and source bytes. Property authority consists of the property inventory, exact Hegel package closure, Rust toolchain, committed runner settings, generator-domain declarations, production source, dependency-independent reference source, and replay fixtures.

Normalized records bind raw SHA-256 content digests for every applicable input. Entries are ordered first by stable target/property ID and then by normalized repository-relative UTF-8 path bytes. Duplicate IDs, duplicate paths, unknown classes, absolute paths, host store paths, timestamps, locale text, durations, traversal order, and unnormalized tool output are rejected or omitted from deterministic records. Raw LLVM exports, Hegel shrink traces, and timing logs may be retained as noncanonical diagnostics.

Alternative considered: bind only `Cargo.lock` and a top-level test command. Rejected because source scope, settings, references, and platform aggregation can change independently.

### 3. Make all in-scope executable Rust sources LLVM-instrumentable

`cargo-llvm-cov` 0.8.5 from the pinned devenv closure is the coverage command authority. Every in-scope production library, binary, platform backend, maintainer generator, independent test authority, standalone validator, and local Dylint detector is listed with an instrumentation class and platform scope. Cargo workspace targets and examples run in the workspace lane; separate Cargo manifests run under the same pinned `cargo-llvm-cov`/Rust/LLVM closure. Standalone `rustc` validators are migrated to Cargo-reachable binary or harness targets rather than silently excluded. Generated data or declaration-only modules may be `not-applicable` only after a machine-checked no-executable-region disposition.

This complete self-application is the coverage dogfood loop: Buoy's own core, shell, validators, local Dylint detectors, and executable FCIS fixtures are measured by the lane they help exercise. The Dylint FCIS gate independently checks the real repository boundary, so a coverage pass cannot substitute for structural conformance and a structural pass cannot exempt its detector or fixtures from coverage.

Each required supported platform emits source- and configuration-bound LLVM counts. Aggregation normalizes repository paths, merges the declared platform union without counting the same executable region twice, and checks integer `covered == count` for lines, regions, functions, and branches. Zero denominators, missing files, unsupported merges, broad exclusions, rounded percentages, or one missed item fail closed.

Alternatives considered:

- A percentage threshold below 100% was rejected because this capability is intended to close the reviewed source inventory, though 100% still does not imply behavioral adequacy.
- Keeping direct-`rustc` and Dylint sources outside the aggregate was rejected because it permits false-green inventory closure.

### 4. Use only the selected deterministic Hegel-rs configuration

“Hegel-rs” means `hegeltest = 0.28.0` from `hegeldev/hegel-rust` with the package, companion package, commit, tree, checksum, and license identities recorded in `docs/tooling/hegel-rs.md`. No similarly named crate or other property framework may satisfy this lane.

Every authoritative property uses `Backend::Default`, 256 cases, a committed property-specific seed under prefix `0x5350_5631_0000_0000`, `derandomize(true)`, `database(None)`, and the ordered phases `Explicit`, `Reuse`, `Generate`, `Target`, and `Shrink`. The property inventory names the exact generator domain and limits, production API, dependency-independent reference API or justified metamorphic relation, assertions, expected polarity, settings digest, source digests, and replay procedure.

A production-conformance obligation calls production code and a reference authority that does not import production semantic implementations, then compares exact results. A helper-only property is labeled `reference-only` and cannot close a production obligation. Reviewed explicit edge examples complement generated samples; independent finite models remain the only exhaustive evidence for their declared finite domains.

Alternatives considered: Hegel ambient defaults and automatic backend selection were rejected as environment-sensitive. Proptest, QuickCheck, and the unrelated `hegel` crate were rejected because they are not the selected upstream.

### 5. Normalize failures without pretending raw shrink output is canonical

Focused negative controls prove that the validators reject a missing coverage source, one uncovered branch, a zero denominator, an ambient/uncommitted Hegel setting, a helper-only production property, and an intentionally false Hegel assertion. Diagnostics carry stable IDs and are sorted by obligation ID/path. A failed Hegel run retains property ID, tool/settings identity, seed, and sufficient replay input; timing, terminal formatting, and shrink traces remain noncanonical. A minimized counterexample becomes deterministic evidence only after review and commitment as a digest-bound replay fixture.

### 6. Use one exact acceptance oracle

The focused commands remain `devenv tasks run buoy:coverage` and `devenv tasks run buoy:properties`. The exact aggregate acceptance oracle is:

```text
devenv test
```

It succeeds only with exit status 0 after both focused lanes execute offline under the pinned closure, all expected negative controls are rejected, every current in-scope coverage count satisfies `covered == count > 0` for all four metrics, every required Hegel property passes under its committed settings, and all normalized evidence bindings are present and current. Static validation alone, a previously recorded pass, or an aggregate command that omits either lane is not success.

### 7. Preserve the proof and trust boundaries

LLVM instrumentation, rustc, LLVM profile merging, `cargo-llvm-cov`, the host kernel, and tests themselves remain trusted for coverage observations. Hegel's generator, pseudo-random behavior, shrinking implementation, and the independence/correctness of the reference authority remain trusted for sampled results. Coverage demonstrates only that declared executable regions ran; properties demonstrate only their assertions over committed sampled domains.

Verus and AeneasVerif are unchanged by this design. These lanes do not prove total correctness, unsafe-code absence outside separately checked boundaries, compiler or machine-code correspondence, operating-system behavior, or prose-to-property correspondence. Formal claims still require exact implementation representation, assumptions, translators, verifier/configuration, proof source, and human-approved correspondence bindings.

## Risks / Trade-offs

- **[Risk] Exact four-metric closure is expensive and may encourage low-value tests.** → Keep behavior assertions, mutation tests, negative fixtures, and requirement traceability as separate obligations; reject call-only tests as substitutes for behavioral evidence.
- **[Risk] Platform-specific regions make one-host closure impossible.** → Declare the supported-platform matrix and merge source-bound platform artifacts; a missing required platform remains blocked rather than excluded.
- **[Risk] LLVM path/order output varies by host.** → Treat exports as raw input and emit only sorted repository-relative normalized summaries.
- **[Risk] Hegel sampling misses defects.** → State exact sample scope, preserve replay, use edge fixtures and independent finite models, and never label sampled evidence exhaustive.
- **[Risk] Test-source edits frequently stale evidence.** → Make stale status automatic and keep focused rerun commands deterministic and documented.
- **[Risk] Separate-manifest instrumentation complicates profile aggregation.** → Use one pinned Rust/LLVM closure and validate each artifact's tool/configuration binding before merge.
- **[Risk] Successful dogfooding may be overstated as general validation of `cargo-llvm-cov`, Dylint, or FCIS.** → Report only this repository's exact self-application scope and retain external suitability, upstream completeness, and architectural universality as explicit non-claims.

## Migration Plan

1. Add positive and negative validator fixtures before tightening either gate.
2. Complete the source and property inventories, including stable IDs, platform/configuration scope, and current bindings.
3. Make standalone validators and separate-manifest tools instrumentable under the pinned coverage closure; do not hide unresolved targets with exclusions.
4. Enforce the exact Hegel identity/settings and production/reference classification, then add deterministic failure replay fixtures.
5. Normalize and validate coverage/property result records, preserving the existing red baseline as historical rather than rewriting it.
6. Add both focused tasks to `devenv test`, run the focused negative controls, then run the exact aggregate oracle.

Rollback reverts the gate wiring and new validators together and marks the change unimplemented. It MUST NOT retain or relabel evidence produced under reverted scope as a current pass.

## Open Questions

None. Tool identity, metric policy, Hegel settings, and the exact final oracle are fixed by this design.
