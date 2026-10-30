## Context

OpenSpec stores current behavior under `openspec/specs/`. Each capability file contains requirement headings, normative prose, and scenarios.

Buoy cannot identify an OpenSpec requirement today. The active parsed-item kind enum contains only `markdown-fence` and `rust-item`.

The shell adapter contract accepts saved repository paths, saved bytes, and explicit configuration. This contract is suitable for an OpenSpec adapter.

The OpenSpec JSON command omits requirement IDs and scenario names. Therefore, it cannot supply the complete identity that Buoy needs.

Buoy already links Rust items and evidence carriers to exact atom revisions. It has no general graph object for issues, pull requests, or documents.

The current product boundary makes all OpenSpec content reference-only. This change revises that boundary only for standing specs under semantic schema v3.

The active Chelis change reports the same parsed-item limit. This change owns the generic protocol update, and the Chelis change will consume it.

## Goals / Non-Goals

**Goals:**

- Create deterministic Buoy atoms from saved OpenSpec standing specs.
- Give each OpenSpec requirement an exact source identity and source span.
- Preserve stable atom IDs across path changes and harmless Markdown layout changes.
- Include requirement prose and scenarios in the atom revision.
- Link exact atom revisions to code, tests, issues, pull requests, and documents.
- Detect stale links after changes to requirements or linked items.
- Keep all parsers, filesystems, processes, and provider clients outside `buoy-core`.
- Use one schema-v3 cutover without legacy parsed-item readers.

**Non-Goals:**

- The adapter does not parse active changes, archived changes, proposals, designs, or tasks as authority.
- The adapter does not infer approval from archive state, Git history, or provider reviews.
- The first version does not create separate semantic objects for scenarios.
- External trace links do not satisfy authority, coverage, execution, structural, or proof obligations.
- This change does not fetch issue, pull-request, or document state.
- This change does not add OpenSpec, JavaScript, Node.js, or a provider client to `buoy-core`.
- This change does not publish the workflow as a community schema.

## Decisions

### 1. Add semantic schema v3 for extensible source-item kinds

Schema v3 replaces the closed `ItemKind` enum field with a validated `SourceItemKind` value. The value contains one versioned kind ID.

Initial kind IDs are:

- `markdown/fence/v1`
- `rust/item/v1`
- `openspec/requirement/v1`
- `openspec/scenario/v1`

Each adapter descriptor declares its supported kind IDs. The shell validates that every materialized item uses one declared kind.

`buoy-core` accepts only a parsed `SourceItemKind`. It does not read the adapter registry or revalidate the source string.

The v3 parsed-item identity binds these fields:

1. the language or syntax schema ID
2. the normalized repository path
3. the source-item kind ID
4. the semantic item bytes
5. the optional enclosing-item digest

The v3 encoder uses a versioned text field for the kind ID. It does not reuse the v2 enum tag.

The v3 profile uses a breaking cutover. It does not read or reinterpret v1 or v2 parsed-item identities.

A new core enum variant was considered. It was rejected because each new host adapter requires another core schema change.

An unvalidated opaque payload was considered. It was rejected because a typo can create a false source identity.

### 2. Parse saved OpenSpec Markdown directly

The adapter ID is `openspec/v1`. Its syntax schema ID is `openspec.spec/v1`.

The adapter uses a pinned CommonMark parser and the exact saved bytes. It does not run the OpenSpec CLI or a subprocess.

The selector accepts only paths under `openspec/specs/`. Each selected capability file must use the `spec.md` name.

The adapter recognizes these structural elements:

- one `## Purpose` section
- one `## Requirements` section
- `### Requirement: <stable-id> <title>` headings
- `#### Scenario: <name>` headings
- scenario bodies under their owning requirement

The stable ID uses the Buoy stable-ID grammar. The append-only registry must reserve the ID before the adapter accepts the requirement.

The adapter creates one atom for each requirement. The complete atom statement contains the title, normative prose, scenario names, and scenario bodies.

The canonical statement uses parsed source slices and schema-defined separators. Source order remains semantic in version 1 of this adapter.

The atom source span covers the complete requirement section. The span starts at the requirement heading and ends before the next requirement heading.

