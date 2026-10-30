## Context

The Nautilus adoption walked the digest chain end to end and tampered with it; the chain held wherever the verifier enforces, and stayed silent wherever it merely carries. Item digests on links and carriers, and runner digests on oracles, are recorded but never compared. Staleness diagnostics name the stale object without the current value, which forced three separate out-of-band probe builds during the adoption and the re-pin. The full-chain audit and the rebind workflow are manual procedures.

## Goals / Non-Goals

**Goals:** every carried digest enforced or explicitly nonnormative; every staleness diagnostic self-sufficient for correction; the chain audit as one deterministic command; regeneration as one reviewed command.

**Non-Goals:** blocking policy, Deep dialect, record-grammar version bump, host adoption, and bind/trace-publication semantics owned by the core.

## Decisions

### Item enforcement compares against the source map, not a new index

`static_check` already materializes every record's parsed item identity. Enforcement joins each carrier's and link's `item-digest` field against the digest of the item materialized for that same record, so no second extraction pass and no new state exist. The comparison lives beside the existing freshness checks in `command.rs`.

### Oracle runner bindings are an optional recognized field

Oracle records gain `runner-config-paths`, a semicolon list of repository-relative artifact paths. When present, the static check digests the current saved bytes of each path (through the snapshot's artifact access, as derivation freshness already does) and compares against `runner-digest` and `runner-configuration-digest` in declaration order. Absent the field, behavior is unchanged, so existing records stay valid under `chelis:provenance/v1`. Alternative: a new record schema version. Rejected: an optional recognized key adds no ambiguity to the total grammar and forces no reingestion.

### Context format is `bound=<digest>;current=<digest>`

One stable, greppable format for every staleness context, appended after the existing object identification. Machine-parsable without being a schema of its own. Report goldens regenerate once.

### `trace` is a projection over one static outcome

`trace --root --config --id <STABLE-ID>` runs the existing static check, then projects the chain for the ID: atom row with revision, every surface row, link, and carrier binding it (with bound versus recomputed values per hop), and referenced oracle digests. Output is canonical JSON under `chelis-trace-projection/v1`. Exit is success only when the static check accepts and every projected hop matches. No new authority: the projection derives from the same graph the static check built.

### `rebind` rewrites only record comment fields

The rewrite plan derives from the same joins the enforcement uses: for each stale field, the plan holds path, byte range of the field line, old value, new value. The dry run prints the plan as canonical bytes; `--write` applies it with exact byte-range splices and re-runs extraction to confirm convergence. Declarations are structurally unreachable because plans only ever target `--`-prefixed record field lines. The tool refuses to write when the parse rejects a selected source, so it can never "fix" a tree it cannot read.

### Rebind covers exactly the mechanical fields

Item digests, atom revisions in `atoms =` references and surface rows, oracle digest references, and declared runner digests. Statements, IDs, scopes, and every human-authored field are out of scope; changing a claim stays a human act, and the resulting revision propagation is what `rebind` mechanizes.

## Risks / Trade-offs

- **`rebind --write` as a laundering vector** → it only moves bindings to the current saved bytes; it cannot change claims, verdicts, or receipts, and receipts still bind the static identity, which changes under any rebind, forcing corpus re-execution. Policy: review the dry-run diff before every write, the same rule as golden regeneration.
- **Context enrichment changes report bytes** → goldens regenerate once through the recorded policy; downstream parsers read JSON fields, not context substrings.
- **Optional oracle field tempts partial adoption** → the advisory report lists oracles without `runner-config-paths` as a visible gap so hosts see unenforced runner bindings.

## Migration Plan

1. Enforcement joins plus context enrichment; regenerate crate goldens and pack cases.
2. `trace` module with goldens; `rebind` module with dry-run/write/idempotence tests.
3. CLI wiring, fixture pack rows, adversarial review rows, trust document update.
4. Standard assurance regeneration to a fixed point; full `devenv test`.
5. Follow-up in Nautilus: declare `runner-config-paths` on the gate oracle, adopt `trace` and `rebind` in the gate scripts, retire the probe pattern.

Rollback before merge reverts the crate modules; records written by `rebind` remain valid because they hold exactly the values the previous manual workflow produced.

## Open Questions

None.
