## Requirement Actions

Use one row for each planned code, evidence, documentation, or external link action.

| Requirement ID | Target class | Action | Owner | Focused Buoy command |
|---|---|---|---|---|
| `DEMO-001` | code | add | implementation owner | `buoy static --root ROOT --config FILE --format machine` |

## Planned Implementation Relations

Use one row for each planned compact implementation relation.

Record the stable relation ID, the consumer repository, and the logical target selector. Do not record a record digest, a subject digest, or an atom revision. Those values exist only after a successful bind operation.

If the change plans no compact relation, keep the table empty. Do not write a placeholder value.

| Requirement ID | Relation ID | Consumer | Logical target selector | Action | Owner | Focused Buoy command |
|---|---|---|---|---|---|---|
| `DEMO-001` | `DEMO-RENDER` | `example/consumer` | `src/lib.rs::render` | add | implementation owner | `buoy trace --requirement-id DEMO-001 --format machine` |

## Link Review

Review each current link for every modified requirement.

| Requirement ID | Current link | Disposition | Evidence |
|---|---|---|---|
| `DEMO-001` | none | add | `path/to/reviewed/evidence` |

## Completion Snapshot

This section is historical. Buoy remains the current-state authority. Run a new query to read current state.

Generate the machine-readable block with `buoy publication evidence`. Do not hand-author its contents.

<!-- BEGIN buoy-trace-publication-evidence-v1 -->
<!-- END buoy-trace-publication-evidence-v1 -->

Record the human completion facts the generated block cannot supply: owner, consumer commit, and review decision. Do not copy the immutable record body. Exclude provider status.

If no link action is complete, keep the generated block empty and the table empty.

| Owner | Consumer commit | Review decision |
|---|---|---|

## External Targets

Record repository-owned target text only. Exclude provider status, fetched titles, users, and timestamps.
