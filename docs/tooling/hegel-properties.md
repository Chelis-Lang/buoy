# Hegel sampled property suites

Status: active, deterministic, source-bound sampled evidence.

## Authoritative scope

`assurance/v1/property-inventory.tsv` is the machine-readable authority for every registered property. It records the property ID, expected polarity, production and reference APIs, source paths, unique seed, generator domain and limits, explicit edges, rejection limit, assertion, replay fixture, scope, and assurance class.

The inventory includes:

- production/reference differentials and justified production metamorphic properties in `tests/hegel_properties.rs`;
- the production semantic-envelope properties in `crates/buoy-core/tests/xxh3_hegel_envelope.rs`;
- visibly `reference-only` XXH3 helper properties in `tests/xxh3_hegel_properties.rs`; and
- the approved-API positive control and intentionally false negative control in `tests/hegel_controls.rs`.

A `reference-only` row never closes a production obligation. `tools/check_reference_independence.rs` rejects production imports, re-exports, path dependencies, and source inclusion from the independent reference crate with `HEGEL-REFERENCE-NOT-INDEPENDENT`.

## Pinned framework and settings

“Hegel-rs” means exactly `hegeltest = 0.28.0`; package, companion-package, commit, tree, checksum, license, and API details are recorded in [`hegel-rs.md`](hegel-rs.md) and `assurance/v1/hegel-tool-binding.toml`.

Every authoritative property uses `tests/support/hegel_settings.rs` and therefore explicitly sets:

- `Backend::Default`;
- exactly 256 cases;
- one committed property-specific seed under `0x5350_5631_0000_0000`;
- `derandomize(true)`;
- `database(None)`; and
- phases in the exact order `Explicit`, `Reuse`, `Generate`, `Target`, `Shrink`.

No environment-derived backend, seed, case count, phase, or database is authoritative. Generator types, bounds, explicit edges, and rejection limits are committed per inventory row rather than inferred from Hegel defaults.

## Effectful lane and evidence lifecycle

Run the complete focused lane with:

```text
devenv tasks run buoy:properties
```

The lane runs offline and, in order:

1. checks generated positive/negative fixtures and validator self-tests;
2. verifies the independent reference source boundary;
3. executes every registered Hegel test target, including the approved API and intentionally false controls;
4. emits normalized source/settings/domain/reference/replay-bound results only after execution succeeds;
5. validates those current results; and
6. runs the independently scoped finite-model and formal-spike checks.

`devenv test` is the exact automatic oracle and must invoke this property lane. Coverage instrumentation requires explicit selection through `buoy:coverage`.

Automatic acceptance rejects stale coverage evidence. It does not create a new coverage execution verdict.

## Negative control and replay

`hegel_expected_failure_is_rejected_exactly_once` executes an assertion that is false for its declared Boolean domain, requires one observed Hegel rejection, verifies the panic carries the stable property identity, and binds `fixtures/property/v1/replays/intentionally-false.tsv`. An unexpected pass fails with `HEGEL-NEGATIVE-CONTROL`; an unrelated or identity-free panic fails with `HEGEL-EXECUTION-INCOMPLETE`.

Focused replay commands are stored in `assurance/v1/property-runs.tsv`. Reviewed replay fixtures are content-digest bindings. Raw shrinking traces, timings, terminal formatting, local databases, and uncommitted minimized inputs remain noncanonical diagnostics and cannot be promoted into normalized evidence.

## Assurance boundary

A passing row establishes only sampled evidence for its exact 256 cases, seed, domain, settings, production/reference identities, and assertions. It does **not** establish exhaustive exploration, formal proof, reference correctness, compiler/rustc/LLVM/OS correctness, cryptographic collision resistance, or natural-language-to-property correspondence. Independent finite models and formal proofs retain separate evidence classes and assumptions.
