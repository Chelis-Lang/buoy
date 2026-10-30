## Context

Buoy supplies deterministic graph, binding, external-link, and trace-projection semantics. It does not own tracked page writes, external-link updates, or consumer packaging.

The Barnacle pilot filled these gaps with three consumer-owned shell tools. The audit reproduced executable substitution, parent-symlink output escape, and nondeterministic parity diagnostics.

The pilot also exposed two lifecycle defects. Recursive selectors can exceed a declared one-file scope, and an exact atom can target one mutable branch page.

The new behavior belongs in Buoy's imperative shell. Consumer repositories retain policy, source, generated outputs, and human decisions.

## Goals / Non-Goals

**Goals:**

- Give consumers one typed publication-plan schema and one Buoy-owned command family.
- Keep trace queries read-only and isolate every local write behind an explicit generation operation.
- Derive selected snapshots from parsed Buoy selectors and logical mounts.
- Check exact row-local trace facts without text searches.
- Generate cycle-free immutable trace pages and current aliases with distinct lifecycles.
- Synchronize OpenSpec version 2 nonauthority links and preserve authority identities.
- Generate historical completion evidence from one retained snapshot.
- Expose a Buoy-owned Nix package interface for pinned private consumers.
- Preserve every current assurance boundary and published machine-report meaning.

**Non-Goals:**

- Network publication or external-target availability checks.
- Automatic Git commits, pushes, pull requests, merges, reviews, or approvals.
- Consumer-specific IDs, paths, repositories, task names, or policy in Buoy code.
- A multi-repository atomic transaction claim.
- A new graph object, authority source, execution receipt, or assurance class.
- A change to `buoy/trace-report/v1` or `buoy/binding-tree-report/v1` meanings.
- Proof of filesystem, Nix, Git, provider, or natural-language behavior.

## Decisions

### 1. Publication remains in the imperative shell

The new plan parser, command handlers, source guards, writers, diff renderer, package identity, and evidence renderer live in the root shell crate.

These modules depend inward on `buoy-core` values and algorithms. `buoy-core` remains `no_std` and never receives paths, file handles, processes, providers, or clocks.

The publication conformance logic is pure for typed plans, snapshots, and trace projections. Filesystem reads and writes remain thin shell adapters around that logic.

No public core interface receives a closure, function pointer, filesystem type, or process type. The crate dependency remains one-way toward the core.

**Alternatives considered:**

- Put publication tools in each consumer. Rejected because consumers then duplicate Buoy schemas, parsing, selection, and write safety.
- Put filesystem publication in `buoy-core`. Rejected because this breaks the functional-core boundary and `no_std` constraint.
- Invoke `buoy trace` as a child process. Rejected because executable substitution and report reparsing create a second trust boundary.

### 2. One versioned plan carries stable consumer policy

The initial plan schema is `buoy/trace-publication-plan/v1`. Parsing produces a closed `TracePublicationPlan` before snapshot construction.

A plan contains these stable values:

- one publication ID,
- one typed trace selector,
- one exact expected row count,
- one source-set conformance policy,
- accepted stable relation IDs,
- expected logical source locations,
- one or more declared outputs,
- one owner mount for each output,
- one output-path template,
- one artifact kind with a fixed `derived` or `source-edit` effect class,
- one `immutable` or `replaceable` lifecycle,
- one normalized absolute page-target base when applicable,
- one external-link ID and `documents` relation when applicable,
- one declared target-page reference for each external link.

Version 1 requires exactly one external-link reference for each immutable target page. A replaceable current alias can remain unreferenced.

The plan does not contain proposed atom, node, graph, page, link, or record digests. Current operations derive those identities from selected bytes.

The plan is explicit operational input. It does not enter the source graph unless a consumer separately selects it as ordinary source.

Unknown fields and duplicate keys fail at the parse boundary. Downstream code trusts the parsed plan and does not repeat raw-input checks.

**Alternatives considered:**

- Put volatile digests in the plan. Rejected because each valid source edit creates manual synchronization and premature identity claims.
- Derive policy from README prose or task names. Rejected because prose and orchestration are not typed publication inputs.
- Extend `buoy.toml` schema v3 directly. Rejected because publication policy has a separate lifecycle and does not define source selection.

