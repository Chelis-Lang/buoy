## Context

Buoy stores exact implementation relations, but a user can read the detailed binding tree only when `buoy bind` creates a record. Later static human output reports graph identity and diagnostics without a current-link inventory.

The OpenSpec version 1 adapter hashes exact requirement spans and treats later level-two sections as nonnormative. The Markdown adapter alone parses `buoy-external-trace-v1` fences.

The existing `buoy-bridge` workflow creates `traceability.md` before tasks. Its template records actions and link review, but it lacks stable relation, consumer, target, and completion fields.

## Goals / Non-Goals

**Goals:**

- Provide explicit forward and reverse queries over current and historical implementation relations.
- Present stable IDs, names, logical paths, source spans, freshness, and concise human digests.
- Preserve complete identities in deterministic machine output.
- Produce implementation-owned Markdown trace pages without repository writes from Buoy.
- Parse external trace fences from safe OpenSpec nonauthority sections under an opt-in format.
- Record planned relation intent and historical completion pointers in `traceability.md`.
- Preserve the functional-core boundary and all current assurance limits.

**Non-Goals:**

- Do not put implementation records, subject digests, or requirement digests inside requirement spans.
- Do not make `buoy bind` write an authority repository.
- Do not add provider queries, editor support, a hosted trace service, or effectful oracle execution.
- Do not add a new assurance class or interpret navigation as correctness.
- Do not change `openspec/standing-spec/v1` identities, syntax, or diagnostics.
- Do not extend the external relation set with `implements`.

## Decisions

### 1. Keep query logic in the functional core

Add a typed trace selector and a pure trace query to `buoy-core`. The query receives validated graph objects, current attachment facts, retained records, and logical source references.

The core remains `no_std` and performs no file, network, JSON, Markdown, CLI, or provider operation. The root crate parses inputs, materializes mounts, maps source spans, and renders output.

The dependency direction remains root shell to `buoy-core`. The core does not depend on adapters or shell types.

**Alternatives considered:**

- Implement the query only in `src/cli.rs`: Rejected because duplicate graph traversal and freshness rules can diverge from core behavior.
- Add a separate service: Rejected because network and provider state are outside deterministic static analysis.

### 2. Add one explicit `buoy trace` command

Use this proposed command shape:

```text
buoy trace (--root ROOT | --mount ID=PATH...) --config FILE
  (--requirement-id ID | --relation-id ID | --implementation-path PATH | --record DIGEST)
  [--include-history] [--format human|machine|markdown]
```

The shell parses exactly one selector into a closed enum at the CLI boundary. Requirement and relation inputs become `StableId` values. Paths become logical `RepositoryPath` values. Records become complete `SemanticDigest` values.

The default format remains `machine`, consistent with every existing command. Human use selects `--format human` explicitly.

Format parsing occurs after command selection. Only `trace` accepts `markdown`. Existing commands retain their current `machine` and `human` set.

A path selector can return multiple items and relations. The query reports all matches instead of selecting one item by traversal order.

Conflicting or absent selectors return usage exit `64`. Invalid typed values or absent selected objects return input exit `70`. Static blockers return exit `2`.

**Alternatives considered:**

- Use one untyped positional selector: Rejected because IDs, paths, and digests can overlap and create ambiguous parse states.
- Accept abbreviated digest input: Rejected because a short value can select more than one immutable record.

### 3. Build one complete trace projection before rendering

The core returns current and historical trace rows. Each row contains the relation ID, navigation kind, record digest, source-node references, atom reference, and current-state facts.

The shell adds parser-backed display data from retained source products. Display data includes requirement title, item name, logical path, source span, and source-item kind.

The existing `buoy/binding-tree-report/v1` remains frozen for successful bind operations. One shared internal row builder supplies equal fresh-link facts to both report paths.

The new trace report supports multiple, stale, and historical rows. The bind report keeps its single fresh record contract and fixed all-current facts.

Rows use this total order:

1. Current rows before historical rows.
2. Relation ID.
3. Requirement ID.
4. Logical implementation path.
5. Implementation source-node digest.
6. Record digest.

The result binds the current graph digest. It excludes physical roots and input traversal order.

**Alternatives considered:**

- Render directly during graph traversal: Rejected because renderer order can then depend on registration or traversal order.
- Treat every retained record as current: Rejected because immutable history does not create a current attachment.

### 4. Derive three views from one projection

Machine output uses schema `buoy/trace-report/v1`. It includes complete digests, logical paths, spans, freshness facts, graph identity, and ordered rows.

Human output leads with stable names and locations. It uses a fixed abbreviated digest unless two displayed records share that prefix. A collision causes complete digest display.

Markdown output uses stable requirement headings and readable tables. It includes a derived-data warning and the navigation-only assurance boundary.

All renderers consume the same complete projection. Human and Markdown bytes do not establish canonical graph or report identity.

The command writes all views to standard output. A repository task can redirect Markdown output and validate exact regenerated bytes.

**Alternatives considered:**

- Commit a mutable alias for each relation: Rejected because the object store forbids mutable aliases.
- Put display fields in the compact Rust attribute: Rejected because duplicated labels can drift and add parser surface.

### 5. Add `openspec/standing-spec/v2` for safe same-file trace links

