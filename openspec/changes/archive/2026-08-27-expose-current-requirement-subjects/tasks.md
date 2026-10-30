## 1. Positive and Negative Fixtures

- [x] 1.1 Add a static report fixture with one current requirement and no binding.
- [x] 1.2 Add fixtures for reverse input order and separate physical roots.
- [x] 1.3 Add a repository fixture with no OpenSpec authority.
- [x] 1.4 Add negative fixtures for wrong-kind, absent-node, absent-atom, duplicate-atom, and revision-mismatch aggregates.
- [x] 1.5 Freeze exact machine and human report bytes before production implementation.
- [x] 1.6 Add a Hegel-rs property for permutation-invariant requirement-subject ordering.
- [x] 1.7 Run the focused fixtures and record their expected implementation failures.

## 2. Shell Report Implementation

- [x] 2.1 Add the typed `RequirementSubjectFact` shell report value.
- [x] 2.2 Implement fail-closed aggregation from accepted source authorities and live atoms.
- [x] 2.3 Sort rows by the complete CAB-014 tuple without physical inputs.
- [x] 2.4 Add the optional `requirement_subjects` field to `MachineReport`.
- [x] 2.5 Populate the field for version 2 and version 3 static commands.
- [x] 2.6 Keep execution reports and repositories without current requirements free of placeholder rows.
- [x] 2.7 Add complete deterministic requirement-subject lines to the human renderer.
- [x] 2.8 Run focused unit, integration, CLI, and Hegel-rs checks.

## 3. Compatibility and Documentation

- [x] 3.1 Update every affected static machine and human golden.
- [x] 3.2 Update examples that expose current OpenSpec requirements.
- [x] 3.3 Document the field contract, total order, bootstrap procedure, and navigation-only boundary.
- [x] 3.4 Confirm that `buoy-core`, canonical digest domains, graph identities, and binding schemas remain unchanged.
- [x] 3.5 Regenerate package source-content identity and all source-bound assurance evidence.
- [x] 3.6 Run package, schema, documentation, fixture, integration, and adversarial focused gates.

## 4. Acceptance and Archive

- [x] 4.1 Run `openspec validate --all --strict --no-interactive`.
- [x] 4.2 Run `openspec schema validate buoy-bridge`.
- [x] 4.3 Run the exact repository acceptance command `devenv test`.
- [x] 4.4 Audit report completeness, deterministic ordering, failure polarity, and assurance claims.
- [x] 4.5 Sync `CAB-014` into the standing specification and validate it strictly.
- [x] 4.6 Archive `expose-current-requirement-subjects` and run post-archive acceptance.
- [x] 4.7 Request fresh approval for the final archive commit.
