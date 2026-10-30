## ADDED Requirements

### Requirement: TPT-001 Versioned trace-publication plan

Buoy MUST parse repository-owned publication plans with schema `buoy/trace-publication-plan/v1` into one strict typed value at the command boundary.

Each plan MUST declare one stable publication ID, one typed trace selector, one expected row count, one source-set policy, and each declared output.

An exact source-set policy MUST contain normalized logical paths. It MUST constrain selector results and MUST NOT replace selector evaluation.

Each output MUST declare its owner mount, logical path template, artifact kind, lifecycle, and applicable stable policy values.

The parser MUST derive `derived` or `source-edit` effect class from the artifact kind. A plan MUST NOT override that class.

A page can declare one normalized absolute target base. An external-link output MUST reference exactly one declared page output.

Buoy MUST join the base with the rendered logical page path to derive that link target.

A plan MUST NOT copy or override the complete target.

In version 1, exactly one external-link output MUST reference each immutable target page. A replaceable alias can remain unreferenced.

The initial lifecycle set MUST contain `immutable` and `replaceable`. Unknown, duplicate, omitted, or malformed fields MUST fail closed.

A publication plan MUST remain operational configuration. It MUST NOT become a graph object, authority source, assurance class, or execution receipt.

#### Scenario: Positive - complete publication plan is accepted

- **GIVEN** one plan with a stable ID, one requirement selector, one expected row, one page, and one external-link output
- **WHEN** publication-plan parsing runs
- **THEN** Buoy returns one typed plan whose invalid states cannot reach publication processing

#### Scenario: Negative - immutable page has no link owner

- **GIVEN** one immutable target page with no referencing external-link output
- **WHEN** publication-plan parsing runs
- **THEN** parsing fails with input exit `70` and one orphan-immutable-page diagnostic

#### Scenario: Negative - plan has an unknown field

- **GIVEN** one publication plan with an undeclared field or duplicate output path
- **WHEN** publication-plan parsing runs
- **THEN** parsing fails with input exit `70` and one deterministic plan diagnostic before repository materialization

### Requirement: TPT-002 Buoy-owned publication command boundary

Buoy MUST expose `buoy publication check`, `buoy publication generate`, and `buoy publication evidence` operations.

Each operation MUST require one explicit plan. `check` and `generate` MUST also require one declared artifact ID.

`check` and `evidence` MUST remain read-only. `generate` MUST write only the selected artifact.

The publication command MUST call Buoy libraries in-process. It MUST NOT invoke a caller-selected Buoy executable or resolve a command from `PATH`.

Conflicting selector forms MUST return usage exit `64`. A malformed or unmatched typed selector MUST return input exit `70`.

The existing `buoy trace` command MUST remain read-only and MUST continue to write projections only to standard output.

#### Scenario: Positive - page check uses the current Buoy process

- **GIVEN** one valid publication plan and one pinned Buoy executable
- **WHEN** the publication check evaluates the declared page
- **THEN** it uses the in-process parser, graph, trace, and renderer implementations

#### Scenario: Negative - environment supplies another Buoy executable

- **GIVEN** a valid plan and an environment variable or `PATH` entry that names a substitute executable
- **WHEN** any publication operation runs
- **THEN** the substitute is ignored and cannot generate or approve publication bytes

### Requirement: TPT-003 Strict row-local trace conformance

Publication conformance MUST parse `buoy/trace-report/v1` into strict report types. It MUST NOT infer structure from text fragments.

The check MUST enforce the plan's exact row count. It MUST evaluate relation, record, atom, nodes, paths, state, and freshness on each same row.

A derived-artifact operation MUST require an empty selected diagnostic set and exact saved source edits.

An external-link operation MUST normalize one declared publication slot before pre-link conformance.

The slot is one standalone level-two nonauthority section keyed by the stable link ID.

Its fixed boundary is immediately after the final normative requirement and before every other nonauthority section.

The slot span MUST own its canonical separator bytes.

The normalizer MUST remove a present slot and owned separators, then insert equal marker bytes at the fixed boundary.

Normalization can suppress only diagnostics caused by that declared link. All relation, requirement, implementation, authority, and unrelated-link diagnostics remain blockers.

All trace facts MUST derive from retained saved bytes. Marker bytes MUST remain operational basis input and MUST NOT become source-graph input.

A stale selected row MUST remain stale and MUST return static-blocker exit `2`.

#### Scenario: Positive - one exact current row satisfies the plan

- **GIVEN** one plan that expects one row and one report with that exact current row
- **WHEN** publication conformance runs
- **THEN** all row-local identities, paths, state, freshness facets, and diagnostics satisfy the plan together

