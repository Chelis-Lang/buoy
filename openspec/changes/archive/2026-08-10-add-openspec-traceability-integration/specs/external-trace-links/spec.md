## ADDED Requirements

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

#### Scenario: Positive - one fence creates one link
- **GIVEN** a selected Markdown file with one valid `buoy-external-trace-v1` fence
- **WHEN** the Markdown adapter processes the file
- **THEN** the graph contains one link with an exact fence source mapping

#### Scenario: Negative - prose URL resembles a link
- **GIVEN** ordinary prose with an issue URL and no authoritative trace-link fence
- **WHEN** the Markdown adapter processes the file
- **THEN** no external trace link is created from the prose URL

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
An external trace link MUST bind exact atom revisions. A changed or retired atom MUST make the old link stale and create explicit change impact.

#### Scenario: Positive - current link remains fresh
- **GIVEN** a trace link whose atom references match all current atom revisions
- **WHEN** link freshness is validated
- **THEN** the link is fresh and available for navigation

#### Scenario: Negative - requirement scenario changes
- **GIVEN** a trace link to an OpenSpec atom revision
- **WHEN** a scenario edit changes that atom revision
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
