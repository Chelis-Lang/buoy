# Trace publication tooling

Status: implemented schema v3 shell command family.

## Overview

Buoy owns local trace publication: writing tracked pages, synchronizing one
OpenSpec nonauthority external-link slot, checking row-local conformance, and
rendering historical completion evidence. `buoy trace` remains read-only.
Every write happens through an explicit `buoy publication` operation.

Every publication module (`src/publication.rs`, `src/publication_fs.rs`,
`src/publication_models.rs`, `src/external_link_generation.rs`) lives in the
effectful `buoy` shell crate. None changes an exported `buoy-core` API or
adds a `no_std` item, so this work opens no new formal (Verus/AeneasVerif)
proof lane and requires no proof-impact spike.

## Bounded reference models

`src/publication_models.rs` defines two pure, finite reference models,
checked independently of the effectful pipeline:

- `publication-transaction/v1` (`TransactionState`): parse, snapshot,
  conformance, candidate, source-guard, write, failure, cancellation,
  cleanup, and committed. Exhaustive tests prove every state has a bounded
  path to a terminal state, `Write` is reachable only through
  `SourceGuard`, `Committed` is reachable only from `Write`, and
  failure/cancellation transition only to `Cleanup`.
- `publication-dependency/v1` (`DependencyKind`): the ten identity kinds a
  rendered path or target can reference. Exactly the three pre-link kinds
  (publication basis, atom revision, record digest) are accepted; every
  other kind (graph, page, link, page-commit, branch-head, provider,
  physical path) is rejected as cycle-prone. A differential test
  (`tests/publication_models_differential.rs`) proves this bounded
  classification agrees with `publication::render_path`'s real
  substitution acceptance for every kind.

## Plan schema

Schema `buoy/trace-publication-plan/v1` (`src/publication.rs::parse_plan`).

A plan declares:

- `schema` — fixed `buoy/trace-publication-plan/v1`.
- `publication_id` — one nonempty stable string.
- `selector` — exactly one of `requirement_id`, `relation_id`,
  `implementation_path`, or `record`.
- `expected_rows` — one nonzero exact row count.
- `source_set` — the exact declared logical source-set policy. Every entry
  must be a nonempty, non-duplicated logical path string. A declared empty
  array is exact equality with the empty resolved selection, never a
  bypass: it still fails closed against any nonempty real selection (see
  `enforce_source_set_policy`).
- `outputs` — one or more declared artifacts, each with `id`, `mount`,
  `path`, `kind` (`page`, `external-link`, `evidence`, `source-edit`),
  `lifecycle` (`immutable` or `replaceable`), and, for external links, one
  `relation` (`documents` only in version 1) and one `page` reference. A
  page referenced by at least one external link must declare
  `target_base` at parse time; a missing `target_base` for a referenced
  page fails plan parsing rather than failing late at `generate` time.

Unknown fields, duplicate object keys anywhere in the document, duplicate
output IDs, and an orphan immutable page (an `immutable` page with no
referencing `external-link` output) all fail parsing before any repository
read. `outputs[].mount` is validated against the loaded repository
configuration's declared logical mounts once repository facts are
available: an undeclared mount identity is rejected in mounted mode, and a
single-root plan must declare one consistent mount label across every
output (single-root repositories have exactly one physical location, so
naming more than one distinct mount label there is rejected as ambiguous
authority/output ownership).

Every declared output's rendered path is compared for source-set
membership, output classification, and cross-output collision in the SAME
logical namespace the retained snapshot's own selected paths use: the
mount-prefixed projected path in mounted mode (`mount` joined with the
rendered mount-relative `path`), never the bare mount-relative string
alone. This is what lets a plan output that collides with a selected
source only once its declared mount is taken into account be detected
correctly (see `tests/publication_mounted_containment.rs`). Physical
descent for the actual write always uses the separate, un-prefixed
mount-relative path against the physical root that declared mount already
resolves to, so the mount prefix is applied exactly once, only for the
logical comparison, and never doubled onto the physical write path.