#### Scenario: Positive - stale declared link can receive a candidate

- **GIVEN** one current relation and one stale declared link for the prior atom revision
- **WHEN** pre-link conformance normalizes that exact publication slot
- **THEN** conformance permits link repair and retains each relation or unrelated-link diagnostic

#### Scenario: Negative - expected fragments occur on different rows

- **GIVEN** two rows whose separate fields collectively match one expected relation
- **WHEN** publication conformance runs
- **THEN** the check fails with a deterministic row-conformance diagnostic and accepts no fragment-level match

### Requirement: TPT-004 One immutable selected snapshot per operation

Each publication operation MUST derive its selected files from parsed repository configuration and explicit logical mounts.

Mount roots MUST come from explicit command inputs. Ambient repository-root environment variables MUST NOT select source bytes.

Buoy MUST read each selected path through the existing no-follow snapshot boundary. Consumer-maintained file-copy lists MUST NOT define the selected set.

After selection, Buoy MUST compare the complete logical path set with the plan's source-set policy. Scope drift MUST return exit `2`.

All reports and candidates from one operation MUST use one retained immutable snapshot. Physical roots, discovery order, and later file reads MUST NOT alter them.

Before a derived-artifact operation, every declared source edit MUST equal its candidate in the retained saved snapshot.

If a source edit is absent or stale, the operation MUST return static-blocker exit `2` before derived rendering.

Before a write, Buoy MUST detect a selected source change after snapshot capture. It MUST abort without replacing the output.

#### Scenario: Positive - equal roots produce equal candidates

- **GIVEN** equal selected bytes and plan bytes under different physical roots
- **WHEN** publication candidate generation runs
- **THEN** both candidates and machine diagnostics are byte-identical

#### Scenario: Negative - recursive selector gains another file

- **GIVEN** one exact source-set policy and one recursive selector that now matches an additional nested file
- **WHEN** publication snapshot conformance runs
- **THEN** the operation returns exit `2` with a deterministic selected-scope-drift diagnostic

#### Scenario: Negative - environment redirects an authority mount

- **GIVEN** explicit authority-mount input and an environment variable that names another worktree
- **WHEN** publication snapshot construction runs
- **THEN** only the explicit mount supplies selected bytes

#### Scenario: Negative - page precedes its source edit

- **GIVEN** one declared page and one absent or stale declared external-link edit in the saved snapshot
- **WHEN** page check or generation runs
- **THEN** the operation returns exit `2` before page render or replacement

#### Scenario: Negative - selected source changes before replacement

- **GIVEN** one retained snapshot and a selected source change before output replacement
- **WHEN** the publication writer performs its final source guard
- **THEN** generation fails with a deterministic source-changed diagnostic and preserves the prior output

### Requirement: TPT-005 Deterministic tracked-page parity and generation

A declared trace page MUST equal the Markdown projection from the plan's selector and retained snapshot.

Page check MUST compare exact bytes and MUST NOT change the page.

An absent or unequal tracked page MUST return static-blocker exit `2` with deterministic fields.

Page generation MUST write the page only after successful materialization, conformance, rendering, source guarding, and output-path guarding.

An `immutable` page MUST be created when absent, accepted when equal, and rejected when existing bytes differ. A `replaceable` page can use atomic replacement.

This lifecycle defines Buoy's local write policy. It MUST NOT claim Git or provider immutability.

A failed operation MUST preserve the prior page and remove its uncommitted temporary artifact.

#### Scenario: Positive - generated current alias replaces stale bytes

- **GIVEN** one conforming current trace and one stale `replaceable` page
- **WHEN** explicit page generation runs
- **THEN** Buoy atomically replaces only that page with the exact Markdown projection

#### Scenario: Positive - immutable basis page is created once

- **GIVEN** one conforming current trace and one absent `immutable` basis-keyed page
- **WHEN** explicit page generation runs
- **THEN** Buoy creates the exact page and later equal generation becomes a no-op

#### Scenario: Negative - immutable page has different bytes

- **GIVEN** one declared `immutable` page whose existing bytes differ from the candidate
- **WHEN** page generation runs
- **THEN** Buoy fails with a deterministic immutable-output-conflict diagnostic and preserves the existing page

#### Scenario: Negative - trace relation is stale

- **GIVEN** one declared page and one selected stale implementation relation
- **WHEN** page generation runs
- **THEN** Buoy returns exit `2`, retains the stale-relation diagnostic, and leaves the prior page byte-identical

### Requirement: TPT-006 Deterministic OpenSpec external-link synchronization

A declared OpenSpec external-link output MUST identify one version 2 standing specification, one current semantic atom, and one declared target page.

