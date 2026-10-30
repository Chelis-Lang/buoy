# Formal toolchain pins

Status: active pinned v2 formal-toolchain decision, initially recorded 2026-07-16.

## Success boundary

A pin is accepted here only when it has:

- an immutable commit/tag and integrity hash;
- a Nix derivation or locked flake output;
- no live-branch resolution during a formal run;
- a locally realizable executable on a declared host;
- an explicit backend/tool compatibility relation; and
- a recorded trust and claim boundary.

“Latest”, a Git tag without a content hash, an uncommitted `elan`/`rustup` download, source availability without an executable derivation, and a command found elsewhere on `PATH` are not accepted pins.

Nix `hash`/NAR fields, Cargo registry checksums, Git commits, and upstream release checksums in this document are exact external supply-chain metadata interpreted only by their owning ecosystem. They are not `SemanticDigest` values and are never converted to `xxh3-128:` identities.

The repository lock is `devenv.lock`. `devenv.yaml` names only the reviewed Aeneas tag.

The Devenv Rust module reads `rust-toolchain.toml`. `config.languages.rust.toolchainPackage` is the primary Rust package.

A Devenv task creates the named Rustup links before the shell lifecycle. These links use only declared Nix packages.

`nix/formal-tools.nix` defines the Verus distribution and Aeneas-compatible Lean derivations.

Formal tools are script runtime packages. They are not common packages in the default shell.

The focused wrappers supply Aeneas, Charon, Lean, Verus, and Z3. The final task graph realizes these packages without a profile flag.

After closure realization, formal commands run without network access. Initial closure realization can use network access for hash-verified inputs.

## Source build boundary

Ordinary Cargo builds need Rust and the public dependencies in `Cargo.lock`.
They do not need formal tools, an OpenSpec policy store, or SSH credentials.

The offline vendor uses the reviewed Hegel package lock at `nix/locks/hegeltest-c-0.29.0.Cargo.lock`.
The header generator reads this lock during the build.
`buoy:deny` checks the workspace, Dylint, and Hegel graphs with the pinned RustSec database.
The registry snapshots record the `yanked` status for the pinned dependency versions.

All three metadata inputs include every Cargo feature.
Advisory exceptions need an exact RustSec ID and a review reason.
The Nix policy rejects the wildcard fixture before Cargo-deny runs.

`nix/tools/declared-source.nix` reads the root Nixpkgs and Rust overlay pins from `devenv.lock`.
The consumer package and the development shell use the same source pins.
The consumer package verifies the reviewed source digest before a build.
A source change needs an explicit digest update and review.
The source filters exclude local credentials, assistant state, and extra worktrees.
Example environment files remain source inputs.
Generated hook files, toolchain links, and fuzz artifacts stay outside consumer source exports.

The source filter resolves the shared root before it compares exact paths.
Physical and alias roots retain the same declared bytes.
Root-only exclusions do not remove valid nested copies.

The committed Lean compiler outputs and `generated-tmp` tree are historical proof artifacts.
The formal commands compile fresh modules in scratch space.
They do not use these committed binaries as proof evidence.
The Nix binary package retains the root MIT, Chelis, Tree-sitter, and Unicode notices in `share/licenses/buoy`.
This set does not cover every linked Cargo dependency.
A binary release needs a separate dependency license review.

`buoy:consumer-package` is an optional private-source development check.
The final acceptance graph does not include this task.
The formal tasks remain mandatory within the full acceptance graph.
The retained GitHub workflows do not run assurance tasks under the local-only policy.

## Selected candidates

### Verus lane

- upstream: `verus-lang/verus`;
- release/tag: `release/0.2026.07.12.0b42f4c`;
- source commit: `0b42f4cee92a178937608cf55e512371d2fd8cd4`;
- license: MIT;
- delivery: upstream release distribution, fetched by Nix with a fixed hash;
- required Rust toolchain from the distribution's `version.json`: `1.96.0-<rustc-target>`;
- Rust delivery: `formalTools.verusRustToolchain` uses the root Rust overlay pin in `devenv.lock`.
- verifier command: `verus`, wrapped with the exact `RUSTUP_TOOLCHAIN` and a repository-local rustup symlink;
- SMT solver for Verus: the `z3` executable bundled inside the same distribution.

