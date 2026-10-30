## Requirement Actions

List each affected requirement. Use one table row for each required link action.

| Requirement ID | Target class | Action | Owner | Buoy command |
|---|---|---|---|---|
| `REQ-001` | code | add | implementation owner | `devenv shell -- cargo test ...` |

## Planned Implementation Relations

List each planned compact implementation relation. Record the stable relation ID, the consumer repository, and the logical target selector.

A planned row records intent only. Do not record a record digest, a subject digest, a requirement digest, or an atom revision. Those values exist only after a successful bind operation.

If the change plans no compact relation, keep the table empty. Do not write a placeholder ID, selector, owner, or digest.

| Requirement ID | Relation ID | Consumer | Logical target selector | Action | Owner | Focused Buoy command |
|---|---|---|---|---|---|---|

## Link Review

List current links that the change must retain, revise, retire, or waive.

| Requirement ID | Current link | Disposition | Evidence |
|---|---|---|---|
| `REQ-001` | `TRACE-001` | revise | `path/to/evidence` |

## Completion Snapshot

This section is historical. Buoy remains the current-state authority. Run a new query to read current state.

Generate the machine-readable block with `buoy publication evidence`. Do not hand-author its contents.

<!-- BEGIN buoy-trace-publication-evidence-v1 -->
<!-- END buoy-trace-publication-evidence-v1 -->

Record the human completion facts the generated block cannot supply: owner, consumer commit, and review decision.

Do not copy the immutable record body. Do not record provider status.

If no link action is complete, keep the generated block empty and the table empty. Do not write a placeholder value.

| Owner | Consumer commit | Review decision |
|---|---|---|

## External Targets

Record only repository-owned target text. Do not record provider status, titles, users, or timestamps.
