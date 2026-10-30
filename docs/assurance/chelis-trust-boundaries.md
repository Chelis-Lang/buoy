# Chelis integration trust boundaries

This document records the trust boundaries for the `chelis-provenance` shell
integration. The integration pins Chelis revision
`c0d4caf16f557406f299f0989bab48fa0f924cbd` for the record grammar boundary
and Chelis revision `fc3232ffe085d15b213bb7457b69a22063a200b9` (`v0.18.5`)
for the vendored Surf grammar. The frozen machine record is
`fixtures/chelis-provenance/v1/boundary.toml`.

## Trust boundaries

- The `buoy-core` crate stays `no_std` and repository-independent. It contains
  no Chelis type, dependency, or capability meaning.
- The `chelis-provenance` crate depends inward on the published `buoy` shell
  and core contracts. The dependency direction is one-way.
- Canonical inputs are saved repository paths, saved bytes, and explicit
  configuration. Daemon state, editor buffers, LSP observations, provider
  metadata, Git state, and clocks supply no authority.
- The adapters declare `chelis/surf-declaration/v1` and `chelis/deep-node/v1`
  through the schema-v3 `SourceItemKind` protocol. No legacy reader, opaque
  payload, cross-adapter kind, or host-specific core enum exists.

## Parser boundary

- Surf extraction parses saved sources with the vendored
  `tree-sitter-chelis-surf` grammar from Chelis `v0.18.5`. The grammar
  accepts canonical Surf; a saved source outside the canonical surface fails
  with `CHELIS-PROV-PARSE-ERROR` and must be formatted before records bind.
- Surf item identity derives from the parsed declaration node's exact source
  bytes and byte span under `chelis-surf-syntax/v2`. Identities recorded
  under the lexical `chelis-surf-syntax/v1` scheme are not migrated; hosts
  reingest their sources. No legacy reader exists.
- The vendored grammar artifacts carry a registered derivation with per-file
  digests. The drift check fails closed on any vendor byte change until the
  registration is reviewed together with the refresh.
- The `cc`-compiled grammar sources are the workspace's only sanctioned FFI
  boundary, confined to the `chelis-surf-sitter` crate with two audited
  unsafe expressions.
- The Deep dialect keeps lexical line anchoring until its grammar is adopted
  in a separate reviewed change.
- The canonical static report (`chelis-static-report/v3`) exposes object
  identity digests and atom revisions. The report stays a projection of the
  graph, never an authority; hosts bind from it without out-of-band helper
  builds.

## Enforcement completeness

- Every record-carried binding digest is enforced at static time: item
  digests against the parsed declaration node bytes
  (`CHELIS-PROV-ITEM-STALE`) and declared runner digests against the saved
  runner configuration artifacts (`CHELIS-PROV-ORACLE-RUNNER-STALE`). No
  record field is decorative.
- An oracle without `runner-config-paths` keeps unenforced runner digests;
  the advisory report lists such oracles as a visible gap.
- Every digest- and revision-staleness context carries
  `bound=<digest>;current=<digest>`, so correction never needs an
  out-of-band recomputation build.
- The `trace` subcommand projects one stable ID's complete chain with every
  hop recomputed from saved bytes. The projection is a rendering of the
  static outcome, never an authority, and rejects when any hop mismatches.
- The `rebind` subcommand regenerates only mechanical digest and revision
  fields, prints the field-level diff, writes only under `--write`, and
  refuses when the grammar rejects a selected source. Policy: review the
  dry-run diff before every write, the same rule as golden regeneration.
  A rebind changes the static identity, so committed receipts force corpus
  re-execution; rebind cannot move claims, verdicts, or receipts.

## Compact content-addressed bindings

- A `binding` record carries one marker line and one `record = <digest>`
  address. The record itself lives in the object store at the frozen upstream
  layout `<binding-store>/buoy/objects/v1/<algorithm>/<hex>.toml`, so the
  Chelis store and the Rust store hold byte-identical objects.
- `binding-store` names a mount prefix over that frozen layout, not a layout of
  its own. It must lie under one selector root, because only selected bytes
  enter a saved snapshot. A store outside every root is a configuration error
  at the boundary, not a late missing object.