Pinned archive identities:

| Nix system | Archive | Raw SHA-256 | Nix SRI |
|---|---|---|---|
| `aarch64-darwin` | `verus-0.2026.07.12.0b42f4c-arm64-macos.zip` | `212da149c07a4621c13d032b0a6af50b2bcb383322d3d65d0120ec436a864b6f` | `sha256-IS2hScB6RiHBPQMrCmr1CyvLODMi09ZdASDsQ2qGS28=` |
| `x86_64-darwin` | `verus-0.2026.07.12.0b42f4c-x86-macos.zip` | `c2266658484637bb80f9c09adca042a5568f8c8cf292d405d3c79de2e5d2a56b` | `sha256-wiZmWEhGN7uA+cCa3KBCpVaPjIzyktQF08ed4uXSpWs=` |
| `x86_64-linux` | `verus-0.2026.07.12.0b42f4c-x86-linux.zip` | `f6f4f5d08e07d3e1ad721d775bda5ba96b9dd0c73b48fc17f2e071866fbd01c0` | `sha256-9vT10I4H0+Gtch13W9pbqWud0Mc7SPwX8uBxhm+9AcA=` |

The GitHub release is not marked immutable, so the Nix hash—not the mutable release metadata—is the acceptance control. Unsupported Nix systems fail evaluation instead of selecting another build.

Verus proves supported Verus/Rust source obligations under its encoding and bundled solver. This does not prove Verus, Z3, rustc, LLVM, or native output.

### AeneasVerif lane

- upstream: `AeneasVerif/aeneas`;
- tag: `nightly-2026.07.01`;
- commit: `678ffd3b797b2a6d8e3568a0bfdd5d027038afad`;
- source NAR hash: `sha256-uQAGj3moRftf1OWIuzfRoFsO/tv0Hhx3X/8qRU0yOqk=`;
- license: Apache-2.0;
- delivery: the locked upstream `aeneas` package overridden only to use the offline-hardened Charon derivation in `nix/formal-tools.nix`;
- translator command: `aeneas`;
- source/backend library: `$AENEAS_SOURCE` and `$AENEAS_LEAN_BACKEND` from the same locked input.

The Aeneas source declares and its flake locks the compatible Charon revision:

- upstream: `AeneasVerif/charon`;
- tag at selection time: `nightly-2026.07.01`;
- commit and `charon-pin`: `74166d896db4ae4f63101c13c22072fb2022092e`;
- source NAR hash: `sha256-luHH/Bj5MfeIrjkxBP4sCNa4mQZwJLcFhsuWaJQP5E0=`;
- license: Apache-2.0;
- extractor command: `charon` from the Aeneas package closure.

The Aeneas pin is not reconstructed from the similarly named `pkgs.ocamlPackages.aeneas`: that package omits the complete Rust-extractor/backend release relation needed by the proof spike. The upstream flake binds the exact Charon extractor and OCaml library.

The upstream Charon wrapper unconditionally realizes an optional `charon-full-mir-sysroots` derivation. Its build attempts a crates.io index update while constructing a Miri sysroot and therefore failed in the Nix sandbox instead of being offline-resolvable. The project override sets Charon's optional `miriSysroots` build argument to `null`, wraps the same pinned `charon-unwrapped` binary with the same pinned Rust toolchain, and injects that package into both `charon-ml` and Aeneas. The selected `charon cargo --preset=aeneas` spike does not use the full-MIR/Miri feature. Full-MIR extraction is an explicit exclusion until upstream vendors that sysroot closure; it cannot be claimed by these proofs.

The upstream `charon-unwrapped` package also runs integration tests that write beneath a read-only Nix source/home and two tests fail with `Read-only file system (os error 30)`. The override disables that package-time check phase without patching source or binaries. This project instead runs a positive Charon→Aeneas translation, committed-output comparison, Lean proof, and negative proof control in task 4.4. Those focused checks do not claim the omitted upstream suite passed.

