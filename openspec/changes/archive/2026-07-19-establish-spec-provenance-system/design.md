## Context

The project currently has only development scaffolding and planning material. The planned system must trace normative requirement atoms across Markdown specifications, governed decision surfaces, executable test carriers, finite-state models, formal properties, proof records, derived artifacts, and implementation locations without conflating linkage, review acknowledgement, execution, or proof. OpenSpec is used to plan and review this change only; it is neither an implementation dependency nor an authoritative source format or lifecycle engine for the resulting product.

The principal constraints are:

- Identical repository bytes, configuration, schema versions, and pinned toolchain inputs produce byte-identical canonical graph and static report bytes.
- Canonical computation is offline and independent of time, network state, issue labels, filesystem traversal order, locale, and host absolute paths.
- Natural-language requirements remain authoritative until a human approves their correspondence to a formal property.
- Formal claims bind exact artifacts and state their trust boundary.
- Requirements receive positive and negative fixtures before production implementation.

## Goals / Non-Goals

**Goals:**

- Define a repository-independent provenance IR named `SpecGraph`.
- Make atom authority, identity, lifecycle, decision-surface completeness, evidence roles and scopes, coverage policy, debt, waivers, derivation freshness, change impact, and proof invalidation explicit.
- Detect missing normative decisions over declared finite surfaces rather than validating only atoms that already exist.
- Keep a pure deterministic kernel behind an imperative filesystem and process boundary.
- Make the CLI/CI check authoritative while allowing lint and LSP adapters to reuse the same library.
- Require selected adapter inputs to receive an explicit semantic disposition or deterministic diagnostic.
- Verify the smallest claim-integrity functions with Verus and/or AeneasVerif.

**Non-Goals:**

- Inferring formal semantics from prose without human approval.
- Treating comments, annotations, implementation markers, test discovery, or test success as mathematical proof.
- Verifying external parsers, operating systems, rustc, LLVM, or generated backends merely because the provenance kernel is verified.
- Making editor state authoritative.
- Designing a distributed evidence service or network protocol in the first version.
- Parsing OpenSpec changes, importing OpenSpec lifecycle semantics, or requiring OpenSpec at runtime.
- Making provider-specific pull-request metadata, labels, issue state, or review APIs canonical inputs.

## Decisions

### 1. Functional core and imperative shell

The deterministic core is physically isolated in the `spec-provenance-core` crate. It is `#![no_std]`, contains safe deterministic transformations, receives an explicit immutable input snapshot containing normalized repository-relative paths, file bytes, configuration bytes, schema identities, toolchain lock data, and, when rendering execution evidence, normalized oracle verdicts, and returns a `SpecGraph`, structured diagnostics, and canonical report values. It may use `core` and deterministic `alloc`-based values when owned data is required, but its complete dependency closure must continue to build for the configured bare-metal target.

The root `spec-provenance` crate is the initial shell and public facade. The shell owns filesystem discovery, symlink policy enforcement, source parsing adapters, process execution, environment setup, temporary files, and human-oriented raw logs. Shell-side crates may depend on the core; the core must never depend on a shell or adapter crate and never reads the filesystem, clock, environment, network, locale, or process state.

Local Dylint rules require the designated core crate to declare `#![no_std]`, reject mutable statics and immutable statics whose type has interior mutability, and reject exported function signatures containing function pointers or `Fn`, `FnMut`, or `FnOnce` bounds. Immutable `Freeze` statics, invocation-local mutation, and private core-owned callable helpers remain allowed. The gate also compiles the core dependency closure for a target without `std`. Allowed-positive and violating-negative fixtures keep each claimed detector form live, and the architecture lints emit no automatic fixes. These checks establish a structural restriction over their fixture-proven forms, not a proof of referential transparency: API review, explicit-input tests, properties, exhaustive models, and formal work remain necessary.

Alternative considered: let the graph builder walk the repository directly or rely on module naming conventions inside one crate. This is rejected because ambient traversal and host state would become hidden semantic inputs, while an unenforced convention would allow effectful dependencies to leak inward and complicate formal verification.

### 2. Parser-backed extraction

