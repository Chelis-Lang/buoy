#!/usr/bin/env bash
# Tasks 10.11/10.12, audit M17: the single registered owner of every
# consumer-package task body.
#
# Exactly one script implements what a consumer-package task does. The
# blocking internal task `buoy:package-contract` and the advisory downstream
# task `buoy:consumer-package` both invoke THIS file with a mode; neither
# duplicates a step inline, and the `buoy-package-contract` devenv script
# delegates here too. Adding a step here therefore changes every owner of
# that step at once, and a task body can no longer drift from the gate it
# claims to run.
#
# Modes:
#   --mode blocking   Internal, blocking, inside the resolved `devenv test`
#                     graph. Runs the offline structural contract gate, then
#                     the real offline local-source realization: reviewed
#                     source-content verification, the real derivation
#                     contract for every explicit variant, realization of
#                     every variant with empty Cargo and Rustup homes, the
#                     static/change/bind/trace/publication offline command
#                     matrix, and the retained noncanonical closure-size and
#                     build-cost measurements.
#   --mode advisory   Downstream, advisory, deliberately OUTSIDE the resolved
#                     `devenv test` graph and with no dependency path to
#                     `devenv:enterTest`. Runs the same shared structural and
#                     derivation contract steps, then the pinned private
#                     `git+ssh` acquisition check, which is skipped unless a
#                     maintainer supplies temporary credentials externally.
#                     It realizes nothing and stores no credential.
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$repo_root"

mode="blocking"
while [ "$#" -gt 0 ]; do
  case "$1" in
  --mode)
    shift
    mode="${1:-}"
    ;;
  *)
    echo "usage: package-contract.sh [--mode blocking|advisory]" >&2
    exit 2
    ;;
  esac
  shift
done

# The shared, offline, Nix-free structural gate. It parses the real closed
# input contract of `nix/consumer-package-v1.nix` and fails closed on any
# interface regression before anything slower runs.
structural_contract() {
  cargo run --quiet --locked --all-features --bin check-package-contract
}

case "$mode" in
blocking)
  structural_contract
  nix/tools/verify-offline-realization.sh
  echo "package-contract: blocking mode complete"
  ;;
advisory)
  structural_contract
  nix/tools/check-derivation-contract.sh
  cargo run --quiet --locked --all-features --bin check-consumer-package
  echo "package-contract: advisory mode complete"
  ;;
*)
  echo "package-contract: unknown-mode ${mode}" >&2
  exit 2
  ;;
esac
