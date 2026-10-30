## Context

Buoy uses `devenv test` as its only final acceptance oracle. The current implementation duplicates acceptance logic in `buoy-gate` and `enterTest`.

The current configuration defines 36 `buoy:*` tasks without dependency edges. Phase scripts call focused scripts directly in one serial shell process.

The configuration also constructs the primary Rust toolchain outside `languages.rust`. Specialized assurance tools enter every interactive shell.

The local-only policy keeps heavy acceptance outside CI. The local Devenv environment therefore owns command identity, task order, package identity, and final status.

This change affects the imperative shell and repository configuration only. `buoy-core` remains `no_std` and receives no Nix, filesystem, process, or task dependency.

## Goals / Non-Goals

**Goals:**

- Define one resolved task graph for final acceptance.
- Preserve one execution of every required child in a reviewed serial order.
- Keep `devenv test` as the exact final oracle.
- Make phase commands use Devenv dependency behavior.
- Use the Rust language module as the primary toolchain authority.
- Keep Dylint and Verus Rustup aliases inside the same pinned closure.
- Split configuration by ownership area.
- Give each repository task one explicit root.
- Install fast Git hooks from pinned packages.
- Move specialized tools from the common shell to script runtime sets.

**Non-Goals:**

- No product model, schema, report, or digest behavior changes.
- No new service, process, container, database, network dependency, or CI acceptance job.
- No task cache for blocking acceptance work.
- No parallel execution until an independent-write review approves specific edges.
- No removal of compatibility task names.
- No proof claim from a task, hook, test, or configuration check.
- No change to the Verus or Aeneas proof scope and exclusions.

## Decisions

### 1. The Rust language module owns the primary toolchain

The root module enables `languages.rust` and sets `languages.rust.toolchainFile = ./rust-toolchain.toml`. Other modules use `config.languages.rust.toolchainPackage`.

The toolchain file keeps the current channel, profile, compiler components, and target. It adds `rust-analyzer` as a development component.

A shell setup task creates the named Rustup links for Buoy, Dylint, and Verus. Each link points to its declared Nix package.

The setup task runs before `devenv:enterShell`. It also runs before `devenv:enterTest` through the built-in lifecycle dependency.

Direct construction through `rustPkgs.rust-bin.fromRustupToolchainFile` is removed. `rust-overlay` remains a pinned input because the language module uses it.

**Alternatives considered:**

- Keep the manual toolchain construction. Rejected because it duplicates supported Devenv behavior and omits language-module integration.
- Remove Rustup aliases. Rejected because Dylint and Verus use Rustup names for tool discovery.

### 2. Local imports define configuration ownership

The root `devenv.nix` retains static imports and the shared package construction. It passes shared values to imported modules through `_module.args`.

The initial module layout is:

```text
nix/devenv/environment.nix
nix/devenv/hooks.nix
nix/devenv/tasks.nix
nix/devenv/scripts/core.nix
nix/devenv/scripts/assurance.nix
nix/devenv/scripts/adversarial.nix
nix/devenv/scripts/acceptance.nix
```

`environment.nix` owns environment values, common packages, generated files, and shell setup. Each script module owns distinct stable executables.

`tasks.nix` owns focused task declarations, dependency edges, compatibility aliases, and `cwd`. `hooks.nix` owns only Git hook configuration.

`acceptance.nix` owns baseline scripts and graph-policy data. It does not duplicate phase child script bodies.

**Alternatives considered:**

- Keep one large file. Rejected because source-text checks already block composition and hide ownership.
- Put each script in one file. Rejected because more than 30 small modules add navigation cost without a separate owner.

### 3. One serial task graph defines acceptance

The graph preserves the present command order. Every ordering rule uses `after` or `before` and the default `@succeeded` state.

The baseline chain contains these task groups:

1. Tool version checks
2. Devenv workflow checks
3. Digest member check
4. Rust and Nix format checks
5. Workspace Clippy checks
6. Workspace tests
7. Workspace documentation
8. Product identity checks

P0 starts after the baseline. Each P0 child has a serial edge, and `buoy:p0` depends on all owned P0 children.

The same pattern defines P1, P2, and P3. The first child of each later phase depends on the prior phase task.

Strict OpenSpec checks run after P3. `buoy:gate` is the graph sink and runs before `devenv:enterTest`.

`buoy:gate` and phase tasks contain no duplicate child commands. They report success only after their declared dependencies succeed.

A command such as `devenv tasks run buoy:p1` now runs its complete upstream subgraph. This behavior makes each later handoff cumulative and monotonic.

Individual child commands remain focused. For example, `devenv tasks run buoy:hashing-policy` runs only its declared child subgraph.

Compatibility tasks remain disconnected aliases. They call the matching stable script and do not become downstream nodes of final acceptance.

**Alternatives considered:**

- Keep phase scripts as serial orchestrators. Rejected because Devenv cannot display or inspect the hidden dependency graph.
- Run independent children in parallel. Rejected because coverage, generated evidence, Cargo targets, and temporary state need an independent-write review.
- Connect aliases to primary tasks. Rejected because `devenv test --mode all` also runs connected downstream aliases.

### 4. A typed checker reads the resolved task file

