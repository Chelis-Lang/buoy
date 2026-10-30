## Implementation Tasks

### 1. Task graph controls

- [x] 1.1 Add positive fixtures for detached coverage and mutation campaign tasks.
- [x] 1.2 Add negative fixtures for direct and transitive automatic dependencies on each manual task.
- [x] 1.3 Add a closed manual-task list to the workflow policy parser.
- [x] 1.4 Reject missing manual tasks and automatic reachability with stable diagnostics.

### 2. Devenv graph

- [x] 2.1 Remove `buoy:coverage` from P0 and its automatic predecessor edge.
- [x] 2.2 Add `buoy:coverage-evidence` to P0 without an LLVM runtime package.
- [x] 2.3 Declare `buoy:coverage` and `buoy:mutation-campaign` as manual tasks.
- [x] 2.4 Keep `buoy:mutation-replay` in P1.
- [x] 2.5 Update resolved task fixtures and runtime package ownership.

### 3. Blocking evidence checks

- [x] 3.1 Check coverage source digests and exact metrics in P0 and ledger closure.
- [x] 3.2 Keep mutation artifact bindings, thresholds, and named tripwires in automatic acceptance.
- [x] 3.3 Add controls that prove automatic checks do not start LLVM instrumentation or `cargo-mutants`.

### 4. Contracts and documentation

- [x] 4.1 Update phase handoffs and the task graph policy.
- [x] 4.2 Update the assurance registry, bindings, reports, and owner mappings.
- [x] 4.3 Update coverage, mutation, phase, and acceptance documentation.
- [x] 4.4 Update each repository-owned link from `traceability.md`.

### 5. Focused validation

- [x] 5.1 Run `devenv tasks run buoy:devenv`.
- [x] 5.2 Run `devenv tasks run buoy:mutation-replay` without `cargo-mutants`.
- [x] 5.3 Run `devenv tasks run buoy:ledger-green` without LLVM instrumentation.
- [x] 5.4 Inspect the resolved graph and confirm that both manual tasks are unreachable from `devenv:enterTest`.

## Archive Tasks

- [x] 6.1 Run `openspec validate --all --strict --no-interactive`.
- [x] 6.2 Run `openspec schema validate buoy-bridge`.
- [x] 6.3 Run the exact automatic repository acceptance command `devenv test`.