Markdown and source carriers are extracted from parser syntax nodes with spans. Markdown adapters distinguish prose, metadata, headings, block quotes, HTML, and fenced code. Rust adapters attach carrier metadata to eligible parsed test or verifier items. For every selected path and syntax class, an adapter must produce supported semantic objects, an explicit schema-defined nonnormative disposition, or a deterministic malformed/unsupported diagnostic. Unsupported schema versions, malformed authority-like constructs, and expected authority roots that produce no atoms cannot silently become a green empty graph.

Raw regular-expression scanning may support migration and debt diagnostics, but it cannot establish authoritative atoms, governed-surface completeness, or coverage.

Alternative considered: line-oriented scanning. This is rejected because fences, malformed syntax, macros, attributes, and attachment boundaries make line proximity ambiguous.

### 3. Dedicated provenance IR

`SpecGraph` is separate from any host compiler IR. It contains canonical collections of:

- `Atom`
- `Tombstone`
- `DecisionSurface`
- `EvidenceCarrier`
- `ImplementationLink`
- `Derivation`
- `CoveragePolicy`
- `DebtRecord`
- `Waiver`
- `FormalProperty`
- `FiniteModel`
- `Oracle`
- `Assumption`
- `ProofRecord`
- diagnostic source locations

All references close within the graph unless a schema explicitly declares an unsupported external reference class. Diagnostics use source locations, while semantic object digests exclude nonsemantic locations as defined by each schema.

Alternative considered: reuse Chelis execution or type IR. This is rejected because repository provenance has different identity, lifecycle, syntax, and evidence semantics.

### 4. Atom identity and lifecycle

A stable atom ID denotes identity. A versioned full SHA-256 digest denotes a semantic revision. The digest covers canonical normative content and semantic metadata, but not authority location. Moving unchanged authority text preserves its revision digest while changing source-map and complete graph digests.

A live atom has exactly one syntactically explicit authority. Allowed transitions are:

```text
proposed -> ratified
proposed -> withdrawn
ratified -> superseded
ratified -> withdrawn
```

Superseded and withdrawn IDs remain reserved through tombstones. They cannot be resurrected or reused.

Alternative considered: derive IDs only from content. This is rejected because editorial changes would destroy stable identity and make supersession history ambiguous.

### 5. Canonicalization and deterministic identity

Every canonical encoding is versioned, domain-separated, algorithm-identified, and unambiguous through length-delimited fields or an equivalently proved encoding. Full digests, not truncated displays, establish identity. Digest algorithm identity is part of the semantic schema. Version 1 uses full SHA-256. BLAKE3 was considered for its throughput, but expected normative corpora are small and SHA-256 currently offers the simpler widely supported audit and proof surface; changing algorithms requires a new schema/domain and explicit migration rather than reinterpreting existing identities.

Collections use schema-defined total ordering. Paths use normalized repository-relative byte ordering. Duplicate normalized paths or keys fail rather than using first-wins or last-wins behavior. Diagnostic ordering uses normalized path, span, diagnostic code, and stable tie-break fields.

Canonical outputs exclude time, random values, host paths, user names, PIDs, inode metadata, network responses, branch or issue state, terminal dimensions, and process completion order.

Open path, symlink, Unicode, line-ending, and Markdown trivia policies must be decided in versioned schemas before parser implementation.

### 6. Static registration and effectful execution

A syntax-attached carrier records potential evidence and an expected atom revision digest. Fresh registration establishes `linked` only.

Oracle invocation is separate. Normalized verdicts are `not-run`, `pass`, `fail`, `error`, and `timeout`. Live process timing and raw logs stay outside canonical output. A deterministic execution report is a pure function of the static graph and normalized verdict set.

Alternative considered: mark discovered tests green. This is rejected because discovery does not show selection, execution, or success.

### 7. Orthogonal assurance facts

Reports retain independent facts for:

- authority validity
- governed decision-surface completeness and disposition authority
- carrier presence, role, declared scope, and exclusions
- carrier freshness
- coverage-policy applicability, enforcement mode, and required-role coverage
- debt and waiver presence without obligation satisfaction
- derivation freshness and consistency-oracle scope
- oracle selection
- execution verdict
- finite-model identity and bounds
- proof freshness
- change-impact disposition and review acknowledgement
- assurance class
- assumptions and exclusions