The initial publication schema MUST accept only relation `documents` for this artifact kind.

Generation MUST add or replace only one standalone level-two nonauthority publication slot after all normative requirements.

The slot MUST contain one canonical heading and the declared trace fence. Generation MUST preserve every other byte and all normative source spans.

A legacy fence can migrate only if its enclosing level-two section contains no other nonwhitespace content.

Buoy MUST materialize and compare authority nodes before and after the edit. Requirement nodes and atom revisions MUST remain equal.

An invocation MUST update at most one declared link artifact. It MUST NOT perform a partial multi-repository transaction.

#### Scenario: Positive - stale atom reference receives a current link

- **GIVEN** one current requirement and one stale declared nonauthority link for its prior revision
- **WHEN** explicit link generation runs
- **THEN** Buoy replaces only that publication slot and binds the current semantic atom revision

#### Scenario: Negative - legacy section contains unrelated text

- **GIVEN** one declared legacy fence whose enclosing section also contains unrelated nonwhitespace text
- **WHEN** publication-slot migration runs
- **THEN** generation fails with a deterministic mixed-publication-slot diagnostic and preserves the source

#### Scenario: Negative - proposed edit changes requirement bytes

- **GIVEN** one link edit candidate that enters a normative requirement span
- **WHEN** authority-identity comparison runs
- **THEN** generation fails with a deterministic authority-mutation diagnostic and writes no candidate

### Requirement: TPT-007 Cycle-safe output and target dependencies

A derived output path MUST remain outside every selected source set for the complete plan configuration.

A source-edit path MUST select exactly one regular source file and MUST pass its artifact-specific semantic guard.

Before link construction, Buoy MUST compute one publication-basis digest from every pre-link input that can affect the final page bytes.

An immutable basis and page-content digest MUST use an active member with the `collision-resistant` assurance class.

These digests identify routing inputs and exact bytes only. They MUST NOT become graph objects or establish authenticity, correctness, or proof.

The basis MUST bind the plan schema, renderer schema, selector, exact trace facts, logical locations, and publication-slot-normalized snapshot.

For equal pre-link inputs, normalization before and after slot generation MUST produce byte-identical snapshot inputs at the same boundary.

It MUST encode parsed template literals before substitution. It MUST NOT encode its own digest or any rendered basis substitution.

A path-template substitution MUST derive only from an identity that exists before external-link construction. Accepted dynamic identities MUST be explicitly registered by schema.

The initial schema MUST support semantic atom revision, current binding-record digest, and publication-basis substitutions. Portable path substitutions MUST retain algorithm identity and payload bytes.

An immutable page path MUST include the publication basis. Atom and record substitutions can provide additional path grouping.

The target base MUST be an absolute normalized `https` URI whose path ends with `/`.

It MUST contain no dynamic substitution, query, or fragment.

The URI join MUST percent-encode normalized UTF-8 path segments, preserve slash boundaries, and reject encoded traversal.

A path or target MUST NOT depend on graph, page, page-commit, branch-head, provider, or generated-link values.

#### Scenario: Positive - immutable target uses the publication basis

- **GIVEN** one current trace, one basis-keyed page path, and one normalized target base
- **WHEN** path rendering, URI joining, and dependency validation run
- **THEN** Buoy produces one immutable normalized target that ends with the rendered logical page path

#### Scenario: Positive - first link generation preserves the basis

- **GIVEN** one standing specification with no declared publication slot
- **WHEN** link generation adds the canonical slot and a later operation normalizes it
- **THEN** both operations compute the same publication basis from equal remaining inputs

#### Scenario: Negative - atom-only key permits unequal immutable pages

- **GIVEN** two page candidates with one atom revision and different page-affecting implementation facts
- **WHEN** an immutable page path uses only that atom revision
- **THEN** validation fails with a deterministic incomplete-publication-key diagnostic

#### Scenario: Negative - immutable basis uses a change-detection member

- **GIVEN** one immutable page plan and active member `xxh3-128`
- **WHEN** publication-basis configuration runs
- **THEN** configuration fails with a deterministic publication-digest-class diagnostic before basis computation

#### Scenario: Negative - target uses the page commit

- **GIVEN** one page path or target base that requires the commit that will contain the generated page
- **WHEN** publication dependency validation runs
- **THEN** the plan fails with a deterministic publication-cycle diagnostic before any file write

### Requirement: TPT-008 Deterministic publication evidence

The evidence operation MUST emit machine schema `buoy/trace-publication-evidence/v1` with status, diagnostics, and an optional completion payload.

It MUST require every declared non-evidence artifact to equal its candidate.

Drift MUST return exit `2` with machine diagnostics and no completion payload.

