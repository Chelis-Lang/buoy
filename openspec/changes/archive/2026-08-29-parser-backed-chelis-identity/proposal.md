## Why

The shipped Chelis adapters anchor `chelis:provenance/v1` records with a lexical rule: a record attaches to the next top-level `def` or `type` line, and the item identity digests that one line of text. The Rust adapter derives parser-canonical identity from the real AST, so the two provenance lanes are unequal at the extraction boundary. The first live adoption in Nautilus surfaced the cost: a canonical-Surf reformat forces manual re-anchoring, and downstream hosts need an out-of-band helper build to learn atom revisions. Chelis ships its own generated tree-sitter Surf grammar, so the parser exists; this change pins it and closes the identity gap plus the three follow-ups the Nautilus adoption recorded.

## What Changes

- Vendor the generated `tree-sitter-chelis-surf` parser sources from one exact pinned Chelis revision, with a registered derivation and a fail-closed drift check.
- Add a `chelis-surf-sitter` workspace crate that binds the vendored grammar through the pinned tree-sitter runtime.
- Rework Surf extraction in `chelis-provenance`: record anchoring resolves against the parsed tree, item identity derives from the parsed declaration node's canonical bytes and span, and parse `ERROR` nodes fail loudly instead of passing as nonnormative text.
- Bump the frozen adapter identities for the new parser (`chelis-surf-syntax/v2`, grammar-pinned parser version) and document source reingestion; the `chelis/surf-declaration/v1` item kind is unchanged.
- Expose atom revisions and object digests in the canonical static report so hosts bind records without helper builds.
- Expose the advisory report through the `chelis-provenance` CLI and expose the `chelis-provenance` binary through the consumer package contract.
- **Non-goals:** parsing the Deep dialect (stays lexical), changing the record comment grammar, blocking policy anywhere, re-pinning Nautilus (a follow-on change), and any Chelis-repo edit.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `chelis-provenance-adapter`: parser-canonical Surf identity over the pinned vendored grammar, revision-bearing canonical reports, and complete command exposure through the CLI and consumer package.

## Impact

- Adds `crates/chelis-surf-sitter` (vendored generated C parser plus binding) and two pinned dependencies (`tree-sitter`, `cc`) under the existing supply-chain policy.
- Reworks `crates/chelis-provenance` extraction, descriptors, tests, fixtures, and goldens; static report schema moves to `chelis-static-report/v2`.
- Updates `nix/consumer-package-v1.nix` and its contract checks to build and expose the `chelis-provenance` binary.
- Requires the sanctioned assurance regeneration: campaign re-binding, execution evidence, ledger, naming inventory, and the reviewed source-content digest.
