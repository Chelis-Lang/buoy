## Why

“Spec Provenance” describes one function of the system but is cumbersome as a product, package, and command name. Renaming the project to **Buoy** gives the repository a concise identity while preserving the frozen provenance semantics and historical artifacts already bound to v1 identifiers.

## What Changes

- Rename the product, repository-facing documentation, package metadata, executable, library crates, Nix/devenv commands, and maintained integration surfaces from Spec Provenance to Buoy.
- **BREAKING**: make `buoy` the primary CLI command and `buoy` / `buoy-core` the primary Rust package and crate identities.
- Provide an explicit, time-bounded compatibility path for the former CLI and package-facing names where technically possible, with deterministic deprecation behavior and removal criteria.
- Keep frozen `spec-provenance-*/v1` canonical schema IDs, domain separators, result protocols, diagnostic codes, and already reviewed artifact bytes unchanged; they become documented historical protocol identifiers rather than current branding.
- Rename repository-owned, noncanonical presentation labels and operational command names without changing static checking, effectful oracle execution, evidence classification, or proof semantics.
- Update the focused cargo-llvm-cov/Hegel-rs testing change to target Buoy package, crate, task, command, inventory, and evidence-binding names, and reconcile its completed obligations exactly once into production-hardening P0.
- Update Chelis integration planning to target Buoy names without changing Chelis compiler semantics.
- Add migration and stale-name checks so accidental mixed branding is rejected or explicitly allowlisted where a compatibility identifier must remain.
- Goal: complete a coherent product rename without invalidating frozen v1 artifacts or laundering assurance.
- Non-goals: redesigning the graph/schema, changing canonical v1 bytes, changing oracle verdict semantics, inferring proof from tests or annotations, implementing unfinished coverage/Hegel behavioral obligations beyond rename-induced migration and evidence replay, or implementing the deferred Chelis integration itself.

## Capabilities

### New Capabilities
- `product-identity`: Defines the Buoy product, CLI, package/crate, documentation, operational-command, compatibility-alias, migration, and frozen-identifier boundaries.

### Modified Capabilities

None. Existing deterministic graph, artifact, evidence, and proof requirements retain their frozen meanings and canonical v1 identifiers.

## Impact

Affected surfaces include Cargo package/crate names, the CLI binary and help text, imports and examples, README and schema documentation, Nix/devenv scripts and tasks, assurance/source inventories, proof and fixture paths, generated presentations, focused cargo-llvm-cov/Hegel-rs planning, Chelis adapter planning, and repository naming guidance. Downstream users must migrate invocations and Rust dependency/import names; canonical v1 readers and stored reports remain compatible and byte-identical.
