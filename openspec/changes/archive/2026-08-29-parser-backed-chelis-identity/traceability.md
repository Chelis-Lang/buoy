## Requirement Actions

List each affected requirement. Use one table row for each required link action.

| Requirement ID | Target class | Action | Owner | Buoy command |
|---|---|---|---|---|
| `CHELIS-PROV-012` | code | add | implementation owner | `devenv shell -- cargo test -p chelis-provenance` |
| `CHELIS-PROV-012` | code | add | implementation owner | `devenv shell -- cargo test -p chelis-surf-sitter` |
| `CHELIS-PROV-013` | code | add | implementation owner | `devenv shell -- cargo test -p chelis-provenance --test goldens_and_mutations` |
| `CHELIS-PROV-014` | code | add | implementation owner | `devenv shell -- cargo test -p chelis-provenance --test integration` |

## Planned Implementation Relations

List each planned compact implementation relation. Record the stable relation ID, the consumer repository, and the logical target selector.

A planned row records intent only. Do not record a record digest, a subject digest, a requirement digest, or an atom revision. Those values exist only after a successful bind operation.

If the change plans no compact relation, keep the table empty. Do not write a placeholder ID, selector, owner, or digest.

| Requirement ID | Relation ID | Consumer | Logical target selector | Action | Owner | Focused Buoy command |
|---|---|---|---|---|---|---|

## Link Review

No current external trace link selects an affected standing requirement.

| Requirement ID | Current link | Disposition | Evidence |
|---|---|---|---|
| `CHELIS-PROV-012` | none | add | parse fixtures, node-identity goldens, and the vendored-grammar drift check |
| `CHELIS-PROV-013` | none | add | report-v2 goldens and the cross-root byte-equality assertion |
| `CHELIS-PROV-014` | none | add | consumer-package contract checks and advisory CLI goldens |

## Completion Snapshot

This section is historical. Buoy remains the current-state authority. Run a new query to read current state.

Generate the machine-readable block with `buoy publication evidence`. Do not hand-author its contents.

<!-- BEGIN buoy-trace-publication-evidence-v1 -->
<!-- END buoy-trace-publication-evidence-v1 -->

Record the human completion facts the generated block cannot supply: owner, consumer commit, and review decision.

| Owner | Consumer commit | Review decision |
|---|---|---|

Do not copy the immutable record body. Do not record provider status.
