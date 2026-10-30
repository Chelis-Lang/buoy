## Why

Chelis implementation links spell every field out inline: ten comment lines per record, and every rebind rewrites digest lines inside saved sources. The Rust side already evolved past this with compact content-addressed bindings: one in-source marker carrying one digest, resolving to an immutable record file whose name is its own hash. That form is tamper-proof by construction — no field exists in-source for drift to touch — and it moves rebind churn out of code diffs into the object store. The record schema (`buoy/implementation-binding/v1`) and the core graph slot (`implementation_bindings`) are repository-independent, so Chelis can reuse the same store and semantics. The integration also closes a missing core atom-revision edge for that graph slot.

## What Changes

- Add the `binding` record kind to the frozen Chelis grammar: a two-line comment form — the marker plus one `record = <digest>` field — anchored to the next declaration like every other record.
- Resolve the address against a configured content-addressed store (`binding-store` in the adapter configuration, required to lie under a selector root); verify that the store bytes hash to the address, parse the `buoy/implementation-binding/v1` record, and materialize it into `graph.implementation_bindings`.
- Enforce the chain fail-closed. Address faults, stale subjects, stale requirements, stale atom revisions, and duplicate inline relations all reject with stable diagnostics.
- Require a collision-resistant active digest member for compact bindings. Nautilus must migrate graph digests to BLAKE3-256 and re-execute receipts in its follow-on adoption.
- Add the `bind` subcommand: convert one existing inline `implementation-link` record into a compact binding — write the content-addressed object, replace the inline record with the two-line form — behind the same dry-run/`--write` discipline as `rebind`.
- Teach `rebind` and `trace` the compact form: rebind plans cover the store object plus the in-source address (a rebind writes the successor object, updates the address, and removes the superseded object); trace expands a binding through the store with every hop recomputed.
- Extend the frozen contract pack and registrations for the new requirements.
- **Non-goals:** compact forms for carriers, surfaces, or oracles; Deep dialect; blocking policy; host adoption (Nautilus converts its three links in a follow-on change).

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `chelis-provenance-adapter`: compact content-addressed implementation bindings with store resolution, fail-closed address and subject enforcement, sanctioned conversion, and rebind/trace integration.

## Impact

- `crates/chelis-provenance`: records (new kind), snapshot configuration (`binding-store`), command (resolution plus enforcement), `bind` module, rebind/trace extensions, CLI wiring, tests, fixtures, goldens.
- `crates/buoy-core`: complete the atom-revision validation closure for `ImplementationBinding`.
- `fixtures/chelis-provenance/v1`: new cases, manifest rows, adversarial-review rows, and schema-registry rows.
- Standard assurance regeneration: campaigns, evidence, ledger, correspondence, quality, naming inventory, reviewed source-content digest.
