## Requirement Actions

| Requirement ID | Target class | Action | Owner | Buoy command |
|---|---|---|---|---|
| `CAB-014` | code | add | Buoy shell owner | `devenv shell -- cargo test --locked --all-features --test requirement_subject_inventory` |
| `CAB-014` | code | modify | Buoy CLI owner | `devenv shell -- cargo test --locked --all-features --test requirement_subject_inventory` |
| `CAB-014` | evidence | add | Buoy fixture owner | `devenv shell -- cargo test --locked --all-features --test requirement_subject_inventory` |
| `CAB-014` | evidence | revise | Buoy golden owner | `devenv tasks run buoy:integration` |
| `CAB-014` | documentation | revise | Buoy documentation owner | `devenv tasks run buoy:docs` |

## Planned Implementation Relations

| Requirement ID | Relation ID | Consumer | Logical target selector | Action | Owner | Focused Buoy command |
|---|---|---|---|---|---|---|

## Link Review

| Requirement ID | Current link | Disposition | Evidence |
|---|---|---|---|
| `CAB-014` | none | add | `tests/requirement_subject_inventory.rs` and `fixtures/requirement-subjects/v1/` |

## Completion Snapshot

This section is historical. Buoy remains the current-state authority. Run a new query to read current state.

<!-- BEGIN buoy-trace-publication-evidence-v1 -->
<!-- END buoy-trace-publication-evidence-v1 -->

| Owner | Consumer commit | Review decision |
|---|---|---|

## External Targets

None.
