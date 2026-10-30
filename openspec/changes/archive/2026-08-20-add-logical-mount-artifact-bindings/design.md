## Context

Buoy currently opens one physical root. Selector roots are canonical repository paths below that descriptor.

A separate OpenSpec repository cannot enter the same snapshot without copies, links, submodules, or a broader common directory. Each option weakens identity or path controls.

Rust attributes supply current compact binding attachments. Opaque files need a language-neutral source node and an out-of-line current attachment.

## Goals / Non-Goals

**Goals:**

- Build one deterministic snapshot from explicit physical roots and canonical logical prefixes.
- Preserve no-follow descriptor reads and global resource budgets.
- Keep physical paths outside all canonical identities and machine output.
- Materialize opaque files without language interpretation.
- Attach immutable records to opaque subjects through compact repository sidecars.
- Preserve the separation between static registration, freshness, execution, and assurance.
- Provide an exact migration path for existing single-root configuration.

**Non-Goals:**

- Fetch or update source repositories.
- Follow links, submodules, nested repositories, or provider references.
- Federate separately accepted graphs.
- Parse Nix or infer semantics from opaque bytes.
- Treat bindings, tests, annotations, or sidecars as proof.
- Change Barnacle in this repository change.

## Decisions

### 1. Add a versioned multi-mount configuration

New configurations use `schema = "buoy/repository-config/v3"`. Each `[[mounts]]` row contains a stable mount ID and one canonical logical prefix.

Each selector names one mount ID and one root below that mount. The selected canonical path is `<prefix>/<selector-root>/<relative-path>`.

The CLI accepts repeated `--mount ID=PATH` inputs. It parses each input into a validated mount ID and a physical `PathBuf` at the boundary.

The physical path never enters the configuration digest. The digest includes the sorted mount declarations, selectors, budgets, adapters, and oracle declarations.

Version 2 configurations retain the existing `--root` path and exact canonical identities. Version 3 rejects `--root`, and version 2 rejects `--mount`.

**Alternatives considered:**

- Put absolute roots in TOML. Rejected because host paths change configuration identity.
- Use one common parent root. Rejected because selection can cross unrelated repositories.
- Copy external sources before every check. Rejected because the copy can drift from either source repository.

### 2. Build one snapshot instead of graph federation

The shell sorts mount declarations by canonical mount ID before it opens directories. It validates the complete physical mapping before selection.

Each mount receives an independent no-follow root descriptor. All child descent and file reads remain relative to that descriptor.

Logical path projection occurs before insertion into the selected-file map. One `BTreeMap<RepositoryPath, SnapshotFile>` detects collisions across mounts.

All selected files then enter the existing adapter and repository aggregation sequence. No mount receives an independent acceptance verdict.

Global counters cover directory entries, selected files, bytes, graph objects, references, diagnostics, depth, and visited nodes. Mount count does not multiply a limit.

Validation uses this fixed precedence:

1. Parse configuration and CLI inputs.
2. Validate mount completeness, uniqueness, and prefix disjointness.
3. Open mount descriptors in canonical mount order.
4. Select files and project logical paths.
5. Materialize adapters in canonical logical path order.
6. Aggregate source products and resolve all references.
7. Run static checks.
8. Run configured oracles only after static acceptance.

**Alternatives considered:**

- Validate one graph per mount and join accepted results. Rejected because partial acceptance can hide unresolved cross-mount references.
- Follow symbolic links into the authority store. Rejected because links weaken descriptor and race controls.
- Use Git submodules. Rejected because repository metadata and checkout state become ambient inputs.

### 3. Add one fixed opaque artifact adapter

The built-in `artifact/v1` adapter uses no language parser. Its descriptor declares `artifact.opaque-bytes/v1` and `artifact/file/v1`.

For each selected file, the adapter creates one `ParsedItemIdentity`. Its payload binds `application/octet-stream`, `saved-bytes/v1`, and the exact saved bytes.

The source node uses the projected logical path and one complete byte span. Invalid UTF-8 is valid input because offsets count bytes.

The adapter emits no authority, atom, implementation link, oracle, coverage fact, or assurance fact. Repository configuration permits only the forbidden authority expectation.

The shell owns file selection and bounded reads. The adapter remains a deterministic function of logical path, saved bytes, explicit configuration, and active digest member.

**Alternatives considered:**

- Add a Nix parser first. Rejected because the Barnacle need is exact artifact identity, not Nix semantics.
- Infer media type from extensions. Rejected because extension policy adds interpretation without value for opaque identity.
- Reuse raw artifact indexes without source nodes. Rejected because implementation bindings require current graph subjects.

### 4. Add a mutable attachment sidecar with immutable targets

A sidecar selects the current immutable record for one stable relation. The immutable object store remains append-only.

The mount-relative path is `buoy/attachments/v1/<algorithm>/<key-payload>.toml`. The key digest uses the active member and domain `buoy/binding-attachment-key/v1` over the relation ID.

The selected artifact mount owns the immutable object and the sidecar. Physical writes use `<mount-root>/buoy/...`. Canonical paths use `<mount-prefix>/buoy/...`.

