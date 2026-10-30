## ADDED Requirements

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