## Command family

```text
buoy publication check --plan FILE --artifact ID [--config FILE] [--root ROOT | --mount ID=PATH...]
buoy publication generate --plan FILE --artifact ID [--config FILE] [--root ROOT | --mount ID=PATH...]
buoy publication evidence --plan FILE [--config FILE] [--root ROOT | --mount ID=PATH...]
```

`check` and `evidence` are read-only. `generate` writes only the declared
artifact. All three ignore executable-selection environment variables and
call the in-process Buoy libraries; they never invoke a caller-selected
executable or shell out to `buoy` itself.

Exit classes match the existing Buoy contract:

- `0` — successful operation.
- `2` — static blocker (stale relation, scope drift, immutable conflict).
- `64` — usage fault (missing required flag, conflicting selector).
- `70` — input fault (malformed plan, unmatched selector, undeclared
  artifact).

When `--config` is supplied, the command derives the complete retained
snapshot from the parsed repository configuration and explicit
`--root`/`--mount` inputs, then checks the plan's `source_set` policy
against the selected logical paths. A recursive selector that grows beyond
an exact declared policy returns exit `2` before any rendering.

## Operation reports

Every `check` and `generate` operation renders machine schema
`buoy/trace-publication-report/v1` (`src/publication.rs::render_report`).
Every field is optional and absent by default; the renderer never
fabricates a placeholder identity for a field the operation did not reach.

- `operation` — `"check"` or `"generate"`.
- `artifact` — the declared output ID this operation evaluated.
- `plan_id` — the plan's declared `publication_id`, when parsed.
- `basis` — the computed publication-basis digest, when the operation
  reached basis computation.
- `graph` — the current source-graph digest the operation observed.
- `prior_state` / `prior_digest` — the artifact's state before this
  operation (`"absent"`, `"equal"`, or `"unequal"`) and its digest, when one
  existed.
- `candidate_digest` — the exact digest of the candidate this operation
  rendered.
- `outcome` — the exact local result this operation completed
  (`"no-op"`, `"created"`, `"replaced"`, `"conflict"`, or `"blocked"`). This
  identifies only the local replacement that occurred; it never claims a
  repository commit or provider publication.
- `diagnostics` — canonically sorted `{code, message}` pairs.

A versioned report is emitted on every publication failure branch, not
only malformed-plan parsing and row-conformance drift: scope drift (a
declared exact source-set policy no longer matching the resolved
selection, `TPT-SOURCE-SET-SCOPE-DRIFT`), output-path classification and
cross-output collision (`TPT-OUTPUT-COLLISION` and the specific
`classify_output` code), a rejected external-link candidate
(`TPT-EXTERNAL-LINK-CANDIDATE-REJECTED`), a page operation whose declared
link is not yet a current saved source edit (`TPT-PAGE-PRECEDES-LINK`), an
unresolved blocking trace diagnostic carried over from the underlying
`buoy trace` query, and a `check`/`generate` on a `source-edit` artifact
(a real `"no-op"` outcome for `check`, or `TPT-SOURCE-EDIT-NOT-WRITABLE`
for `generate`, since Buoy never authors a human source edit) all render a
report with a stable code rather than a bare exit code.

A report that claims a cross-repository or provider scope fails to render
(`PublicationReport::with_scope_claim` is rejected by `render_report`).

## Page lifecycle

A declared page uses renderer schema `buoy/readable-trace-markdown/v1`. An
`immutable` page follows create-if-absent semantics: absent creates,
existing-and-equal is a no-op, existing-and-different fails without
replacement. A `replaceable` page always uses one atomic same-directory
replacement. Both lifecycles use no-follow descriptor-relative descent,
exclusive-create temporary siblings, and a bounded output lock
(`src/publication_fs.rs`).

## Path templates and target bases

