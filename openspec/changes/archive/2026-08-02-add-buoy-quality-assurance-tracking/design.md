## Context

Buoy already has several overlapping sources of quality-assurance information. `devenv.nix` pins tools and defines focused commands; `assurance/v1/toolchain.toml` records selected tool identities; source, coverage, property, proof, and phase records describe narrower evidence; documentation states trust boundaries; and active OpenSpec changes own unfinished work. These sources show that Hegel-rs and Dylint are implemented, while tools such as `cargo-fuzz` and `cargo-mutants` may be installed before their campaigns are implemented or made blocking. There is no single truthful answer to “what QA is implemented?”

The tracker is repository engineering metadata, not part of Buoy's product graph or canonical product report. Maintainers are its authors and reviewers. Contributors consume its generated report. The design remains deterministic and offline. Heavy coverage, fuzz, mutation, and full acceptance commands run locally.

The root crate and imperative test shell may read repository files, task definitions, tool bindings, OpenSpec ownership references, and normalized evidence. `buoy-core` remains safe, sequential, pure, and `#![no_std]`; it MUST NOT depend on the registry, TOML parsing, filesystem APIs, process execution, Hegel-rs, Dylint, coverage tools, or any generated QA report. The existing one-way dependency from the shell toward the core does not change.

## Goals / Non-Goals

**Goals:**

- Provide one versioned, machine-checked inventory of implemented, planned, blocked, and retired QA controls.
- Keep tool availability, implementation, enforcement, freshness, execution verdict, and assurance class independent.
- Generate a deterministic report that answers what exists, how to run it, whether it is blocking, what evidence supports it, and what it does not establish.
- Map each control to exactly one implementation owner while allowing one control to reference several orthogonal evidence records.
- Fail closed when a registry claim is unsupported, stale, ambiguously owned, or broader than its trust boundary.

**Non-Goals:**

- No implementation of the controls being inventoried.
- No second assurance-obligation ledger and no replacement for source, property, coverage, proof, campaign, or phase records.
- No dynamic execution of every inventoried lane by the registry validator.
- No promotion of tests, coverage, Dylint, Hegel samples, finite models, or formal source proofs beyond their exact scopes.
- No product API, schema, canonical graph, or core dependency change.

## Decisions

### 1. Store orthogonal control facts instead of one progress status

Add `assurance/v1/quality-assurance-controls.toml` with a schema identity and one row per stable `QA-*` control. Each row records:

- stable ID, name, category, scope, and implementation owner;
- implementation state: `planned`, `available`, `implemented`, `blocked`, or `retired`;
- enforcement state: `none`, `advisory`, `focused`, or `blocking`;
- exact focused command when applicable;
- tool-binding IDs and normalized repository-relative implementation, configuration, and evidence paths;
- assurance class, trust statement, assumptions, exclusions, and non-claims;
- optional normalized execution-result authority, never an ambient raw log; and
- references to owning and related OpenSpec obligations without using task checkboxes as implementation evidence.

`available` means the tool is pinned or exposed but no complete project control is claimed. `implemented` requires executable project behavior and positive/negative controls appropriate to its scope. `focused` means a stable focused command exists. `blocking` additionally means the lane is an explicit part of `devenv test`. Freshness and execution verdict are derived from bound evidence and are never encoded by overloading these states.

Alternative considered: one `planned/in-progress/done` field. Rejected because an installed tool, committed implementation, current run, and gate membership can change independently.

### 2. Keep the registry in the imperative repository shell

Implement a focused root-package validator/generator, exposed as `buoy-quality`, that reads the registry and repository-owned assurance inputs. It may reuse deterministic value and digest utilities from `buoy-core`, but no registry, parser, filesystem, process, or tool-specific type enters `buoy-core`. The generated report is repository engineering documentation and not a canonical Buoy graph or product assurance report.

Alternative considered: model the tracker as product evidence objects. Rejected because project tool adoption is not a repository-independent product semantic and would blur OpenSpec's reference-only boundary.

### 3. Validate claims from explicit authoritative inputs

Authoritative inputs are the registry schema and rows, referenced repository-relative file bytes, `devenv.nix`, `devenv.lock`, Cargo locks/manifests, tool bindings, focused assurance manifests/results, trust-boundary documentation, and named OpenSpec ownership artifacts. Host paths, directory traversal order, timestamps, process IDs, raw terminal output, network state, and provider metadata are excluded.

The validator normalizes UTF-8 repository-relative paths, rejects path traversal and duplicates, sorts controls by canonical ID bytes and bindings by `(control ID, path)`, and computes committed content bindings using the repository's selected deterministic digest policy. Ecosystem hashes, package checksums, Git commits, and Nix hashes remain opaque fields interpreted only by their owning ecosystems. A generated binding record and report become stale whenever a referenced authoritative input changes.

Alternative considered: trust manually entered status and prose links. Rejected because they can drift while still rendering a convincing dashboard.

### 4. Require evidence appropriate to each claimed state

