## Requirement Actions

| Requirement ID | Target class | Action | Owner | Buoy command |
|---|---|---|---|---|
| `DEV-001` | configuration | revise `devenv.yaml` and `devenv.lock` version agreement | environment owner | `devenv tasks run buoy:devenv` |
| `DEV-001` | fixture | add matching and mismatched CLI controls | environment owner | `devenv tasks run buoy:devenv` |
| `DEV-002` | configuration | enable `languages.rust` and bind `rust-toolchain.toml` | toolchain owner | `devenv tasks run buoy:devenv` |
| `DEV-002` | fixture | add module, LSP, Dylint, Verus, and duplicate-authority controls | toolchain owner | `devenv tasks run buoy:devenv` |
| `DEV-003` | configuration | split Devenv configuration into imported local modules | environment owner | `devenv tasks run buoy:devenv` |
| `DEV-003` | fixture | add resolved-value and duplicate-script-owner controls | environment owner | `devenv tasks run buoy:devenv` |
| `DEV-004` | configuration | set `cwd = config.git.root` for repository tasks | task-graph owner | `devenv tasks run buoy:devenv` |
| `DEV-004` | fixture | add root, subdirectory, and missing-root controls | task-graph owner | `devenv tasks run buoy:devenv` |
| `DEV-005` | configuration | replace duplicate gate bodies with one serial task graph | task-graph owner | `devenv tasks run buoy:devenv` |
| `DEV-005` | fixture | add lifecycle, order, duplicate-path, and cache controls | task-graph owner | `devenv tasks run buoy:devenv` |
| `DEV-006` | code | add the typed resolved-task graph checker | task-graph owner | `devenv tasks run buoy:devenv` |
| `DEV-006` | fixture | add malformed graph and exact phase-ownership controls | task-graph owner | `devenv tasks run buoy:devenv` |
| `DEV-007` | configuration | add pinned lightweight Git hooks | environment owner | `devenv tasks run buoy:devenv` |
| `DEV-007` | fixture | add hook-scope and non-authority controls | quality owner | `devenv tasks run buoy:devenv` |
| `DEV-008` | configuration | assign specialized packages to focused scripts | environment owner | `devenv tasks run buoy:devenv` |
| `DEV-008` | evidence | revise tool exposure and closure inventory data | supply-chain owner | `devenv tasks run buoy:closure-size` |
| `ADV-013` | code | replace source-text phase checks with resolved graph checks | task-graph owner | `devenv tasks run buoy:gate` |
| `ADV-013` | evidence | revise phase fixtures, executions, and ledger bindings | assurance owner | `devenv tasks run buoy:obligations` |
| `RSC-001` | code | check CLI, module, hook, and runtime package identities | supply-chain owner | `devenv tasks run buoy:closure-size` |
| `RSC-001` | evidence | revise immutable closure fixtures and bindings | supply-chain owner | `devenv tasks run buoy:closure-size` |
| `RSC-002` | code | extend the clean offline oracle for task runtime packages | supply-chain owner | `devenv tasks run buoy:closure-size` |
| `RSC-002` | evidence | revise clean-sandbox fixtures and execution records | supply-chain owner | `devenv tasks run buoy:closure-size` |
| `RSC-004` | code | add the tool exposure mode to closure inventory checks | supply-chain owner | `devenv tasks run buoy:closure-size` |
| `RSC-004` | evidence | regenerate the closure inventory and size bindings | supply-chain owner | `devenv tasks run buoy:closure-size` |
| `QAR-001` | code | include script runtime tools in registry completeness | quality owner | `devenv tasks run buoy:quality` |
| `QAR-001` | evidence | revise QA controls, fixtures, and generated reports | quality owner | `devenv tasks run buoy:quality` |
| `QAR-005` | code | bind blocking inclusion to the resolved task graph | quality owner | `devenv tasks run buoy:quality` |
| `QAR-005` | fixture | revise final-path and missing-edge controls | quality owner | `devenv tasks run buoy:quality` |
| `QAR-008` | code | keep task inspection in the imperative shell | quality owner | `devenv tasks run buoy:quality` |
| `QAR-008` | evidence | revise focused and final command bindings | quality owner | `devenv tasks run buoy:quality` |

