# Buoy rename release checklist

This checklist governs provider-side release actions. Provider state is operational input only: repository slug, checkout basename, registry ownership, approval timestamps, and network responses never alter canonical graph or report identity.

## MIT source release

The current source release does not change repository names, checkout names, package names, APIs, or schema IDs.
All real Cargo packages retain `publish = false`.
The clean source export excludes the private repository's Git history and GitHub content.

1. Retain the root `LICENSE` for repository-owned code.
2. Retain `crates/chelis-surf-sitter/LICENSE-CHELIS` for the pinned Chelis grammar.
3. Retain `crates/chelis-surf-sitter/LICENSE-TREE-SITTER` for the copied Tree-sitter headers.
4. Retain `unicode/16.0.0/LICENSE` and the copyright header in `CaseFolding.txt`.
5. Retain dependency license files when an export includes dependency source or binaries.
6. Get owner approval for source release rights and any private business material.
7. Review the exact export file list and bytes for credentials and private data.
8. Rotate any exposed credential before any approved publication.
9. Review reachable Git history separately before any approved public Git publication.
10. Refresh the consumer source digest after all export changes.
11. Do not commit, push, change visibility, rewrite history, or create a release without separate approval.

The local review covers tracked files and blobs reachable from local Git refs.
It does not cover server-only refs, deleted unreachable objects, external stores, or remote access rights.
Pattern scans do not prove that all sensitive data is absent.
The 2026 copyright notice in the root `LICENSE` names 2nd Set AI Corp. as the copyright owner.
Git author records do not prove legal ownership.

## Source-only publication scope

The release owner selects a public source release, not production acceptance.
This decision does not waive the production blockers below or authorize a visibility change.

| Check | Observed result |
| --- | --- |
| `devenv --no-tui test` | Miri cannot execute its artifacts through the ambient `kache test-runner`. |
| `check-exact-coverage --check-inventory` | 27 Chelis Rust files lack inventory rows. |

The focused `--clean` adversarial command reaches the same Miri runner failure.
The saved ledger does not replace these failed checks.


## Public repository controls

1. Reconcile local changes with the intended publication revision before the final review.
2. Review public Git refs, issue text, pull request comments, wiki pages, releases, and retained workflow artifacts.
3. Confirm source release rights for repository-owned and third-party material.
4. Keep local environment files, credentials, assistant state, and extra worktrees outside source exports.
5. Retain example environment files only when they contain no credentials.
6. Select GitHub private vulnerability reports as the private security channel described in [`SECURITY.md`](../../SECURITY.md).
7. Enable private vulnerability reports during the approved publication and verify the report form before any public announcement.
8. Enable GitHub secret scanning and push protection where the public repository plan supports them.
9. Review branch protection, fork access, and issue access for the public contributor model.
10. Refresh the consumer source digest after the final source changes, then run `devenv test`.
11. Record failed checks and unresolved risks. A source build does not replace release acceptance.

### Clean source publication

The original GitHub repository and its history remain private.
The initial public commit contains the reviewed source and has no parent commit.
Do not copy old refs, issues, pull requests, or wiki content into it.

The selected public address is `Chelis-Lang/buoy`.
The private repository currently uses that address.
An owner must rename the private repository before the clean repository can use it.
Keep the original repository private during that change.
Repository creation, renaming, and publication need separate approval.

Before publication, scan the exact source archive and every Git object in the clean repository.
Review each result.
A fixture digest can match a credential pattern.
A clean pattern scan does not prove that all private data is absent.

## Before releasing 0.2.0

1. Confirm the desired Git repository slug is available.
2. Confirm the `buoy`, `buoy-core`, and any separately published compatibility package names are available in each intended registry.
3. Obtain fresh human approval immediately before changing a remote slug, transferring ownership, or publishing packages.
4. Record provider actions outside canonical reports; retain only repository-owned migration records and immutable published artifact identities as reviewed evidence.
5. Run `devenv test --no-tui` from a clean checkout whose directory basename differs from the repository slug.

## Compatibility window

The repository-local `spec-provenance` and `spec-provenance-core` facades are nonauthoritative aliases for `>=0.2.0,<0.3.0`. Their deprecation warning is deterministic stderr and is excluded from canonical output. Do not extend this window without a new reviewed change.

## Before releasing 0.3.0

1. Remove the old package, crate, binary, and operational task aliases.
2. Keep frozen `spec-provenance-*/v1` and `spec-provenance-*/v2` protocol identifiers and historical readers byte-identical.
3. Run the alias-expiry, clean-install, upgrade, rollback, stale-name, and canonical-corpus gates.
4. Obtain fresh approval for provider-side package publication or repository renaming; repository changes alone do not authorize those external actions.