State validation is fail closed:

- `available` requires a current pinned tool binding or exact project configuration.
- `implemented` requires implementation/configuration paths, a runnable focused or containing command, and applicable positive and negative controls.
- `focused` requires the exact command to resolve to a repository-defined task or script.
- `blocking` requires structural registration in the exact final gate and a gate-omission negative control.
- a current execution verdict requires a normalized source/tool/configuration-bound result; absence yields `not-run`, not `pass`.
- `blocked` requires a repository-owned blocker and discharge condition; `retired` requires a replacement or explicit terminal rationale.

Static validation proves only registry consistency and freshness. It does not execute Hegel, Dylint, LLVM instrumentation, mutants, fuzzers, formal tools, or other child lanes. Local `devenv test` executes blocking lanes directly.

Alternative considered: infer implementation solely from command names or checked task boxes. Rejected because a command may be a stub and a planning checkbox is not execution evidence.

### 5. Seed one portfolio without taking ownership from focused changes

The initial registry covers code hygiene, compilation and ordinary tests, structural enforcement, deterministic fixtures and references, sampled properties, source coverage, finite models, formal lanes, adversarial campaigns, platform/runtime checks, and supply-chain/reproducibility controls. It includes implemented controls such as rustfmt, Clippy, Cargo tests, OpenSpec validation, Dylint FCIS, bare-metal `no_std` compilation, Hegel-rs, LLVM coverage, frozen fixtures/goldens, independent references/models, Verus, and Aeneas/Lean. It also records available or planned controls such as nextest adoption, fuzz campaigns, `cargo-mutants`, Miri/sanitizers, cross-platform campaigns, `cargo-deny`, and clean-machine closure according to repository evidence at implementation time.

Each implementation obligation has one owner. This change owns registry schema, validation, report generation, and gate integration only. Existing focused and hardening changes continue to own their QA mechanisms. Cross-references are informative mappings and MUST NOT automatically complete either change or count the same evidence twice.

Alternative considered: copy all QA implementation tasks into this change. Rejected because it would create conflicting completion state and duplicate evidence ownership.

### 6. Generate one deterministic maintainer report

`docs/assurance/quality-assurance-status.md` is generated from validated inputs. It is ordered by category and control ID and shows implementation state, enforcement, command, evidence freshness, last normalized execution verdict if present, owner, assurance scope, and limitations. It contains no wall-clock “last updated” value, host path, duration, or manually editable status cell. A check mode fails on generated-report drift.

Alternative considered: maintain a hand-edited Markdown checklist. Rejected because prose status can disagree with machine-readable authority and silently lose retired or blocked controls.

### 7. Use a focused static lane and one exact final oracle

The stable focused command is:

```text
devenv tasks run buoy:quality
```

It runs positive/negative registry fixtures, validates the live registry and bindings, regenerates the report in a temporary location, and compares it byte-for-byte with the committed report. It succeeds only when all registry claims are structurally supported and deterministic. It does not claim that every referenced effectful QA lane passed.

The exact implementation acceptance oracle is:

```text
devenv test
```

Success requires exit status 0 after `buoy:quality` is included in the aggregate gate and its gate-omission negative control passes, in addition to all other lanes already required by that oracle. A green `buoy:quality` alone is not production acceptance.

## Risks / Trade-offs

- **[Risk] The registry duplicates facts from toolchain, evidence, and OpenSpec files.** → Store references and state classifications, generate digest bindings, and require one ownership mapping instead of copying detailed evidence.
- **[Risk] Static command inspection reports a wrapper as blocking when it skips work.** → Require gate-omission controls. Registry success never substitutes for local lane success.
- **[Risk] Frequent source edits make bindings noisy.** → Bind only authoritative implementation/configuration/evidence paths named by each control and regenerate deterministically through one command.
- **[Risk] A broad inventory creates assurance inflation.** → Require explicit assurance class, scope, trust statement, assumptions, exclusions, and non-claims for every row and reject cross-class inference.
- **[Risk] Active changes disagree about ownership or completion.** → Require one owner per obligation, treat related changes as references, and reject task checkboxes as evidence of implementation or execution.

## Migration Plan

1. Add positive and negative fixtures for schema, state, ordering, path, ownership, freshness, gate, and no-laundering rules.
2. Implement the parser, validator, deterministic binding generator, report renderer, and self-test mode in the root package.
3. Audit current repository controls and seed the registry from executable commands and bound evidence, conservatively classifying unsupported claims as `available`, `planned`, or `blocked`.
4. Generate and review the initial status report, then add maintainer update documentation.
5. Add `buoy:quality` and its omission control to `devenv test`; run the focused command and exact final oracle.

Rollback removes the focused gate entry, validator, generated report, fixtures, and registry together. Existing QA implementations and their evidence remain unchanged, and no historical registry report may be presented as current after rollback.

## Open Questions

None. The state model, ownership boundary, deterministic report, focused command, and final acceptance oracle are fixed by this design.
