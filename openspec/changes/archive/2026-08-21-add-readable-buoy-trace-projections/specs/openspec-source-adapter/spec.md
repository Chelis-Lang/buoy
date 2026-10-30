## MODIFIED Requirements

### Requirement: OSA-005 Explicit authority metadata configuration

The adapter configuration MUST supply each format version, atom kind, scope, lifecycle state, registry binding, and required approval. The adapter MUST NOT infer these fields.

The accepted format values MUST be `openspec/standing-spec/v1` and `openspec/standing-spec/v2`. Version 1 behavior MUST remain unchanged. Version 2 MUST add only declared nonauthority trace side products.

Both formats MUST use `openspec/standing-spec/v1` as the source-item schema for document, requirement, scenario, and step nodes. Equal authority bytes under either format MUST retain equal source-node and atom identities.

#### Scenario: Positive - capability configuration is complete

- **GIVEN** a selected capability with supported format, scope, kind, lifecycle, approval data, and registry binding
- **WHEN** the adapter constructs its atoms
- **THEN** each atom receives only the configured metadata for its stable ID

#### Scenario: Positive - version 1 configuration remains stable

- **GIVEN** a selected capability with format `openspec/standing-spec/v1`
- **WHEN** the adapter constructs its atoms and source tree
- **THEN** all accepted bytes, identities, dispositions, and diagnostics remain compatible with the published version 1 contract

#### Scenario: Positive - version 2 configuration is explicit

- **GIVEN** a selected capability with complete metadata and format `openspec/standing-spec/v2`
- **WHEN** configuration parsing runs
- **THEN** the adapter enables nonauthority trace parsing and retains version 1 authority-node schema identities

#### Scenario: Positive - format adoption preserves a current binding

- **GIVEN** equal authority bytes and one current implementation binding under version 1
- **WHEN** the capability format changes to version 2
- **THEN** all authority-node and atom identities remain equal and the implementation binding remains fresh

#### Scenario: Negative - format version is unsupported

- **GIVEN** an adapter configuration that declares another OpenSpec standing-spec format
- **WHEN** configuration parsing runs
- **THEN** parsing fails with `SPG-OPENSPEC-CONFIG-FORMAT`

### Requirement: OSA-007 Total OpenSpec disposition

For each accepted file, primary requirement spans and nonnormative spans MUST form a complete non-overlapping partition of the selected bytes.

Nested scenario source spans MUST remain inside their owning requirement spans. A version 2 trace source mapping MUST nest only inside one nonnormative span.

A secondary trace mapping MUST NOT replace, split, or overlap another primary partition entry. Supported purpose text and nonauthority sections MUST remain nonnormative.

Malformed authority-like syntax and unsupported syntax MUST fail with deterministic diagnostics. The adapter MUST NOT absorb a nonauthority section into the preceding requirement.

#### Scenario: Positive - accepted file has a total byte partition

- **GIVEN** a selected file with purpose text, pre-requirement prose, requirements, scenarios, and a later notes section
- **WHEN** the adapter processes the file
- **THEN** primary requirement and nonnormative spans partition all bytes while scenarios remain inside their requirements

#### Scenario: Positive - trace mapping nests in nonnormative text

- **GIVEN** a version 2 trace fence inside one later nonnormative section
- **WHEN** the adapter retains the fence source mapping
- **THEN** the primary partition remains unchanged and the trace mapping nests inside one nonnormative span

#### Scenario: Negative - trace mapping crosses a primary boundary

- **GIVEN** one trace source mapping that crosses a requirement and nonnormative boundary
- **WHEN** partition validation runs
- **THEN** validation fails with a deterministic OpenSpec partition diagnostic

#### Scenario: Negative - unsupported authority-like section appears

- **GIVEN** a selected file with unsupported authority-like syntax inside the requirements section
- **WHEN** the adapter processes the file
- **THEN** graph construction fails with a deterministic unsupported-syntax diagnostic and no partial graph is accepted

## ADDED Requirements

### Requirement: OSA-013 Same-file nonauthority trace sections

The version 2 adapter MUST accept `buoy-external-trace-v1` fences only under level-two nonauthority sections after the requirements section. It MUST use the existing strict external trace parser.

Each accepted fence MUST retain its exact source span and MUST create navigation data only.

#### Scenario: Positive - standing spec documents an implementation trace page

- **GIVEN** one version 2 standing spec with a `documents` trace fence after all requirements
- **WHEN** the OpenSpec adapter materializes the file
- **THEN** the graph contains the external trace link and its exact nonauthority source mapping

#### Scenario: Negative - trace fence appears inside a requirement

- **GIVEN** one `buoy-external-trace-v1` fence inside a requirement span
- **WHEN** the version 2 adapter processes the file
- **THEN** materialization fails with a deterministic trace-in-authority-span diagnostic and no partial graph is accepted

### Requirement: OSA-014 Cycle-free trace-section identity

Trace-section bytes MUST remain outside every requirement source-node identity and atom revision. A trace-link atom reference MUST bind the current revision without changing that revision.

A trace-section edit MUST change the link identity and nonnormative disposition only. It MUST NOT change an otherwise equal authority node or atom identity.

#### Scenario: Positive - trace locator changes alone

- **GIVEN** two version 2 specs with equal requirements and different trace targets
- **WHEN** the adapter materializes both files
- **THEN** authority-node and atom identities remain equal while the external trace-link identity changes

#### Scenario: Positive - format versions retain authority identity

- **GIVEN** equal requirement bytes under version 1 and version 2 configurations
- **WHEN** the adapter materializes both files
- **THEN** document, requirement, scenario, step, and atom identities remain equal across both formats

#### Scenario: Negative - trace bytes enter requirement identity

- **GIVEN** an adapter implementation that includes one trace-section byte in the preceding requirement
- **WHEN** identity parity validation runs
- **THEN** validation fails with a deterministic requirement-boundary diagnostic