## Link Review

| Requirement ID | Current link | Disposition | Evidence |
|---|---|---|---|
| `ADV-013` | `fixtures/hardening/v2/hardening-manifest.tsv` | revise scenario names and graph authority | `devenv tasks run buoy:gate` |
| `ADV-013` | `assurance/v1/scenario-requirement-executions.tsv` | regenerate task and result bindings | `devenv tasks run buoy:obligations` |
| `ADV-013` | `assurance/v1/obligation-ledger.tsv` | regenerate requirement and scenario rows | `devenv tasks run buoy:obligations` |
| `RSC-001` | `fixtures/hardening/v2/hardening-manifest.tsv` | revise the negative runtime-package control | `devenv tasks run buoy:closure-size` |
| `RSC-001` | `assurance/v1/scenario-requirement-executions.tsv` | regenerate scenario bindings | `devenv tasks run buoy:obligations` |
| `RSC-001` | `assurance/v1/obligation-ledger.tsv` | regenerate requirement and scenario rows | `devenv tasks run buoy:obligations` |
| `RSC-002` | `fixtures/hardening/v2/hardening-manifest.tsv` | revise the clean-sandbox scenarios | `devenv tasks run buoy:closure-size` |
| `RSC-002` | `assurance/v1/scenario-requirement-executions.tsv` | regenerate scenario bindings | `devenv tasks run buoy:obligations` |
| `RSC-002` | `assurance/v1/obligation-ledger.tsv` | regenerate requirement and scenario rows | `devenv tasks run buoy:obligations` |
| `RSC-004` | `fixtures/hardening/v2/hardening-manifest.tsv` | revise the exposure-mode scenarios | `devenv tasks run buoy:closure-size` |
| `RSC-004` | `assurance/v1/scenario-requirement-executions.tsv` | regenerate scenario bindings | `devenv tasks run buoy:obligations` |
| `RSC-004` | `assurance/v1/obligation-ledger.tsv` | regenerate requirement and scenario rows | `devenv tasks run buoy:obligations` |
| `QAR-001` | `assurance/v1/quality-assurance-controls.toml` | revise environment and tool scope | `devenv tasks run buoy:quality` |
| `QAR-001` | `assurance/v1/quality-fixture-executions.tsv` | regenerate fixture results | `devenv tasks run buoy:quality` |
| `QAR-001` | `assurance/v1/scenario-requirement-executions.tsv` | regenerate scenario bindings | `devenv tasks run buoy:obligations` |
| `QAR-001` | `assurance/v1/obligation-ledger.tsv` | regenerate requirement and scenario rows | `devenv tasks run buoy:obligations` |
| `QAR-005` | `assurance/v1/quality-assurance-controls.toml` | revise blocking graph bindings | `devenv tasks run buoy:quality` |
| `QAR-005` | `assurance/v1/quality-fixture-executions.tsv` | regenerate omission and not-run results | `devenv tasks run buoy:quality` |
| `QAR-005` | `assurance/v1/scenario-requirement-executions.tsv` | regenerate scenario bindings | `devenv tasks run buoy:obligations` |
| `QAR-005` | `assurance/v1/obligation-ledger.tsv` | regenerate requirement and scenario rows | `devenv tasks run buoy:obligations` |
| `QAR-008` | `assurance/v1/quality-assurance-controls.toml` | revise focused and final graph bindings | `devenv tasks run buoy:quality` |
| `QAR-008` | `assurance/v1/quality-fixture-executions.tsv` | regenerate functional-boundary results | `devenv tasks run buoy:quality` |
| `QAR-008` | `assurance/v1/scenario-requirement-executions.tsv` | regenerate scenario bindings | `devenv tasks run buoy:obligations` |
| `QAR-008` | `assurance/v1/obligation-ledger.tsv` | regenerate requirement and scenario rows | `devenv tasks run buoy:obligations` |

## External Targets

This change creates no external provider target or external trace link.