Accepted vocabulary is `linked`, `tested`, `exhaustively-checked`, `formally-proved`, `structurally-enforced`, and `review-only`. These are scoped evidence classes, not interchangeable rungs on one ladder.

Aggregation is fail-closed. A required stale, absent, not-run, failed, errored, or timed-out input cannot be hidden by passing siblings. Debt, waiver, and reaffirmation records explain or authorize process state but never synthesize coverage, freshness, execution, structural enforcement, or proof.

### 8. Formal properties and proof records

A formal property names exact atom revision digests, its implementation abstraction, assumptions, and a human approval artifact for prose-to-property correspondence.

A proof record binds:

- atom and property digests
- implementation source, MIR, LLBC, or other representation digest
- assumption IDs and digests
- translator identities, versions, configurations, and executable digests
- verifier identity, version, configuration, and executable digest
- external model and proof-source digests
- normalized verdict, scope, and exclusions

Any binding change makes the proof stale.

For Verus, bindings include Verus, Z3, verified source, specifications, proof code, and relevant configuration. Verus does not verify itself or rustc/LLVM.

For AeneasVerif, bindings include Charon, LLBC, Aeneas, the selected backend, external models, generated files, and handwritten proofs. The initial verified kernel stays within safe sequential Rust, avoids unsafe code and concurrency, and avoids exiting an outer loop from a nested loop. Aeneas's generated functional model does not by itself prove external models or downstream native compilation.

### 9. Testing and formal-verification sequence

Before production algorithms:

1. Ratify schemas and write positive and negative fixtures.
2. Use only `hegeltest` 0.28.0 from `hegeldev/hegel-rust` at the immutable commits and registry checksums recorded in `docs/tooling/hegel-rs.md`. No similarly named framework may be substituted silently.
3. Write property tests for canonicalization, lifecycle, decision-surface completeness, carrier and derivation freshness, coverage-policy monotonicity, change-impact closure, and no-laundering aggregation.
4. Write independent finite reference models and exhaustively explore all reachable declared lifecycle, surface, policy, change-disposition, freshness, aggregation, and proof-invalidation states.
5. Build small Verus and Aeneas proof spikes for lifecycle, decision-surface completeness, carrier and derivation freshness, policy/debt/waiver/reaffirmation no-laundering, change-impact closure, and canonical field encoding.
6. Select at least one primary formal lane for the integrity kernel.

Exhaustive reports name the exact finite model and digest. Formal reports name exact proof bindings and exclusions.

### 10. Product and package surfaces

The Cargo workspace starts with `spec-provenance-core` as the pure `no_std` semantic kernel and `spec-provenance` as the imperative-shell facade. Future syntax, CLI, and host-integration crates remain shell-side and depend inward on the core only. The Dylint library is development tooling outside the production dependency graph because it uses nightly compiler internals.

The authoritative future command is an offline repository-wide check exposed through the project CLI. Chelis may later expose `chelis spec check` through an adapter. A future LSP calls the same core library but cannot replace the CLI/CI oracle.

### 11. Acceptance oracle

The final acceptance command for this change is:

```text
devenv test
```

It is green only when the pinned environment validates OpenSpec planning artifacts, formatting, Clippy with warnings denied, Dylint architecture rules and their negative fixtures, the bare-metal `no_std` core build, all positive and negative tests, Hegel-rs properties, exhaustive finite-state models, governed-surface completeness, coverage-policy and debt/waiver no-laundering behavior, derivation freshness, deterministic base-to-head change-impact closure, adapter totality, deterministic golden and permutation tests, integration tests, and the selected formal-verification gates. OpenSpec validation checks the planning workflow only and is not a product conformance oracle. During scaffolding, `devenv test` covers only the checks that exist and does not imply the planned system is implemented.

### 12. Governed decision surfaces detect specification silence

A `DecisionSurface` declares a versioned finite member domain, a closed disposition vocabulary, and the authority and coverage policy applicable to each member. Every expected member must appear exactly once with a schema-valid disposition and, when required by policy, a live atom ID and exact revision digest. Missing, duplicate, unknown, or unauthorised members fail closed. A disposition records that a normative decision exists; it does not establish that an implementation honors the decision.

