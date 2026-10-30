## Context

The standalone Buoy v1 implementation and its exact `devenv test` oracle are green before this proposal begins. Chelis is an external host with its own capability vocabulary, execution lanes, generated artifacts, tests, review workflow, and release gate. The integration must consume repository-independent value/adapter/CLI contracts without changing their meaning or introducing an outward dependency from the `no_std` core.

## Goals / Non-Goals

**Goals:**

- Materialize Chelis-owned requirement atoms, complete finite decision surfaces, carriers, implementation links, derivations, policies, debt, waivers, reaffirmations, and neutral change manifests.
- Preserve byte-identical static graph/report output for identical explicit snapshots, configuration, schemas, and pinned tools under separate absolute roots.
- Add one Chelis command adapter with separate static, change, and effectful execution modes.
- Stage advisory reporting before blocking policy and retain exact evidence scopes and no-laundering facts.
- Bind Chelis formal claims to exact implementation representations, properties, assumptions, tools, models, generated artifacts, proof sources, scopes, and exclusions.

**Non-Goals:**

- Adding Chelis capability rows, tag meanings, provider semantics, or lifecycle rules to the generic core.
- Parsing OpenSpec as product authority or importing OpenSpec state.
- Treating annotations, discovered/passing tests, finite models, review records, debt, waivers, or derivation consistency as mathematical proof.
- Claiming source/IR proof establishes rustc, LLVM, emitted backends, or machine code.

## Decisions

### Pinned integration boundary

The first integration target is Chelis revision `c0d4caf16f557406f299f0989bab48fa0f924cbd` (tree `389a4a38be8d6f1b2bc381e405b5d8dede1acca8`, version `0.16.1`, MIT). The pin covers tracked Git trees only; ambient worktree state is excluded. The exact revision-bound build inputs are the pinned `Cargo.lock` blob `60f8f86e1df4977a21f876fb3ab2503368d71388` and `rust-toolchain.toml` blob `73cb934de4706a914c15e8db2a3c037ce75699d9`; Chelis declares channel `stable`, which stays recorded as not exactly reproducible. Buoy compatibility is workspace `0.2.0` with `buoy.adapter-sdk/v3`. Shell-side ownership: the `chelis-provenance` library crate lives in the Buoy workspace inside this change; the `chelis-cli` command crate and the `chelis provenance` command namespace stay reserved for a separate Chelis-side change. The final oracle is `devenv test` from the pinned Buoy Chelis-integration worktree, whose `devenv.lock` pins the exact integration toolchain. `fixtures/chelis-provenance/v1/boundary.toml` is the frozen machine record.

### Shell-side host adapter and one-way dependency

The Chelis adapter lives in Chelis or another shell-side integration crate. It depends on the published Buoy shell/core interfaces; `buoy-core` remains `no_std` and never depends on Chelis, parsers, filesystems, processes, providers, or clocks.

Alternative: add Chelis variants to core enums. Rejected because it would destroy repository independence and couple generic schema evolution to one host.

### Published adapter SDK and parsed-item schema are prerequisites

The integration depends on the versioned shell-side Buoy adapter SDK established by production hardening. Chelis owns its annotation grammar, code-unit interpretation, editor/dashboard/search behavior, and domain policy; these consumer concerns remain outside canonical Buoy semantics. Materialization consumes only canonical saved Chelis repository paths/bytes and explicit configuration. Daemon state, editor state, unsaved buffers, LSP observations, MCP sessions, and provider metadata cannot supply authority or alter canonical output.

Buoy schema v3 uses validated, versioned `SourceItemKind` values. Adapter descriptors declare their complete kind sets.

The Chelis adapter uses `chelis/surf-declaration/v1` and `chelis/deep-node/v1`. Schema v3 requires source reingestion and has no legacy item-kind reader.

The adapter MUST NOT use a kind from another adapter. It also MUST NOT use an opaque payload or a host-specific core enum.

Alternative: use a temporary cross-adapter kind or opaque declaration. Rejected because it would establish a false canonical identity.