Purpose text and unmatched supported prose receive explicit nonnormative dispositions. Malformed authority-like headings produce deterministic diagnostics.

The adapter rejects duplicate IDs across all selected inputs. It also rejects a requirement with no positive or negative scenario.

The adapter does not parse `openspec show --json`. That output omits requirement IDs and scenario names at the reviewed OpenSpec revision.

An OpenSpec CLI dependency was considered. It was rejected because the CLI adds a process boundary and returns incomplete identity data.

A heading-only revision was considered. It was rejected because scenario changes do not invalidate linked tests.

### 3. Use explicit adapter configuration for missing authority fields

OpenSpec requirements do not contain all fields that a Buoy atom needs. The adapter uses `buoy.openspec-adapter-config/v1` for these fields.

The configuration contains:

- explicit capability-path to scope mappings
- a default requirement kind
- optional per-requirement kind overrides
- lifecycle state for each requirement
- approval bindings for ratified requirements
- the stable ID registry binding
- the accepted OpenSpec format version
- the required scenario polarity policy

The default lifecycle state is `proposed`. A ratified state requires an exact repository-owned approval binding.

The adapter never infers ratification from `openspec archive`, a Git commit, a branch name, or a provider review.

The configuration is part of the adapter input digest. A configuration change can change the graph and produce explicit impact.

Path-derived scope was considered. It was rejected because a directory move can silently change normative scope.

Automatic ID allocation was considered. It was rejected because traversal order or provider state can influence identity.

### 4. Keep code and evidence links in language adapters

The OpenSpec adapter creates atoms and source mappings. It does not scan implementation code or test code.

Language adapters continue to create `ImplementationLink` and `EvidenceCarrier` objects. Each object binds the exact OpenSpec atom revision.

A requirement change makes old code and evidence links stale. The normal impact graph exposes every affected link and carrier.

The first version reports traceability at requirement level. A future schema can add scenario objects only after it defines stable scenario IDs.

A single OpenSpec adapter for all repositories was considered. It was rejected because one adapter cannot provide parser-backed identities for every language.

### 5. Add external trace links as navigation-only graph objects

Schema v3 adds `ExternalTraceLink`. This object links one or more exact atom revisions to one external target.

The object contains:

- a stable link ID
- a nonempty atom-revision set
- one closed relation value
- one normalized external target
- an optional repository-owned display label

The initial relation values are `tracks`, `discusses`, `documents`, and `reviews`.

The target is an absolute `https` URI or a versioned `urn` value. User information and URI fragments are not accepted.

The target text is a repository-owned fact. Buoy does not fetch the target or interpret its current state.

A Markdown fence with info string `buoy-external-trace-v1` stores the object. The normal Markdown adapter parses this syntax from explicit roots.

An atom revision change makes the trace link stale. An external target change creates a new link revision and normal change impact.

External trace links never satisfy an evidence role. A report displays them in a separate navigation section.

Live provider objects were considered. They were rejected because provider state is mutable, unavailable offline, and outside repository authority.

Debt-only external references were considered. They were rejected because normal issues and documents do not always represent debt.

### 6. Add a `buoy-bridge` OpenSpec workflow schema

The project forks the built-in `spec-driven` workflow into `openspec/schemas/buoy-bridge/`.

The workflow adds a `traceability` artifact after specs and design. Tasks require the traceability artifact.

The artifact records planned actions for:

- stable ID allocation
- atom metadata
- implementation links
- evidence carriers
- external trace links
- link removals and replacements
- required Buoy commands

The traceability artifact is planning data. It is not a Buoy graph input and does not satisfy any technical obligation.

Project rules require stable requirement IDs and positive and negative scenarios. Apply guidance tells the agent to update repository-owned Buoy inputs.

Archive guidance tells the agent to run strict OpenSpec validation and the Buoy change gate. The guidance remains advisory.

CI and `devenv test` provide enforcement. OpenSpec artifact existence alone does not establish valid or fresh traceability.

A custom workflow without a Buoy adapter was considered. It was rejected because prompt rules cannot detect stale or false links.

### 7. Preserve the functional-core boundary

