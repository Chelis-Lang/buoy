## 1. Freeze fixtures before production code

- [x] 1.1 Add accepted fixtures with preamble text, multibyte text, requirements, scenarios, and trailing nonauthority sections.
- [x] 1.2 Add accepted fixtures with prose between the requirements heading and the first requirement.
- [x] 1.3 Add negative fixtures for missing and unsupported capability format values.
- [x] 1.4 Add negative fixtures for duplicate requirements sections and unsupported authority-like headings.
- [x] 1.5 Add a revision fixture that changes trailing notes without changing the preceding atom revision.
- [x] 1.6 Add a documentation fixture that rejects the title-first bracketed-ID heading form.
- [x] 1.7 Freeze stable diagnostics for format and unsupported-syntax failures.

## 2. Add models and properties before production algorithms

- [x] 2.1 Add a finite interval model for valid primary partitions and nested scenario spans.
- [x] 2.2 Add negative model cases for gaps, primary overlap, out-of-bounds spans, and invalid scenario containment.
- [x] 2.3 Add Hegel-rs properties for partition completeness, non-overlap, bounds, and scenario containment.
- [x] 2.4 Add Hegel-rs properties for Unicode prefixes, legal section lengths, and deterministic diagnostic order.
- [x] 2.5 Record that this shell parser adds no Verus or AeneasVerif proof claim.

## 3. Implement typed format and document parsing

- [x] 3.1 Parse each raw format value into a closed `OpenSpecFormatVersion` type.
- [x] 3.2 Return `SPG-OPENSPEC-CONFIG-FORMAT` for a missing or unsupported format value.
- [x] 3.3 Add the exact format field to all positive OpenSpec configuration fixtures.
- [x] 3.4 Parse saved Markdown into typed requirement, scenario, and nonauthority sections.
- [x] 3.5 End each requirement before the next heading at level three or lower.
- [x] 3.6 Build complete nonnormative intervals outside primary requirement spans.
- [x] 3.7 Validate the primary partition and nested scenario containment before atom construction.
- [x] 3.8 Reject malformed documents without a partial semantic graph.
- [x] 3.9 Remove inferred authority metadata from the public production extraction path.
- [x] 3.10 Keep all parser and configuration effects in the root shell crate.

## 4. Update documentation and evidence

- [x] 4.1 Document `### Requirement: <stable-id> <title>` as the only accepted heading form.
- [x] 4.2 Add the required format field to the repository configuration example.
- [x] 4.3 Document primary partitions, nested scenario spans, and fail-closed unsupported syntax.
- [x] 4.4 Regenerate affected deterministic fixtures and assurance records.
- [x] 4.5 Review every current link from `traceability.md` after revision changes.

## 5. Run focused checks

- [x] 5.1 Run `devenv shell -- cargo test -p buoy --all-features --test openspec_traceability`.
- [x] 5.2 Run `devenv shell -- cargo test -p buoy --all-features --test adapters`.
- [x] 5.3 Run `devenv shell -- cargo test -p buoy --all-features --test openspec_workflow`.
- [x] 5.4 Run `devenv shell -- cargo clippy -p buoy --all-targets --all-features -- -D warnings`.
- [x] 5.5 Run `devenv shell -- cargo fmt --all -- --check`.

## 6. Archive acceptance

- [x] 6.1 Run `openspec validate --all --strict --no-interactive`.
- [x] 6.2 Run `openspec schema validate buoy-bridge`.
- [x] 6.3 Run the exact repository acceptance command `devenv test`.
- [x] 6.4 Accept the change only when every required command exits with code `0`.
