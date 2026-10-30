## 1. Enforcement

- [x] 1.1 Join each carrier's and link's `item-digest` against the parsed item identity materialized for the same record; emit `CHELIS-PROV-ITEM-STALE` with the static exit class on mismatch.
- [x] 1.2 Recognize the optional oracle field `runner-config-paths`; digest the referenced saved artifacts in declaration order and emit `CHELIS-PROV-ORACLE-RUNNER-STALE` when the recorded runner digests differ. Absent the field, behavior is unchanged.
- [x] 1.3 Enrich every digest- and revision-staleness context with `bound=<digest>;current=<digest>`; keep contexts byte-identical across distinct roots.
- [x] 1.4 Regenerate the crate report goldens under the recorded policy and add positive and negative crate tests for both new diagnostics.

## 2. Trace projection

- [x] 2.1 Implement the `trace` module: project one stable ID's chain (atom revision, surface rows, links, carriers, oracle references) with bound and recomputed values per hop, rendered as canonical `chelis-trace-projection/v1` JSON.
- [x] 2.2 Wire `trace --root --config --id` into the CLI preserving the exit classes: success only when the static check accepts and every hop matches.
- [x] 2.3 Add `tests/trace_projection.rs`: golden for a current chain, a tampered-statement rejection, a tampered-body rejection, and cross-root byte equality.

## 3. Rebind regeneration

- [x] 3.1 Implement the `rebind` module: derive the field-level plan (path, byte range, old value, new value) for item digests, atom revisions in references and surface rows, oracle digest references, and declared runner digests; render the plan as canonical bytes.
- [x] 3.2 Wire `rebind --root --config [--write]`: dry run never mutates; `--write` splices only `--`-prefixed record field lines, refuses on any parse rejection, and re-extracts to confirm convergence.
- [x] 3.3 Add `tests/rebind_regeneration.rs`: dry-run immutability, write-then-empty-diff idempotence, declaration byte immutability, static acceptance after write, and the parse-rejection refusal.

## 4. Contract pack and registrations

- [x] 4.1 Add frozen cases for `CHELIS-PROV-015..018` with manifest rows and extend the requirement coverage loop.
- [x] 4.2 Add adversarial-review rows for the four requirements, including the rebind-laundering review.
- [x] 4.3 Update the trust document: enforcement completeness, the context format, the trace projection boundary, and the rebind review policy; list unbound oracle runner configs as a visible advisory gap.

## 5. Assurance convergence

- [x] 5.1 Re-run the fuzz and mutation campaigns through the sanctioned tools with honest artifact updates.
- [x] 5.2 Run the sanctioned regeneration to a fixed point: evidence, ledger, naming inventory, quality, digest correspondence, and the reviewed source-content digest.
- [x] 5.3 Final oracle: full `devenv test` exits 0.
