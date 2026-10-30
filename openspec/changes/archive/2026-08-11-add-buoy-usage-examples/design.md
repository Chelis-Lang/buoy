## Context

Buoy has production interfaces for typed graph values, saved repository checks, OpenSpec authority, external links, and provider-neutral change checks. Current demonstrations exist mainly in tests and fixtures.

The repository also has a project-local `buoy-bridge` OpenSpec schema. That schema encodes Buoy development rules and does not form a general consumer template.

The examples must remain executable under the pinned toolchain. They must not add new canonical semantics or create an alternate authority path.

## Goals / Non-Goals

**Goals:**

- Add small Rust examples for core graph validation and repository static checks.
- Add complete example repositories for OpenSpec authority, external links, and change checks.
- Add a copyable `buoy-traceability` OpenSpec schema with generic templates.
- Bind each example to deterministic positive and negative controls.
- Add one focused example gate and retain `devenv test` as final acceptance.
- Keep coverage evidence and mutation replay available only through explicit focused tasks.

**Non-Goals:**

- Define new `buoy-core`, adapter, canonical encoding, diagnostic, or command exit behavior.
- Parse active OpenSpec changes as standing-spec authority.
- Add a provider connector or effectful oracle example.
- Claim that compilation, tests, templates, links, or green reports are proof.
- Replace or generalize the internal `buoy-bridge` schema.

## Decisions

### 1. Separate compiled examples from complete repository examples

Root Cargo examples will contain `examples/core_graph.rs` and `examples/repository_check.rs`. Cargo will compile both files under the normal workspace lint policy.

Complete repository examples will live under `examples/repositories/`. Each repository will contain all configuration, source, manifest, and expected-output files for its command.

The example index will live at `examples/README.md`. The main README will link to this index.

**Alternatives considered:**

- Put all examples in documentation code blocks. Rejected because code blocks do not verify complete files or command behavior.
- Put examples only in tests. Rejected because tests obscure consumer paths and include internal test helpers.

### 2. Preserve the functional-core and imperative-shell boundary

`examples/core_graph.rs` will pass complete typed values to the public `buoy::core` facade. It will not access files, processes, clocks, locale, or network state.

`examples/repository_check.rs` will use shell APIs for configuration parsing, repository snapshots, static checks, and report presentation. It will pass parsed values inward and will not revalidate raw input later.

No example will add a dependency from `buoy-core` to the root crate. The core will remain safe, deterministic, and `no_std`.

**Alternatives considered:**

- Add filesystem helpers to `buoy-core` for a shorter example. Rejected because this violates the one-way dependency and FCIS boundaries.
- Construct untyped maps in examples. Rejected because typed constructors keep invalid states outside the core path.

### 3. Provide three complete command-line repositories

The repository set will contain these examples:

- `openspec-authority`: one proposed capability, one reserved requirement, scenarios, and all four external trace relations.
- `stale-external-link`: a requirement revision with a link that retains the old atom revision.
- `change-check`: base and head roots, exact graph digests, complete and incomplete manifests, and expected exit classes.

The positive OpenSpec repository will use one explicit `spec-provenance-config/v2` file. Its embedded `buoy.openspec-authority/v1` configuration will own capability metadata.

The implementation will correct repository materialization for existing `openspec/v1` selectors. It will also retain external links when it combines graph fragments.

These corrections satisfy existing standing specifications. They do not add adapter syntax or canonical fields.

The change example will commit exact base and head graph digests. A control will recompute these digests and reject stale manifest bytes.

**Alternatives considered:**

- Generate all repositories in a test. Rejected because users need complete files that they can inspect and copy.
- Read live Git or provider state. Rejected because those values are not canonical inputs.

### 4. Add a separate reusable OpenSpec workflow project

The copyable workflow will live under `examples/openspec-workflow/`. It will contain its own `openspec/config.yaml` and local `buoy-traceability` schema.

The artifact graph will be:

```text
proposal -> specs -> design -> traceability -> tasks
```

The specification template will require Buoy-compatible requirement headings and positive and negative scenarios. The traceability template will require complete planned link actions.

