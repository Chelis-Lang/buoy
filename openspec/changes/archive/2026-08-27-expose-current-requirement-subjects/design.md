## Context

The binding command accepts an exact requirement source-node digest. The first binding has no trace row that exposes this value.

The static checker already retains current source nodes, source authorities, atoms, logical paths, and atom revisions. Its machine report exposes opaque artifact subjects but omits requirement subjects.

This omission forces a consumer to reimplement canonical identity logic or copy a digest from private debugging output. Both paths weaken the binding boundary.

## Goals / Non-Goals

**Goals:**

- Expose exact current requirement subjects through the existing read-only static report.
- Preserve explicit digest input for `buoy bind`.
- Preserve graph identity, source identity, binding identity, and assurance meaning.
- Produce equal rows for equal logical inputs under different physical roots.
- Fail closed if retained authority facts disagree after accepted checking.

**Non-Goals:**

- Add an implicit stable-ID selector to `buoy bind`.
- Change `buoy trace` or its stored-relation selection contract.
- Add a canonical digest domain or graph object.
- Add provider, network, clock, locale, or filesystem-root data.
- Add execution, coverage, approval, proof, correctness, or semantic correspondence.

## Decisions

### 1. Add a derived shell report fact

`src/check.rs` will define `RequirementSubjectFact`. `MachineReport` will contain `requirement_subjects: Vec<RequirementSubjectFact>`.

Each fact will contain these fields:

- `id`: the stable requirement ID.
- `subject`: the complete current source-node digest.
- `atom_revision`: the complete semantic atom revision.
- `path`: the logical standing-spec path.
- `scopes`: the atom's canonically ordered scopes.

Serialization will omit the top-level field when the vector is empty. Machine JSON will retain the existing field order around the new field.

The human renderer will emit one fixed line for each row. It will not abbreviate either digest.

Alternative: add `--requirement-id` to `buoy bind`. Rejected because digest-explicit review and existing command semantics remain useful.

Alternative: add a new source-query command. Rejected because the static report already owns current source inventory and graph identity.

### 2. Derive rows only from accepted current facts

A shell helper will consume one completed `CheckResult`. A result with diagnostics is not accepted, so the helper will return an unaccepted classification.

The static CLI will preserve the original report, diagnostics, and static exit class for an unaccepted result. It will omit the inventory.

For each source authority in a diagnostic-free result, the helper will resolve:

- The exact retained source node.
- The corresponding live atom by stable ID and revision.
- The logical path from the source item.
- The atom scope set in its canonical order.

The helper will retain only source-item kind `openspec/requirement/v1`. It will report an error for an incomplete or inconsistent accepted aggregate.

The CLI will stop report construction only for an aggregate error after acceptance. It will not omit one inconsistent row or fabricate a value.

The binding boundary will accept only a diagnostic-free `CheckResult`. Therefore, a subject from an unaccepted report cannot create a binding.

Active and archived change documents remain absent because repository selectors do not place them in the accepted authority aggregate.

Alternative: scan all source nodes by item kind. Rejected because only an accepted source authority establishes the requirement-to-atom correspondence.

### 3. Use one total ordering

The helper will sort rows by this tuple:

1. Stable requirement ID.
2. Ordered scope vector.
3. Logical path.
4. Requirement source-node digest.
5. Atom revision.

Equal rows will remain equal. Unequal rows will remain visible.

The helper will not use parser traversal order, physical roots, hash-map order, locale, or operating-system directory order.

### 4. Keep the functional core unchanged

The new fact, aggregation helper, machine serialization, and human rendering remain in the effectful root crate.

`buoy-core` keeps its current `#![no_std]` API and one-way dependency rule. The shell reads validated core values but the core never depends on shell report types.

No canonical encoder, digest preimage, lifecycle transition, freshness rule, verdict, or assurance class changes.

This work opens no Verus or AeneasVerif proof lane. Existing proofs do not establish shell serialization, parser behavior, rustc behavior, or report correctness.

### 5. Update every report constructor and compatibility control

Both version 2 root mode and version 3 mounted mode will populate the inventory. Execution reports will keep the field empty because they are not binding bootstrap output.

Focused tests will cover:

- One current requirement before a binding exists.
- Multiple requirements supplied in reverse input order.
- Equal logical inputs under separate physical roots.
- A repository with no OpenSpec authority.
- An injected inconsistent authority aggregate and one valid non-OpenSpec authority.
- Exact machine and human output.
- Existing opaque subject and attachment fields.

Static CLI goldens and examples will include the additive field where applicable. Reports without current requirements will remain byte-identical.

### 6. Preserve assurance boundaries

A requirement-subject row identifies current saved inputs only. It supplies a selector for a later reviewed binding command.

The row does not approve the requirement or establish implementation correspondence. The later binding remains navigation and impact data only.

The exact acceptance command is:

```text
devenv test
```

Success requires all unit, integration, property, fixture, package, documentation, assurance, and OpenSpec gates to pass.

## Risks / Trade-offs

- **Static report bytes change for OpenSpec repositories.** Update every affected golden and source-bound evidence artifact.
- **A strict downstream JSON parser rejects the additive field.** Document the field and retain omission for repositories without current requirements.
- **Authority aggregation hides an inconsistency.** Return an error for every incomplete or mismatched row.
- **Consumers treat inventory as approval.** State the navigation-only boundary in machine documentation and user documentation.
- **Physical roots enter output through source locations.** Read only canonical logical paths from retained source items.