### 3. One command family has explicit effect classes

The proposed CLI namespace is:

```text
buoy publication check --plan PLAN --artifact ID [repository and mount options]
buoy publication generate --plan PLAN --artifact ID [repository and mount options]
buoy publication evidence --plan PLAN [repository and mount options]
```

`check` and `evidence` are read-only. `generate` writes exactly one declared local artifact.

Check and generate emit `buoy/trace-publication-report/v1`. Evidence emits `buoy/trace-publication-evidence/v1`.

Machine output remains the command contract. A Markdown completion artifact requires an explicit `generate` operation.

The command calls the in-process snapshot, adapter, validation, trace-selection, and rendering libraries. It ignores executable-selection environment variables.

`buoy trace` keeps its existing command contract. It remains effect-free and writes only to standard output.

Conflicting selector forms and other usage faults return `64`.

Malformed or unmatched typed selectors, plans, roots, path templates, or target bases return `70`.

Current static blockers and publication drift return `2`.

**Alternatives considered:**

- Add `--output` to `buoy trace`. Rejected because this weakens its established read-only contract.
- Let `check` repair drift. Rejected because a read-only operation must never convert an unexpected identity into accepted state.
- Generate all outputs in one call. Rejected because a failure can create a partial multi-root transaction and unclear ownership.

### 4. Each operation uses one retained selected snapshot

The command parses repository configuration and explicit mount inputs. It ignores ambient repository-root environment variables.

It opens all logical mounts through existing no-follow root descriptors.

Selector evaluation determines the complete selected set. A consumer copy list never narrows or expands that set.

After selection, the shell compares all logical paths with the plan's source-set policy. An exact policy detects recursive-selector expansion and returns exit `2`.

The shell reads every selected file once into a retained `RepositorySnapshot`. Static validation and trace projection use only those retained bytes.

Before any derived-artifact operation, each declared source edit must equal its candidate in that saved snapshot.

An absent or stale source edit returns exit `2`. The maintainer must generate that source edit and start a new snapshot operation.

Before replacement, a source guard reopens each selected path through no-follow descent. It compares the current object and bytes with the retained snapshot.

A changed, replaced, removed, or link-like selected path aborts the write. The candidate still reports the retained snapshot identity but never reaches the output.

Physical roots, discovery order, metadata times, locale, and caller directory remain noncanonical.

**Alternatives considered:**

- Copy a fixed downstream file list. Rejected because it can diverge from recursive or changed selectors.
- Require a clean Git worktree. Rejected as the only guard because candidate worktrees are valid inputs and Git is not canonical source authority.
- Read source files again during each format render. Rejected because concurrent edits can produce mixed-snapshot outputs.

### 5. Publication conformance uses typed rows

The shell exposes a closed typed trace-report representation to the publication module. Machine JSON serialization remains a presentation boundary.

Conformance first checks exact row count. It then checks each complete row against stable plan policy.

The following facts remain row-local:

- relation ID,
- record digest,
- requirement ID,
- semantic atom revision,
- requirement node,
- implementation node,
- both logical paths,
- state,
- all freshness facets,
- row diagnostics.

A second row cannot supply a missing path, state, or freshness value for the selected row.

Selected stale rows retain their existing diagnostic and exit `2`.

External-link operations use a pre-link view with one declared publication slot.

A present slot is one standalone level-two section keyed by link ID.

The fixed boundary follows the final normative requirement and precedes every other nonauthority section.

The slot source map owns its canonical leading and trailing separators.

Normalization removes a present slot and owned separators, then inserts marker bytes at that boundary.

The absent form inserts equal marker bytes there. Normalization omits only the declared link's diagnostics.

All trace facts still derive from retained saved bytes. Marker bytes enter only the operational basis encoder.

All binding, authority, requirement, implementation, unrelated-link, and unrelated-section diagnostics remain.

Derived-artifact operations use the complete saved snapshot and require all declared source edits to conform.

The publication module also round-trips machine values through strict schema tests. Production conformance does not search serialized text.

**Alternatives considered:**

- Search JSON or Markdown fragments. Rejected because unrelated rows can collectively satisfy fragment checks.
- Pin every current digest in stable policy. Rejected because explicit generation and code review already expose current identity changes.

