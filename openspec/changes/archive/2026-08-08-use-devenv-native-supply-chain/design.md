## Context

The reproducible-supply-chain capability uses one large Rust checker, `tools/check_supply_chain.rs`, and a committed `assurance/supply-chain/v1/closure-inventory.tsv`. The checker reads `devenv.lock` and `Cargo.lock`, copies every pinned revision, narHash, and checksum into the inventory, hashes the repository tool sources, and validates that the committed inventory matches. It also orchestrates `cargo-deny`, a clean-sandbox build, and a closure-size report.

The inventory duplicates the pins that `devenv.lock` and `Cargo.lock` already hold. It does not recompute those pins independently. It copies them. The result is a second manifest that drifts on every input bump and on every tool edit, which this session demonstrated repeatedly.

`devenv`, Nix, and Cargo provide the pinning, provenance, and content verification natively. `cargo-deny` provides the advisory, license, and source policy. A Nix build sandbox provides the offline, empty-home guarantee.

## Goals / Non-Goals

**Goals:**

- Remove the committed closure inventory and the checker that maintains it.
- Trust `devenv.lock` and `Cargo.lock` as the authoritative immutable pins.
- Keep the advisory, license, source, and duplicate policy through a Devenv task that runs `cargo-deny`.
- Keep the clean-sandbox, network-disabled acceptance of the final oracle.
- Keep a closure-size report through a Devenv task that reads `nix path-info`.
- Reconcile the specs, the workflow checker, and the assurance evidence so the gate stays coherent.

**Non-Goals:**

- Change the semantic core, canonical identity, or the digest algorithm.
- Change the pinned advisory database, the registry index, or the `deny.toml` policy.
- Weaken the offline acceptance guarantee. This change keeps it and makes it simpler.

## Decisions

### 1. Trust the locks; delete the inventory

`devenv.lock` pins every Nix input by revision and narHash. `Cargo.lock` pins every crate by version and checksum. Nix verifies input content on realization. Cargo verifies crate content against the checksum. Git verifies repository file content. The inventory adds no independent check because it copies these values.

Delete `tools/check_supply_chain.rs`, `assurance/supply-chain/v1/closure-inventory.tsv`, `assurance/supply-chain/v1/repository-inputs.tsv`, `assurance/supply-chain/v1/update-bindings.tsv`, and `assurance/supply-chain/v1/non-cargo-inputs.tsv`.

**Alternatives considered:**
- Keep the inventory but generate it purely as a build output that no reviewer edits. Rejected. It still drifts against the locks and still needs regeneration on every input move.
- Keep the inventory only for the trust-scope and exclusions annotations. Rejected for this change. The user accepts dropping the annotations. A slim trust-boundaries note can hold the few claims that matter.

### 2. Advisory and license policy as a Devenv task

Add a `buoy:deny` task that runs `cargo-deny` against the pinned offline advisory database, the pinned registry index, and `deny.toml`, with the network disabled. This keeps the exact policy the current script runs, without the inventory.

**Alternatives considered:**
- Run `cargo-deny` inside the deleted checker. Rejected. The checker is what this change removes.

### 3. Offline clean-sandbox acceptance without the checker

The final oracle MUST still build and run with the network disabled and empty user tool homes. Provide this by running the acceptance with `CARGO_HOME` and `RUSTUP_HOME` set to empty temporary directories, with the network disabled, from the declared Devenv closure. A required tool that is absent from its script runtime package set MUST make the run fail.

**Alternatives considered:**
- Realize the whole acceptance inside a single `nix build` sandbox. Rejected for now. It is a larger change and the empty-home, network-disabled run gives the same guarantee.

### 4. Closure size as a Devenv task

Add a `buoy:closure-size` task that reads `nix path-info -S` on the Devenv profile and reports component sizes. A size budget MAY block release under repository policy. Size MUST NOT change semantic identity or count as correctness evidence.

### 5. Reconcile the workflow checker and the assurance evidence

`tools/check_devenv_task_graph.rs` asserts the supply-chain script name, its runtime packages, and the task graph. `nix/devenv/workflow-policy.toml` lists the task. Update both for the new tasks. Remove the RSC-004 requirement and scenario rows from `assurance/v1/scenario-requirement-executions.tsv` and `assurance/v1/obligation-ledger.tsv`, remove the RSC-004 authority mapping from `tools/check_hardening_manifest.rs`, and regenerate the digest-bound evidence.

## Risks / Trade-offs

- The change deletes the per-input trust-scope and exclusions annotations. This removes the one part that was not duplication. A slim trust-boundaries note can preserve the few claims that matter.
- `tools/check_supply_chain.rs` appears in the coverage source inventory. Deleting it removes its coverage rows, which requires a coverage regeneration in CI.
- The offline-acceptance task MUST enforce empty homes and a disabled network, or it silently weakens RSC-002. Add a negative control that a developer cache and an ambient tool both fail the run.
- Removing an assurance capability control is hard to reverse. The change keeps RSC-001, RSC-002, RSC-003, and RSC-006 and removes only RSC-004, so the reproducibility and policy guarantees remain.
