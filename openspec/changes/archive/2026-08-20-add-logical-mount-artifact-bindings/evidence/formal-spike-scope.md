# Logical mount formal spike scope

Status: feasibility evidence for lane selection.

## Bound representation

The logical projection model concatenates canonical sequences of path-segment identities. Disjoint prefixes contain one different segment before either prefix ends.

The attachment-key model binds field tag `1` and the exact relation value before hashing.

The positive obligations establish these facts:

- Distinct prefixes produce distinct projected pairs.
- One prefix preserves distinct relative paths.
- Distinct relations produce distinct attachment-key preimages.

The intentionally false control claims that one projected pair differs from itself. Verus must reject this claim.

## Exclusions

The result excludes filesystems, path parsers, TOML, CLI parsing, operating systems, descriptor behavior, hash internals, and collision resistance.

The result also excludes natural-language correspondence, authenticity, authorization, compilation, native code, and implementation correctness.

## Lane decision rule

Select Verus only if its positive obligations pass and its false control fails. Run the AeneasVerif translation spike before this selection.

## Recorded result

The pinned Verus tool accepted all three segment-sequence obligations. It rejected the equal-input false control with the expected postcondition diagnostic.

Charon and AeneasVerif translated the equivalent safe Rust functions. Lean accepted both generated-field proofs without an admission.

This change selects Verus for the reviewed logical projection proof. AeneasVerif remains comparative translation evidence only.

Verus represents segment-sequence concatenation directly. The bounded AeneasVerif structure confirms translation feasibility but adds no stronger claim.
