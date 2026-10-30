## Why

Repositories need a deterministic way to detect both missing normative decisions and stale links from authored requirements to current evidence without presenting annotations, discovered tests, passing tests, waivers, or review acknowledgements as stronger assurance than they provide. This change establishes a repository-independent provenance model and verification workflow, with Chelis as the first intended integration.

## What Changes

### Goals

- Introduce explicit requirement atoms with stable identities, semantic revision digests, authority rules, and irreversible lifecycle transitions.
- Detect specification silence through declared finite decision surfaces whose members require explicit, atom-bound dispositions.
- Build a canonical repository-wide `SpecGraph` from syntax-attached atoms, governed decision surfaces, carriers, implementation links, derivations, coverage policies, debt and waiver records, finite models, formal properties, assumptions, oracles, tombstones, and proof records.
- Produce byte-identical static graph and report bytes from identical repository, configuration, schema, and pinned toolchain inputs.
- Produce deterministic base-to-head change-impact reports that enumerate affected claims and require explicit reaffirm, revise, retire, or waive dispositions without treating those dispositions as proof.
- Make intentional duplication and generation explicit through digest-bound derivation records and consistency oracles.
- Isolate the pure deterministic kernel in a `no_std` core crate behind a one-way functional-core/imperative-shell dependency boundary, with Dylint rules rejecting implicit `std`, ambient mutable statics, and callable public interfaces plus a bare-metal target build.
- Report static registration, freshness, role coverage, evidence scope, policy coverage, debt, waiver status, oracle execution, and assurance class as independent facts.
- Require selected syntax inputs to receive a supported semantic disposition or a loud deterministic diagnostic rather than being silently skipped.
- Support comprehensive positive and negative fixtures, Hegel-rs property testing after its exact upstream is pinned, exhaustive declared finite-state models, and formal verification with Verus and/or AeneasVerif.
- Expose one authoritative offline CLI/CI check while allowing lint and future LSP surfaces to consume the same library.

### Non-goals

- Automatically proving equivalence between natural-language requirements and formal properties.
- Treating annotations, implementation markers, discovered tests, or passing tests as mathematical proof.
- Claiming that a proof over Rust or compiler IR proves emitted C, HIP, Metal, LLVM, or machine code without a separately justified translation chain.
- Making editor or unsaved-buffer state authoritative.
- Depending on network state, time, issue labels, filesystem order, locale, or host absolute paths for canonical results.
- Depending on OpenSpec as a runtime library, source format, authority model, lifecycle engine, or parser; OpenSpec remains a planning and reference workflow only.
- Embedding provider-specific pull-request trailers, labels, review APIs, or `CODEOWNERS` semantics in the deterministic core.

### Static and effectful boundaries

The static check is a deterministic function of an explicit repository snapshot and pinned semantic inputs. Oracle invocation is a separate effectful operation whose normalized verdicts can be incorporated into a deterministic execution report without changing the static graph.

## Capabilities

### New Capabilities

- `spec-graph`: Authoritative atom extraction, lifecycle, governed decision-surface completeness, derivation consistency, graph closure, total adapter disposition, carrier attachment, source mapping, and implementation navigation.
- `deterministic-artifacts`: Versioned canonical encoding, algorithm-identified domain-separated digests, normalized paths and ordering, deterministic diagnostics, provider-neutral base-to-head change-impact reports, and offline reproducibility.
- `evidence-assurance`: Versioned coverage policies, explicit evidence roles and scopes, freshness, repository-owned debt and waivers, change reaffirmation, oracle execution, conservative aggregation, bounded exhaustive-model claims, and narrow assurance vocabulary.
- `formal-proof-records`: Formal property approval, exact proof bindings, Verus and AeneasVerif trust boundaries, invalidation, and reproducible proof acceptance.

### Modified Capabilities

None. This project has no baseline OpenSpec capabilities yet.

## Impact

- Adds a Rust workspace with a `no_std` deterministic core and a root imperative-shell facade, followed later by a CLI surface.
- Adds pinned local Dylint architecture checks for `no_std`, ambient mutable statics, and callable public interfaces plus a bare-metal core build; these structurally enforce the boundary but do not prove semantic purity.
- Adds schema-owned Markdown and Rust syntax adapters while keeping the deterministic core repository-independent and fail-loud on selected unsupported or malformed inputs.
- Uses OpenSpec to plan and review versioned schemas and fixture corpora without making OpenSpec an implementation dependency or authoritative input format.
- Adds repository-owned policy, debt, waiver, derivation, and change-manifest formats that remain deterministic without provider metadata.
- Requires a pinned Rust toolchain, Hegel-rs upstream decision, and offline-resolvable Verus/AeneasVerif toolchain decisions before their implementation phases.
- Does not change Chelis until a later, independently reviewed integration phase.