Surface inventories are explicit semantic inputs or parser-backed declarations. They are not inferred from the atoms that happen to exist, because such inference cannot detect absent requirements. Host adapters may model capability matrices, tag vocabularies, protocol messages, or other finite surfaces without placing host-specific semantics in the core.

Alternative considered: report only uncovered authored atoms. This is rejected because it detects stale and unsupported claims but cannot detect a cell for which no requirement was ever authored.

### 13. Coverage policy, evidence scope, debt, and waivers

A versioned `CoveragePolicy` declares its atom and decision-surface scope, required evidence roles, blocking or advisory enforcement, and adoption-ratchet identity. Scope reductions, weaker required roles, or blocking-to-advisory changes are explicit change impacts and cannot disappear from reports. Policy adoption may grow incrementally, but removal requires a repository-owned migration disposition and remains visible in the change report.

Ordinary evidence carriers bind their exact executable item, oracle, configuration and fixture or input-corpus digests, declared claim scope, assumptions, and exclusions. A passing point test supports only that declared point scope; it cannot become property or exhaustive coverage by aggregation or naming.

`DebtRecord` and `Waiver` are repository-owned, digest-bound objects with a local stable ID, affected obligation, rationale, ownership, and remediation or discharge condition. External issue URLs may be informational but never supply canonical status. Debt and waivers remain reportable and may authorize a process gate to proceed, but they never satisfy the waived obligation or upgrade assurance. A known-failing or intentionally unselected carrier remains linked with its actual `fail` or `not-run` verdict.

Alternative considered: use issue labels, ignored-test annotations, or provider exemptions as policy state. This is rejected because they are ambient, mutable, provider-specific, and easy to misreport as evidence.

### 14. Deterministic change impact and reaffirmation

The core can compare explicit base and head `SpecGraph` values plus a provider-neutral repository change manifest. The change report enumerates changed or retired atoms and every transitively affected carrier, implementation link, decision-surface disposition, policy, derivation, property, assumption, and proof. Each required impact receives one explicit disposition: `reaffirmed`, `revised`, `retired`, or `waived`.

Reaffirmation binds the old and new object digests and a repository-owned review artifact. It records that the claim was deliberately revisited; it is `review-only` evidence and does not prove behavioral correspondence. Missing dispositions fail the change gate even when the head snapshot is statically valid. Provider adapters may translate pull-request trailers or reviews into the neutral manifest outside the core, but provider metadata never changes static graph identity.

Alternative considered: rely only on head-snapshot freshness. This is rejected because mechanically updating expected digests can erase evidence that affected claims were never reconsidered.

### 15. Derivations make intentional duplication explicit

A `Derivation` binds source object and byte digests, target artifact digests, generator or consistency-checker identity and configuration, and the oracle that validates the relation. It is statically fresh only when every binding remains current. Structural-consistency assurance is a separate fact that additionally requires the selected consistency oracle to have the appropriate normalized verdict. The resulting assurance is limited to the declared derivation relation; byte or structural consistency does not prove behavioral correctness.

Intentional mirrored artifacts without a derivation remain visible debt. Generated output is never trusted merely because it is checked in, and raw generator logs remain noncanonical.

Alternative considered: treat copies and generated artifacts as implementation details. This is rejected because silent drift between normative tables, configuration, generated code, and duplicated gates is a recurring provenance failure.

### 16. Reference-tool independence and adapter totality

OpenSpec informs the planning flow and requirement-writing discipline but contributes no runtime library, source format, authority, parser, or lifecycle semantics. Product schemas and adapters are versioned and owned by this project. OpenSpec validation remains one development gate alongside formatting and tests, not an input to canonical graph construction.

Every configured adapter declares supported syntax and schema versions. Selected inputs must become semantic objects, explicit nonnormative dispositions, or loud deterministic diagnostics. Unknown versions and malformed authority-like syntax cannot be replaced with empty results or guessed defaults.

Alternative considered: make OpenSpec changes the product's source-of-truth format. This is rejected to preserve repository independence and avoid coupling canonical identity to a planning tool.

### 17. Enforcement strength and phase handoffs