Aeneas/Charon translate supported safe sequential Rust into LLBC and then backend source. Translation and external models remain trusted/bound artifacts unless separately proved.

### Selected comparative Aeneas backend: Lean 4

Lean is the candidate backend for task 4.4 because Aeneas documents Lean and HOL4 as its most mature backends, the selected Aeneas source contains a Lean tutorial and standard library, and Lean is reproducibly buildable on every project host.

The exact Aeneas `backends/lean/lean-toolchain` requires:

- release/tag: `leanprover/lean4:v4.30.0-rc2`;
- source commit: `3dc1a088b6d2d8eafe25a7cd7ec7b58d731bd7cc`;
- recursive source hash: `sha256-szNuHemvJ5hlbJtyDOGCF/w4xKlLIxbr4QB3BaVPwTI=`;
- license: Apache-2.0;
- derivation: `formalTools.leanAeneas`, overriding the locked Nixpkgs Lean recipe with the exact source.

Lean verifies generated and handwritten Lean terms relative to Lean's kernel and Aeneas libraries. It does not prove Charon/Aeneas translation faithfulness, external Rust models, rustc, LLVM, or machine code.

### Standalone Z3

The formal script runtime also pins standalone Z3 for version probes and non-Verus SMT checks:

- package: `pkgs.z3` uses the root Nixpkgs pin in `devenv.lock`.
- version: `4.16.0`;
- upstream tag: `z3-4.16.0`;
- Nix source hash: `sha256-DnhX3kxggnFmyYwXEPBsBA1rh4oor1oIJR5TMJk/jvc=`;
- license: MIT.

This standalone solver is not silently substituted for Verus's bundled solver. A proof record binds the actual executable used.

## Approach registry

| Family | Mechanism | State | Reason |
|---|---|---|---|
| Verus release distribution | Hash-verified upstream binaries with bundled solver | selected | Small, exact release closure and upstream-supported layout |
| Verus live source build | Build moving `main`/weekly state | rejected | Version and solver closure could drift; unnecessary for the spike |
| Aeneas upstream flake | Lock Aeneas plus its declared Charon commit and backend source | selected | Preserves translator/extractor compatibility and complete backend files |
| Upstream optional full-MIR sysroot | Build Miri sysroots through Cargo during Charon realization | excluded | Attempts live registry access in a Nix sandbox; not used by the Aeneas preset spike |
| Root Nixpkgs OCaml packages | Independently package Aeneas/Charon ML components | rejected for spike | Does not by itself preserve the complete Rust extractor/backend release |
| Lean backend | Exact `lean-toolchain` version and source hash | selected candidate | Mature Aeneas backend, reviewable generated terms, supported host closure |
| F*/Coq/HOL4 | Alternative backend | deferred | More lanes add no discrimination before equivalent Lean spikes are measured |
| Network-resolved elan/rustup | Download tools during proof execution | rejected | Not offline and can resolve mutable metadata |

## Validation commands

Tool identity probe:

```text
devenv tasks run buoy:versions
```

Offline closure probe after initial realization:

```text
devenv tasks run buoy:properties
```

The focused scripts receive the formal runtime packages. An ambient command cannot satisfy the script contract.

This proves each executable's Nix closure is present. The executables themselves perform no dependency resolution; the formal gate separately rejects a missing store path rather than fetching a live tool.

The accepted probe produced Verus `0.2026.07.12.0b42f4c`, standalone Z3 `4.16.0`, Aeneas's build identity (`unknown` because the Nix source has no `.git` directory; the lock commit remains authoritative), a realized Charon executable, and Lean `4.30.0` from tag `v4.30.0-rc2`. The closure query succeeded for every executable.

The v2 spikes prove only their named lifecycle, closure, field-header, envelope-shape, lane-order, parser-shape, and migration-preservation obligations over the bound representations. Exact v2 source digests detect drift but do not prove source-to-model correspondence. Neither Verus nor Aeneas/Lean establishes XXH3 implementation correctness or collision properties, annotation completeness, parser correctness, compiler/backend correctness, or native behavior.