- The marker's field set is closed to exactly `record`. A second field would be
  an unenforced claim beside the address, so it fails with
  `CHELIS-PROV-MALFORMED-RECORD` and extracts no record.
- Resolution reuses the upstream object parser: it recomputes the record
  digest from the parsed fields and compares it with the digest the store
  path claims. Lookup is exact, so an object planted at any other path answers
  for no address. A missing, substituted, or malformed object fails with
  `CHELIS-PROV-BINDING-ADDRESS` and materializes nothing; where the substituted
  bytes still parse, the diagnostic names the address they now hash to as
  `bound=<address>;current=<recomputed>`. Trace rows mark these refused object
  fields as `verified = false`; they supply diagnostics but no authority.
- The recorded subject is enforced against the attached declaration's parsed
  node bytes (`CHELIS-PROV-BINDING-SUBJECT-STALE`).
- The recorded requirement is enforced against the exact authority record
  block (`CHELIS-PROV-BINDING-REQUIREMENT-STALE`). A formatting-only edit
  changes this source-node digest and requires a reviewed rebind.
- A valid but stale binding remains in the rejected graph. This preserves
  conservative impact data and matches inline-link behavior. An invalid object
  never materializes.
- Core graph validation includes `implementation_bindings` in its
  atom-revision closure. A stale compact binding therefore rejects with
  `SPG-REF-DANGLING-OR-STALE`, which matches the inline form.
- Content addressing requires a collision-resistant member, and every digest
  in one stored binding shares the member the rest of the graph uses.
  A build whose active member is change-detection only (`xxh3-128`) resolves
  no compact binding at all and says so; adopting the compact form means
  pinning a collision-resistant digest feature for the whole host. A host
  migrating to the compact form must therefore migrate its graph digests to
  that member and re-execute its receipts, because the migration moves every
  static identity the receipts are bound to.
- `bind` converts one inline link into one compact binding, and refuses any
  link it cannot re-encode exactly: navigation other than `implements`, an atom
  count other than one, or a stale recorded item digest. Narrowing a claim to
  fit the stored schema would launder it.
- `rebind` regenerates a stale binding by writing a successor object, updating
  the address, and removing the superseded object, filtered against every
  successor the same plan writes. Objects are never edited in place, a
  destination holding different bytes refuses at planning time, dry runs never
  touch the store, and declarations stay byte-identical. The same
  review-the-diff-before-writing policy applies.

## Manual approvals

- Atom ratification, policy adoption, waivers, reaffirmations, and proof
  correspondence require repository-owned approval records.
- Approval records bind an authority scope, a record path, and a digest.
- `fixtures/chelis-provenance/v1/registrations/promotion-review.toml` records
  the advisory-to-blocking review. Promotion is not approved.

## Evidence scopes

- Every registered carrier binds an exact item, role, oracle, configuration,
  corpus, scope, assumptions, and exclusions.
- Registered Chelis carriers retain verdict `not-run` at the pinned revision.
  A registration is not an execution result.
- Debt, waivers, reaffirmations, and derivation consistency cannot synthesize
  a passing verdict. The mutation suite checks this rule.

## Provider materialization

- Provider workflows can propose repository-owned review or change-manifest
  records. Deterministic checking consumes only committed neutral values.
- Labels, users, review timestamps, issue state, and branch names cannot
  change graph identity or report bytes.

## Compiler and backend exclusions

- No Chelis claim is reported formally proved at the pinned revision.
- Any future proof acceptance excludes rustc, LLVM, emitted backends, and
  machine code unless a separate verified chain covers them.
- `fixtures/chelis-provenance/v1/registrations/proof-bindings.toml` records
  the complete acceptance gate.

## Rollback

- Before blocking adoption, rollback removes the `chelis-provenance` crate,
  its fixtures, and its configuration. No core contract changes.
- After publication, a replacement item-kind schema requires source
  reingestion and new schema identities.

## Deferred evidence

- Chelis normative sources are not atomized at the pinned revision. Every
  registered carrier binds `pending-atomization`.
- No Chelis finite blocking surface is declared in a saved Chelis tree.
- The advisory report keeps every deferred obligation visible: unatomized
  normative text, uncovered atoms, undecided surface members, possible
  untracked copies, active debt and waivers, and stale derivations.
