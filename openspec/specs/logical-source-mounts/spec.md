# Logical Source Mounts

## Purpose

Define stable logical source namespaces across one or more physical repository roots.

## Requirements

### Requirement: LSM-001 Explicit logical mount declarations

Repository configuration MUST declare each logical mount ID and its canonical path prefix. Each invocation MUST supply exactly one physical directory for each mount ID.

Physical directory text MUST remain operational input. It MUST NOT enter semantic configuration identity or canonical output.

#### Scenario: Positive - two physical roots form one snapshot
- **GIVEN** distinct `project` and `authority` mounts with complete physical directory mappings
- **WHEN** snapshot construction runs
- **THEN** selected files appear under their configured logical prefixes in one snapshot

#### Scenario: Negative - one physical mapping is absent
- **GIVEN** configuration that declares two logical mounts and an invocation that maps only one mount
- **WHEN** input parsing runs
- **THEN** parsing fails with a deterministic missing-mount diagnostic before any file is selected

### Requirement: LSM-002 Physical location independence

Canonical paths MUST start at configured logical prefixes. Equal declarations and equal selected bytes MUST produce equal snapshot, graph, diagnostic, and report bytes.

Host-specific absolute paths MUST NOT appear in canonical values or machine diagnostics.

#### Scenario: Positive - equal roots move between hosts
- **GIVEN** equal selected bytes under different physical directory names
- **WHEN** both invocations use equal logical mount declarations
- **THEN** both snapshots and static reports are byte-identical

#### Scenario: Negative - physical path reaches a diagnostic
- **GIVEN** one rejected selected file under a host-specific absolute directory
- **WHEN** machine diagnostics are rendered
- **THEN** validation fails the canonical-output control if the physical directory text appears

### Requirement: LSM-003 Closed logical namespace

Logical mount IDs and prefixes MUST be unique. Prefixes MUST be non-overlapping canonical repository paths.

Two selected inputs MUST NOT produce one logical path. Logical path collisions MUST fail before adapter materialization.

#### Scenario: Positive - disjoint prefixes are order independent
- **GIVEN** two mounts with disjoint logical prefixes and equal declarations in different orders
- **WHEN** configuration identity and snapshot construction run
- **THEN** both orders produce one equal canonical mount set and equal selected paths

#### Scenario: Negative - two mounts overlap
- **GIVEN** logical prefixes `source` and `source/specs`
- **WHEN** configuration parsing runs
- **THEN** parsing fails with a deterministic mount-prefix-overlap diagnostic

### Requirement: LSM-004 One joined static snapshot

The shell MUST select all mounted files before adapter aggregation. The core MUST receive logical paths and saved bytes without physical root handles.

The joined result MUST be one snapshot. It MUST NOT be a federation of separately accepted graphs.

#### Scenario: Positive - implementation and requirement resolve across mounts
- **GIVEN** one implementation subject under `project` and one requirement under `authority`
- **WHEN** repository materialization and graph closure run
- **THEN** one binding resolves both nodes inside the joined snapshot

#### Scenario: Negative - one mount is accepted separately
- **GIVEN** an implementation reference whose requirement exists only in another declared mount
- **WHEN** one mount is validated without the complete declared mount set
- **THEN** validation fails with a deterministic incomplete-mount-set diagnostic

### Requirement: LSM-005 Aggregate resource budgets

Snapshot count and byte budgets MUST apply across the complete mount set. A caller MUST NOT multiply a limit by adding mounts.

Mount traversal order MUST NOT change the first reported budget category.

#### Scenario: Positive - aggregate input remains within limits
- **GIVEN** selected files across two mounts whose combined counts and bytes remain within every limit
- **WHEN** snapshot construction runs
- **THEN** construction succeeds with one aggregate budget account

#### Scenario: Negative - separate mounts exceed one total
- **GIVEN** each mount remains below a byte limit but their combined selected bytes exceed it
- **WHEN** snapshot construction runs
- **THEN** construction fails with the deterministic aggregate-byte-budget diagnostic
