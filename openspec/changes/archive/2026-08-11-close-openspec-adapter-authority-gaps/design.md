## Context

The OpenSpec shell adapter uses `pulldown-cmark` ranges to create requirement atoms and scenario source items. Its current section calculation stops only at the next requirement.

A later peer or top-level section can enter the preceding atom revision. Text before the first requirement can also lack a nonnormative disposition.

The authority configuration omits the accepted OpenSpec format version. The public unconfigured extraction path also creates inferred atom metadata.

These defects affect deterministic static registration and freshness. They do not provide execution verdicts or assurance classes.

## Goals / Non-Goals

**Goals:**

- Parse format identity and authority metadata at one configuration boundary.
- Represent accepted documents with typed sections and legal span relationships.
- Partition every accepted file into primary requirement and nonnormative spans.
- Keep nested scenario spans inside their owning requirement spans.
- Reject malformed or unsupported authority syntax before graph materialization.
- Preserve deterministic diagnostics, ordering, and repository-relative identities.
- Correct the maintained requirement-heading guidance.

**Non-Goals:**

- Add OpenSpec workflow enforcement.
- Add repository-wide OpenSpec authority configuration.
- Change external trace-link semantics.
- Change `buoy-core` values or the `no_std` boundary.
- Add an effectful parser, provider client, clock, locale, or network input.
- Claim formal proof for the CommonMark parser or shell adapter.

## Decisions

### 1. Parse one explicit format identity at the configuration boundary

Each capability configuration will require `format = "openspec/standing-spec/v1"`.

The raw string will parse into a closed `OpenSpecFormatVersion` type. Downstream extraction will accept the parsed type and will not check the raw string again.

A missing or unsupported value will return `SPG-OPENSPEC-CONFIG-FORMAT`. Existing positive configurations and documentation will receive the explicit field.

**Alternatives considered:**

- Infer the version from the adapter ID. Rejected because configuration must bind the accepted authority format.
- Declare the version in each Markdown file. Rejected because current OpenSpec standing specs have no reviewed version declaration syntax.
- Accept unknown versions as version 1. Rejected because this action can silently reinterpret authority.

### 2. Parse a complete typed document before atom construction

The shell will parse saved bytes into a document with these typed regions:

- one title and preamble region
- one `## Requirements` section
- zero or more nonauthority top-level sections
- one or more valid requirement sections
- scenario sections that belong to one requirement
- explicit nonnormative byte intervals

A malformed heading, duplicate requirements section, unsupported authority-like heading, or invalid scenario structure will produce stable diagnostics. Invalid documents will not materialize a partial graph.

The parser will order diagnostics by start byte, end byte, code, and context. It will order semantic products by canonical parsed-item identity.

**Alternatives considered:**

- Extend the current heading loop with more conditions. Rejected because separate checks can create uncovered or overlapping regions.
- Validate spans after atom construction. Rejected because illegal document states must fail at the parse boundary.

### 3. Use a primary partition with explicit nested scenario spans

Primary requirement spans and nonnormative spans will form a complete, non-overlapping byte partition for each accepted file.

A requirement starts at its level-three requirement heading. It ends at the next heading with level three or lower.

A scenario span can nest inside only its owning requirement span. Scenario nesting does not alter the primary partition.

Text before the first requirement and after the last requirement will receive nonnormative spans when its section class permits that disposition.

Authority-like headings inside `## Requirements` will fail with a stable unsupported-syntax diagnostic. They will not become nonnormative text.

A focused partition validator will check sorted intervals, adjacency, bounds, overlap, and scenario containment before adapter product construction.

**Alternatives considered:**

- Exclude scenarios from the requirement span. Rejected because the atom revision must bind scenario names and bodies.
- Permit overlapping nonnormative and semantic spans. Rejected because one byte could receive conflicting primary dispositions.
- Treat every unknown heading as nonnormative. Rejected because authority-like syntax must fail closed.

### 4. Remove inferred authority from the production extraction path

Production materialization will require `OpenSpecConfiguration`. Tests will use explicit fixture configuration for atom construction.

The unconfigured public extractor will no longer create atoms with inferred kind, scope, lifecycle, or registry state. Parser-only helpers will remain private to the adapter module.

Lifecycle state and approval will continue to come only from parsed repository configuration. A source archive operation will not change lifecycle state.

**Alternatives considered:**

- Keep inferred defaults for tests. Rejected because a public bypass contradicts the authority boundary.
- Repeat metadata checks after extraction. Rejected because the parsed configuration type already makes invalid metadata unavailable.

### 5. Keep the functional core unchanged

All CommonMark parsing, configuration parsing, spans, and diagnostics remain in the root shell crate. The shell continues to depend inward on `buoy-core`.

`buoy-core` remains `no_std` and receives only validated complete values. It will not depend on OpenSpec, Markdown, files, processes, or ambient state.

Corrected atom revisions can make existing links stale. Static freshness reports will expose that result through existing graph rules.

Oracle execution remains separate. This change will not create execution receipts or change execution verdicts.

### 6. Use tests as bounded evidence only

Fixtures will cover accepted partitions, malformed boundaries, unsupported formats, Unicode ranges, trailing sections, and documentation parity.

A finite interval model will enumerate short region sequences. It will compare accepted partition outcomes with the production partition validator.

Hegel-rs properties will vary legal region lengths and Unicode prefixes. They will check partition completeness, non-overlap, containment, and deterministic output.

Verus and AeneasVerif will not verify this shell parser. Existing formal results do not prove CommonMark parsing, UTF-8 libraries, allocation, rustc, LLVM, or natural-language correspondence.

### 7. Use one exact acceptance oracle

The exact acceptance oracle is `devenv test` from the repository root. Acceptance requires exit code `0` after all focused adapter and OpenSpec checks pass.

The focused commands are evidence for their named scopes only. They do not replace the final oracle or create mathematical proof.

## Risks / Trade-offs

- **Existing configurations fail**: Each OpenSpec capability needs the new exact `format` field.
- **Atom revisions change**: Correct section boundaries can invalidate links that currently bind absorbed nonauthority text.
- **Strict syntax rejects prior input**: The adapter will reject unsupported authority-like headings that previously entered partial output.
- **Partition rules add parser complexity**: Typed intervals and a finite model reduce duplicate checks and expose boundary errors.
- **Public API narrows**: Removal of inferred extraction can require changes to tests or internal consumers.