`buoy-core` owns v3 value types, encoding, graph closure, impact edges, and conservative report facts. It remains safe, sequential, and `#![no_std]`.

The root shell owns OpenSpec parsing, path selection, configuration parsing, source spans, external-trace syntax, and human navigation output.

The dependency direction remains `buoy shell -> buoy-core`. No core code depends on OpenSpec, CommonMark, TOML, Git, or provider libraries.

Static registration, freshness, execution verdicts, and assurance classes remain separate facts. The adapter adds no new effectful oracle.

### 8. Define exact impact and trust boundaries

Schema v3 adds impact edges from atom revisions to external trace links. Existing edges continue from atom revisions to code and evidence links.

The complete graph binds adapter descriptors, configuration, source bytes, source maps, external links, and the schema/tool lock.

Static output excludes network results, provider users, issue state, labels, reviews, timestamps, local absolute roots, and traversal order.

Property and fixture results establish behavior only for their exact inputs. They do not prove parser correctness or natural-language correspondence.

Verus can cover selected v3 encoding, ordering, closure, and impact functions. It cannot prove CommonMark, OpenSpec, URI meaning, or provider correctness.

AeneasVerif can provide a narrow translation sentinel for selected pure functions. It cannot prove Rust compilation, LLVM, native code, or shell behavior.

Human approval remains necessary for requirement meaning, scope, kind, lifecycle, and prose-to-code correspondence.

### 9. Assign one owner for the generic protocol

This change owns the v3 source-item protocol. The active Chelis change references this change as its prerequisite.

The Chelis change does not duplicate protocol tasks or evidence. Its adapter work starts only after the v3 protocol is accepted.

The identifier-newtype changes remain owners of identifier type boundaries. This change consumes their final public types.

The digest-algorithm change remains owner of algorithm selection. This change uses the active registry member when v3 implementation starts.

## Risks / Trade-offs

- **OpenSpec syntax changes** → Bind the accepted format version and reject unsupported syntax with stable diagnostics.
- **A scenario edit invalidates many links** → Report exact impact and require explicit dispositions for each affected link.
- **Custom workflow rules appear enforceable** → Report them as guidance and keep Buoy plus CI as the enforcement source.
- **External links become false evidence** → Use a separate object and exclude it from all assurance aggregation.
- **Schema v3 breaks old parsed-item data** → Require complete schema-v3 regeneration before release.
- **A source-item kind typo creates a new identity** → Parse kind IDs through the descriptor and reject undeclared kinds.
- **An OpenSpec path move changes complete identity** → Preserve the atom revision and report the source-map change separately.
- **A provider outage blocks validation** → Read only repository-owned link text and never contact the provider.
- **Chelis and OpenSpec duplicate protocol ownership** → Keep protocol tasks and evidence in this change only.

## Cutover Plan

1. Complete the identifier type changes and resolve the active digest member.
2. Add the schema-v3 inventory and negative fixtures.
3. Add `SourceItemKind` and descriptor-bound kind declarations.
4. Move the built-in Markdown and Rust adapters to v3 identities.
5. Add the OpenSpec adapter with proposed-only output first.
6. Add explicit ratification configuration and approval bindings.
7. Add external trace links, reports, closure rules, and impact edges.
8. Add the `buoy-bridge` workflow schema and project guidance.
9. Regenerate all schema-v3 fixtures and evidence.
10. Run the complete acceptance oracle before release.

Rollback requires a source rollback and complete artifact regeneration. The v3 profile does not include a v2 runtime reader.

## Open Questions

- The first community release can live in the Buoy repository or a separate schema repository. This change defers that choice.
- OpenSpec Stores require a separate saved cross-repository snapshot contract. This change supports only repository-local standing specs.
- A future provider adapter can create candidate link records. This change does not define provider authentication or state import.

## Acceptance Oracle

The exact acceptance oracle is:

```text
devenv test
```

Acceptance requires exit status `0` from the pinned offline environment. The oracle must include all new focused controls and strict OpenSpec validation.

The focused controls cover the v3 cutover, adapter totality, source spans, link freshness, root permutations, unsupported versions, and provider-state exclusion.
