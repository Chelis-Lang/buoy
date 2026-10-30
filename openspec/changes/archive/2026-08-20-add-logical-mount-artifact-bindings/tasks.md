## 1. Test Inputs and Negative Controls

- [x] 1.1 Add version 3 configuration fixtures for one mount, two mounts, reordered mounts, and version 2 compatibility.
- [x] 1.2 Add negative fixtures for missing mappings, duplicate IDs, overlapping prefixes, logical collisions, and mixed root arguments.
- [x] 1.3 Add descriptor fixtures for regular roots, link roots, link entries, nested repositories, and root replacement races.
- [x] 1.4 Add aggregate budget fixtures that distribute counts and bytes across multiple mounts.
- [x] 1.5 Add opaque artifact fixtures for UTF-8, invalid UTF-8, empty bytes, byte drift, and forbidden authority roles.
- [x] 1.6 Add canonical sidecar fixtures for valid, wrong-path, duplicate, stale-subject, wrong-record, and foreign-member cases.
- [x] 1.7 Add transaction fixtures for each write, synchronization, replacement, rollback, and retry failure stage.
- [x] 1.8 Add a two-root Barnacle-shaped fixture with Rust, Nix, README, object-store, and OpenSpec inputs.
- [x] 1.9 Freeze machine goldens only through reviewed generation commands. Keep acceptance read-only.

## 2. Reference Models, Properties, and Proof Spikes

- [x] 2.1 Add an independent logical projection model before production mount code.
- [x] 2.2 Add an independent sidecar key and transaction-plan model before production binding code.
- [x] 2.3 Add Hegel-rs properties for mount permutations, projection injection, collisions, bytes, keys, and transaction invariants.
- [x] 2.4 Bind Hegel properties to the exact selected upstream, seeds, settings, production targets, and independent references.
- [x] 2.5 Add a bounded exhaustive model for three mounts, four files, two relations, and all transaction failures.
- [x] 2.6 Run Verus and AeneasVerif feasibility spikes for logical projection and attachment key construction.
- [x] 2.7 Select Verus only after the spikes confirm the design scope and exact representation.
- [x] 2.8 Add an intentionally false proof control and bind every proof input, assumption, exclusion, tool, and configuration.
- [x] 2.9 Document that formal results exclude filesystems, parsers, hash internals, operating systems, and prose correspondence.

## 3. Pure Core Types

- [x] 3.1 Add parsed logical mount IDs and canonical mount declarations without `std` types. Cover `LSM-001` and `LSM-003`.
- [x] 3.2 Add pure logical path projection and disjoint-prefix validation. Cover `LSM-002` and `LSM-003`.
- [x] 3.3 Add canonical artifact attachment values and relation-key derivation under the active member. Cover `CAB-009`.
- [x] 3.4 Add exact sidecar-to-record correspondence validation. Cover `CAB-010` and `CAB-012`.
- [x] 3.5 Add canonical bytes and schema inventory rows for every new public value and enum.
- [x] 3.6 Preserve the one-way dependency rule and bare-metal `no_std` build.

## 4. Configuration and Mounted Snapshots

- [x] 4.1 Parse `buoy/repository-config/v3` into strong mount, selector, oracle, and budget types at the boundary.
- [x] 4.2 Preserve exact version 2 parsing, digest, path, report, and CLI behavior.
- [x] 4.3 Parse repeated `--mount ID=PATH` values and reject incomplete or extra mappings before reads.
- [x] 4.4 Reject `--root` with version 3 and reject `--mount` with version 2.
- [x] 4.5 Open every mount with independent no-follow descriptors in canonical mount order. Cover `REH-009`.
- [x] 4.6 Project selected relative paths into one closed logical namespace. Cover `LSM-001` through `LSM-004`.
- [x] 4.7 Apply one aggregate budget account across all mounts. Cover `LSM-005` and `REH-010`.
- [x] 4.8 Retain no physical path in snapshots, diagnostics, graph values, reports, or digests.
- [x] 4.9 Bind each oracle to one declared working mount without changing static identity. Cover `REH-011`.

## 5. Opaque Artifact Adapter

