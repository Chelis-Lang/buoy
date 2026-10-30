## Requirement Actions

| Requirement ID | Target class | Action | Owner | Buoy command |
|---|---|---|---|---|
| `BUE-001` | documentation | add example index and README link | documentation owner | `devenv tasks run buoy:examples` |
| `BUE-001` | evidence | add index boundary controls | example test owner | `devenv tasks run buoy:examples` |
| `BUE-002` | code | add `examples/core_graph.rs` | Rust API owner | `devenv shell -- cargo run --example core_graph` |
| `BUE-002` | evidence | add valid and invalid graph controls | core test owner | `devenv tasks run buoy:examples` |
| `BUE-003` | code | add `examples/repository_check.rs` | repository API owner | `devenv shell -- cargo run --example repository_check -- --help` |
| `BUE-003` | evidence | add deterministic output and invalid configuration controls | integration test owner | `devenv tasks run buoy:examples` |
| `BUE-004` | configuration | add the OpenSpec authority example repository | OpenSpec adapter owner | `devenv shell -- cargo run --bin buoy -- static --root examples/repositories/openspec-authority --config examples/repositories/openspec-authority/buoy.toml --format machine` |
| `BUE-004` | evidence | add reserved-ID and active-path controls | OpenSpec test owner | `devenv tasks run buoy:examples` |
| `BUE-005` | external link | add all four closed relation examples | traceability owner | `devenv tasks run buoy:examples` |
| `BUE-005` | evidence | add the stale atom-revision control | graph validation owner | `devenv tasks run buoy:examples` |
| `BUE-006` | configuration | add base, head, and manifest example files | change-check owner | `devenv tasks run buoy:examples` |
| `BUE-006` | evidence | add complete and incomplete disposition controls | change-check test owner | `devenv tasks run buoy:examples` |
| `BUE-007` | code | add the focused example verification test | quality owner | `devenv tasks run buoy:examples` |
| `BUE-007` | evidence | add deterministic aggregate failure controls | quality test owner | `devenv tasks run buoy:examples` |
| `BUE-007` | configuration | add `buoy:examples` to the task graph and acceptance path | devenv owner | `devenv tasks run buoy:devenv` |
| `DEV-009` | configuration | detach coverage evidence and mutation replay from automatic acceptance | devenv owner | `devenv tasks run buoy:devenv` |
| `DEV-009` | evidence | keep both checks as explicit focused tasks | workflow test owner | `devenv tasks run buoy:devenv` |
| `DEV-009` | configuration | classify coverage source rows as manual while preserving focused selection | assurance owner | `devenv tasks run buoy:coverage-evidence` |
| `DEV-009` | evidence | classify manual coverage rows as not applicable in the automatic ledger | assurance owner | `devenv tasks run buoy:ledger-green` |
| `BOT-001` | schema | add the project-local `buoy-traceability` schema | OpenSpec workflow owner | `devenv tasks run buoy:examples` |
| `BOT-001` | evidence | add schema selection and internal-rule exclusion controls | OpenSpec test owner | `devenv tasks run buoy:examples` |
| `BOT-002` | schema | add the required artifact dependency graph | OpenSpec workflow owner | `devenv tasks run buoy:examples` |
| `BOT-002` | evidence | add ready and blocked artifact-flow controls | OpenSpec test owner | `devenv tasks run buoy:examples` |
| `BOT-003` | schema | add the Buoy-compatible specification template | OpenSpec workflow owner | `devenv tasks run buoy:examples` |
| `BOT-003` | evidence | add stable-ID and scenario-shape controls | OpenSpec test owner | `devenv tasks run buoy:examples` |
| `BOT-004` | schema | add the complete traceability template | traceability owner | `devenv tasks run buoy:examples` |
| `BOT-004` | evidence | add required-field and provider-state exclusion controls | traceability test owner | `devenv tasks run buoy:examples` |
| `BOT-005` | documentation | add authority and lifecycle boundary guidance | OpenSpec workflow owner | `devenv tasks run buoy:examples` |
| `BOT-005` | evidence | add configuration-ownership controls | OpenSpec test owner | `devenv tasks run buoy:examples` |
| `BOT-006` | documentation | add active-change materialization guidance | change-check owner | `devenv tasks run buoy:examples` |
| `BOT-006` | evidence | add synchronized-head and active-path controls | integration test owner | `devenv tasks run buoy:examples` |
| `BOT-007` | code | add reusable schema structural controls | OpenSpec test owner | `devenv tasks run buoy:examples` |
| `BOT-007` | evidence | add strict schema validation and drift controls | quality owner | `devenv tasks run buoy:examples` |

## Link Review

The usage and template requirements are new. `DEV-009` is current and requires revision.

| Requirement ID | Current link | Disposition | Evidence |
|---|---|---|---|
| `BUE-001` through `BUE-007` | none | add reviewed implementation and evidence links | `openspec/changes/add-buoy-usage-examples/specs/buoy-usage-examples/spec.md` |
| `BOT-001` through `BOT-007` | none | add reviewed schema and evidence links | `openspec/changes/add-buoy-usage-examples/specs/buoy-openspec-template/spec.md` |
| `DEV-009` | `openspec/specs/devenv-workflow/spec.md` | revise automatic task ownership | `openspec/changes/add-buoy-usage-examples/specs/devenv-workflow/spec.md` |

## External Targets

The external-link example will contain only these repository-owned target strings:

- `https://example.com/issues/100`
- `https://example.com/discussions/100`
- `https://example.com/documents/100`
- `https://example.com/reviews/100`
- `urn:buoy:document:v1`

The examples will not record provider status, titles, users, or timestamps.