The binding adapter interprets object and sidecar path classes relative to that mount prefix. Thus, `--artifact` selects the write mount without another host path.

The sidecar schema is `buoy/binding-attachment/v1`. Canonical fields contain the relation ID, logical artifact path, subject digest, and complete binding-record digest.

The `binding/v1` adapter parses object records and sidecars from distinct path classes. A valid sidecar produces one existing `BindingAttachment` after exact cross-field checks.

The record relation, record subject, sidecar relation, sidecar subject, and current artifact node must agree. The requirement and atom checks remain unchanged.

Old object records stay in `buoy/objects/v1`. They remain historical unless a current inline attribute or sidecar selects them.

Sidecars provide navigation and freshness only. They do not provide execution, coverage, approval, assurance, proof, authenticity, or implementation correctness.

**Alternatives considered:**

- Treat every stored record as current. Rejected because preserved history creates stale active bindings.
- Put full provenance in a mutable sidecar. Rejected because mutable data replaces the immutable record authority.
- Add a comment to each opaque artifact. Rejected because Buoy cannot edit unsupported language syntax safely.

### 5. Extend `buoy bind` with an artifact transaction

`--artifact LOGICAL_PATH` and `--implementation DIGEST` are exclusive subject selectors. The artifact path must resolve to one current `artifact/file/v1` node.

The command constructs the unchanged `ImplementationBinding` core object. It then writes the object and replaces one relation-keyed sidecar.

The artifact logical path selects its physical mount. The command does not write the object or sidecar into another declared mount.

The transaction reads and validates all inputs before its first write. It stages temporary files beside final destinations and synchronizes them before replacement.

A failure removes only the new temporary or uncommitted object. It restores the prior sidecar bytes and leaves the selected artifact unchanged.

The pure transaction plan is separate from filesystem effects. A bounded finite model covers prior object existence, sidecar state, each one-shot failure point, rollback, and successful cleanup retry.

Persistent filesystem refusal is outside this finite model. The effectful transaction reports a failure if its bounded cleanup retries fail.

**Alternatives considered:**

- Reuse the Rust attribute editor. Rejected because unsupported artifact syntax has no safe edit operation.
- Refresh every stale artifact binding. Rejected because updates require explicit reviewed selectors.

### 6. Keep the functional core effect free

`buoy-core` remains `no_std` and has no filesystem, process, parser, or host-path types. The root package continues to depend inward on the core.

If shared canonical validation requires them, the core gains only pure attachment key and sidecar value types. The shell owns TOML, CLI, paths, descriptors, and transactions.

No core crate dependency points toward the root package or an adapter. Existing bare-metal and Dylint controls remain mandatory.

### 7. Define deterministic and formal assurance limits

Hegel properties cover mount permutations, logical projection, collision rejection, opaque byte identity, sidecar key identity, and transaction-plan invariants.

A bounded finite model covers at most three mounts, four selected files, two relations, and every transaction failure transition. Exhaustiveness applies only to this model.

A selected Verus proof covers injective logical path projection under unique disjoint prefixes. It binds the exact model, implementation representation, assumptions, tool, and configuration.

AeneasVerif is not selected for this change. No new arithmetic or lifecycle theorem needs a second representation.

The Verus result excludes parser correctness, filesystem behavior, operating-system descriptors, hash collision resistance, TOML, CLI parsing, and prose correspondence.

Tests, fixtures, mutation controls, Hegel samples, and exact coverage are evidence for their named scopes. None is mathematical proof.

### 8. Preserve explicit trust boundaries

Buoy trusts the caller to map each logical mount ID to the intended physical directory. Buoy verifies selected bytes, not repository ownership or origin.

Descriptor operations trust the operating system and Rust platform libraries. Atomic replacement trusts documented filesystem semantics for one directory.

Digest equality supports identity under the registered algorithm assumptions. It does not establish authenticity, authorization, or collision impossibility.

Opaque artifact identity establishes exact saved-byte correspondence only. It does not establish language meaning, build use, runtime behavior, or correctness.

### 9. Use one exact final acceptance oracle

The exact final acceptance oracle is `devenv test` from the repository root. Success means exit status zero for every task in its declared graph.

Before that oracle, `devenv tasks run buoy:coverage` must report exactly 100% lines, regions, functions, and branches for the governed source inventory.

Strict OpenSpec validation, frozen goldens, finite-model replay, selected Verus checks, and read-only evidence checks must pass before final acceptance.

## Risks / Trade-offs

- Multiple root descriptors expand the effect surface and platform test matrix.
- Logical prefixes intentionally change source identities for version 3 configurations.
- A mutable sidecar is a current-selection pointer and needs stronger transaction controls.
- If each root runs alone, the root can pass while the complete mount set exceeds global budgets.
- The configuration migration increases fixture, documentation, and compatibility work.
- Opaque byte identity enables unsupported languages but supplies no semantic item precision.
- A caller must pin and materialize every physical input for reproducible cross-repository checks.
