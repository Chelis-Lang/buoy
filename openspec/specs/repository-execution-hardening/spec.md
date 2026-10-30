# Repository Execution Hardening

## Purpose
This specification defines repository snapshot and process execution controls.

## Requirements

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

### Requirement: REH-012 Race-resistant no-follow publication writes

The shell MUST open each declared publication root through one retained directory descriptor or documented platform equivalent.

All output-path descent, temporary-file creation, synchronization, and replacement MUST remain relative to that descriptor.

The writer MUST reject symlinked or link-like ancestors, root escapes, non-regular existing paths, and component swaps.

It MUST reject a derived output selected as source input. A source edit MUST select one regular source and pass its semantic guard.

The temporary artifact MUST be a unique regular sibling. Successful replacement MUST synchronize required file and directory state under the supported durability contract.

#### Scenario: Positive - regular sibling replaces a tracked page

- **GIVEN** one regular output root, regular parent components, and one successful publication candidate
- **WHEN** publication replacement runs
- **THEN** the candidate becomes visible at the declared path through one atomic same-directory replacement

#### Scenario: Negative - parent directory is a symlink

- **GIVEN** one declared page whose parent component redirects outside the publication root
- **WHEN** output traversal runs
- **THEN** generation fails with a deterministic output-link diagnostic and writes no byte outside the root

#### Scenario: Positive - controlled edit targets selected source

- **GIVEN** one declared source edit that selects one regular standing specification
- **WHEN** path and semantic guards run
- **THEN** the edit remains eligible for no-follow atomic replacement

#### Scenario: Negative - derived output becomes selected source

- **GIVEN** one derived output path that matches an active source selector
- **WHEN** publication-plan validation runs
- **THEN** validation fails with a deterministic selected-output diagnostic before temporary-file creation

### Requirement: REH-013 Serialized publication and complete failure cleanup

Concurrent generation for one normalized output path MUST use one bounded interprocess exclusion mechanism.

A second operation MUST wait within a declared bound or fail with a deterministic busy-output diagnostic. Scheduling order MUST NOT alter successful candidate bytes.

Any parse, materialization, conformance, render, synchronization, replacement, cancellation, or injected failure MUST preserve the prior committed output.

Failure cleanup MUST remove only temporary artifacts owned by that operation. It MUST NOT remove another operation's cache, lock, or candidate.

#### Scenario: Positive - two equal page generations overlap

- **GIVEN** two operations for one output and equal retained snapshot bytes
- **WHEN** both operations overlap
- **THEN** serialization permits one replacement at a time and the final page equals the unique candidate

#### Scenario: Negative - first operation stops before replacement

- **GIVEN** one existing page and one injected failure after temporary candidate synchronization
- **WHEN** cleanup runs
- **THEN** the existing page remains byte-identical and no owned temporary sibling remains

#### Scenario: Negative - concurrent initializer removes another cache

- **GIVEN** two publication operations with one shared realized package or cache
- **WHEN** both initialize concurrently
- **THEN** neither operation removes, nests, or invalidates state owned by the other operation
