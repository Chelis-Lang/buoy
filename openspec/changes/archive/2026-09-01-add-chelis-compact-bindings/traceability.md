## Requirement Actions

List each affected requirement. Use one table row for each required link action.

Compact bindings are content addressed, so their positive suites compile only
when one collision-resistant member is active without `xxh3-128`. The default
build must refuse every compact binding. The dedicated devenv task lints and
tests both BLAKE3-256 and SHA-256 configurations.

| Requirement ID | Target class | Action | Owner | Buoy command |
|---|---|---|---|---|
| `CHELIS-PROV-019` | code | add | implementation owner | `devenv tasks run buoy:chelis-compact-bindings` |
| `CHELIS-PROV-019` | code | add | implementation owner | `devenv shell -- cargo test --locked --offline -p chelis-provenance --test adapter_contract --test binding_change_detection` |
| `CHELIS-PROV-020` | code | add | implementation owner | `devenv tasks run buoy:chelis-compact-bindings` |
| `CHELIS-PROV-020` | code | add | implementation owner | `devenv shell -- cargo test --locked --offline -p buoy-core --test implementation_binding_contract` |
| `CHELIS-PROV-021` | code | add | implementation owner | `devenv tasks run buoy:chelis-compact-bindings` |

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
| `CHELIS-PROV-019` | none | add | address-integrity fixtures and store-resolution tests over the frozen pack |
| `CHELIS-PROV-020` | none | add | subject, requirement, and atom-revision currency fixtures with bound/current contexts |
| `CHELIS-PROV-021` | none | add | bind conversion and rebind successor-object tests with dry-run immutability |

## Completion Snapshot

This section is historical. Buoy remains the current-state authority. Run a new query to read current state.

Generate the machine-readable block with `buoy publication evidence`. Do not hand-author its contents.

<!-- BEGIN buoy-trace-publication-evidence-v1 -->
<!-- END buoy-trace-publication-evidence-v1 -->

Record the human completion facts the generated block cannot supply: owner, consumer commit, and review decision.

| Owner | Consumer commit | Review decision |
|---|---|---|

Do not copy the immutable record body. Do not record provider status.