The templates will not require Verus, AeneasVerif, Hegel, `devenv test`, or Buoy repository paths. A downstream project can add those controls through its own configuration.

**Alternatives considered:**

- Rename the current `buoy-bridge` schema. Rejected because that schema governs Buoy development and contains necessary internal constraints.
- Make `buoy-traceability` the repository default. Rejected because this change provides an example, not a workflow migration.

### 5. Keep workflow guidance separate from canonical authority

OpenSpec planning artifacts will remain noncanonical workflow data. Buoy will read only selected saved repository bytes and explicit adapter configuration.

The adapter will continue to accept standing specifications only under `openspec/specs/<capability>/spec.md`. A candidate change check requires a head root with synchronized standing specifications.

Lifecycle, reserved IDs, atom kind, scope, and approval bindings will remain in `buoy.openspec-authority/v1` configuration. Templates will not infer these values.

External trace links will contain repository-owned target text only. They will not contain provider status, fetched titles, users, or timestamps.

**Alternatives considered:**

- Parse OpenSpec delta files directly. Rejected because active changes are plans and do not have standing-spec authority.
- Treat template metadata as lifecycle approval. Rejected because workflow text cannot replace reviewed authority configuration.

### 6. Bind deterministic results and preserve fact separation

Every command example will name its authoritative inputs and expected exit class. Machine-output goldens will preserve the production total order without example-specific sorting.

The controls will compare exact bytes where the command contract promises deterministic machine output. Diagnostics will use stable production codes.

Static registration, freshness, execution verdict, and assurance class will remain separate facts. The public examples will demonstrate static and change checks only.

No example will execute an oracle. The example index will state that oracle receipts remain effectful and separate from static graph identity.

**Alternatives considered:**

- Normalize machine output in the example tests. Rejected because normalization can hide an ordering or path-independence defect.
- Present all green results as one assurance score. Rejected because aggregation cannot synthesize absent facts.

### 7. Verify examples through focused and final gates

A new `buoy:examples` task will compile Rust examples, execute positive and negative controls, validate the reusable schema, and compare deterministic outputs.

The exact final acceptance oracle will remain `devenv test`. Success requires exit code `0` from the pinned environment with every blocking task complete.

The focused gate will include the example sources in exact coverage and source inventories where current policy requires them. It will fail closed on an absent example or stale expected result.

**Alternatives considered:**

- Run documentation-only checks. Rejected because they do not execute command behavior.
- Make `buoy:examples` the final oracle. Rejected because examples do not cover the complete repository acceptance surface.

### 8. State formal and trust limits

The examples add no Verus or AeneasVerif theorem. Existing formal results do not prove these examples, rustc, LLVM, operating-system behavior, or natural-language correspondence.

A successful OpenSpec schema check proves only that OpenSpec accepts the local artifact graph. It does not prove Buoy graph validity or requirement correctness.

A successful Rust compile proves only acceptance by the pinned compiler for the selected source. Positive and negative controls demonstrate their exact reviewed cases.

### 9. Keep coverage evidence and mutation replay explicit

`buoy:coverage-evidence` and `buoy:mutation-replay` will remain available as focused tasks. No automatic task or phase will depend on them.

The task graph policy will classify both tasks as manual. The resolved task fixture will bind the same separation.

Coverage-bearing source rows will use manual coverage dispositions. The explicit coverage task will continue to select these rows.

The automatic ledger will classify manual coverage rows as not applicable. Therefore, stale coverage evidence will not block `devenv test`.

**Alternatives considered:**

- Remove the tasks. Rejected because maintainers still need explicit access to both checks.
- Keep the tasks in automatic acceptance. Rejected by the confirmed acceptance scope.

## Risks / Trade-offs

- `devenv test` no longer rejects stale coverage evidence or named mutation replay failures.
- Complete examples add files to source, coverage, and acceptance inventories.
- Exact graph digests create deliberate update work after semantic source changes.
- A nested OpenSpec project can confuse root selection if commands run from the wrong directory.
- Copyable templates can drift from adapter syntax unless structural controls bind exact required fields.
- A broad example can become a second reference implementation. Each example will remain focused and small.
