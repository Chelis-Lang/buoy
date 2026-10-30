# Implementation phase handoffs

Status: local phase contracts for task 2.8. These contracts refine the phase summaries in the OpenSpec task list.

## Standing boundary rules

- A phase begins only after every prior required phase task is complete and its oracle is green.
- Run the phase oracle before work to record its expected red or missing set, then again at exit.
- A frozen output can change only through a reviewed schema/design update with affected fixtures and downstream contracts in the same change.
- New findings create separately identified obligations. They do not silently broaden the current phase or weaken its frozen oracle.
- Controls and negative fixtures do not move to match an implementation. A behavior becomes green by fixing behavior or by an explicit reviewed schema change.
- The command names below are stable interfaces. A rename requires updates to this document, the task policy, fixtures, and downstream references.
- `devenv test` remains the only final acceptance oracle. Focused phase commands establish only their named phase contract.

## Production P0–P3 handoffs

`assurance/v1/phase-handoffs.toml` defines the handoff obligations. `nix/devenv/workflow-policy.toml` defines their task ownership and order.

| Slice | Exact command | Direct children | Current gate boundary |
|---|---|---|---|
| P0 | `devenv tasks run buoy:p0` | schema, reference, fixtures, properties, models, recorded coverage checks | Blocks on stale evidence without LLVM instrumentation |
| P1 | `devenv tasks run buoy:p1` | hashing policy, identifier policy, core, FCIS, mutation replay, selected Verus proof | Runs the baseline, P0, and P1 dependency graph |
| P2 | `devenv tasks run buoy:p2` | Unicode, adapters, integration, supply chain, bounded adversarial gate | Runs all earlier dependencies and requires current-host evidence |
| P3 | `devenv tasks run buoy:p3` | obligation closure, gate wiring, QA registry, ledger-green | Rejects stale coverage or mutation evidence and all other blocked ledger rows |

Each phase task depends directly on all owned children. The child tasks also form one serial chain.

A later focused phase command runs its complete upstream graph. Thus, the phase commands are cumulative and monotonic.

The phase scripts only report completion. They do not run child commands.

`buoy-gate-wiring` parses `DEVENV_TASK_FILE`. It rejects missing phases, duplicate owners, nested phases, direct sink children, cache controls, and missing lifecycle paths.

The checker rejects any automatic path to `buoy:coverage` or `buoy:mutation-campaign`. Maintainers must select these tasks explicitly.

The checker also requires `cwd = config.git.root` for each repository task. Its diagnostics use stable task names instead of host paths.

Broad workspace tests can repeat assertions for baseline compatibility. The repeated assertions receive no second evidence credit.

`devenv test` reaches `buoy:gate` through `devenv:enterTest`. Each required task runs once in the reviewed graph.

The `spec-provenance-phase-result/v2` records are optional exported attestations. They are not ledger prerequisites.

## Phase 3 — fixtures and independent models

**Inherits**

- FCIS boundary and scaffold oracle from phase 1.
- The frozen active v2 schema, immutable v1 baseline, migration decision, and Hegel upstream decision.
- No production provenance algorithms.

**Delivers**

- Positive and negative fixtures for every SPG, DET, EVD, and FPR scenario.
- Independent finite reference models.
- A fail-closed fixture manifest mapping requirement, polarity, fixture, focused slice, and final oracle.
- The focused command `devenv tasks run buoy:fixtures`.

**Frozen at exit**

- Fixture input bytes and expected semantic result/diagnostic codes.
- Independent model state/event/transition definitions.
- Manifest completeness and polarity.
- The rule that every requirement scenario has an executable manifest row.

**Explicitly not this phase**

- Production core algorithms.
- Markdown/Rust production adapters.
- Updating a fixture to agree with production code.
- Property-run, proof, CLI, or live oracle orchestration.

**Exact oracle**

```text
devenv tasks run buoy:fixtures
```

The command succeeds only when every positive row accepts, every negative row rejects for the intended stable diagnostic family, no requirement/scenario is absent, no undeclared fixture runs, and independent model self-checks pass.

## Phase 4 — properties, exhaustive runners, and formal spikes

**Inherits**

- Frozen fixtures, manifest, and independent finite models from phase 3.
- Exact Hegel packages/checksums and deterministic settings from `docs/tooling/hegel-rs.md`.
- No production implementation to mirror.

**Delivers**

- Hegel properties over independent schema/reference functions.
- Fixed-point exhaustive model runners.
- Offline-resolvable Verus/Aeneas candidate pins.
- Equivalent nonproduction proof spikes or a reviewed infeasibility blocker.
- The primary formal-lane decision.
- The focused command `devenv tasks run buoy:properties`.

**Frozen at exit**

- Property declarations, generator domains, test-case counts, seeds/settings, and no-laundering assertions.
- Exhaustive model IDs, bounds, transition relations, and fixed-point criteria.
- Proof-spike obligations and comparison evidence.
- Primary formal lane and explicit trust/exclusions.

**Explicitly not this phase**

- Production core algorithms.
- Reusing production code as the independent reference model.
- Calling sampled Hegel runs exhaustive or formal proof.
- Selecting a verifier by preference without proved spikes or blocker evidence.

**Exact oracle**

```text
devenv tasks run buoy:properties
```

