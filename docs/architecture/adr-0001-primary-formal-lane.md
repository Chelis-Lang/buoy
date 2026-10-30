# ADR-0001: Primary formal lane

- Status: accepted
- Date: 2026-07-16
- Decision owners: Buoy maintainers
- Spike evidence command: `devenv tasks run buoy:properties`
- Production proof/accounting commands: `devenv tasks run buoy:p1` and `devenv tasks run buoy:obligations`

## Context

FPR-008 requires equivalent nonproduction spikes before a production lane is selected. The frozen spike matrix covers lifecycle terminality, decision-surface completeness, carrier and derivation freshness, policy monotonicity, process-record no-laundering, change-impact closure, canonical field-header binding, the version 2 envelope, structural XXH128 half/byte order, digest parser shape, and migration lifecycle preservation. Both candidates replay positive proofs and a deliberately false waiver-laundering control.

## Decision

**Verus is the primary production formal lane.** The authoritative representation for its initial proof class is the exact Verus/Rust source plus specifications and proof code accepted by pinned Verus and its bundled Z3. Production proof records must also bind the corresponding executable Rust implementation source and every assumption; source-level proof does not establish rustc, LLVM, or machine-code correctness.

**Lean 4 is the selected Aeneas backend.** Aeneas/Charon/Lean remains a required comparative translation spike and may become a secondary production lane after its translation and external-model costs are justified. It is not required for every production kernel function in v2.

## Evidence

The accepted spike result is:

```text
formal-spikes: verus=13/13 aeneas-lean=11/11 negative-controls=2/2
```

Verus proves thirteen functions/specification obligations directly. Charon extracts the safe sequential Rust functions, Aeneas emits a committed Lean derivation, and Lean checks eleven equivalent theorem groups. Regeneration must reproduce the normalized generated Lean bytes. Normalization removes only three backend-environment declarations that are unavailable in the deliberately minimal bound runtime model: two nonsemantic linter options and the enum `discriminant` attributes. Definitions are not rewritten.

The Aeneas spike binds `proofs/spikes/aeneas/lean/Aeneas.lean` as an explicit external model for only `Result`, `U8`, and `U64`. This avoids resolving the full Mathlib-dependent backend during the offline gate, but it narrows the claim to generated constructs represented by that model. It is not evidence that the complete Aeneas Lean library or arbitrary generated output verifies.

For the production Verus lane, `proofs/core/derivation-record.toml` names each production/proof symbol mapping and `proofs/core/correspondence-approval.toml` records the reviewed assumptions and exclusions. Lifecycle, surface completeness, freshness, policy monotonicity, impact closure, and no-laundering mappings are approved as review-only correspondence assumptions. The detached canonical field-identity theorem remains blocked from a production `formally-proved` claim because no shared/generated encoder representation or approved production symbol mapping exists. Source digests detect drift only.

Observed warm-cache wall times on the selected `aarch64-darwin` host were approximately:

- Verus positive plus negative control: under 6 seconds;
- Charon and Aeneas regeneration: under 10 seconds after tool realization;
- minimal-model Lean compilation and controls: under 25 seconds;
- initial Nix realization: materially larger (about 31 minutes, dominated by building Lean), but outside steady-state CI after cache population.

## Comparison

| Concern | Verus | Aeneas/Lean |
|---|---|---|
| Faithfulness surface | Verus source/spec encoding | Charon LLBC extraction, Aeneas translation, generated Lean, external models |
| Trusted tools | Verus and bundled Z3 | Charon, Aeneas, bound models, Lean kernel |
| Supported initial subset | safe sequential Verus/Rust; explicit specs | safe sequential Rust; no unsupported nested-loop exits; modeled externals only |
| Maintenance | one source/proof language and direct diagnostics | committed derivation, normalization, models, and handwritten Lean proofs |
| Steady-state CI | lower | higher, but suitable as a small translation sentinel |
| Main exclusion | downstream compilation | translation faithfulness, full backend library, downstream compilation |

## Consequences

- The selected integrity-kernel functions are verified in Verus and bind exact v2 source, proof, solver, tool, assumption, and exclusion identities.
- Aeneas/Lean remains in the phase-4 and final formal gate as architecture evidence and a deterministic translation sentinel.
- A production Aeneas proof must bind the full generated artifact and every external model it uses; the spike runtime model cannot be generalized silently.
- Neither lane can justify prose correspondence without the independent repository-owned approval required by FPR-001.
- Neither lane establishes XXH3 internals, collision properties, parser implementations, filesystem, process, provider, compiler-backend, or native-code correctness. Matching digest values or generated bytes is only a bound consistency observation.
- The version 2 comparison, exact tool/source bindings, assumptions, unsupported constructs, and exclusions are recorded in `openspec/changes/archive/2026-07-17-standardize-all-hashing-on-xxhash/evidence/formal-spike-comparison.md`.

## Rejected alternatives

- Selecting Aeneas as primary solely because its functional model is attractive: rejected because the spike exposes a larger trusted translation/model surface and higher maintenance cost.
- Requiring dual production proofs for every function: rejected because equivalent spike evidence does not justify duplicating all proof maintenance.
- Selecting by preference without negative controls or generated-output replay: rejected by FPR-008.
