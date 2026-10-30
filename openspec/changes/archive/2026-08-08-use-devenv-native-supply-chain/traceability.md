## Requirement Actions

List each affected requirement. Use one table row for each required link action.

| Requirement ID | Target class | Action | Owner | Buoy command |
|---|---|---|---|---|
| `RSC-001` | code | revise | supply-chain maintainer | `devenv shell -- true` |
| `RSC-002` | code | revise | supply-chain maintainer | `devenv tasks run buoy:offline-acceptance` |
| `RSC-003` | code | revise | supply-chain maintainer | `devenv tasks run buoy:deny` |
| `RSC-004` | code | remove | supply-chain maintainer | `devenv tasks run buoy:supply-chain` |
| `RSC-005` | spec | revise | supply-chain maintainer | `openspec validate use-devenv-native-supply-chain --strict` |
| `RSC-006` | code | revise | supply-chain maintainer | `devenv tasks run buoy:closure-size` |

## Link Review

List current links that the change must retain, revise, retire, or waive.

| Requirement ID | Current link | Disposition | Evidence |
|---|---|---|---|
| `RSC-004` | closure-inventory execution rows | retire | `assurance/v1/scenario-requirement-executions.tsv` |
| `RSC-003` | supply-chain policy rows | revise | `assurance/v1/obligation-ledger.tsv` |

## External Targets

Record only repository-owned target text. Do not record provider status, titles, users, or timestamps.

None. This change adds no external trace link.