### 6. Page lifecycle separates immutable history from a current alias

A publication page derives from renderer schema `buoy/readable-trace-markdown/v1`.

RTP-007 adds complete publication identities and labels its graph as the source graph. A byte-affecting renderer change requires a new schema version.

An exact external link targets an `immutable` page. Its rendered path contains the complete publication-basis path encoding.

For an immutable page, generation follows this state table:

| Existing state | Candidate state | Result |
|---|---|---|
| absent | valid | create atomically |
| equal | valid | successful no-op |
| unequal | valid | fail without replacement |
| any | invalid | fail without replacement |

A consumer can declare a separate `replaceable` current alias. The exact external link never targets that alias.

Page check compares exact bytes and never writes. Page generation runs conformance before any temporary-file creation.

The page-byte digest uses a versioned publication-content domain. It identifies exact derived bytes but creates no graph object or assurance.

**Alternatives considered:**

- Use one mutable `main` page for exact atom links. Rejected because old links later reach new page content.
- Use only an atom or record key. Rejected because locations, selected bytes, or renderer changes can alter page bytes without changing that key.
- Use the page commit, graph digest, or page digest in the target. Rejected because each value depends on the link or page.
- Overwrite a basis-keyed page. Rejected because the target then provides no durable historical navigation.

### 7. A pre-link publication basis breaks the identity cycle

Before link construction, Buoy encodes one `buoy/trace-publication-basis/v1` value under the active digest member.

Immutable publication requires a member with the `collision-resistant` assurance class. Change-detection members remain available for other Buoy operations.

The basis identifies routing inputs only. It creates no graph object, authenticity, correctness, or proof claim.

The basis binds these complete pre-link inputs:

- publication-plan schema and parsed semantic fields,
- Markdown renderer schema,
- active digest member,
- typed selector and exact trace facts,
- logical locations and source spans,
- canonically ordered selected paths and bytes.

For basis encoding, Buoy normalizes the complete publication slot to one marker keyed by stable link ID.

The normalized snapshot is byte-identical before first slot creation and after any canonical slot replacement when all other inputs are equal.

Parsed template literals enter the basis before substitution. The rendered basis value never enters its own input.

This normalization excludes the generated slot scaffolding, target, and link. It retains every other selected byte.

A page-affecting pre-link input therefore changes the basis.

The plan parser compiles each page-path template into literal segments and registered substitutions.

A page target base is an absolute normalized `https` URI whose path ends with `/`.

It contains no substitutions, query, or fragment.

The versioned join percent-encodes normalized UTF-8 path segments, preserves slash boundaries, and rejects encoded traversal.

An external-link output references one immutable page artifact and reuses that joined target. It cannot copy or override the target.

Version 1 accepts only `documents` for publication-managed external links. Other external-link relations remain ordinary read-only graph inputs.

Version 1 supports these dynamic substitutions:

```text
{atom_revision}
{atom_revision_path}
{record_digest}
{record_digest_path}
{publication_basis}
{publication_basis_path}
```

Each `_path` form encodes `<algorithm>/<lowercase-payload>`. The selected trace must provide one unambiguous value for every substitution.

An immutable page path and exact external target must contain `{publication_basis_path}`. Atom and record substitutions can add stable grouping.

The dependency order is:

```text
normalized selected bytes + plan + renderer + trace facts
                         |
                         v
                publication basis
                         |
                         v
               rendered page path
                         |
                         v
             target-base URI join
                         |
                         v
          external link -> source graph -> trace page
```

Graph, link, page, commit, provider, clock, and physical-path values are not registered substitutions. Unknown substitutions fail plan parsing.

The joined URI still passes ETL-002 normalization and retains the complete rendered page path.

A derived local path still passes repository-path and output-exclusion checks.

A source-edit path must select exactly one regular source file. Its semantic adapter guard permits only the declared nonauthority edit.

### 8. External-link generation uses adapter source maps

The OpenSpec adapter retains the final normative boundary, each link fence span, and its enclosing level-two section span.

Generation follows this sequence:

