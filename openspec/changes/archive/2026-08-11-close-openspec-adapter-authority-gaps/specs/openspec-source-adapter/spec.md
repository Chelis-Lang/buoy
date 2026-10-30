## MODIFIED Requirements

### Requirement: OSA-002 Stable requirement IDs and allocation
Each accepted heading MUST use `### Requirement: <stable-id> <title>`. The append-only Buoy registry MUST reserve each stable ID exactly once.

Maintainer documentation and checked examples MUST show this exact field order. Alternate title-first or bracketed-ID forms MUST NOT appear as accepted syntax.

#### Scenario: Positive - reserved ID appears once
- **GIVEN** one requirement heading with a valid ID that the registry reserves
- **WHEN** all selected standing specs are materialized
- **THEN** the adapter creates one atom with that stable ID

#### Scenario: Negative - documented heading uses title first
- **GIVEN** a maintained example uses `### Requirement: <title> [<stable-id>]`
- **WHEN** documentation parity validation runs
- **THEN** validation fails and identifies the rejected heading form

### Requirement: OSA-003 Parser-backed requirement structure and spans
The adapter MUST parse requirement and scenario headings through the pinned CommonMark syntax tree. Each atom span MUST cover only its complete requirement section.

A requirement section MUST end before the next heading at level three or higher. A later peer or higher-level section MUST NOT enter the requirement statement or revision.

#### Scenario: Positive - multibyte text precedes a requirement
- **GIVEN** a valid spec with multibyte Unicode before one requirement heading
- **WHEN** the adapter reports the requirement span
- **THEN** the byte offsets identify the exact heading, prose, scenarios, and final requirement boundary

#### Scenario: Negative - later notes enter the requirement revision
- **GIVEN** a valid requirement followed by a peer or higher-level notes section
- **WHEN** the adapter constructs the requirement statement and revision
- **THEN** the notes section remains outside the atom span and cannot change the atom revision

### Requirement: OSA-005 Explicit authority metadata configuration
The adapter configuration MUST supply each format version, atom kind, scope, lifecycle state, registry binding, and required approval. The adapter MUST NOT infer these fields.

The accepted format value MUST be `openspec/standing-spec/v1`. A missing or different value MUST fail at the configuration parse boundary.

#### Scenario: Positive - capability configuration is complete
- **GIVEN** a selected capability with the supported format, scope, kind, lifecycle, approval data, and registry binding
- **WHEN** the adapter constructs its atoms
- **THEN** each atom receives only the configured metadata for its stable ID

#### Scenario: Negative - format version is unsupported
- **GIVEN** an adapter configuration that declares an unsupported OpenSpec standing-spec format
- **WHEN** configuration parsing runs
- **THEN** parsing fails with `SPG-OPENSPEC-CONFIG-FORMAT`

### Requirement: OSA-007 Total OpenSpec disposition
For each accepted file, primary requirement spans and nonnormative spans MUST form a complete non-overlapping partition of the selected bytes.

A scenario source span MAY nest only inside its owning primary requirement span. Supported top-level purpose text and nonauthority sections MUST be nonnormative.

Malformed authority-like syntax and unsupported syntax MUST fail with deterministic diagnostics. The adapter MUST NOT absorb a nonauthority section into the preceding requirement.

#### Scenario: Positive - accepted file has a total byte partition
- **GIVEN** a selected file with purpose text, pre-requirement prose, one valid requirement, scenarios, and a later notes section
- **WHEN** the adapter processes the file
- **THEN** primary requirement and nonnormative spans partition all bytes while each scenario span remains inside its requirement span

#### Scenario: Negative - unsupported authority-like section appears
- **GIVEN** a selected file with an unsupported authority-like heading inside the requirements section
- **WHEN** the adapter processes the file
- **THEN** graph construction fails with a deterministic unsupported-syntax diagnostic and no partial graph is accepted