A new root binary, `check-devenv-task-graph`, reads `DEVENV_TASK_FILE`. An explicit argument supports self-tests and fixture tests.

The script `buoy-devenv` runs the checker self-tests and the live check. The stable focused task is `devenv tasks run buoy:devenv`.

The parser converts each JSON task into a typed task name, task type, command state, root, dependency set, and cache state. Invalid input fails at this boundary.

A repository policy file declares the baseline order, phase order, child ownership, graph sink, and lifecycle node. The checker compares policy with resolved tasks.

The checker rejects these states:

- An unknown task dependency
- A dependency cycle
- A missing baseline or phase
- A child with zero or multiple phase owners
- A nested phase owner
- A direct focused child under the final sink
- A missing path to `devenv:enterTest`
- An acceptance task with `status` or `execIfModified`
- A repository task without `cwd = config.git.root`

The checker normalizes the configured root before any report or digest calculation. Diagnostics sort by task-name bytes and contain no host absolute path.

`buoy-gate-wiring` becomes a stable wrapper for this binary. Its existing command name and negative-control role remain unchanged.

**Alternatives considered:**

- Continue to parse `devenv.nix` text. Rejected because local imports make source location unrelated to resolved behavior.
- Parse `devenv tasks list` output. Rejected because presentation text is not a stable machine interface.

### 5. Task roots are explicit

Every `buoy:*` task sets `cwd = config.git.root`. A shared task constructor adds this value before task-specific fields.

Scripts continue to use repository-relative paths. Task results therefore do not depend on the directory from which a maintainer starts Devenv.

The graph checker rejects a missing root. Compatibility aliases use the same root value.

**Alternatives considered:**

- Add `cd` to each script. Rejected because repeated shell code can drift and does not expose task configuration.

### 6. Git hooks remain fast and non-authoritative

`devenv.yaml` adds the pinned `git-hooks` input. `.gitignore` excludes the generated `.pre-commit-config.yaml` link.

The hook set checks Rust format, Nix format, and strict OpenSpec artifacts. Each hook uses a declared package and disables filename arguments when required.

The acceptance baseline runs equivalent or stronger checks. Hook success never changes a QA execution verdict or an assurance class.

Coverage, fuzz, mutation, formal, adversarial, and full acceptance commands stay outside the hook set.

**Alternatives considered:**

- Run `devenv test` before each commit. Rejected because the full local gate is too expensive for commit-time feedback.
- Keep hooks unmanaged. Rejected because CI does not provide the local acceptance safety net.

### 7. Specialized tools belong to script runtime sets

The common package list keeps Rust, Rustup, Git, OpenSpec, native build libraries, and common format tools.

Focused scripts receive specialized packages as follows:

- `buoy-fcis` receives Dylint tools.
- Formal scripts receive Aeneas, Lean, Verus, and Z3.
- `buoy-coverage` receives `cargo-llvm-cov`.
- Fuzz scripts receive `cargo-fuzz`.
- Mutation campaign scripts receive `cargo-mutants`.
- Supply-chain scripts receive `cargo-deny` and closure runners.
- The tool-version task receives every tool whose identity it reports.

Aggregate scripts call focused script wrappers. Each wrapper supplies its own runtime package set.

The closure inventory adds an exposure-mode field. Moving a tool out of the common shell does not retire its QA control or evidence identity.

**Alternatives considered:**

- Keep all tools in the common shell. Rejected because every shell receives tools that only one focused lane uses.
- Use profiles for heavy tools. Rejected because the exact command `devenv test` must work without a profile flag.

### 8. Version agreement fails before project work

`devenv.yaml` sets `require_version: true`. The existing lock remains the module and input authority.

A mismatched CLI stops before shell setup, hook setup, task execution, or acceptance output. The supply-chain inventory binds the CLI and module identities separately.

**Alternatives considered:**

- Use a minimum version range. Rejected because a newer task engine can change graph behavior before review.

### 9. Deterministic and assurance boundaries remain unchanged

The authoritative workflow inputs are the Nix modules, `devenv.yaml`, `devenv.lock`, toolchain file, task policy, Cargo locks, and vendor closure.

The generated task JSON is an execution input. It is not a canonical product input and does not change a graph or static report identity.

Normalized workflow bindings exclude store paths, repository absolute paths, timestamps, durations, process IDs, terminal output, and task completion order.

Task success establishes only execution of the named gate. Git hooks, task edges, and source bindings do not prove product correctness.

Verus and Aeneas retain their exact proof bindings and exclusions. This change does not prove rustc, LLVM, native code, translators, or prose correspondence.

The exact acceptance oracle is `devenv test`. Acceptance requires offline execution of the complete graph and exit status `0`.

## Risks / Trade-offs

- Later phase commands run earlier prerequisites. Documentation must state this cumulative task behavior.
- Runtime package wrappers can change `PATH` order. Focused tests must compare resolved executable identities before and after the move.
- The Rust module can add default C and LSP packages. Closure checks must classify each added package.
- Local imports can create module cycles. Shared values must flow through one `_module.args` owner.
- Generated task JSON contains host paths. The checker must normalize roots before report or digest output.
- Git hooks add commit latency. The hook set therefore excludes heavy lanes.
- The first graph remains serial. This choice preserves state behavior but does not reduce final gate duration.
