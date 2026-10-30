## 1. Freeze rename boundaries and test the migration controls

- [x] 1.1 Record the pre-rename canonical v1 value, graph, static/execution/impact report, protocol, diagnostic, and historical-reader corpus with exact full digests.
- [x] 1.2 Generate a machine-readable inventory of every tracked `Spec Provenance`, `spec-provenance`, `spec_provenance`, and related old-name occurrence with path, class, rationale, owner, and retention/removal rule.
- [x] 1.3 Define the closed naming dispositions `rename-product`, `compatibility-alias`, `frozen-v1-identifier`, `historical-reference`, and `generated-or-external`, including the `>=0.2.0,<0.3.0` alias window.
- [x] 1.4 Add positive naming-inventory fixtures for current Buoy branding, each retained disposition, exact frozen identifiers, generated files, and historical migration prose.
- [x] 1.5 Add negative naming-inventory fixtures for missing, duplicate, broad-exclusion, wrong-class, modified-frozen-identifier, and expired-alias rows.
- [x] 1.6 Implement a standalone naming-inventory checker with negative self-tests proving every fixture field and disposition affects acceptance.
- [x] 1.7 Add a hardening manifest mapping every PID-001 through PID-008 positive and negative scenario to an executable operation, expected stable diagnostic/fact, focused command, and final oracle.
- [x] 1.8 Make every rename fixture operation and mutation executable and reject unknown, zero-target, multiple-target, no-op, stale-expected, and substituted-authority cases.

## 2. Establish Buoy Cargo and dependency identities

- [x] 2.1 Add tests that require primary Cargo package names `buoy`, `buoy-core`, and `buoy-reference` and Rust import names `buoy`, `buoy_core`, and `buoy_reference`.
- [x] 2.2 Add compile-fail and structural fixtures proving `buoy-core` remains `no_std`, effect-free, and independent of shell, adapter, compatibility, repository, and provider crates.
- [x] 2.3 Add dependency-graph fixtures proving all primary and compatibility crates depend one-way toward `buoy-core` and that no compatibility facade is a second implementation.
- [x] 2.4 Rename the root package/library to `buoy`, the pure package/crate to `buoy-core`/`buoy_core`, and the independent test authority to `buoy-reference`/`buoy_reference`.
- [x] 2.5 Migrate workspace manifests, exact dependency declarations, imports, doctests, examples, tests, feature references, package filters, and generated Cargo metadata to the Buoy identities.
- [x] 2.6 Add repository-local `spec-provenance` and `spec-provenance-core` compatibility facade packages for the declared window, containing only delegation/re-exports and deterministic deprecation metadata.
- [x] 2.7 Add package-facade parity tests and an expiry mutation proving the old packages fail closed at `0.3.0`.
- [x] 2.8 Re-run dependency-boundary, `no_std` target, compile-fail, Clippy, documentation, and complete workspace tests under the renamed package graph.

## 3. Rename CLI and operational surfaces

- [x] 3.1 Add positive CLI fixtures for `buoy static`, `buoy change`, and `buoy execute`, including exact help, success, usage, input, policy, and internal exit classes.
- [x] 3.2 Add old/new CLI parity fixtures proving authoritative bytes and exit classes match while deterministic deprecation text is noncanonical.
- [x] 3.3 Add negative CLI fixtures that inject the invoked binary name, current brand, warning text, elapsed time, PID, scheduling, or host path into authoritative reports.
- [x] 3.4 Make `buoy` the primary binary and route the temporary `spec-provenance` binary through a thin shell-side compatibility entry point.
- [x] 3.5 Rename repository-owned Nix/devenv tasks, scripts, environment names, temporary artifact labels, and focused commands to Buoy identities, including primary `buoy:coverage` and `buoy:properties` tasks, retaining tested aliases only for the declared window.
- [x] 3.6 Add operational-command existence, delegation, exact-output, and expiry tests for every retained `spec-provenance-*` or `spec-provenance:*` alias, including the former coverage and property task names.
- [x] 3.7 Rename default noncanonical configuration, cache, state, log, generated-output, and local tool-home labels where they exist; preserve explicit repository inputs and reject host absolute paths.
- [x] 3.8 Add a release checklist for the external repository slug and package-registry ownership, requiring fresh provider approval and proving neither provider state nor checkout basename affects canonical identity.

## 4. Preserve frozen canonical and protocol identity

