## Requirement Actions

| Requirement ID | Target class | Action | Owner | Buoy command |
|---|---|---|---|---|
| `ETL-006` | code | modify | Buoy core owner | `devenv shell -- cargo test -p buoy-core --test graph_validation_contract external_trace_link -- --nocapture` |
| `ETL-006` | evidence | add | Buoy core owner | `devenv shell -- cargo test -p buoy-core --test graph_validation_contract external_trace_link -- --nocapture` |
| `ETL-006` | evidence | add | OpenSpec adapter owner | `devenv shell -- cargo test --test openspec_v2_identity version_two_external_trace_link -- --nocapture` |
| `ETL-006` | existing proof binding | rebind | proof owner | `devenv tasks run buoy:core-proofs --mode single` |
| `ETL-006` | bounded fuzz evidence | refresh | adversarial owner | `devenv shell -- buoy-fuzz-campaign` |
| `ETL-006` | bounded mutation evidence | refresh | adversarial owner | `devenv shell -- buoy-mutation-campaign` |
| `ETL-006` | generated assurance evidence | refresh | assurance owner | `devenv test` |
| `ETL-006` | documentation | modify | Buoy documentation owner | `git diff --check -- docs/schema/v1/reports.md docs/schema/v3/openspec-traceability.md` |

## Defect Snapshot

The saved Barnacle graph produced these exact values before the fix:

| Fact | Canonical value |
|---|---|
| Semantic atom revision | `blake3-256:fe1be1041c73c0bc9540b73353118bccb3f1e0b0180450764168718872f36c19` |
| Atom object digest | `blake3-256:12d107ef623a148fd92b649529f9c61a0d601ca146b4718cf990c9b3778ad0c5` |
| External-link digest | `blake3-256:5f8876b2dcbb73cde849a30675810318d53d7c0d7bc6c65cdf49ae3f4bc38820` |
| Graph digest | `blake3-256:18af7bc0a0c23ea84df555707c8e83685e87c7ed5935d04f16a7f6ed52b27c32` |
| False diagnostic | `SPG-REF-DANGLING-OR-STALE` |

The fix changes no machine schema, fence syntax, relation, target, or provider boundary.

## Planned Implementation Relations

This change plans no compact implementation relation.

| Requirement ID | Relation ID | Consumer | Logical target selector | Action | Owner | Focused Buoy command |
|---|---|---|---|---|---|---|

## Link Review

No current compact implementation relation or external trace link targets `ETL-006`.

| Requirement ID | Current link | Disposition | Evidence |
|---|---|---|---|
| `ETL-006` | `ExternalTraceLink` atom-revision closure | revise | `crates/buoy-core/src/model.rs`, `crates/buoy-core/src/algorithms.rs`, and focused regressions |

## Completion Snapshot

This section is historical. Buoy remains the current-state authority. Run a new query to read current state.

This change creates no compact implementation binding record. The completion table remains empty.

The final Barnacle candidate static result was exit `0` with graph `blake3-256:18af7bc0a0c23ea84df555707c8e83685e87c7ed5935d04f16a7f6ed52b27c32` and no diagnostics.

After integration, Barnacle must pin the exact squash-merged Buoy commit. A feature commit is not the final pin.

| Requirement ID | Canonical record locator | Consumer commit | Final static result |
|---|---|---|---|

## External Targets

This change adds no external target.
