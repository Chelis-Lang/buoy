## 1. Freeze the defect and compatibility boundary

- [x] 1.1 Reproduce the false `SPG-REF-DANGLING-OR-STALE` result with a current semantic atom revision.
- [x] 1.2 Record the unequal semantic revision, atom object digest, link digest, and graph digest.
- [x] 1.3 Confirm that the fix requires no machine-schema, fence-syntax, relation, target, or provider change.

## 2. Add failing regression tests first

- [x] 2.1 Change the current-link core contract to use `atom.revision()` and require clean graph validation.
- [x] 2.2 Add a core negative case that puts `atom.meta().digest()` in the semantic revision field.
- [x] 2.3 Add an OpenSpec version 2 graph regression with a dynamically current semantic revision.
- [x] 2.4 Add an OpenSpec version 2 negative case with the atom object digest in the revision field.
- [x] 2.5 Run the focused tests before production edits and confirm both expected failures.

## 3. Repair atom-revision closure

- [x] 3.1 Remove semantic atom revisions from generic `ObjectRef` metadata in `ExternalTraceLink`.
- [x] 3.2 Add every external trace link to dedicated atom-revision closure validation.
- [x] 3.3 Preserve canonical link fields, link digests, diagnostic ordering, and the existing stale code.
- [x] 3.4 Confirm that implementation-link and generic object-reference behavior remain unchanged.

## 4. Complete focused evidence and documentation

- [x] 4.1 Run the positive current-revision and negative object-digest core contracts.
- [x] 4.2 Run the OpenSpec version 2 current, object-digest, and requirement-drift regressions.
- [x] 4.3 Document the semantic atom revision and atom object digest distinction.
- [x] 4.4 Confirm that a fresh external link still supplies navigation and impact only.
- [x] 4.5 Run Rust formatting, focused Clippy, and `git diff --check`.
- [x] 4.6 Refresh any generated assurance inventory that binds changed source bytes.

## 5. Run final acceptance and prepare the prerequisite pin

- [x] 5.1 Run strict validation for this change and all OpenSpec specifications.
- [x] 5.2 Run the exact `devenv test` acceptance oracle.
- [x] 5.3 Inspect the final diff for digest-domain separation, deterministic failure, and rollback completeness.
- [x] 5.4 Record the final static result without a fabricated compact binding locator.
- [x] 5.5 Record that Barnacle must pin the squash-merged Buoy commit after integration.
