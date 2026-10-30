## ADDED Requirements

### Requirement: CHELIS-PROV-012 Parser-canonical Surf identity
Surf extraction MUST parse saved sources with the pinned vendored `tree-sitter-chelis-surf` grammar. Every record MUST anchor to a parsed top-level declaration node, and every parsed-item identity MUST derive from that node's canonical bytes and exact byte span. A parse `ERROR` node in a selected source MUST fail extraction loudly. The vendored grammar artifacts MUST carry a current registered derivation from the pinned Chelis revision with a fail-closed drift check.

#### Scenario: Positive - identity survives reformatting around the declaration
- **GIVEN** one Surf declaration whose surrounding lines move without changing the declaration node's canonical bytes
- **WHEN** extraction runs before and after the move
- **THEN** the parsed-item identity digest is unchanged while the span tracks the new location

#### Scenario: Negative - parse error cannot pass as nonnormative text
- **GIVEN** a selected Surf source that the pinned grammar cannot parse
- **WHEN** extraction runs
- **THEN** the check fails with a stable parse diagnostic and MUST NOT emit an empty successful graph

#### Scenario: Negative - vendored grammar drift is rejected
- **GIVEN** a vendored grammar file whose bytes differ from the registered derivation digests
- **WHEN** the drift check runs
- **THEN** the gate fails with a deterministic derivation diagnostic

### Requirement: CHELIS-PROV-013 Revision-bearing canonical reports
The canonical static report MUST expose every atom's revision digest and every graph object's identity digest, deterministically ordered, so a host repository can bind carriers, links, and surface rows without building out-of-band tooling. Report bytes MUST stay byte-identical for identical canonical inputs.

#### Scenario: Positive - host binds from the report alone
- **GIVEN** a static report for a repository with authority records
- **WHEN** a host reads the report
- **THEN** it obtains each atom's exact revision digest and each object's identity digest without any helper build

#### Scenario: Negative - report bytes stay canonical
- **GIVEN** identical saved inputs under two distinct absolute roots
- **WHEN** the revision-bearing report renders in both
- **THEN** the report bytes are identical

### Requirement: CHELIS-PROV-014 Complete command exposure
The consumer package MUST expose the `chelis-provenance` binary alongside `buoy`, and the `chelis-provenance` CLI MUST expose the advisory report as a deterministic command. Both surfaces MUST preserve the core exit classes.

#### Scenario: Positive - pinned package exposes the command
- **GIVEN** the realized consumer package
- **WHEN** a consumer lists its binaries
- **THEN** `chelis-provenance` is present and answers with the core usage exit class

#### Scenario: Positive - advisory report through the CLI
- **GIVEN** a repository with registered records and no receipts
- **WHEN** the advisory command runs twice
- **THEN** both runs emit byte-identical advisory bytes that keep uncovered atoms and `not-run` carriers visible without changing the exit class