The command succeeds only when every Hegel property passes under committed deterministic settings, every declared finite model reaches a fixed point with all invariants green, proof spikes produce their expected positive and negative results, and the formal-lane decision references those exact results.

## Phase 5 — deterministic core

**Inherits**

- Frozen schemas, fixtures, independent models, property declarations, and formal-lane decision.
- Enforced `no_std` FCIS boundary.

**Delivers**

- All v2 core values and deterministic algorithms.
- Canonical encoding/digests, lifecycle, surface completeness, closure, freshness, policy, aggregation, and impact analysis.
- Selected formal proofs bound to exact executable functions.
- The focused command `devenv tasks run buoy:core`.

**Frozen at exit**

- Public core value and function signatures.
- Canonical golden bytes and diagnostic codes.
- Pure algorithm behavior against fixtures/models/properties.
- Proof bindings for selected integrity functions.

**Explicitly not this phase**

- Filesystem, environment, parser, process, provider, clock, locale, or human-presentation effects.
- Callable behavior supplied through exported core interfaces.
- Host-specific surface semantics.
- Raw tool logs or process orchestration.

**Exact oracle**

```text
devenv tasks run buoy:core
```

The command succeeds only when core unit, fixture, property, exhaustive-model, golden, permutation, mutation, bare-metal `no_std`, Dylint, and selected formal checks all pass with no shell dependency in the core closure.

## Phase 6 — total syntax adapters

**Inherits**

- Frozen core API and canonical diagnostics.
- Frozen Markdown/Rust adapter fixtures and selected-root policy.

**Delivers**

- Total schema-owned Markdown and Rust adapters.
- Syntax-attached source spans and semantic/nonnormative/diagnostic disposition.
- Cross-adapter graph parity and source-root independence.
- The focused command `devenv tasks run buoy:adapters`.

**Frozen at exit**

- Supported syntax/version declarations.
- Attachment, malformed-input, unsupported-version, fence, macro, and Unicode-span behavior.
- Adapter-to-core value mapping.
- The OpenSpec reference-only boundary.

**Explicitly not this phase**

- OpenSpec parsing or lifecycle semantics.
- Raw-regex authority.
- Filesystem discovery and process execution.
- Independent authority, freshness, or assurance semantics in an adapter.

**Exact oracle**

```text
devenv tasks run buoy:adapters
```

The command succeeds only when every selected fixture receives one supported semantic, explicit nonnormative, or deterministic diagnostic disposition; all mutation and cross-order fixtures pass; unsupported versions fail loudly; and equal snapshots under distinct roots produce equivalent adapter outputs.

## Phase 7 — repository, CLI, and oracle shells

**Inherits**

- Frozen total adapters and core API.
- Provider-neutral change and report schemas.

**Delivers**

- Filesystem snapshot/configuration shell.
- Authoritative static and change CLI checks.
- Deterministic machine/human rendering.
- Effectful oracle and derivation-consistency orchestration with normalized receipts.
- Advisory migration/debt reports and lint/LSP parity.
- The focused command `devenv tasks run buoy:integration`.

**Frozen at exit**

- CLI command names, exit classes, machine-report schema, and offline behavior.
- Static/change/execution separation.
- Normalized verdict and impact receipt behavior.
- Lint/LSP semantic parity with saved repository inputs.

**Explicitly not this phase**

- Provider state in canonical inputs.
- Raw logs in canonical reports.
- Network-required static checking.
- Chelis-specific semantics or command integration.
- Editor/unsaved state as CI authority.

**Exact oracle**

```text
devenv tasks run buoy:integration
```

The command succeeds only when isolated-repository tests prove path/link rejection, offline and cross-root identity, exact CLI exit classes, static/change/execution separation, partial selection, crashes, timeouts, noncanonical logs, policy ratchets, impact closure, report determinism, and adapter-surface parity.

## Phase 8 — assurance and release

**Inherits**

- Every frozen phase output and focused oracle.
- A complete repository-wide implementation with all intended blocking scopes active.

**Delivers**

- Golden regeneration and mutation policies.
- Trust, exclusion, enforcement, duplicate/derivation, and deferred-evidence documentation.
- Full offline gate registration.
- Adversarial assurance-inflation review and resolutions.
- Final acceptance evidence.

**Frozen at exit**

- Published v2 schema/report behavior and immutable v1 migration/rollback provenance.
- Final trust and exclusion statements.
- Exact final gate composition.
- The boundary that Chelis integration remains a separate change.

**Explicitly not this phase**

- New v2 semantics added after fixture freeze without migration.
- Chelis integration code or capability rows.
- Claims that scaffolding, test counts, structural lints, exhaustive finite models, or formal subset proofs establish broader correctness than their declared scope.

**Exact oracle**

```text
devenv test
```

The command succeeds only when every local blocking obligation is current and passing. Advisory debt remains visible. No acceptance check requires provider state.

## Handoff evidence

At each phase exit, the implementation session records:

- the exact command invocation;
- pinned tool/schema identities;
- normalized success/failure summary;
- frozen artifacts delivered;
- unresolved advisory debt;
- discoveries forked to later obligations; and
- an explicit statement that the phase's exclusions were not claimed.

This record is review evidence, not a substitute for the command's executable result.
