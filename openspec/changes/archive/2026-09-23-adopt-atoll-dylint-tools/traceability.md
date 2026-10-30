## Requirement Actions

| Requirement ID | Target class | Action | Owner | Buoy command |
|---|---|---|---|---|
| `RSC-001` | configuration | revise | supply-chain maintainer | `devenv tasks run buoy:fcis` |
| `RSC-005` | evidence | revise | supply-chain maintainer | `devenv tasks run buoy:obligations` |
| `RSC-006` | evidence | retain | supply-chain maintainer | `devenv tasks run buoy:closure-size` |
| `QAR-003` | evidence | revise | quality-assurance registry owner | `devenv tasks run buoy:quality` |

## Planned Implementation Relations

| Requirement ID | Relation ID | Consumer | Logical target selector | Action | Owner | Focused Buoy command |
|---|---|---|---|---|---|---|

## Link Review

| Requirement ID | Current link | Disposition | Evidence |
|---|---|---|---|
| `RSC-001` | Dylint tool derivation in `nix/dylint-tools.nix` | revise | `devenv.yaml` and `devenv.lock` |
| `RSC-005` | `lock:devenv` obligation ledger row | revise | `assurance/v1/obligation-ledger.tsv` |
| `RSC-006` | `dylint-tools`, `dylint-driver-vendor`, and `dylint-driver-source` size budgets | retain | `assurance/supply-chain/v1/closure-size-policy.tsv` |
| `QAR-003` | `QA-0021` configuration binding to `nix/dylint-tools.nix` | revise | `assurance/v1/quality-assurance-bindings.tsv` |

## Completion Snapshot

This section is historical. Buoy remains the current-state authority. Run a new query to read current state.

<!-- BEGIN buoy-trace-publication-evidence-v1 -->
<!-- END buoy-trace-publication-evidence-v1 -->

| Owner | Consumer commit | Review decision |
|---|---|---|

## External Targets

None.
