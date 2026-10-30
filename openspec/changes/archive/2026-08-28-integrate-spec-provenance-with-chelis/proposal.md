## Why

Chelis needs an explicit adapter that materializes its repository-owned requirement authorities, governed capability surfaces, evidence carriers, derivations, policies, and neutral change manifests into the now-established repository-independent Buoy interfaces. Proposing this only after the standalone `devenv test` oracle is green prevents Chelis-specific semantics from becoming implicit core behavior.

## What Changes

- Add a Chelis shell-side adapter and command integration that depends inward on the published Buoy core and total adapter contracts.
- Declare complete Chelis-owned finite decision surfaces and capability rows in Chelis-owned schemas rather than adding host semantics to the generic core.
- Materialize provider-neutral repository snapshots, review records, debt, waivers, reaffirmations, and base-to-head change manifests before calling deterministic checks.
- Bind Chelis tests, models, formal properties, generated artifacts, and consistency oracles with exact roles, scopes, revisions, tools, assumptions, and exclusions.
- Add positive, negative, mutation, cross-root, offline, and change-impact fixtures before enabling blocking policy.
- Expose static, change, and effectful execution surfaces without treating discovered carriers, passing tests, waivers, or reviews as proof.
- Keep initial policy advisory until every selected surface member and required role is current and visible.
- **Non-goals:** changing Buoy v1 semantics, importing Chelis capability meanings into the core, using GitHub/provider state as canonical authority, or claiming downstream compiler/backend proof.

## Capabilities

### New Capabilities
- `chelis-provenance-adapter`: Chelis-owned authority, governed-surface, carrier, derivation, policy, change-manifest, command, and report integration with the repository-independent Buoy interfaces.

### Modified Capabilities

None.

## Impact

- Adds a Chelis integration crate or module on the imperative-shell side of the dependency boundary.
- Adds Chelis-owned configuration, schema payloads, finite domain declarations, review records, fixtures, and CI tasks.
- Adds a Chelis command adapter that may expose the authoritative static/change/execute checks while preserving their exit classes and machine-report meanings.
- Requires a pinned Chelis repository revision and separately reviewed rollout; this proposal adds no Chelis-specific production code to the standalone core.
