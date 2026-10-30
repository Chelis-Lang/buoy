## Why

The first live adoption proved the Chelis digest chain end to end but exposed four verifier gaps. Bound item digests and oracle runner digests are recorded yet never compared at static time, so a body edit on a bound declaration or a toolchain bump passes silently. Staleness diagnostics name the stale object but not the current expected value, so every rebind required an out-of-band probe build. The full-chain audit (atom revision to surface row to link to parsed node bytes to carrier to receipt) exists only as a manual procedure. And every legitimate spec edit means hand-editing digest fields across saved sources. This change makes every carried digest enforced, every failure actionable, the audit one command, and regeneration sanctioned.

## What Changes

- Enforce bound item digests at static time: a carrier or link whose `item-digest` differs from the current parsed declaration node digest fails with `CHELIS-PROV-ITEM-STALE`.
- Enforce oracle runner bindings: oracle records gain optional `runner-config-paths`; when present, the static check compares the recorded runner digests against the current saved artifact bytes and fails with `CHELIS-PROV-ORACLE-RUNNER-STALE` on drift.
- Make staleness diagnostics actionable: every digest- or revision-staleness diagnostic context carries both the bound value and the current expected value.
- Add the `trace` subcommand: print the full digest chain for one stable ID with every hop recomputed from saved bytes, deterministic machine output, and the static exit class on any mismatch.
- Add the `rebind` subcommand: recompute every record-carried digest and revision field from the current graph, print a deterministic field-level diff, and rewrite the saved sources only under an explicit `--write` flag.
- Extend the frozen contract pack with cases for the four new requirements and register the review rows.
- **Non-goals:** blocking policy anywhere, Deep dialect changes, record-grammar version bumps (the new oracle key is an optional recognized field under `chelis:provenance/v1`), and host-side adoption (a Nautilus follow-up consumes these).

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `chelis-provenance-adapter`: complete digest enforcement, actionable staleness contexts, a deterministic trace projection command, and sanctioned rebind regeneration.

## Impact

- `crates/chelis-provenance`: records/map (optional oracle field), command (enforcement plus context enrichment), new `trace` and `rebind` modules, CLI wiring, tests, fixtures, goldens.
- `fixtures/chelis-provenance/v1`: new cases, manifest rows, and adversarial-review rows for `CHELIS-PROV-015..018`.
- The standard assurance regeneration: campaigns, evidence, ledger, correspondence, quality, naming inventory, and the reviewed source-content digest.
