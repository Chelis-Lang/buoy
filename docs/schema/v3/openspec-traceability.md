# OpenSpec Traceability

## Architecture

Schema v3 uses versioned text for source-item kinds. The core stores the validated kind, but it does not define host-specific variants.

Each adapter descriptor declares its complete kind set. The adapter registry rejects duplicate declarations and output with an undeclared kind.

The OpenSpec adapter reads standing requirements and scenarios from Markdown. `pulldown-cmark` supplies exact byte ranges for each accepted item.

A repository configuration supplies authority data for each capability. The data includes the format, scope, atom kind, lifecycle, reserved IDs, and approval.

Each requirement creates one atom at requirement scope. Scenarios bind the atom statement and revision, but they do not create coverage targets.

Primary requirement spans and nonnormative spans partition all selected bytes. Each scenario span remains inside its requirement span.

The adapter rejects unsupported authority syntax before graph construction. This shell parser adds no Verus or AeneasVerif proof claim.

External trace links are navigation data. They do not supply assurance status, approval authority, or proof results.

The graph binds each link to exact semantic atom revisions. A change to an atom revision makes the related link stale.

An atom revision is not the complete atom object digest. Lifecycle or approval data can change the object digest without changing the semantic revision.

External-link validation compares the saved revision with `Atom::revision()` for the same stable ID. An atom object digest in that field is stale.

## Configure a Repository

1. Add an `openspec/v1` selector for the standing-spec paths.
2. Set `authority_expectation = "required"` for authoritative standing specifications.
3. Add the exact OpenSpec configuration under `[adapter_configurations]`.
4. Declare each capability in the OpenSpec configuration.
5. Reserve explicit IDs when a requirement heading contains an ID.
6. Add an approval block for each ratified capability.

Use this configuration shape:

```toml
schema = "spec-provenance-config/v2"
digest_algorithms = ["xxh3-128"]

[adapter_configurations]
"openspec/v1" = '''
schema = "buoy.openspec-authority/v1"

[[capabilities]]
name = "demo"
format = "openspec/standing-spec/v1"
scope = "openspec.demo"
atom_kind = "behavioral"
lifecycle = "proposed"
reserved_ids = ["OSA-001"]
'''

[[selectors]]
root = "openspec/specs"
adapter = "openspec/v1"
include = ["**/spec.md"]
authority_expectation = "required"
```

## Write a Standing Specification

1. Use `### Requirement: <stable-id> <title>` for each requirement.
2. Put one normative statement after the requirement heading.
3. Use an RFC 2119 term in the normative statement.
4. Use `#### Scenario: <title>` for each scenario.
5. Add `- **GIVEN**`, `- **WHEN**`, and `- **THEN**` steps in that order.
6. Keep each stable ID in the reserved-ID set for its capability.
7. Use level-two headings for nonauthority sections after the requirements.

## Select One Standing-Spec Format

The adapter accepts two formats. Declare one exact value for each capability.

- `openspec/standing-spec/v1` keeps every recognized trace fence nonnormative.
- `openspec/standing-spec/v2` adds nonauthority trace sections.

Both formats use `openspec/standing-spec/v1` as the source-item schema for document, requirement, scenario, and step nodes. Equal authority bytes therefore keep equal node and atom identities under either format.

A format change alone never stales one implementation binding. Version 2 adds trace side products and changes the complete graph digest.

## Add External Links

1. Add a fenced block with the exact info string `buoy-external-trace-v1`.
2. Add one TOML link record for each external target.
3. Use one closed relation: `tracks`, `discusses`, `documents`, or `reviews`.
4. Use an absolute HTTPS URI or a versioned URN for the target.
5. Bind each link to one or more exact atom revisions.
6. Do not put provider status in the link.

One shared parser owns the fence value grammar. The Markdown adapter and the version 2 OpenSpec adapter produce equal link values from equal fence bytes. Each host keeps its own item identity and exact source mapping.

## Add a Version 2 Trace Section

1. Select `format = "openspec/standing-spec/v2"` for the capability.
2. Add one level-two nonauthority section after the requirements section.
3. Put the `buoy-external-trace-v1` fence inside that section.
4. Use `relation = "documents"` for one implementation-owned trace page.

The adapter finds requirement boundaries before it reads trace sections. Trace bytes therefore stay outside every requirement identity and atom revision.

A recognized fence that intersects one requirement or scenario span fails with `SPG-OPENSPEC-TRACE-IN-AUTHORITY`. The adapter accepts no partial product for that file.

The primary requirement and nonnormative spans still partition every selected byte. One trace mapping is secondary and nests inside one nonnormative span.

An external `documents` link points at a page. The exact implementation relation stays in the immutable binding record.

## Use the OpenSpec Workflow

1. Select `schema: buoy-bridge` in `openspec/config.yaml`.
2. Complete `proposal.md`, `spec.md`, and `design.md`.
3. Complete `traceability.md` before `tasks.md`.
4. Record every added, modified, and removed requirement in `traceability.md`.
5. Record each planned compact relation without a current digest.
6. Keep the planned and completion tables empty when no row applies.
7. Record one completion row for each implemented link action.
8. Keep provider-specific operations in the generated command plan.
9. Run the project validation command before implementation.

The completion snapshot is historical. Buoy remains the current-state authority. Run a new query to read current state.
