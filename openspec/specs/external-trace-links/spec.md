# External Trace Links

## Purpose

Define deterministic navigation links from exact atom revisions to repository-owned external targets.

## Requirements

### Requirement: ETL-001 Exact external trace-link identity
Each external trace link MUST have a stable ID, atom-revision set, relation, and normalized target. The link revision MUST bind every field.

#### Scenario: Positive - issue link is complete
- **GIVEN** a link with a stable ID, current atom revision, `tracks` relation, and normalized issue URI
- **WHEN** the graph constructs the external trace link
- **THEN** the link has one deterministic revision and one navigation target

#### Scenario: Negative - target or atom set is empty
- **GIVEN** an external trace link with no atom revision or no target
- **WHEN** the graph constructs the link
- **THEN** construction fails with a deterministic incomplete-link diagnostic

### Requirement: ETL-002 Closed relations and target forms
The initial relation set MUST contain `tracks`, `discusses`, `documents`, and `reviews`. Targets MUST use a versioned `urn` or an absolute `https` URI. An `https` URI MUST use a lowercase DNS host without a port, user information, or fragment.

#### Scenario: Positive - document URI uses an accepted form
- **GIVEN** a `documents` link with an absolute `https` URI and no user information
- **WHEN** target validation runs
- **THEN** the target is accepted and stored in normalized form

#### Scenario: Negative - unknown relation or unsafe URI appears
- **GIVEN** a link with an unknown relation, relative URI, embedded user information, or URI fragment
- **WHEN** target validation runs
- **THEN** construction fails with a deterministic relation or target diagnostic

### Requirement: ETL-003 Repository-owned link syntax
A `buoy-external-trace-v1` Markdown fence MUST contain one strict link document. Unknown, duplicate, or malformed fields MUST fail closed.

The Markdown adapter and the opt-in OpenSpec version 2 nonauthority parser MUST produce equal link values from equal fence bytes.

#### Scenario: Positive - one Markdown fence creates one link
- **GIVEN** a selected Markdown file with one valid `buoy-external-trace-v1` fence
- **WHEN** the Markdown adapter processes the file
- **THEN** the graph contains one link with an exact fence source mapping

#### Scenario: Positive - OpenSpec version 2 reuses the parser
- **GIVEN** equal valid fence bytes in a Markdown file and an OpenSpec version 2 nonauthority section
- **WHEN** both adapters process their files
- **THEN** both outputs contain equal external trace-link values with file-specific source mappings

#### Scenario: Negative - prose URL resembles a link
- **GIVEN** ordinary prose with an issue URL and no authoritative trace-link fence
- **WHEN** either adapter processes the file
- **THEN** no external trace link is created from the prose URL

#### Scenario: Negative - parser behavior differs by host adapter
- **GIVEN** one malformed trace fence supplied through both accepted host adapters
- **WHEN** deterministic validation runs
- **THEN** both paths fail with the same primary trace-document diagnostic

### Requirement: ETL-004 Navigation does not create assurance
An external trace link MUST provide navigation and impact only. It MUST NOT satisfy authority, coverage, freshness, execution, structural, review, or proof obligations.

#### Scenario: Positive - issue link appears in navigation
- **GIVEN** a valid issue link for a current atom revision
- **WHEN** the static report is created
- **THEN** the issue target appears in navigation without a new assurance class

#### Scenario: Negative - closed issue satisfies missing evidence
- **GIVEN** a ratified atom with one issue link and no required evidence carrier
- **WHEN** coverage and execution facts are aggregated
- **THEN** the required role remains unsatisfied regardless of the live issue state

### Requirement: ETL-005 No provider-state input
Buoy MUST NOT fetch or interpret an external target during static, change, or execution processing. Only the repository-owned target text is an input.

#### Scenario: Positive - offline report contains the saved target
- **GIVEN** a valid saved trace link and no network access
- **WHEN** the static report is created
- **THEN** the report contains the normalized target and completes without provider access

#### Scenario: Negative - provider title changes
- **GIVEN** equal repository inputs and different live issue titles, labels, authors, or states
- **WHEN** static and change processing run
- **THEN** all canonical graph and report bytes remain equal

### Requirement: ETL-006 Link freshness and impact