1. Parse the retained standing specification and resolve the selected current atom.
2. Find zero or one publication slot with the declared stable link ID.
3. Remove any present slot and insert canonical marker bytes at the fixed boundary.
4. Run pre-link conformance without only that link's diagnostics.
5. Compute the publication basis from the normalized saved snapshot.
6. Render the basis-keyed page path and join it with the page target base.
7. Render one canonical level-two heading and `buoy-external-trace-v1` fence.
8. Replace the complete old slot or insert the complete new slot at the fixed boundary.
9. Materialize the candidate bytes through the normal OpenSpec version 2 adapter.
10. Compare every normative span, requirement node, atom ID, and atom revision before and after.
11. Verify that candidate slot normalization reproduces the pre-link basis input.
12. Validate the complete candidate graph and external-link closure.
13. Run the source guard and safe writer.

A later page or evidence operation starts a new snapshot. It requires these exact candidate source bytes before full graph and trace rendering.

Malformed duplicate links fail before edit planning. A stale old link does not prevent derivation of the current atom from its normative source.

A legacy link can migrate only when its enclosing level-two section contains no other nonwhitespace content.

A mixed-content legacy section fails without a write because Buoy cannot claim ownership of the other bytes.

Generation changes no active or archived OpenSpec change document. It operates only on the declared standing specification.

**Alternatives considered:**

- Use a text anchor supplied by the consumer. Rejected because prose edits and duplicate anchors do not identify authority boundaries.
- Let OpenSpec generate Buoy link syntax. Rejected because Buoy owns the link parser, atom semantics, and closure rules.

### 9. Output writes reuse no-follow repository primitives

The writer opens the declared owner mount independently. It descends each parent component with no-follow operations.

The writer rejects link-like ancestors, root escape, unsupported object types, and component replacement.

A derived output must remain outside every selected source set. A source edit must select exactly one regular source file.

A unique sibling candidate uses exclusive creation. The writer writes complete bytes, applies declared regular-file permissions, and performs required synchronization.

Replacement remains descriptor-relative. Immutable outputs use create-if-absent semantics. Replaceable outputs use one same-directory atomic replacement.

`immutable` describes this local Buoy write policy. It does not establish Git history or provider storage behavior.

A normalized output-path lock serializes concurrent generation. Lock wait and timeout values come from bounded shell policy, not graph identity.

Cleanup removes only candidates owned by the current operation. A fixed-label diff supplies optional readable detail without timestamps or physical paths.

**Alternatives considered:**

- Use `test -L`, `mktemp`, and `mv` with path strings. Rejected because parent links and component swaps can redirect output.
- Use a global publication lock. Rejected because unrelated output paths can proceed independently.

### 10. Evidence separates generated identities from human decisions

The evidence renderer consumes the typed publication report and retained trace facts. It does not re-read repositories.

Machine evidence uses `buoy/trace-publication-evidence/v1`. It contains status, diagnostics, and an optional completion payload.

The operation first requires every non-evidence artifact to equal its candidate. Drift returns exit `2` without a completion payload.

A declared Markdown artifact is a deterministic projection of the successful typed completion value.

Generated evidence includes:

- publication and selector identities,
- publication-basis digest,
- source and static graph digests,
- complete current trace rows,
- link identity and target,
- page content digest,
- logical locations,
- freshness facets,
- canonical diagnostics.

It excludes record bodies, provider status, commits, users, reviews, approvals, clocks, and physical paths.

Freshness fields record one static result. The evidence, page, link, and digest do not establish later freshness or any assurance claim.

OpenSpec traceability artifacts retain human owners, commits, reviews, and task completion separately. A generated section cannot supply those facts.

### 11. Buoy owns its Nix consumer package

Buoy adds `nix/consumer-package-v1.nix`. The function builds the locked workspace executable from pinned Buoy source.

It requires source, source revision, source-content digest, Rust toolchain, system, and one digest-feature set.

Missing source identity fails evaluation. The file path and function fields remain stable for interface version 1.

The initial package variants correspond to reviewed production members:

- `xxh3-128`,
- `blake3-256`,
- `sha-256`.

Each variant disables default features and enables only its declared member. Consumers select one variant through pinned Nix or Devenv input data.

The executable embeds operational package identity supplied by the derivation. Read-only `buoy package-identity` emits `buoy/package-identity/v1`.

