## Requirement Actions

| Requirement ID | Target class | Action | Owner | Buoy command |
|---|---|---|---|---|
| `PBD-001` | declared source closure | exclude generated and runtime paths | Buoy Nix | `devenv tasks run buoy:package-contract --mode single` |
| `PBD-001` | reviewed source-content digest | re-derive | Buoy Nix | `nix/tools/derive-source-content-digest.sh --verify` |
| `PBD-005` | package checks | add a declared-source filter check to the derivation contract | Buoy Nix | `nix/tools/check-derivation-contract.sh` |

## Planned Implementation Relations

| Requirement ID | Relation ID | Consumer | Logical target selector | Action | Owner | Focused Buoy command |
|---|---|---|---|---|---|---|

## Link Review

| Requirement ID | Current link | Disposition | Evidence |
|---|---|---|---|
| `PBD-001` | reviewed digest in `nix/consumer-package-v1.source-content.json` | revise | `nix/tools/derive-source-content-digest.sh --verify` |
| `PBD-005` | derivation contract in `nix/tools/check-derivation-contract.sh` | revise | `derivation-contract: declared source excludes generated paths` |

## Defect Snapshot

On `main` at `a88fd2e`, the declared source closure derived these digests:

| Worktree state | Derived digest |
|---|---|
| Devenv worktree after `devenv test` (the committed value) | `sha256-UhgI5BunpO0QkwIirvQ6Jjb3cmi9AhDwMN0hRfkelms=` |
| The same worktree without `fuzz/artifacts` | `sha256-/bkYE50Knz1e3AWdlVWY5o2uY2r/GS5AylWdGXc5uIE=` |
| Clean export of the commit | `sha256-WpJKnGKo8+40wwSisLKH0c+g2TrXY+5VVuyfWR/X4hk=` |

## Completion Snapshot

This section is historical. Buoy remains the current-state authority. Run a new query to read current state.

<!-- BEGIN buoy-trace-publication-evidence-v1 -->
<!-- END buoy-trace-publication-evidence-v1 -->

| Owner | Consumer commit | Review decision |
|---|---|---|

## External Targets

None.
