# Chelis provenance preimplementation contract v1

This directory is a frozen, fail-closed contract pack for OpenSpec change
`integrate-spec-provenance-with-chelis`. It is **not** an adapter implementation and does not
claim canonical Chelis publication or any assurance level.

## Pinned inspection boundary

- Buoy revision: `61b2c25afd55f8295715868cb854d2b4777de9a2` (`0.2.0`).
- Chelis revision: `c0d4caf16f557406f299f0989bab48fa0f924cbd` (`0.16.1`, MIT).
- Vendored Surf grammar revision: `fc3232ffe085d15b213bb7457b69a22063a200b9` (`v0.18.5`),
  recorded as a registered derivation in `boundary.toml` with per-file digests and a
  fail-closed drift check.
- Chelis inspection excludes the live checkout's unrelated tracked `.gitignore` change and
  untracked `.devenv/` directory.
- Chelis declares Rust channel `stable`; that is recorded as not exactly reproducible.
- `inventory.tsv` assigns every listed item `assurance=unassigned`.

The public inventory omits private integration metadata.
It does not describe the complete pinned tree.

`boundary.toml`, `schema-registry.tsv`, and `selectors.toml` freeze repository ownership,
reserved valid Buoy SDK identities, parser identities, selected saved-snapshot inputs, path/link
policy, authority expectations, and noncanonical ambient/provider inputs.

## Executable contract

- `cases/*.toml`: 120 positive, negative, and negative-control scenarios spanning
  `CHELIS-PROV-001` through `CHELIS-PROV-021`. The pack includes one current and one drift
  case for each of the nine inventoried generated or mirrored Chelis artifacts, plus neutral
  base-to-head change-manifest cases that cover every policy weakening protocol failure.
- `manifest.tsv`: one focused command, one stable diagnostic where rejection is expected, and the
  pinned final Chelis-worktree oracle for every case.
- `models/*.toml`: six finite models, 7,131 enumerated states, and 41 invariants covering surface
  completeness, policy adoption, exact required roles, fixed-point impact closure,
  debt/waiver/reaffirmation no-laundering, and conservative proof invalidation.
- `tests/chelis_preimplementation.rs`: Hegel properties plus boundary, inventory, manifest,
  source-item protocol, and adapter SDK controls.
- `tools/check_fixture_manifest.rs`: includes this model root in the repository model oracle.

Focused validation:

```text
devenv shell -- cargo test --locked --offline --test chelis_preimplementation
devenv shell -- cargo run --quiet --locked --offline --bin check-fixture-manifest
devenv shell -- cargo check --locked --offline --package buoy-core --lib --target thumbv7em-none-eabi
```

The final integration oracle remains `devenv test` in a clean, pinned Chelis worktree. This checkout does not claim that result.

## Schema v3 boundary

Buoy SDK v3 requires descriptor-declared source-item kinds. The Chelis descriptor declares these kinds:

- `chelis/surf-declaration/v1`
- `chelis/deep-node/v1`

The protocol requires source reingestion. It does not provide a legacy reader or an identity migration.

Chelis must not use a cross-adapter kind, an opaque payload, or a host-specific `buoy-core` enum.

Cases labeled `execution = "preimplementation-contract"` are expected behavior contracts. They are not implementation claims.

## Implementation and registrations

The `crates/chelis-provenance` workspace crate implements the shell-side adapters,
snapshot construction, static/change/execute command surfaces, advisory reports,
and saved-snapshot editor adapters. `registrations/` holds the assurance pack:
carrier, derivation, exhaustive-report, proof-binding, promotion-review, and
adversarial-review records. `goldens/` freezes canonical machine and human report
bytes; regenerate one golden only by deleting it, running the golden suite twice,
and reviewing the diff before commit.

## Devenv registration

`devenv test` from the pinned Chelis integration worktree is the final oracle.
It registers every Chelis check through these gates:

- `buoy:test` runs the workspace test surface: `chelis_preimplementation` plus the
  `chelis-provenance` suites `integration`, `adapter_contract`, `registrations`,
  and `goldens_and_mutations` (protocol, fixtures, properties, total adapters,
  cross-root/offline integration, derivations, and mutation controls).
- `buoy:fixtures` runs `check-fixture-manifest`, which executes the six Chelis
  finite models to their fixed points.
- `buoy:core-proofs` replays the selected Verus lane, including the
  `source_item_kind` integrity spike that covers the Chelis item-kind protocol.
- `buoy:specs` strictly validates the OpenSpec artifacts for this change.