Private consumers can pin a non-flake `git+ssh` source input and call the Buoy-owned package function.

Their lock stores revision and content identity. Acquisition uses noninteractive read-only SSH and no repository-stored credential.

The blocking internal `buoy:package-contract` task realizes local-source variants offline from the declared closure.

It uses empty user build state and has a required path to `devenv:enterTest`.

The downstream `buoy:consumer-package` task remains advisory. Its private SSH mode uses temporary credentials and never enters the final task graph.

After realization, the executable needs no Git checkout, Cargo registry, SSH command, or consumer wrapper.

**Alternatives considered:**

- Keep a consumer-owned clone-and-build script. Rejected because it duplicates package logic and introduces cache races and executable substitution.
- Publish only one default-feature package. Rejected because repository digest configuration must agree with the executable feature closure.
- Embed package identity in graph reports. Rejected because package provenance is operational and must not alter semantic graph identity.

### 12. Test-first validation covers distinct failure mechanisms

Fixtures freeze plan parsing, row conformance, output lifecycles, link edits, path templates, URI joins, reports, evidence, and package identity.

Hegel-rs properties cover permutation, root, locale, and plan-field independence. They also cover exact row locality and deterministic diagnostics.

The bounded model `publication-transaction/v1` covers parse, snapshot, conformance, candidate, source-guard, write, failure, and cleanup states.

The bounded model `publication-dependency/v1` covers allowed pre-link identities and rejects graph, page, link, and commit cycles.

Adversarial tests include:

- substituted executables,
- parent and final symlinks,
- component swaps,
- stale source guards,
- concurrent writers,
- interrupted writes,
- immutable-output conflicts,
- fragments split across rows,
- recursive-selector expansion,
- stale and malformed external links,
- provider and network variation,
- different physical roots,
- isolated package realization from the declared closure,
- empty user Cargo and Rustup homes,
- advisory private-source acquisition.

Mutation tests target every branch that prevents a write or cycle. Fuzzing covers plan, path-template, URI-base, and evidence parsers under explicit bounds.

Publication filesystem code remains effectful shell code. Verus and AeneasVerif cannot prove operating-system atomicity, Nix reproducibility, Git provenance, or provider behavior.

No new formal claim is required unless implementation changes an existing verified core function. Any affected proof binding must retain its exact assumptions and exclusions.

### 13. Migration is additive and consumer-controlled

Buoy first ships the package, plan schema, command family, tests, and migration documentation.

A consumer then adds a publication plan and pinned package input. All downstream Buoy tasks remain explicit and outside automatic acceptance.

The maintainer first generates the external-link source edit in the authority worktree. Page generation then reads those saved candidate bytes from a new snapshot.

The maintainer reviews and integrates each repository separately. The immutable page reaches its repository before the external link reaches authority.

After equal behavior, the consumer removes local fetch, page-manager, report-checker, and link-synchronizer tools in a separately reviewed change.

Generated current aliases remain optional. Existing constant targets remain valid ordinary links but fail publication-managed basis-key policy.

Rollback removes the new plan and command use. Existing trace, binding, page, and link bytes remain readable through their published schemas.

The exact final acceptance oracle is `devenv test` in the pinned Buoy worktree.

Success requires all local tests, models, offline package checks, hardening-manifest checks, and strict OpenSpec checks to pass.

The advisory private-source task and all downstream consumer tasks remain outside this oracle.

## Risks / Trade-offs

- **The command family becomes large.** One plan and shared typed pipeline limit duplicated operation logic.
- **External-link generation edits authority files.** Source-map comparison and one-artifact writes constrain the edit to nonauthority bytes.
- **Immutable pages increase repository size.** Consumers retain durable navigation only for explicitly generated revisions.
- **The package adds Nix maintenance.** Buoy ownership removes repeated and less controlled consumer wrappers.
- **Source guarding adds a second selected-file pass.** The pass blocks output replacement after live source drift.
- **A consumer can keep legacy scripts.** Migration guidance marks them nonconforming but does not edit another repository.
- **No cross-repository atomic commit exists.** Basis-keyed immutable targets permit page-first integration and preserve old target content.
- **Formal tools cover only pure bounded models.** Filesystem, package, and provider guarantees remain test and review obligations.
