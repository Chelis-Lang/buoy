## ADDED Requirements

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
