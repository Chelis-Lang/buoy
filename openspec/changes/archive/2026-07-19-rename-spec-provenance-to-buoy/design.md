## Context

The repository currently uses “Spec Provenance” across Cargo packages and crate imports, the `spec-provenance` executable, Nix/devenv commands, documentation, paths, fixtures, proof bindings, and planned Chelis integration. The same words also appear in frozen v1 schema IDs, semantic-digest domain separators, result protocols, diagnostic fixtures, and reviewed canonical report bytes. Those two classes cannot be renamed as if they had the same compatibility cost.

The rename is cross-cutting but does not alter system semantics. The pure functional core remains `no_std`; the imperative shell continues to own parsing, repository I/O, process execution, environment handling, and presentation. Static facts, freshness, execution verdicts, and assurance classes remain orthogonal.

Because production hardening and its focused cargo-llvm-cov/Hegel-rs slice are still in progress, this change should be applied before completing that focused slice, the remaining hardening closure, or Chelis work. Every source-, path-, command-, package-, or presentation-bound artifact made stale by the rename must then be regenerated and revalidated rather than inferred current.

## Goals / Non-Goals

**Goals:**

- Establish **Buoy** as the only current product brand and primary user/developer interface.
- Rename primary Cargo packages/crates, CLI binaries, operational commands, documentation, and integration planning coherently.
- Preserve byte-identical frozen v1 canonical artifacts and historical readers.
- Provide deterministic, explicitly bounded compatibility aliases for old operational and package-facing names where feasible.
- Make all intentionally retained `spec-provenance` strings machine-auditable.
- Replay source/path/name-bound evidence after the rename.
- Reconcile the focused coverage/Hegel contract to Buoy identities and account for its completed obligations exactly once within production-hardening P0.

**Non-Goals:**

- Renaming frozen `spec-provenance-*/v1` wire/schema identifiers or semantic-digest domains.
- Changing graph, report, evidence, proof, policy, oracle, or adapter semantics.
- Adding ambient effects to `buoy-core` or reversing the one-way shell-to-core dependency.
- Treating renamed tests, annotations, proof files, or compatibility wrappers as new assurance.
- Implementing unfinished coverage/Hegel behavioral obligations beyond rename-induced migration and evidence replay.
- Implementing Chelis integration or changing Chelis compiler behavior.

## Decisions

### 1. Separate product identity from frozen protocol identity

Current branding SHALL become `Buoy`, `buoy`, `buoy-core`, and `buoy-*` as appropriate. Frozen v1 identifiers beginning with `spec-provenance-`, stable diagnostics, digest domain strings, result protocols, fixture schemas, and already reviewed canonical bytes SHALL remain unchanged and SHALL be documented as compatibility identifiers.

This is preferred over a global text replacement because changing a domain separator or schema ID changes semantic digests and historical bytes. A future protocol namespace change would require a separately versioned schema migration, dual readers, and new goldens.

### 2. Use a reviewed naming inventory and allowlist

Implementation begins by generating a machine-readable inventory of every old-name occurrence with one disposition:

- `rename-product`: current user/developer-facing branding;
- `compatibility-alias`: temporary old executable/package/operational alias;
- `frozen-v1-identifier`: immutable canonical or historical identifier;
- `historical-reference`: prose explicitly discussing the former name;
- `generated-or-external`: governed by a named generator or upstream source.

A stale-name checker fails on any occurrence absent from the inventory, any expired alias, or any frozen identifier changed from its reviewed bytes. This is preferred over a broad grep exclusion because mixed branding and accidental canonical changes must fail closed.

### 3. Rename primary Rust and command surfaces, with thin compatibility wrappers

The root package/binary becomes `buoy` and the pure crate becomes `buoy-core` (`buoy_core` in Rust). Test-only independent authority becomes `buoy-reference`. Where local compatibility is technically feasible, separate thin packages/binaries retain `spec-provenance` and `spec-provenance-core` for one declared compatibility window and only delegate/re-export the new primary implementation.

Compatibility wrappers live in the imperative shell or facade layer, never in `buoy-core`. They emit a deterministic deprecation diagnostic outside canonical report bytes and preserve exit classes and authoritative binary output. They cannot establish a second implementation or assurance class. The first renamed release is `0.2.0`; aliases are supported for `>=0.2.0,<0.3.0` and become blocking expired artifacts at `0.3.0`. Package publication and external registry ownership are release concerns; repository-local compatibility behavior is testable offline.

### 4. Preserve architecture and deterministic behavior

`buoy-core` remains `#![no_std]`, safe, sequential, and free of filesystem, environment, clock, locale, network, parser, subprocess, and provider dependencies. `buoy` depends inward on `buoy-core`; compatibility crates depend on Buoy and never the reverse.

Authoritative inputs, total ordering, digest bindings, lifecycle transitions, and trust boundaries do not change. Both deterministic static checking and effectful oracle execution are exposed through the `buoy` command, but process logs, timing, PIDs, paths, scheduling, and deprecation messages remain noncanonical. Exact old/new CLI parity tests compare canonical bytes separately from noncanonical stderr.

