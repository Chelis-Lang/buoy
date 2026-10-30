# Trust boundaries, assumptions, and exclusions

Status: v3 release assurance statement.

## Claim discipline

The product reports only `linked`, `tested`, `exhaustively-checked`, `formally-proved`, `structurally-enforced`, and `review-only` in an exact declared scope. These classes are orthogonal. Counts, discovery, annotations, implementation links, debt, waivers, reaffirmations, or passing siblings never synthesize a stronger class.

The OpenSpec CLI supplies planning support only. Strict OpenSpec validation establishes only the internal validity of planning artifacts.

The OpenSpec adapter reads selected standing specifications as optional authority sources. Repository configuration supplies lifecycle and approval authority.

External trace links supply navigation and impact only. They do not supply an assurance class or provider state.

Fresh implementation bindings also supply navigation and impact only. They do not supply coverage, execution, proof, approval, correctness, or prose-to-code correspondence.

Readable trace projections carry the same limit. A machine, human, or Markdown projection reports stored navigation facts for exact saved inputs. It adds no assurance class, reads no provider state, and writes no repository file.

A generated Markdown trace page is derived output. It never becomes an authority source. A current query remains the current-state authority.

The [quality-assurance status](quality-assurance-status.md) is generated from the [quality-assurance registry](quality-assurance-registry.md). Registry success establishes static consistency, current content bindings, command presence, and report bytes only; it never substitutes for an inventoried child lane's effectful result or `devenv test` exit status.

The [production-readiness assurance statement](production-readiness.md) and `assurance/v1/obligation-ledger.tsv` provide the finer P3 accounting authority. Ledger closure means every identity has exact evidence or an explicit `not-applicable`, `unsupported`, or `blocked` disposition. It does not mean production readiness is green; only the separate ledger-green check may establish that no blocking disposition remains.

## Accepted trust boundaries

`devenv.lock` is the authority for Nix input revisions and content hashes. `Cargo.lock` is the authority for Cargo versions, sources, and checksums.

Nix and Cargo verify the pinned content. Git records the provenance of repository files. No second inventory copies these pins.

| Boundary | Trusted for | Explicitly not established |
|---|---|---|
| Repository-owned configuration and review records | Canonical selectors, approvals, policy/process facts | Provider identity, reviewer authenticity beyond configured repository facts |
| `pulldown-cmark`, `toml`, and `syn` pins | Syntax nodes and spans under tested supported forms | Natural-language meaning, macro expansion semantics, compiler acceptance |
| Repository source graph and immutable binding store | Current reachability, exact source navigation, authority correspondence, and impact edges | Coverage, execution, proof, approval, correctness, or semantic correspondence |
| Deterministic core and the closed digest registry | Values and transformations tested for the selected registry member | Total correctness outside tested functions, authenticity, or collision impossibility |
| Dylint and bare-metal build | Fixture-proven `no_std`, no ambient mutable statics, no callable public inputs | Referential transparency or semantic correctness |
| Pinned local LLVM coverage lane | Structural source execution under the current inventory, commands, artifacts, and host | Behavioral completeness, other-host execution, rustc/LLVM/OS correctness, or absence of defects |
| Hegel-rs sampled runs | The committed 256 cases, seed, domain, production/reference bindings, and assertions for each declared property | Exhaustiveness, formal proof, reference correctness, or prose-to-property correspondence |
| Independent finite models | Every reachable state in each exact declared finite model/digest | States or events outside those bounds |
| Verus and bundled Z3 | Named mathematical obligations over the bound Verus representation | Verus/Z3 soundness, representation correspondence, rustc, LLVM, machine code |
| Charon, Aeneas, bound minimal model, Lean kernel | Equivalent phase-4 generated-model spike | Full Aeneas backend, arbitrary generated constructs, translation faithfulness, native code |
| Shell filesystem/process APIs | Effectful snapshot and oracle observations normalized by tests | Host kernel correctness, raw-log canonicality, clock/network authority |
| Caller-supplied mount mappings | Physical directories for declared logical mount IDs | Repository ownership, origin, authenticity, or authorization |
| Opaque artifact identities and sidecars | Exact saved-byte identity, current navigation, and impact | Language meaning, build use, runtime behavior, coverage, proof, or correctness |
| Exact Cargo/Nix/Git/upstream integrity fields | Opaque pass-through to the ecosystem that defines and verifies each field | Buoy semantic identity, cryptographic review authority, or broad hash-string exceptions |