A declared completion-evidence artifact MUST render deterministic Markdown from the same typed value through explicit generation.

Evidence MUST identify the plan, selector, publication basis, source and static graphs, exact rows, external link, page digest, paths, and freshness facets.

Evidence MUST exclude immutable record bodies, physical roots, provider fields, clocks, users, approvals, raw logs, and temporary paths.

Evidence MUST state that Buoy remains the current-state authority. Archived evidence MUST remain historical after later source changes.

Recorded freshness facets describe one static result. The evidence artifact, page, link, and digests MUST NOT establish current freshness.

#### Scenario: Positive - completion evidence derives from one snapshot

- **GIVEN** one conforming publication plan with a current page and external link
- **WHEN** machine evidence and the declared Markdown artifact render
- **THEN** both forms contain equal identities from one retained snapshot in canonical order

#### Scenario: Negative - page drift blocks new evidence

- **GIVEN** one current relation and one declared page whose saved bytes differ from its candidate
- **WHEN** the evidence operation runs
- **THEN** it returns exit `2` with a machine blocker and no completion payload

#### Scenario: Negative - evidence claims current authority after drift

- **GIVEN** one archived evidence document whose recorded relation later becomes stale
- **WHEN** a reader or checker evaluates its declared scope
- **THEN** the document remains historical and MUST NOT replace a current Buoy query

### Requirement: TPT-009 Deterministic publication diagnostics

Publication machine diagnostics MUST use stable codes, logical paths, structured fields, and canonical order.

Machine and human diagnostics MUST exclude timestamps, random temporary names, physical roots, process IDs, locale data, and raw platform diff headers.

A readable parity difference MUST use fixed labels and deterministic line ordering. The machine diagnostic MUST remain the automation contract.

#### Scenario: Positive - equal stale pages exist under separate roots

- **GIVEN** equal stale page and candidate bytes under different physical roots and times
- **WHEN** page check runs
- **THEN** both machine diagnostic byte sequences are equal

#### Scenario: Negative - diff includes a temporary path

- **GIVEN** one parity renderer that includes a random temporary path or file timestamp
- **WHEN** publication diagnostic validation runs
- **THEN** validation fails with a deterministic forbidden-publication-field diagnostic

### Requirement: TPT-010 Publication preserves ownership and assurance boundaries

Buoy publication tooling MUST remain repository-independent. Stable plans and fixtures MUST NOT contain Barnacle-specific IDs, paths, repositories, or policy.

Generation MUST NOT edit normative prose, implementation code, plan policy, task completion, or human review facts.

It MUST NOT commit, push, merge, approve, fetch a target, inspect provider state, or claim publication success beyond local declared files.

A generated page, link, state, or evidence document MUST provide navigation and impact data only.

A recorded freshness facet MUST NOT become an assurance claim or replace a current static query.

#### Scenario: Positive - consumer supplies all repository policy

- **GIVEN** one downstream plan with consumer-owned IDs, paths, mounts, and target base
- **WHEN** Buoy processes that plan
- **THEN** generic Buoy code applies the declared policy without a consumer-specific branch

#### Scenario: Negative - generated evidence claims correctness

- **GIVEN** one generated publication artifact that claims correctness, coverage, execution, proof, approval, or prose-to-code correspondence
- **WHEN** assurance-boundary validation runs
- **THEN** generation fails with a deterministic claim-boundary diagnostic

### Requirement: TPT-011 Versioned publication operation report

Each publication `check` and `generate` operation MUST emit machine schema `buoy/trace-publication-report/v1` by default.

The evidence operation MUST emit its evidence schema instead of a second top-level operation report.

The report MUST identify available plan, operation, artifact, basis, graph, prior-state, candidate-state, and diagnostic fields.

Its stage tag MUST make unavailable fields absent. The report MUST NOT fabricate an identity or placeholder digest.

A drift check MUST show complete old and candidate identities before any update operation.

A successful generation report MUST identify only the local replacement that completed. It MUST NOT claim repository commit, provider publication, or cross-repository completion.

Every machine digest MUST use its complete canonical form.

Human output MUST derive from the same typed report and can abbreviate display digests.

A new report field or changed meaning MUST require a new schema version.

#### Scenario: Positive - page generation emits one completed replacement

- **GIVEN** one successful declared page generation
- **WHEN** operation-report rendering runs
- **THEN** machine output identifies the exact prior and generated page digests with no provider or commit fact

#### Scenario: Negative - report claims both repositories changed

- **GIVEN** one operation that replaced only a local page
- **WHEN** its report claims completion of an external authority edit
- **THEN** report validation fails with a deterministic publication-scope diagnostic