An external trace link MUST bind exact semantic atom revisions. Validation MUST compare each reference with `Atom::revision()` for the same stable ID.

Validation MUST NOT reinterpret an atom revision as `Atom::meta().digest()`. A changed or retired atom MUST make the old link stale and create explicit change impact.

#### Scenario: Positive - current link remains fresh

- **GIVEN** a trace link whose references match all current semantic atom revisions
- **WHEN** link freshness is validated
- **THEN** the link is fresh and available for navigation even when each atom object digest differs

#### Scenario: Negative - atom object digest occupies the revision field

- **GIVEN** a trace link whose atom reference contains the current atom object digest instead of its semantic revision
- **WHEN** link freshness is validated
- **THEN** validation fails with `SPG-REF-DANGLING-OR-STALE` and accepts no fresh link

#### Scenario: Negative - requirement scenario changes

- **GIVEN** a trace link to an OpenSpec atom revision
- **WHEN** a scenario edit changes that semantic atom revision
- **THEN** the link becomes stale and the change report requires one explicit disposition

### Requirement: ETL-007 Deterministic trace-link reports
Reports MUST order external trace links by canonical link identity. Human and machine output MUST exclude fetched titles, statuses, timestamps, and provider users.

#### Scenario: Positive - insertion order changes
- **GIVEN** equal links supplied in different input orders
- **WHEN** the static report is created
- **THEN** both machine reports have byte-identical trace-link rows

#### Scenario: Negative - cached provider fields enter output
- **GIVEN** a trace-link renderer with cached provider title or status data
- **WHEN** canonical report validation runs
- **THEN** validation fails with a deterministic forbidden-provider-field diagnostic

### Requirement: ETL-008 Generated current external trace links

Buoy MUST generate a declared external trace link from the current atom revision, stable link ID, relation, target base, and rendered page path.

The generator MUST use the same strict trace-document parser as repository materialization. It MUST NOT copy an atom object digest into the revision field.

For OpenSpec version 2, generation MUST change only one declared standalone nonauthority publication slot after all normative requirements.

The slot MUST contain one canonical heading and one trace fence keyed by the stable link ID.

The operation MUST prove that every normative source span, requirement node, and semantic atom revision is equal before and after the link edit.

#### Scenario: Positive - current atom revision replaces a stale reference

- **GIVEN** one declared OpenSpec link whose saved atom reference is stale
- **WHEN** explicit external-link generation runs
- **THEN** the new fence contains the current `Atom::revision()` value and passes normal closure validation

#### Scenario: Negative - generator uses the atom object digest

- **GIVEN** one current atom whose object digest differs from its semantic revision
- **WHEN** a candidate link uses the object digest
- **THEN** candidate validation fails with `SPG-REF-DANGLING-OR-STALE` and writes no source file

### Requirement: ETL-009 Cycle-free immutable publication targets

A publication-managed `documents` target MUST equal the normalized target base joined with the declared page's rendered logical path.

That page path MUST contain the complete publication-basis path encoding. The page MUST use the `immutable` publication lifecycle.

Atom-revision and binding-record path encodings can provide additional stable grouping.

A constant mutable branch page or atom-only path MUST NOT satisfy immutable publication. A separate current-page alias can remain outside the exact external link.

Path rendering MUST reject substitutions from graph, page, generated-link, commit, provider, clock, or physical-path values.

The target base MUST reject all substitutions. The join MUST reject traversal, query, fragment, or loss of the complete rendered page path.

#### Scenario: Positive - basis-keyed page preserves an old target

- **GIVEN** one page path keyed by the complete pre-link publication basis
- **WHEN** one page-affecting input changes and a new page path and link are generated
- **THEN** the new link receives a new target while Buoy leaves the old target text and local page unchanged

#### Scenario: Negative - exact atom uses one mutable branch page

- **GIVEN** one publication-managed link to an exact atom and one target with no publication-basis identity
- **WHEN** page-path and target validation run
- **THEN** validation fails with a deterministic mutable-publication-target diagnostic

#### Scenario: Negative - target depends on generated page commit

- **GIVEN** one target whose completion requires the commit that will contain its generated page
- **WHEN** target dependency validation runs
- **THEN** validation fails with a deterministic publication-cycle diagnostic before graph or page generation
