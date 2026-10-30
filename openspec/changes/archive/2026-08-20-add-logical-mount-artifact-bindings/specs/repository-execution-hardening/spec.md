## ADDED Requirements

### Requirement: REH-009 Independent no-follow mount descriptors

The shell MUST open each physical mount root independently and retain its directory descriptor during selection.

All descent and reads MUST remain descriptor-relative. The shell MUST reject link-like roots, link-like entries, submodules, nested repository links, and root replacement races.

#### Scenario: Positive - two regular roots remain descriptor bound
- **GIVEN** two regular physical mount roots with regular selected files
- **WHEN** snapshot construction opens and reads both roots
- **THEN** every read remains relative to its original mount descriptor

#### Scenario: Negative - one mounted root is replaced
- **GIVEN** two mounted roots and replacement of one pathname after its descriptor opens
- **WHEN** selection continues
- **THEN** construction never reads replacement bytes and reports the reviewed race result

### Requirement: REH-010 Mount-wide bounded failure

Global path, entry, file, byte, graph, reference, diagnostic, depth, and visited-node budgets MUST cover the complete mount set.

One mount failure MUST fail the complete snapshot. Static acceptance and oracle execution MUST reject every partial mount product.

#### Scenario: Positive - complete mount set passes all budgets
- **GIVEN** a complete mount set whose aggregate snapshot and graph stay within every configured limit
- **WHEN** static validation runs
- **THEN** all mounted products reach one accepted static result

#### Scenario: Negative - later mount exceeds an entry budget
- **GIVEN** an earlier valid mount and a later mount that exceeds the aggregate directory-entry limit
- **WHEN** snapshot construction runs under any mount order
- **THEN** construction fails with the deterministic entry-budget diagnostic and retains no partial accepted graph

### Requirement: REH-011 Static mounts remain oracle independent

Logical mount declarations, saved bytes, adapter products, source graphs, and binding freshness MUST remain independent from oracle execution.

Oracle working directories MUST use one explicitly selected physical mount. Oracle output MUST NOT alter logical source identity.

#### Scenario: Positive - static check runs without oracles
- **GIVEN** a complete mounted snapshot and no configured oracle commands
- **WHEN** static validation runs
- **THEN** it produces the same static graph and report as an invocation before equivalent oracle execution

#### Scenario: Negative - oracle requests an unknown mount
- **GIVEN** one oracle whose working mount is absent from the logical mount declarations
- **WHEN** configuration parsing runs
- **THEN** parsing fails with a deterministic oracle-mount diagnostic before process execution