`render_path` (`src/publication.rs`) accepts these registered
substitutions: `publication_basis_path`, `atom_revision_path`,
`record_digest_path`, `atom_revision`, `record_digest`. Every other
substitution, and every `..` traversal segment, fails closed. An immutable
page path must contain `{publication_basis_path}`.

A page target base is joined with `join_target_base`: it must be an
absolute `https` URI ending in `/` with no query or fragment, and no `.`
or `..` path segment (validated at plan-parse time, before any repository
operation runs); the join percent-encodes each normalized path segment and
rejects traversal. `validate_immutable_target` requires the rendered
target to contain the exact basis path encoding.

### Rejected-cycle rules

A path or target must not depend on a value that can exist only after the
write it helps produce. Version 1 registers only pre-link identities
(semantic atom revision, current binding-record digest, and the publication
basis itself); it rejects every graph, page, page-commit, branch-head,
provider, or generated-link substitution as unregistered. This keeps
publication acyclic: the basis derives from saved bytes only, the page path
derives from the basis, and the external-link target derives from the
already-rendered page path.

## Safe output behavior

`src/publication_fs.rs` opens each declared output root through one
retained no-follow directory descriptor and descends every parent
component the same way, rejecting a symlinked or non-directory component
before any temporary file exists. A source-edit path must select exactly
one regular selected source file
(`publication::require_source_edit_conforms`); a derived output must remain
outside every selected source set. A normalized per-output-path lock
(`acquire_output_lock`) serializes concurrent generation with a bounded
wait, returning a deterministic busy diagnostic rather than blocking
indefinitely. Failure cleanup removes only the temporary sibling the
current operation owns. This writer targets Unix platforms with `openat`,
`linkat`, `renameat`, and `flock`; it has no non-Unix implementation in
version 1.

## External-link generation

`src/external_link_generation.rs` edits one standalone OpenSpec level-two
section per standing specification: `## Publication: <link-id>` followed by
one ```` ```buoy-external-trace-v1 ```` fence. Generation:

1. Finds the fixed boundary immediately after the final normative
   requirement and before any other nonauthority section
   (`find_insertion_boundary`).
2. Finds zero or one present slot for the declared link ID (`find_slot`);
   more than one present slot fails closed as a duplicate.
3. Replaces a present slot or inserts a new one at the boundary
   (`apply_slot`), preserving every other byte.
4. Rejects migration of a legacy slot section that contains unrelated
   nonwhitespace content (`SlotError::MixedLegacySection`).

## Completion evidence

`buoy publication evidence` renders machine schema
`buoy/trace-publication-evidence/v1` (`src/publication.rs::render_evidence`).
It first requires every declared non-evidence artifact to pass parity;
drift returns exit `2` with no completion payload.

The machine document carries:

- `publication_id`, `status` (always `"historical"`).
- `source_digest` — the retained-snapshot identity this evidence derives
  from.
- `selector` — the plan's typed selector (`"requirement-id <value>"`, etc.).
- `basis` — the computed publication-basis digest.
- `graph` — the current source-graph digest.
- `rows` — the exact current trace rows this evidence summarizes,
  each with `relation`, `record`, `requirement_id`, `atom_revision`,
  `requirement_path`, `implementation_path`, `state`, and a dedicated
  `freshness` object (`record`, `attachment`, `implementation`,
  `requirement`, `authority`).
- `link_target` / `page_digest` — the declared external-link target and
  page-content digest, when the plan declares them.
- `diagnostics`.

It excludes immutable record bodies, provider status, commits, users,
reviews, approvals, clocks, and physical paths. A recorded freshness facet
describes one static result; it establishes no later or ongoing assurance
claim.

`generate --artifact <evidence-id>` renders the equivalent deterministic
Markdown through the same safe no-follow writer as any other declared
output.

The completed-traceability workflow artifact embeds the equivalent
generated identities in one marker-delimited block:

```text
<!-- BEGIN buoy-trace-publication-evidence-v1 -->
Publication ID: <id>
<!-- END buoy-trace-publication-evidence-v1 -->
```

A separate human-authored Markdown table records owner, consumer commit,
and review decision — facts the generated block cannot supply. A manual
table row that conflicts with the generated block's recorded identity fails
closed. A document that claims to itself be the current state (rather than
historical, superseded-by-query data) fails closed.

## Package interface

`nix/consumer-package-v1.nix` builds the pinned `buoy` executable from
committed source and `Cargo.lock` for one explicit digest-feature variant
(`xxh3-128`, `blake3-256`, or `sha-256`), with default Cargo features
disabled. `buoy package-identity` emits machine schema
`buoy/package-identity/v1` with the package version, source revision,
toolchain identity, and enabled feature set.

The blocking `buoy:package-contract` Devenv task runs an offline structural
check (`tools/check_package_contract.rs`) that the package file still
declares its closed required-input contract and every variant. It has a
resolved path to `devenv:enterTest`. The advisory `buoy:consumer-package`
task validates a pinned private `git+ssh` source with externally supplied
temporary credentials; it has no edge into `devenv:enterTest` and never
runs automatically.

## Package variants, updates, rollback, and offline use

A consumer pins one of the three declared variants (`xxh3-128`,
`blake3-256`, `sha-256`) matching its repository's configured active digest
member; a mismatch fails configuration parsing before graph construction
(`PBD-002`). An update replaces the pinned `sourceRevision` and
`sourceContentHash` inputs to `nix/consumer-package-v1.nix` and re-realizes
the derivation; a rollback simply repins the prior known-good revision and
content hash — the package function performs no history-dependent state,
so rollback is exact. After one realization, static/change/bind/trace/
publication commands run with no further source fetch, registry query, or
package discovery (`PBD-004`); this is a fixed-output Nix derivation
property, not a Buoy runtime behavior. A private source uses a pinned
non-flake `git+ssh` input with noninteractive read-only SSH and no
repository-stored credential; the advisory `buoy:consumer-package` task
(never part of `devenv test`) exercises that path when a maintainer
supplies temporary external credentials.

## Generic mounted example

A downstream consumer plan uses only its own logical IDs and mounts; no
Buoy code branches on a consumer name. A minimal mounted invocation:

```text
buoy publication check \
  --plan consumer-plan.json \
  --artifact trace-page \
  --config buoy.toml \
  --mount repository=/path/to/consumer/checkout
