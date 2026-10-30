## ADDED Requirements

### Requirement: REH-001 Pinned full Unicode path conformance
Path NFC normalization and portable collision keys MUST use pinned full Unicode normalization and default case-fold data for the declared schema/tool lock. The implementation MUST pass the applicable conformance corpus and MUST NOT use a handcrafted subset as the canonical case-fold authority.

#### Scenario: Positive - multi-code-point case fold collides
- **GIVEN** distinct valid UTF-8 path spellings whose pinned full default case folds are equal
- **WHEN** snapshot paths are normalized
- **THEN** selection fails with a deterministic portable-collision diagnostic

#### Scenario: Negative - ASCII lowercase misses a Unicode collision
- **GIVEN** a collision represented only by non-ASCII full case-fold behavior
- **WHEN** a simplified lowercase implementation accepts both paths
- **THEN** the conformance suite detects the mismatch and the snapshot MUST NOT be accepted

### Requirement: REH-002 Race-resistant no-follow snapshot reads
The shell MUST classify and read each selected entry through descriptor/handle-relative no-follow operations or a documented equivalent that validates the opened object. It MUST reject symlinks, junctions, reparse points, mount aliases exposed as links, submodules, and component swaps without a classify-then-reopen race.

#### Scenario: Positive - stable regular file is opened beneath root
- **GIVEN** an explicit root handle and a selected regular file whose traversed components are non-link directories
- **WHEN** descriptor-relative snapshotting runs
- **THEN** the exact opened bytes and canonical relative path enter the snapshot

#### Scenario: Negative - attacker swaps file for symlink
- **GIVEN** a selected regular path changed to a symlink between directory enumeration and open
- **WHEN** snapshotting opens and validates the entry
- **THEN** the operation fails closed and MUST NOT read the link target

### Requirement: REH-003 Versioned deterministic resource budgets
Repository configuration MUST declare versioned semantic budgets for path depth, directory entries, selected files, per-file bytes, total bytes, graph objects/references, canonical nesting/counts, diagnostics, oracle output, concurrency, and execution timeout. Budget checks MUST occur before unbounded allocation or work and MUST emit stable deterministic diagnostics.

#### Scenario: Positive - snapshot is within every budget
- **GIVEN** explicit inputs whose counts and sizes are at or below all active limits
- **WHEN** snapshot, parsing, graph, encoding, and execution run
- **THEN** budget validation succeeds without adding host-memory facts to canonical output

#### Scenario: Negative - one file exceeds its byte budget
- **GIVEN** a selected regular file one byte larger than the configured maximum
- **WHEN** the shell attempts to snapshot it
- **THEN** it fails with a deterministic file-budget diagnostic before retaining the complete bytes

### Requirement: REH-004 Bounded-memory deterministic snapshot construction
Snapshot construction MUST avoid using complete file bytes as map keys, MUST detect duplicate bytes through domain-bound digests plus exact collision confirmation when required, and MUST use iterative or explicitly bounded traversal. Equal semantic inputs discovered in different orders MUST produce identical outputs.

#### Scenario: Positive - many equal files remain bounded
- **GIVEN** many selected files containing equal bytes within configured total limits
- **WHEN** duplicate/derivation advisory analysis runs
- **THEN** memory use is bounded by declared budgets and canonical results are path-order independent

#### Scenario: Negative - deeply nested tree exhausts recursion
- **GIVEN** a selected tree exceeding the configured path-depth budget
- **WHEN** traversal runs
- **THEN** it terminates with a stable budget diagnostic rather than overflowing the call stack or hanging

### Requirement: REH-005 Isolated process-tree lifecycle
Effectful oracle execution MUST launch an explicit pinned executable with empty/default-denied environment, explicit working root, bounded stdin/stdout/stderr, controlled concurrency, and a platform process group/job object. Timeout, abnormal exit, or cancellation MUST terminate and reap the complete descendant tree.

#### Scenario: Positive - selected oracle exits cleanly
- **GIVEN** a pinned oracle producing a valid bounded semantic result and exiting with the configured success protocol
- **WHEN** the supervisor runs it
- **THEN** one normalized receipt is returned and all process/pipe resources are reaped

#### Scenario: Negative - timed-out shell leaves a child
- **GIVEN** an oracle whose immediate shell spawns a long-running descendant and exceeds timeout
- **WHEN** cleanup runs
- **THEN** the complete group/job is terminated and no descendant or output pipe remains alive

### Requirement: REH-006 Concurrent bounded output draining
The supervisor MUST drain stdout and stderr concurrently while enforcing independent and combined output budgets. A child filling one pipe MUST NOT deadlock the supervisor. Truncation/overflow MUST map to the configured deterministic semantic error while bounded raw bytes remain noncanonical.

#### Scenario: Positive - both streams fill concurrently within limits
- **GIVEN** an oracle writing substantial interleaved stdout/stderr within budgets
- **WHEN** it executes
- **THEN** both streams are drained without deadlock and the configured result normalizer receives complete required bytes

#### Scenario: Negative - stdout exceeds budget
- **GIVEN** an oracle continuously writing beyond its stdout limit
- **WHEN** supervision runs
- **THEN** it terminates the process tree, emits normalized `error`, and MUST NOT allocate or retain unbounded output

### Requirement: REH-007 Explicit semantic result normalization
Each oracle MUST bind a versioned result protocol and normalizer. Discovery or exit status alone MUST NOT synthesize `pass`; malformed/missing semantic results and abnormal termination MUST normalize to `error`, a valid domain failure MUST normalize to `fail`, and timeout MUST remain `timeout`.

#### Scenario: Positive - valid failure result exits as configured
- **GIVEN** a selected oracle emitting a schema-valid negative result and configured process status
- **WHEN** normalization runs
- **THEN** the receipt verdict is `fail` with only allowed semantic fields

#### Scenario: Negative - exit zero has malformed result
- **GIVEN** an oracle exiting zero but emitting bytes that violate its bound result schema
- **WHEN** normalization runs
- **THEN** the verdict is `error`, not `pass`

### Requirement: REH-008 Static identity remains effect independent
Snapshot/graph/static report identity MUST remain independent of oracle scheduling, elapsed time, process IDs, temporary paths, environment, raw logs, and normalized verdicts. Execution reports MUST be deterministic functions of static identity and normalized receipt sets.

#### Scenario: Positive - completion order differs
- **GIVEN** the same static graph and normalized receipt set produced under different process completion orders
- **WHEN** execution reports are encoded
- **THEN** canonical report bytes are identical

#### Scenario: Negative - timeout changes graph digest
- **GIVEN** one oracle timing out on a run
- **WHEN** static and execution outputs are compared
- **THEN** timeout appears only in execution facts and MUST NOT alter static graph identity