The implementation pushes invariants to the strongest practical channel: type and constructor constraints first, then structural lint or gate failures, then mutation-tested tripwires, with checklists and prose only as supporting guidance. Intentional duplication requires a derivation and consistency oracle rather than a standing instruction to update both copies.

Each implementation phase records what it inherits, delivers, freezes at exit, explicitly excludes, and the exact oracle that decides completion. New findings may create additional tracked obligations but cannot silently weaken a frozen oracle or expand a phase until its original failures disappear. This keeps multi-session and multi-agent work aligned with stable interfaces.

Alternative considered: rely on contributor discipline and one global task list. This is rejected because prose-only constraints and implicit phase boundaries do not reliably reach an implementer at the point of change.

## Risks / Trade-offs

- **Natural-language ambiguity can survive perfect linkage** -> Require explicit human approval of formal-property correspondence and report it as review evidence, not proof.
- **Parser behavior can create false authorities, carriers, or a misleading empty graph** -> Use parser-backed spans, total adapter dispositions, adversarial fixtures, mutation tests, expected-authority policies, and fail closed on malformed, unsupported, or ambiguous attachment.
- **A declared surface can be incomplete or manipulated to omit hard cells** -> Bind the complete surface schema and domain digest, reject missing and duplicate members, and require independent fixtures for domain growth and shrinkage.
- **Waivers or reaffirmations can launder missing evidence** -> Keep them orthogonal and review-only, preserve the unsatisfied obligation, and exercise no-laundering combinations exhaustively.
- **Canonicalization bugs can silently split or merge identity** -> Version encodings and algorithm identities, reject duplicates, use domain separation, golden tests, metamorphic tests, and a small formally verified encoder core.
- **Finite-state results can be overstated** -> Include model identity, bounds, and digest in every exhaustive claim.
- **Verifier success can be overstated** -> Bind exact artifacts, assumptions, tools, and exclusions; stale bindings cannot report proved.
- **Formal-tool subsets can distort production design** -> Keep only the claim-integrity kernel in the verified subset and isolate adapters.
- **Dual Verus and Aeneas lanes can be expensive** -> Compare proof spikes first; require one primary lane and reserve dual proof for the smallest critical functions.
- **Live execution is host-dependent** -> Keep static identity independent of execution and normalize only semantic verdicts into execution reports.
- **Derived artifacts can be mistaken for independently authored evidence** -> Report derivation scope and freshness separately and never upgrade consistency into behavioral assurance.
- **Provider workflows can leak into canonical semantics** -> Accept only explicit neutral change manifests in the core and isolate provider adapters in the shell.
- **`no_std` and lint success can be mistaken for proof of purity** -> Report them only as structural enforcement and retain explicit-input, property, model, and formal obligations for semantic claims.

## Migration Plan

1. Keep this material as an unimplemented OpenSpec change.
2. Ratify schemas, exact atom syntax, ID allocation, path policy, governed decision surfaces, derivations, coverage policies, debt and waivers, neutral change manifests, and fixtures while recording OpenSpec as reference-only.
3. Freeze each implementation phase's inherit, deliver, frozen-at-exit, explicitly-not-owned, and oracle contract before that phase starts.
4. Resolve Hegel-rs and formal-tool pins through architecture decisions.
5. Keep the `no_std` core boundary and its Dylint/bare-metal checks green; implement deterministic core behavior only after failing tests and finite models exist.
6. Add total Markdown and Rust syntax adapters on the shell side of the boundary.
7. Add repository snapshotting, governed-surface and debt reports, deterministic base-to-head change-impact checking, CLI reporting, and explicit oracle execution.
8. Add derivation consistency orchestration and the selected formal gate with proof records.
9. Integrate with Chelis through a separate reviewed change.

Rollback before external adoption is removal of the new package and OpenSpec change. After a schema is published, rollback requires retaining tombstones and readers for supported schema versions; identities must not be silently reused.

## Open Questions

- Which future syntax, CLI, and integration concerns warrant dedicated shell-side crates while preserving the one-way core dependency boundary?

The formal-lane questions are resolved by `docs/architecture/adr-0001-primary-formal-lane.md`: Verus is primary, Lean 4 is the selected Aeneas backend, and exact Verus/Rust source plus specifications and proof code is authoritative for the initial source-level proof class.