### Chelis owns finite domains and payload schemas

Chelis registers immutable payload schema IDs for each axis, member key, disposition, scope, and report rendering. Complete products include unsupported/prohibited/deferred rows; no adapter predicate filters difficult combinations before core completeness checks. Domain construction, evolution, and every authority citation are repository-owned and digest-bound.

Alternative: infer capability rows from code/tests. Rejected because current implementation cannot expose specification silence.

### Explicit snapshots and neutral change inputs

A shell adapter maps one saved Chelis repository snapshot into canonical relative paths/bytes and configured selectors. GitHub labels, reviews, issue state, authors, timestamps, branch names, and live APIs cannot change graph identity. Provider workflows may propose repository-owned review or change-manifest records, but deterministic checking consumes only committed neutral values.

### Orthogonal evidence and effect separation

Static registration, freshness, role/scope coverage, debt/waiver state, derivation freshness, oracle selection, five-state verdict, exhaustive-model identity/bounds, proof freshness, review acknowledgement, and assurance class remain separate facts. Static graph identity never changes because an oracle runs. Raw logs and timing remain noncanonical.

### Test-first staged rollout

Chelis fixtures freeze positive/negative expected behavior before adapter production code. Initial policy is advisory and reports all undecided/uncovered/stale/unselected obligations. Promotion to blocking retains the same ratchet identity and occurs only after every selected obligation is current and passing or has a separately valid process waiver that leaves the obligation visibly unsatisfied.

### Formal claim boundary

Any Verus proof binds Verus, bundled Z3, exact Chelis source/spec/proof/configuration and exclusions. Any Aeneas proof binds Charon, LLBC, Aeneas, Lean, every external model/generated file/proof source and exclusions. Formalization correspondence remains human-approved review evidence. Neither lane proves external compiler/backend output without a separate verified chain.

### Acceptance oracle

The exact final oracle is `devenv test` executed from the pinned Chelis integration worktree. It succeeds only when the Buoy protocol, Chelis fixtures, complete surfaces, adapter totality, deterministic reports, impact closure, derivations, oracles, mutation controls, and formal bindings are current and green.

## Risks / Trade-offs

- **Chelis domains omit difficult rows** → independently materialize complete products, mutation-test missing/unknown rows, and review domain evolution.
- **Provider workflow leaks into authority** → require committed neutral review/change records and vary provider metadata in deterministic tests.
- **Mechanical digest bumps launder impact** → require exact per-impact reaffirm/revise/retire/waive records and review-only reaffirmation facts.
- **Point tests are generalized** → bind exact corpus/configuration/scope and keep property/exhaustive/formal classes separate.
- **Generated Chelis artifacts drift** → require derivations and consistency oracles; never treat consistency as behavior.
- **Dual formal lanes increase CI cost** → use the standalone Verus-primary ADR and require secondary Aeneas only where its translation model adds reviewed value.

## Migration Plan

1. Pin the Chelis revision, adapter identities, `buoy.adapter-sdk/v3`, and the approved schema-v3 source-item kinds.
2. Inventory existing normative sources and propose stable atom/surface/schema namespaces without changing behavior.
3. Freeze positive/negative fixtures and complete finite domains.
4. Implement total shell adapters and advisory static/change reports.
5. Bind carriers, derivations, models, properties, proof records, debt, waivers, and review records.
6. Run cross-root/offline/mutation/integration gates and eliminate advisory gaps.
7. Review promotion to blocking under the same ratchet identities.
8. Run the pinned Chelis-worktree `devenv test` final oracle.

Rollback before blocking adoption removes the adapter and configuration. After publication, a replacement requires source reingestion and new schema identities.

## Open Questions

- Which pinned Chelis revision and command namespace own the first integration?
- Which Chelis capability matrices form the first finite blocking surfaces?
- Which existing generated or mirrored artifacts require derivations at initial adoption?
- Which Chelis integrity functions warrant an additional Aeneas/Lean binding beyond the Verus-primary lane?