- [x] 5.1 Register `artifact/v1`, `artifact.opaque-bytes/v1`, and `artifact/file/v1`. Cover `ESI-009`.
- [x] 5.2 Build one full-file item from logical path, fixed metadata, exact saved bytes, and active member.
- [x] 5.3 Emit one complete byte-offset source mapping without UTF-8 decoding. Cover `RAS-001` through `RAS-003`.
- [x] 5.4 Reject optional or required authority expectations for opaque selectors. Cover `RAS-004`.
- [x] 5.5 Emit no semantic object, atom, authority, oracle result, or assurance claim from opaque bytes.
- [x] 5.6 Add adapter SDK controls for undeclared kinds, incomplete spans, unsupported capabilities, and malformed configuration.

## 6. Sidecar Attachments and Graph Closure

- [x] 6.1 Extend `binding/v1` to parse immutable objects and relation-keyed sidecars from distinct path classes.
- [x] 6.2 Reject malformed schemas, wrong keys, wrong members, duplicate relations, and noncanonical sidecar bytes.
- [x] 6.3 Convert valid sidecars into current binding attachments only after all exact correspondence checks.
- [x] 6.4 Keep unselected immutable records historical and traversable without current freshness. Cover `CAB-010`.
- [x] 6.5 Aggregate all mounted source products before attachment and authority resolution. Cover `SRG-006`.
- [x] 6.6 Make graph identity, traversal, and diagnostic precedence mount-order invariant. Cover `SRG-007`.
- [x] 6.7 Suppress derivative diagnostics after one stale or unresolved primary binding failure.

## 7. Raw-Artifact Binding Command

- [x] 7.1 Add exclusive `--artifact LOGICAL_PATH` and `--implementation DIGEST` subject selectors.
- [x] 7.2 Resolve an artifact selector to exactly one current `artifact/file/v1` source node.
- [x] 7.3 Build the unchanged immutable implementation record with explicit requirement, relation, and navigation inputs.
- [x] 7.4 Plan object and sidecar changes before the first filesystem write.
- [x] 7.5 Write, synchronize, and replace each file through bounded temporary paths.
- [x] 7.6 Restore the prior sidecar and remove only uncommitted files after each injected failure.
- [x] 7.7 Leave selected artifact bytes unchanged on success and failure. Cover `CAB-011`.
- [x] 7.8 Report malformed, stale, missing, ambiguous, wrong-kind, and foreign-member selectors as input failures.

## 8. Reports, Migration, and Documentation

- [x] 8.1 Render logical mount facts, opaque subjects, sidecar attachments, and freshness without physical paths.
- [x] 8.2 Add version 3 configuration, adapter, sidecar, and CLI documentation.
- [x] 8.3 Update the adapter SDK, trust boundary, migration, and golden policy documents.
- [x] 8.4 Document version 2 compatibility and the explicit version 3 cutover.
- [x] 8.5 Document the Barnacle pilot layout without modifying Barnacle.
- [x] 8.6 Add hardening scenario authority mappings for `LSM` and `RAS` requirements.
- [x] 8.7 Update source, schema, digest, product identity, quality, and obligation inventories.

## 9. Focused and Repository Validation

- [x] 9.1 Run every focused command in `traceability.md`.
- [x] 9.2 Run workspace tests with all targets, all features, the lockfile, and offline mode.
- [x] 9.3 Run workspace Clippy with all targets, all features, and warnings denied.
- [x] 9.4 Run schema, fixture, property, finite-model, proof, hashing, quality, and obligation gates.
- [x] 9.5 Regenerate governed evidence through each reviewed write command in dependency order.
- [x] 9.6 Run `devenv tasks run buoy:coverage` and require exactly 100% for all four metrics.
- [x] 9.7 Run negative golden acceptance without any write flag.
- [x] 9.8 Confirm that no repository-owned external target needs an update.

## 10. Archive Tasks

- [x] 10.1 Run `openspec validate --all --strict --no-interactive`.
- [x] 10.2 Run `openspec schema validate buoy-bridge`.
- [x] 10.3 Confirm that the OpenSpec apply state is `all_done`.
- [x] 10.4 Run the exact final acceptance oracle `devenv test` and require exit status zero.
- [x] 10.5 Sync all delta requirements into the main specifications.
- [x] 10.6 Archive the completed change and request fresh approval for one archive commit.