- [x] 4.1 Add exact pre/post-rename differential tests for every canonical type tag, nested value, semantic domain, graph, verdict set, change value, and complete static/execution/impact report.
- [x] 4.2 Add negative vectors replacing each frozen `spec-provenance-*/v1` schema, result-protocol, digest-domain, and historical-reader identifier with a Buoy-branded string.
- [x] 4.3 Keep frozen v1 schema IDs, domain separators, result protocols, stable diagnostics, fixture schemas, and migration reader identifiers unchanged in production code and generated tables.
- [x] 4.4 Rename only noncanonical or explicitly presentation-versioned product labels in JSON/human output and regenerate reviewed presentation goldens.
- [x] 4.5 Add static and effectful permutation tests proving product name, binary alias, traversal order, completion order, locale, timezone, and checkout root do not change authoritative bytes.
- [x] 4.6 Run the independent canonical authority and historical readers and compare the full pre-rename corpus byte-for-byte before accepting any renamed artifact.

## 5. Migrate documentation, examples, and planning artifacts

- [x] 5.1 Update README, architecture, contribution, schema, assurance, formal-method, command, installation, and migration documentation to use Buoy as current branding.
- [x] 5.2 Update maintained examples, comments, doc tests, package descriptions, generated human presentations, and source-inventory rationales to Buoy where they are not frozen or historical.
- [x] 5.3 Add a migration guide mapping old/new product, package, crate, binary, command, configuration, and repository names and documenting the `0.3.0` alias removal.
- [x] 5.4 Reconcile the in-progress production-hardening proposal, design, specs, tasks, commands, and source paths to Buoy without changing frozen v1 meanings or marking unfinished work complete.
- [x] 5.5 Reconcile the focused `add-cargo-llvm-cov-hegel-rs-testing-requirements` proposal, design, specs, and tasks to `buoy`, `buoy-core`, `buoy-reference`, `buoy:coverage`, and `buoy:properties`; update source-inventory and evidence-binding names; retain former commands only as bounded tested aliases; and map completed focused obligations exactly once into production-hardening P0 without changing `LCOV-*` or `HEGEL-*` meanings or marking unfinished work complete.
- [x] 5.6 Reconcile the deferred Chelis integration proposal, design, spec, and tasks to consume Buoy shell-side APIs while preserving saved-snapshot behavior and one-way dependency boundaries.
- [x] 5.7 Add checks proving no Chelis compiler-core semantic change, provider state, OpenSpec state, network, clock, locale, or Git state becomes canonical because of the rename.
- [x] 5.8 Run strict validation for every baseline and in-progress OpenSpec artifact, including focused coverage/Hegel and broader hardening artifacts, and reject unclassified mixed branding or duplicate obligation mappings.

## 6. Rebind source-, tool-, path-, and name-bound assurance

- [x] 6.1 Regenerate the complete source inventory with Buoy paths/packages and explicit compatibility, generated, formal, platform, and no-executable-region dispositions.
- [x] 6.2 Rebind canonical/presentation generators, standalone validators, Dylint detector/fixtures, formal sources, and exact coverage aggregation to their renamed targets.
- [x] 6.3 Review the Verus/Aeneas correspondence against renamed implementation representations; update exact proof/derivation bindings and replay positive and negative controls without broadening scope.
- [x] 6.4 Invalidate and replay every coverage, fuzz, mutation, fault, sanitizer, Miri, compile-fail, platform, and supply-chain artifact whose source path, package identity, command, or configuration changed.
- [x] 6.5 Add negative freshness fixtures proving unchanged source hashes, renamed tests, annotations, compatibility parity, or aggregate green status cannot carry pre-rename assurance forward.
- [x] 6.6 Update the assurance-obligation ledger so each PID scenario, naming row, compatibility alias, source target, platform blocker, proof, and campaign has exact current evidence or an allowed explicit nonpassing disposition.
- [x] 6.7 Re-run exact line, region, function, and branch coverage over renamed Cargo targets without rounding or broad exclusions; record the exact post-rename result and retain unfinished 100% closure as an explicit nonpassing `LCOV-*` P0 disposition owned by the focused coverage change.

## 7. Close compatibility and final acceptance

- [x] 7.1 Add clean-install and upgrade fixtures for new Buoy users and users migrating from each old package, crate, binary, and operational command.
- [x] 7.2 Add rollback fixtures proving product/package aliases can be restored without rewriting frozen v1 bytes or accepted historical artifacts.
- [x] 7.3 Add a `0.3.0` removal fixture proving expired aliases are absent, migration diagnostics remain stable, and historical v1 readers still succeed.
- [x] 7.4 Run the complete PID hardening manifest against production and independent authorities and require exact diagnostics, facts, exit classes, and bytes.
- [x] 7.5 Run formatting, workspace Clippy with warnings denied, all targets/features, documentation, schema/reference/Unicode/fixture/property/model/formal gates, primary and compatibility coverage/property task checks, strict OpenSpec validation, and the stale-name checker.
- [x] 7.6 Run the exact acceptance oracle `devenv test --no-tui` from a clean network-disabled checkout and require exit code 0 with all prior phase/evidence records current.
