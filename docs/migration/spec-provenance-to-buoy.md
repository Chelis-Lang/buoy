# Migrating from Spec Provenance to Buoy

Buoy 0.2.0 is the renamed release of the same deterministic provenance system. The rename changes product, package, crate, binary, and operational names; it does not change frozen protocol bytes or evidence meaning.

## Name mapping

| Former interface | Buoy 0.2 primary interface | Compatibility through |
|---|---|---|
| Product: Spec Provenance | Product: Buoy | Historical prose only |
| Cargo package `spec-provenance` | Cargo package `buoy` | `<0.3.0` facade |
| Rust crate `spec_provenance` | Rust crate `buoy` | `<0.3.0` re-export |
| Cargo package `spec-provenance-core` | Cargo package `buoy-core` | `<0.3.0` facade |
| Rust crate `spec_provenance_core` | Rust crate `buoy_core` | `<0.3.0` re-export |
| Test authority `spec-provenance-reference` | Test authority `buoy-reference` | No public compatibility promise |
| CLI `spec-provenance` | CLI `buoy` | `<0.3.0` delegating binary |
| Devenv task prefix `spec-provenance:` | Devenv task prefix `buoy:` | `<0.3.0` delegating tasks |
| Configuration filename `spec-provenance.toml` | Configuration filename `buoy.toml` | Explicit paths remain accepted |

Examples:

```text
buoy static --root ROOT --config buoy.toml --format machine
devenv tasks run buoy:properties
devenv tasks run buoy:coverage
```

The compatibility CLI emits one deterministic deprecation line on stderr. It preserves authoritative stdout and exit classes. Deprecation text is never part of canonical graph or report bytes.

## Frozen identifiers that do not change

Do not rename existing `spec-provenance-*/v1`, `spec-provenance-*/v2`, `spec-provenance/v1`, `spec-provenance/v2`, result-protocol, report-schema, fixture-schema, diagnostic, or digest-domain identifiers. They are protocol identities, not current product branding. Historical readers and accepted artifacts remain byte-identical.

Active schema v3 work can add new `buoy-*` identities after a reviewed schema decision. It must not relabel published v1 or v2 identities.

Content-addressed bindings add new v3 source-graph and binding identities. The frozen v1 and v2 inventories and fixtures remain unchanged.

## Adopt the OpenSpec version 2 format

`openspec/standing-spec/v2` is opt-in for each capability. Version 1 behavior stays unchanged.

Both formats keep `openspec/standing-spec/v1` as the source-item schema for document, requirement, scenario, and step nodes. A format change alone therefore keeps every authority identity and never stales one implementation binding.

Version 2 adds nonauthority trace side products, so the complete graph digest changes. Regenerate every artifact that binds that digest.

Move each trace fence into one level-two section after the requirements section before you select version 2. A fence inside one requirement or scenario span fails closed.

## Evidence migration

Source paths, Cargo package identities, commands, proof representations, derivations, coverage records, campaign records, and supply-chain closures affected by the rename become stale. Re-run each authority against the post-rename paths and tools. An unchanged source hash, compatibility facade, annotation, review, or aggregate green command cannot carry old evidence forward.

The frozen migration policy now records package version `0.2.0`. Its historical schema identities and reader requirements remain unchanged.

Focused cargo-llvm-cov and Hegel-rs obligations retain their `LCOV-*` and `HEGEL-*` meanings. Their primary commands become `buoy:coverage` and `buoy:properties`; each completed obligation maps to production-hardening P0 at most once.

## Repository configuration version 3

Version 2 keeps `schema = "spec-provenance-config/v2"` and one `--root` argument. Its configuration, snapshot, report, and CLI identities remain unchanged.

Version 3 uses `schema = "buoy/repository-config/v3"`. It declares logical mounts and uses repeated `--mount ID=PATH` arguments.

Version 3 prefixes change source identities by design. Regenerate bindings when a repository adopts logical prefixes.

Use `artifact/v1` for unsupported language files. Use repository sidecars for current artifact attachments.

Do not relabel a version 2 identity as a version 3 identity. Reingest the selected bytes under the new logical paths.

See [`../schema/v3/logical-mount-artifacts.md`](../schema/v3/logical-mount-artifacts.md) for the cutover procedure.

## Rollback and alias removal

Rollback may restore primary operational names and wrappers, but it must not rewrite frozen protocol identifiers or historical artifact bytes. At 0.3.0, remove the old package, crate, binary, and devenv task aliases while retaining migration diagnostics and historical readers.