Version 2 preserves all version 1 requirement, scenario, step, source-tree, and authority behavior. It adds external trace fences only under level-two sections after `## Requirements`.

The adapter finds requirement boundaries before it parses trace sections. Therefore, trace bytes cannot enter requirement source-node identity or atom revision.

Both formats retain `openspec/standing-spec/v1` as the source-item schema for document, requirement, scenario, and step nodes. Equal authority bytes retain equal node and atom identities.

A version 2 format change alone does not stale an existing implementation binding. Version 2 adds external trace side products and changes the complete graph digest.

The primary requirement and nonnormative spans still partition all file bytes. A trace source mapping nests inside one nonnormative span as a secondary mapping.

Move the strict external trace document parser to a shared adapter helper. The Markdown adapter and OpenSpec version 2 adapter call that helper and produce equal link values.

Extend `OpenSpecOutput` with external links. The OpenSpec shell adapter adds these links and their source mappings to the materialized graph product.

A recognized trace fence that intersects a requirement span fails with a stable diagnostic. Unknown prose and ordinary links in later sections remain nonnormative.

Version 1 continues to classify the same later fence as nonnormative. Existing version 1 fixtures and identities remain frozen.

**Alternatives considered:**

- Add trace parsing to version 1: Rejected because existing nonnormative bytes can gain new graph meaning without an explicit migration.
- Exclude inline trace metadata from requirement bytes: Rejected because hidden subtraction rules make authority boundaries hard to review.

### 6. Use external `documents` links for standing-spec navigation

A standing spec can link its atom revision to an implementation-owned trace page with `relation = "documents"`. The actual `implements` relation remains in the immutable binding record.

The target uses an absolute HTTPS URI or versioned URN under the current closed target rules. Buoy never fetches the target.

A requirement change makes both the implementation binding and the external documents link stale. Updating the outside trace fence cannot change the atom revision.

An implementation-only change requires a new binding record and trace-page refresh. The standing-spec target remains stable, so the authority repository needs no update.

**Alternatives considered:**

- Put `implements` in the external relation set: Rejected because a web target is not an exact implementation source node.
- Store the complete binding TOML in the standing spec: Rejected because it creates circular requirement identity and duplicate authority.

### 7. Expand the `buoy-bridge` traceability artifact

Add planned implementation columns for relation ID, consumer repository, logical target selector, action, owner, and focused command. Planned rows contain no fabricated current digest.

Add a completion snapshot with the canonical record locator, consumer commit, and final static result. The template labels this data as historical.

The snapshot points to the immutable record and does not copy its body. Current truth remains the result of a new Buoy query over saved inputs.

OpenSpec templates and artifact presence remain advisory. Repository controls validate template shape, but Buoy validates all current links.

Downstream stores can adopt the project schema separately. This repository change does not modify another repository or its default workflow.

**Alternatives considered:**

- Put completion data in delta requirement text: Rejected because sync can copy it into current authority spans.
- Treat archived completion rows as current: Rejected because later source changes can make the recorded relation stale.

### 8. Preserve explicit lifecycle transitions

A new relation follows this sequence:

1. `traceability.md` records stable intent without current digests.
2. `buoy bind` creates one immutable record and current implementation locator.
3. `buoy trace` produces the current human, machine, or Markdown view.
4. The completion snapshot records the object locator and consumer commit.
5. Later source edits make the affected side stale until an explicit bind operation creates a replacement.

Old immutable records remain historical. No query, projection, template, or external documents link makes an old record current.

### 9. Preserve deterministic inputs and trust boundaries

Authoritative inputs are saved selected bytes, repository configuration, schema versions, active digest member, and explicit mount mappings. Canonical identity excludes physical roots and provider state.

Static registration, source freshness, effectful verdicts, and assurance classes remain separate. The trace command performs static analysis only and never invokes `execute`.

The command emits a projection after successful materialization. Exit `2` applies to a selected stale current attachment or a graph blocker that prevents safe selection.

Historical rows never change a successful exit class because they create no current attachment.

A fresh trace supplies navigation and impact data only. It does not establish coverage, execution, proof, approval, correctness, or prose-to-code correspondence.

This change adds no Verus or AeneasVerif model. Existing formal results do not cover the new shell, adapter, query, or renderer behavior.

Tests, fixtures, finite controls, and differential render checks provide scoped evidence only. They do not prove total correctness or natural-language correspondence.

### 10. Use one exact acceptance oracle

The exact acceptance command is:

```text
devenv test
```

Acceptance requires exit code `0`. The command must include focused trace-query, renderer, adapter-version, workflow-template, fixture, and negative-control tasks in its dependency graph.

## Risks / Trade-offs

- A new standing-spec version increases fixture and migration work. Version 1 remains frozen to contain this risk.
- Display names add parser-owned presentation data. Machine rows retain exact source identities when names change.
- Large object stores can make historical queries expensive. Explicit history selection and deterministic limits contain this cost.
- Generated Markdown can appear authoritative. Every projection includes a derived-data warning and assurance boundary.
- External targets can become unavailable. Buoy stores target text only and makes no availability claim.
- A shared-store adoption can require separate review. This change ships the schema contract and examples without cross-repository writes.