```

with a plan such as:

```json
{
  "schema": "buoy/trace-publication-plan/v1",
  "publication_id": "consumer-example",
  "selector": { "requirement_id": "EXAMPLE-001" },
  "expected_rows": 1,
  "source_set": ["docs/spec.md"],
  "outputs": [
    {
      "id": "trace-page",
      "mount": "repository",
      "path": "docs/trace/current.md",
      "kind": "page",
      "lifecycle": "replaceable"
    }
  ]
}
```

No field in this example encodes anything specific to any one downstream
project; every ID, path, and mount name is consumer-declared configuration.
This example declares a `replaceable` current alias; an `immutable`
basis-keyed page additionally requires exactly one referencing
`external-link` output (see "Page lifecycle" above).

## Downstream tool migration

After a consumer adopts the pinned Buoy package and a publication plan, it
can remove its own repository-owned fetch script, page-writer script,
report-checker script, and external-link synchronizer script. Buoy's
`publication check`/`generate`/`evidence` commands replace each of those
respectively: the safe writer (`src/publication_fs.rs`) replaces a
hand-written atomic-write script, the typed row-conformance path
(`publication::row_conformance`) replaces a text-based JSON check, and
`src/external_link_generation.rs` replaces a hand-written link-fence
editor. This removal is downstream-owned, explicit, and reviewed in the
consumer's own repository; Buoy makes no automatic change there.

## Assurance boundary

Every generated page, link, report, and evidence artifact supplies
navigation and impact data only. None establishes coverage, execution,
proof, approval, correctness, or prose-to-code correspondence. Buoy remains
the current-state authority; a completion snapshot is historical and does
not replace a fresh query.
