# Readable trace projections: selected verification lanes

Status: reviewed scope record for the active `add-readable-buoy-trace-projections` change.

## Selected lane decision

This change selects no Verus proof lane. It also selects no AeneasVerif proof lane.

No formal property, proof record, derivation record, or correspondence approval changes for this change.

## Reason

The change adds one read-only query, three renderers, one opt-in adapter format, and one workflow template. Each addition sits in the imperative shell or in shell-adjacent presentation code.

The primary Verus lane targets a small safe, sequential, pure `no_std` kernel. Renderers, adapters, and command parsing hold ambient effects, so they stay outside that kernel scope.

The trace query itself is pure, but it supplies navigation and impact data only. An order proof does not add an assurance class.

## Evidence this change does supply

- Independent bounded models over record classification and digest abbreviation.
- Sampled Hegel properties for row order, root independence, and selector permutation.
- Positive and negative fixtures for each selector, relation state, adapter format, and workflow artifact.
- Deterministic output contracts for the machine, human, and Markdown views.

Each item establishes only its declared scope. Together they do not establish proof.

## Exclusions

Existing Verus and AeneasVerif results do not cover the new shell, adapter, query, or renderer behavior.

This record does not promote any existing theorem. It does not change any obligation disposition.

A later change can select a formal lane for this behavior. That change must bind its own exact representation, assumptions, translators, verifier, configuration, and proof-source identities.
