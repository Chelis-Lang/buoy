## Requirement Actions

| Requirement ID | Target class | Action | Owner | Buoy command |
|---|---|---|---|---|
| `OSA-002` | documentation and test | revise | adapter owner | `devenv shell -- cargo test -p buoy --all-features --test openspec_traceability` |
| `OSA-003` | parser, fixture, model, and test | revise | parser owner | `devenv shell -- cargo test -p buoy --all-features --test openspec_traceability` |
| `OSA-005` | configuration parser, fixture, and test | revise | configuration owner | `devenv shell -- cargo test -p buoy --all-features --test openspec_traceability` |
| `OSA-007` | parser, adapter validation, fixture, model, and test | revise | adapter owner | `devenv shell -- cargo test -p buoy --all-features --test openspec_traceability` |

## Link Review

| Requirement ID | Current link | Disposition | Evidence |
|---|---|---|---|
| `OSA-002` | `src/adapters/openspec.rs` | revise | `tests/openspec_traceability.rs` and `docs/schema/v3/openspec-traceability.md` |
| `OSA-003` | `src/adapters/openspec.rs` | revise | `fixtures/openspec/v1/` and `tests/openspec_traceability.rs` |
| `OSA-005` | `src/adapters/openspec.rs` | revise | `fixtures/openspec/v1/positive/configuration.toml` and negative configuration fixtures |
| `OSA-007` | `src/adapters/openspec.rs` | revise | `src/adapter_sdk.rs`, fixtures, finite model, and property tests |

No current repository-owned external trace link binds these requirements. No link needs retirement or waiver.

## External Targets

This change adds no external target.
