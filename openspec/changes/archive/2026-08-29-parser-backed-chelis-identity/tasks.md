## 1. Pin and vendor the grammar

- [x] 1.1 Record the grammar pin: Chelis `v0.18.5` (`fc3232ffe085d15b213bb7457b69a22063a200b9`), the four grammar source files, and their digests in a registration under `fixtures/chelis-provenance/v1/`.
- [x] 1.2 Vendor `parser.c`, `scanner.cc`, `grammar.json`, and `node-types.json` from the pinned revision into `crates/chelis-surf-sitter/vendor/` byte-for-byte.
- [x] 1.3 Add the blocking drift check that recomputes the vendored digests against the registration and fails closed on any mismatch.

## 2. Binding crate

- [x] 2.1 Create `crates/chelis-surf-sitter` with exact-pinned `tree-sitter` and `cc` dependencies, a build script confined to this crate, and a safe API returning the `Language` plus typed top-level declaration nodes with byte spans.
- [x] 2.2 Clear supply-chain review: `deny.toml` entries, workspace lint conformance, and the crate-purity gates.
- [x] 2.3 Add parse fixtures: representative Surf sources, an `ERROR`-node source, and a `MISSING`-node source; assert deterministic node inventories across two distinct roots.

## 3. Parser-backed extraction

- [x] 3.1 Switch Surf record anchoring in `crates/chelis-provenance` to the parsed tree: a record attaches to the first top-level declaration node after its block, blank lines only between.
- [x] 3.2 Derive `ParsedItemIdentity` from the declaration node's canonical bytes and byte span; keep comments and non-declaration nodes nonnormative through the total classification.
- [x] 3.3 Emit `CHELIS-PROV-PARSE-ERROR` with the static exit class for any `ERROR` or `MISSING` node in a selected source; add negative fixtures.
- [x] 3.4 Bump the frozen identities to `chelis-surf-syntax/v2` with parser version `0.18.5+grammar.fc3232ff`; keep the `chelis/surf-declaration/v1` item kind; document reingestion in the trust document.
- [x] 3.5 Regenerate crate tests, the frozen fixture pack cases that assert identity bytes, and the goldens; add the reformat-stability case from CHELIS-PROV-012.

## 4. Reports, CLI, and package exposure

- [x] 4.1 Ship `chelis-static-report/v2`: per-object identity digests and per-atom revision digests in canonical order; update goldens and the cross-root byte-equality assertion.
- [x] 4.2 Add the `advisory` CLI subcommand rendering the existing advisory report deterministically with unchanged exit classes; add goldens.
- [x] 4.3 Expose the `chelis-provenance` binary through `nix/consumer-package-v1.nix`; extend the package-contract checks to assert the binary and its usage exit class.

## 5. Assurance convergence

- [x] 5.1 Extend the fixture manifest and registrations for the new cases; update `docs/assurance/chelis-trust-boundaries.md` with the parser boundary and the retired helper-bootstrap caveat.
- [x] 5.2 Re-run the mutation and fuzz campaigns over the changed extraction code through the sanctioned tools; record honest outcomes only.
- [x] 5.3 Run the sanctioned regeneration to a fixed point: execution evidence, ledger, digest-correspondence inventory, naming inventory, quality registry, and the reviewed source-content digest.
- [x] 5.4 Final oracle: full `devenv test` exits 0 with every gate green and no synthesized verdicts.