### 5. Keep canonical v1 corpora byte-identical

Before renaming, the implementation records the reviewed canonical vector/report corpus digests. After each rename phase, independent vectors, strict readers, historical-reader fixtures, and presentation bindings are replayed. Any canonical byte difference is a blocking migration error rather than an expected rename effect.

The old brand may remain inside canonical v1 bytes only where the frozen schema already requires it. New human and JSON presentation labels use Buoy only when those labels are explicitly noncanonical or presentation-versioned.

### 6. Treat rename-induced evidence drift as staleness

Cargo metadata, source paths, implementation artifact digests, proof records, derivations, source inventories, coverage objects, fuzz/mutation campaign bindings, and supply-chain closure records are rebound and replayed. No pre-rename proof, coverage result, campaign, or aggregate green status is copied forward merely because code behavior appears unchanged.

Verus/Aeneas evidence retains its existing limitations: it applies only to named safe sequential pure representations and exact bindings; it does not prove natural-language correspondence, parser/filesystem behavior, rustc/LLVM correctness, or host integrations. Human correspondence approvals remain required.

### 7. Sequence other in-flight changes explicitly

This rename is implemented before the focused cargo-llvm-cov/Hegel-rs testing change, before resuming the broader unfinished production-hardening closure, and before Chelis integration. The focused testing change remains a bounded slice of production-hardening P0: its `LCOV-*` and `HEGEL-*` capabilities remain independently visible, while completed tasks and evidence satisfy mapped hardening obligations exactly once rather than being duplicated.

During the rename, the focused testing artifacts are reconciled to the `buoy`, `buoy-core`, and `buoy-reference` identities and to primary commands `devenv tasks run buoy:coverage` and `devenv tasks run buoy:properties`. The former `spec-provenance:coverage` and `spec-provenance:properties` commands may remain only as tested, time-bounded compatibility aliases. Rename reconciliation does not mark unfinished coverage, property, or hardening work complete.

The hardening and Chelis planning artifacts are likewise updated only for noncanonical product/package/command names. Frozen v1 identifiers and semantics in all in-flight changes remain unchanged. Chelis continues to consume a one-way adapter and does not change compiler semantics for Buoy.

## Risks / Trade-offs

- **Accidental canonical namespace replacement** → Freeze pre-rename bytes, use an exact retained-identifier allowlist, and run independent corpus parity after every phase.
- **Mixed branding remains in obscure scripts or generated files** → Inventory all tracked sources and fail on unclassified old-name occurrences.
- **Compatibility aliases become permanent** → Bind them to one explicit compatibility window, add expiry fixtures, and keep their removal as a blocking migration task.
- **Cargo package rename breaks downstream manifests** → Document package/import mapping and provide repository-local facade crates where dependency direction remains one-way.
- **Proof or campaign evidence appears current after path changes** → Invalidate and replay every source/path/tool-bound record; no cross-class or hash-only inference.
- **Repository directory/remote cannot be renamed atomically by code changes** → Keep filesystem root and provider state noncanonical; document the external repository-slug step separately.
- **Concurrent OpenSpec changes drift or double-count evidence** → Reconcile the focused coverage/Hegel, broader hardening, and Chelis plans immediately after the rename; map overlapping P0 obligations exactly once; and validate all changes strictly.

## Migration Plan

1. Freeze current canonical v1 vectors and generate the complete old-name inventory.
2. Add Buoy package/crate/binary names and compatibility wrappers while old names still work.
3. Migrate internal imports, tests, scripts, Nix/devenv tasks, docs, examples, and generated presentations to Buoy.
4. Update the focused coverage/Hegel, broader hardening, and Chelis planning artifacts without rewriting frozen v1 identifiers or marking unfinished work complete; map completed focused obligations exactly once into hardening P0.
5. Rebind proof, source, coverage, campaign, and supply-chain records; replay their exact authorities. A replayed nonpassing lane remains explicit and owned by its existing hardening change rather than being falsely completed by rename bookkeeping.
6. Make the stale-name checker blocking and verify only allowlisted compatibility/historical identifiers remain.
7. Complete the compatibility window, remove aliases in a separately reviewable step, and retain historical readers.

Rollback restores primary operational/package names and wrappers, but SHALL NOT rewrite canonical v1 bytes or accepted historical artifacts. A rollback is incomplete unless its naming inventory and source-bound evidence are current.

## Open Questions

- The external Git repository slug and crates.io ownership/availability must be confirmed during release preparation; neither is an authoritative static input.
- A future `buoy-*/v2` protocol namespace, if desired, requires a separate OpenSpec change and is not implied by this rename.

## Acceptance Oracle

The exact acceptance oracle is `devenv test --no-tui` from a clean, network-disabled checkout. Success requires exit code 0, strict OpenSpec validation, exact canonical v1 corpus parity, primary and compatibility command tests including the renamed coverage/property lanes, stale-name inventory closure, architecture checks, single-accounted hardening obligations, and current source-/proof-/coverage-/campaign-bound evidence; presentation or deprecation text cannot substitute for any assurance class.
