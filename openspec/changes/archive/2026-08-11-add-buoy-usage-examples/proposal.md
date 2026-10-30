## Why

Buoy exposes command-line, Rust, and OpenSpec interfaces, but the repository lacks small consumer examples. Users must derive valid configurations and workflows from tests and internal fixtures.

The current `buoy-bridge` schema governs development of Buoy itself. Its formal-method, test, and acceptance rules do not fit a general downstream project.

## What Changes

- Add maintained examples for the Rust core API, repository checks, OpenSpec standing specifications, external trace links, and change checks.
- Add a reusable OpenSpec schema named `buoy-traceability` with proposal, specification, design, traceability, and task artifacts.
- Add deterministic positive and negative controls for each example.
- Add an example index that states prerequisites, commands, expected results, and assurance limits.
- Keep static checks separate from effectful oracle execution.
- Keep active OpenSpec change artifacts separate from authoritative standing specifications.
- Correct repository composition so that existing OpenSpec and external trace-link behavior works in the examples.
- Keep coverage evidence and mutation replay as explicit tasks outside `devenv test`.

**Goals:**

- Provide small examples that users can copy and adapt.
- Verify Rust examples with the pinned workspace toolchain.
- Verify OpenSpec examples with strict OpenSpec validation and focused Buoy commands.
- Show the boundary between workflow guidance and Buoy authority.

**Non-goals:**

- Define new Buoy graph semantics, canonical encoding, adapter syntax, or exit classes.
- Make active OpenSpec change artifacts authoritative inputs.
- Add provider access, network state, or effectful oracle demonstrations.
- Treat templates, annotations, tests, or trace links as proof.
- Replace the internal `buoy-bridge` schema.

## Capabilities

### New Capabilities

- `buoy-usage-examples`: Maintained Rust and repository examples with deterministic commands and expected results.
- `buoy-openspec-template`: A reusable OpenSpec schema and example project for Buoy traceability workflows.

### Modified Capabilities

- `devenv-workflow`: Keep coverage evidence and mutation replay outside the automatic acceptance dependency graph.

## Impact

- Adds example source, configuration, specifications, schema templates, documentation, and expected outputs.
- Adds focused checks for Rust example compilation, OpenSpec schema validity, and deterministic Buoy behavior.
- Corrects materialization for the existing `openspec/v1` adapter and external trace-link graph fields.
- Detaches `buoy:coverage-evidence` and `buoy:mutation-replay` from `devenv test`.
- Updates the main README with links to the example index.
- Does not change production APIs, canonical encoding, or documented adapter syntax.
