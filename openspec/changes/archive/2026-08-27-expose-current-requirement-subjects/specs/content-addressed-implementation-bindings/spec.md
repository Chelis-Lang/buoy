## ADDED Requirements

### Requirement: CAB-014 Current requirement subject inventory

A static report MUST expose one `requirement_subjects` row for each current OpenSpec requirement authority in a diagnostic-free accepted source graph.

A static report with diagnostics MUST omit `requirement_subjects`. It MUST preserve the complete report, diagnostics, and static exit class.

`buoy bind` MUST reject a repository when its current static check contains diagnostics.

Each row MUST contain the stable requirement ID, exact requirement source-node digest, exact atom revision, logical source path, and ordered authority scopes.

Rows MUST use complete canonical digests. They MUST use logical paths and MUST exclude physical roots, provider data, clocks, locale, and traversal order.

Rows MUST sort by stable ID, authority scopes, logical path, source-node digest, and atom revision. Unequal current subjects MUST NOT collapse.

A report with no current OpenSpec requirement authority MUST omit `requirement_subjects`. The inventory MUST NOT change graph identity or add an assurance class.

#### Scenario: Positive - consumer selects an initial binding subject

- **GIVEN** one current standing requirement with stable ID `SDT-001` and no stored implementation binding
- **WHEN** `buoy static --format machine` renders the accepted repository
- **THEN** one requirement-subject row exposes the exact digest that `buoy bind --requirement` accepts

#### Scenario: Positive - physical roots differ

- **GIVEN** equal selected bytes and equal logical mount declarations under two different physical roots
- **WHEN** both static machine reports are rendered
- **THEN** both complete `requirement_subjects` inventories are byte-identical

#### Scenario: Positive - no OpenSpec authority exists

- **GIVEN** an accepted repository with no current OpenSpec requirement authority
- **WHEN** the static machine report is rendered
- **THEN** the report omits `requirement_subjects` without adding a placeholder row

#### Scenario: Negative - a non-requirement authority enters aggregation

- **GIVEN** a retained source authority identifies a node outside `openspec/requirement/v1`
- **WHEN** requirement-subject aggregation runs
- **THEN** the inventory omits that authority without omitting any current OpenSpec requirement row

#### Scenario: Negative - active change data enters the inventory

- **GIVEN** an active or archived OpenSpec change document that is outside current standing authority selectors
- **WHEN** the static machine report is rendered
- **THEN** no requirement-subject row derives from that planning document

#### Scenario: Negative - an inventory row changes graph identity

- **GIVEN** one accepted source graph and its derived requirement-subject inventory
- **WHEN** report rendering adds the derived rows
- **THEN** the report keeps the original graph digest and adds no graph object, edge, or assurance fact