## Formal assumptions

The selected primary lane is Verus. `proofs/core/proof-record.toml` binds exact implementation, proof, negative-control, verifier, solver, configuration, assumption, scope, and exclusion identities. Its key assumptions are:

- the named representation corresponds to the named safe sequential Rust functions;
- mathematical value semantics are appropriate for those functions;
- explicit input values are complete;
- Verus's encoding and bundled Z3 are sound for accepted obligations; and
- the repository-owned formalization approval correctly identifies the intended prose relation.

Exact source digests and mutation controls detect drift but do not prove representation correspondence. Formalization approval is review-only. The symbol-level mappings live in `proofs/core/derivation-record.toml`, and the exact reviewed scope lives in `proofs/core/correspondence-approval.toml`. The detached canonical field-identity theorem had neither a shared/generated production representation nor an approved encoder mapping, so it and its mapping were removed from the selected production theorem inventory instead of being promoted or dispositioned as satisfied; its mathematical helper remains explicitly unselected.

## Evidence-class scopes

- `linked`: one current syntax-attached carrier and exact atom revision.
- `tested`: all required ordinary carriers are current, selected, passing, and limited to their exact corpus/configuration scope.
- LLVM `structural-source-execution`: all four integer metrics are exact over the current reviewed source/artifact/platform scope; this remains structural rather than behavioral evidence.
- Hegel `sampled`: the exact committed cases, seed, domain, settings, and assertion passed (or the declared false control was rejected); this does not become exhaustive or formal evidence.
- `exhaustively-checked`: one named finite model reached a fixed point under its exact state/event/bound digest.
- `formally-proved`: one approved property has complete fresh bindings, accepted positive proof, intended negative-control rejection, assumptions, and exclusions.
- `structurally-enforced`: one named checker passed under its exact detector scope.
- `review-only`: one repository-owned approval or exact reaffirmation exists.

No class implies another.

## Digest algorithms

Schema v3 binds one active registry member into the `spec-provenance/v3` preimage. The closed registry contains these members:

- The `xxh3-128` member uses tag `2`, 16 bytes, and the `change-detection` class.
- The `blake3-256` member uses tag `3`, 32 bytes, and the `collision-resistant` class.
- The `sha-256` member uses tag `4`, 32 bytes, and the `collision-resistant` class.

Tags `0` and `1` are obsolete. The parser rejects these tags and all unregistered tags.

The `change-detection` class does not establish collision resistance. The `collision-resistant` class does not establish authenticity, authorization, tamper evidence, or proof.

Each repository configuration selects exactly one available member. A member change regenerates identities from authoritative source bytes. It never relabels existing digest bytes.

`assurance/v1/digest-correspondence-index.tsv` maps stable object IDs across members. The index is derived evidence only. Canonical graph, report, and freshness paths do not read it.

## Effects and exclusions

Canonical static graph identity excludes physical mount paths, network state, provider state, clocks, locale, traversal order, process IDs, terminal dimensions, and raw logs.

Oracle invocation is effectful. Only its normalized verdict and explicit semantic receipt can enter deterministic execution reporting.

Buoy trusts each caller-supplied mount mapping. Descriptor-relative reads verify selected bytes, but they do not verify repository ownership or origin.

An opaque artifact identity establishes exact saved-byte correspondence only. A sidecar adds navigation and freshness, not semantic correspondence.

The release makes no claim that it verifies:

- natural-language truth or prose-to-code equivalence;
- correctness from a fresh implementation binding;
- host-specific capability semantics, including Chelis rows;
- parser libraries beyond supported tested forms;
- operating systems, filesystems, process schedulers, or link classification APIs, including the currently blocked Windows handle-relative snapshot and Job Object backends;
- unsafe code, concurrency, network protocols, or external services;
- rustc, LLVM, emitted C/HIP/Metal, or machine code;
- cryptographic reviewer identity; or
- unsaved editor buffers.

## Manual prerequisites

A repository must deliberately allocate IDs, configure authority roots and approval authorities, commit review records, pin semantic tools, author finite domains without implicit filtering, attach exact carriers, materialize neutral change manifests, and review formalization correspondence. The tool cannot infer these governance decisions from provider state.
