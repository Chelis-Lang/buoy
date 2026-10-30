# OpenSpec Source Adapter

## Purpose

Define deterministic extraction of requirement atoms from saved OpenSpec standing specs.

## Requirements

### Requirement: OSA-001 Standing-spec authority boundary
The adapter MUST accept authority only from selected `openspec/specs/**/*.md` files. It MUST reject change artifacts as atom authority.

#### Scenario: Positive - standing requirement becomes an atom
- **GIVEN** a selected `openspec/specs/auth/spec.md` file with one valid requirement
- **WHEN** the OpenSpec adapter materializes the saved file
- **THEN** the graph contains one atom with an exact requirement source mapping

#### Scenario: Negative - active delta is selected
- **GIVEN** a selected path under `openspec/changes/add-auth/specs/`
- **WHEN** the OpenSpec adapter processes the path
- **THEN** materialization fails with a deterministic forbidden-OpenSpec-path diagnostic

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

### Requirement: OSA-004 Complete requirement revision
The atom revision MUST bind the title, normative prose, scenario names, and scenario bodies. Schema-declared Markdown layout trivia MUST remain nonsemantic.

#### Scenario: Positive - blank-line layout changes alone
- **GIVEN** two requirement sections with equal parsed content and different allowed blank-line layout
- **WHEN** the adapter creates both atom revisions
- **THEN** both revisions have the same digest while their raw source bindings differ

#### Scenario: Negative - scenario result changes
- **GIVEN** a current requirement with one positive scenario
- **WHEN** the scenario result changes by one semantic byte
- **THEN** the atom revision changes and every old code, evidence, and trace link becomes stale

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

### Requirement: OSA-006 Explicit lifecycle and approval
The default atom state MUST be `proposed`. A `ratified` atom MUST have an exact repository-owned approval binding in adapter configuration.

#### Scenario: Positive - configured ratification is accepted
- **GIVEN** a requirement with `ratified` state and a valid approval record path, digest, and authority ID
- **WHEN** the adapter validates atom authority
- **THEN** the atom is ratified with that exact approval binding

#### Scenario: Negative - archive state implies ratification
- **GIVEN** an archived OpenSpec change and no repository-owned approval binding for its standing requirement
- **WHEN** the adapter materializes the standing requirement
- **THEN** the atom remains proposed and MUST NOT gain review-only assurance

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

### Requirement: OSA-008 Deterministic offline materialization
Byte-identical saved specs and configuration MUST produce byte-identical adapter output. Root, locale, traversal order, provider state, and network state MUST not affect output.

#### Scenario: Positive - separate roots have equal output
- **GIVEN** equal saved specs and configuration under two different absolute roots
- **WHEN** both adapter runs use different traversal orders and locales
- **THEN** both runs produce byte-identical graph fragments, source maps, dispositions, and diagnostics

#### Scenario: Negative - issue state changes during materialization
- **GIVEN** unchanged saved inputs and different live issue or pull-request state
- **WHEN** the OpenSpec adapter runs offline
- **THEN** the adapter output remains byte-identical and contains no fetched provider facts

### Requirement: OSA-009 Requirement-level traceability scope
The first adapter version MUST link code and evidence to requirement atom revisions. It MUST NOT report separate scenario coverage.

#### Scenario: Positive - test links to the requirement revision
- **GIVEN** a test carrier that binds the current revision of an OpenSpec requirement atom
- **WHEN** registration and freshness are validated
- **THEN** the report shows a fresh requirement-level test link within its exact declared scope

#### Scenario: Negative - one test name implies all scenario coverage
- **GIVEN** a test whose name matches one OpenSpec scenario but whose carrier binds only the requirement
- **WHEN** assurance is aggregated
- **THEN** the report MUST NOT claim separate or complete scenario-level coverage

### Requirement: OSA-010 Content-addressed OpenSpec source tree

The adapter MUST materialize each selected standing spec as a content-addressed source tree. The tree MUST contain document, requirement, scenario, and step nodes.

The document MUST contain requirements in source order. Each requirement MUST contain its scenarios in source order. Each scenario MUST contain its parsed steps in source order.

#### Scenario: Positive - complete standing-spec tree
- **GIVEN** one standing spec with two requirements and three scenarios
- **WHEN** the OpenSpec adapter materializes the file
- **THEN** the source tree preserves each parent, child, and source position

#### Scenario: Negative - scenario escapes its requirement
- **GIVEN** a malformed source tree whose scenario parent is a different requirement
- **WHEN** OpenSpec tree validation runs
- **THEN** validation fails with a deterministic OpenSpec-parent diagnostic

### Requirement: OSA-011 Requirement node and atom correspondence

Each authoritative requirement node MUST map to exactly one atom. Its stable ID and canonical semantic subtree MUST agree with the atom ID and revision.

A scenario or step node MUST NOT map directly to an atom in the first binding version.

#### Scenario: Positive - requirement maps to its atom
- **GIVEN** one valid OpenSpec requirement source node
- **WHEN** the adapter constructs authority
- **THEN** the node maps to one atom with the same stable ID and semantic revision

#### Scenario: Negative - scenario is used as atom authority
- **GIVEN** an OpenSpec scenario node and no requirement node
- **WHEN** authority mapping runs
- **THEN** validation fails and no atom is created from the scenario

### Requirement: OSA-012 Child changes propagate to requirement identity

A semantic change to a scenario or step MUST change each containing scenario and requirement node digest. Unchanged sibling requirement digests MUST remain stable.

#### Scenario: Positive - one scenario result changes
- **GIVEN** two requirements with separate scenarios
- **WHEN** one result step changes in the first requirement
- **THEN** the first requirement digest changes and the second requirement digest remains equal

#### Scenario: Negative - child changes but parent digest remains equal
- **GIVEN** a requirement with one changed semantic descendant
- **WHEN** the adapter calculates the source tree
- **THEN** validation fails if the requirement digest equals its prior digest

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
