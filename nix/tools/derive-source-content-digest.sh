#!/usr/bin/env bash
# Task 10.5 / audit M13: maintain and check the *committed, reviewed*
# source-content digest for `nix/consumer-package-v1.nix`.
#
# The reviewed constant lives in `nix/consumer-package-v1.source-content.json`
# and is the only expected value any verification path uses. Verification
# derives the declared source closure's content hash *independently*
# (`nix hash path --mode nar`, and, at realization, Nix's own fixed-output
# check of `passthru.verifiedSource`) and compares it against that committed
# constant, failing closed on any difference. No verification path may take
# its expected value from the tree it is about to check: that circularity is
# exactly the defect this tool exists to prevent.
#
# Modes:
#   --show     Print the committed reviewed digest. Derives nothing.
#   --derive   Print the independently derived digest of the current declared
#              source closure. Changes nothing.
#   --verify   Compare the independently derived digest against the committed
#              reviewed constant. Exit 0 when they agree; exit 1 with the
#              deterministic `stale-source-content-digest` diagnostic when
#              they differ. This is the verification path gates use.
#   --update   Rewrite the committed reviewed constant from the current
#              declared source closure. An explicit maintainer action, never
#              part of any gate: the new value must be re-reviewed together
#              with the source change that caused it.
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
lock_file="$repo_root/nix/consumer-package-v1.source-content.json"

committed_digest() {
  sed -n 's/.*"reviewed_source_content_hash"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p' "$lock_file"
}

# Derives the declared source closure's content hash from the closure itself,
# with no reference to the committed constant: the declared source is
# instantiated into the store and hashed with the same recursive NAR sha256
# encoding `nix/consumer-package-v1.nix` declares as `outputHashMode` /
# `outputHashAlgo`, so an equal value is a real independent agreement and an
# unequal value is a real disagreement.
derive_current_digest() {
  local source_path
  source_path="$(
    nix-instantiate --eval --read-write-mode --json \
      -E "\"\${(import ${repo_root}/nix/tools/declared-source.nix { }).source}\"" |
      tr -d '"'
  )"
  if [ -z "$source_path" ] || [ ! -e "$source_path" ]; then
    echo "source-content-digest: could-not-instantiate-declared-source" >&2
    return 1
  fi
  nix hash path --mode nar --type sha256 --sri "$source_path"
}

committed="$(committed_digest)"
if [ -z "$committed" ]; then
  echo "source-content-digest: missing-reviewed-digest $lock_file" >&2
  exit 1
fi

case "${1:---verify}" in
--show)
  printf '%s\n' "$committed"
  ;;
--derive)
  derive_current_digest
  ;;
--verify)
  current="$(derive_current_digest)"
  printf 'committed\t%s\n' "$committed"
  printf 'derived\t%s\n' "$current"
  if [ "$committed" != "$current" ]; then
    echo "source-content-digest: stale-source-content-digest committed=${committed} derived=${current}" >&2
    echo "source-content-digest: re-review the declared source closure, then run nix/tools/derive-source-content-digest.sh --update" >&2
    exit 1
  fi
  echo "source-content-digest: ok ${committed}"
  ;;
--update)
  current="$(derive_current_digest)"
  if [ "$committed" = "$current" ]; then
    echo "source-content-digest: unchanged ${committed}"
    exit 0
  fi
  tmp="$(mktemp)"
  sed "s|\"reviewed_source_content_hash\": \".*\"|\"reviewed_source_content_hash\": \"${current}\"|" \
    "$lock_file" >"$tmp"
  mv "$tmp" "$lock_file"
  echo "source-content-digest: updated ${committed} -> ${current}"
  echo "source-content-digest: re-review this reviewed constant with the source change that caused it" >&2
  ;;
*)
  echo "usage: derive-source-content-digest.sh [--show|--derive|--verify|--update]" >&2
  exit 2
  ;;
esac
