#!/usr/bin/env bash
# Tasks 10.1/10.2/10.5, PBD-002 and PBD-004: check the *real* consumer-package
# derivation, not the text of the file that produces it.
#
# Every assertion below reads a value Nix itself computed by evaluating
# `nix/consumer-package-v1.nix` through `nix/tools/realize-consumer-package.nix`
# and instantiating the resulting derivation, so a claim can only pass when
# the real package function really produced that value:
#
#   * each explicit digest variant is a distinct real derivation whose
#     `cargoBuildFlags` select the `buoy` and `chelis-provenance` packages,
#     disable default features, and enable exactly that per-package
#     member, and whose embedded package identity reports the same member
#     (PBD-002 positive);
#   * an undeclared digest member cannot even evaluate (PBD-002 negative);
#   * the produced executable's derivation is not fixed-output and declares
#     no impure environment, so its build cannot reach the network, and the
#     reviewed source verification is a real build input of it (PBD-004
#     positive, and the M13 source-content wiring);
#   * an unreviewed source content hash fails realization closed (M13
#     negative), checked against the committed reviewed constant;
#   * the declared source closure leaves out the symlinks Devenv generates
#     and the directories cargo-fuzz creates, and keeps committed paths with
#     the same names (M13, PBD-005).
#
# Evaluation only; the single realization it performs is the small
# fixed-output source-verification copy. Full variant realization and the
# offline command matrix live in `nix/tools/verify-offline-realization.sh`.
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
realize="$repo_root/nix/tools/realize-consumer-package.nix"
variants=(xxh3-128 blake3-256 sha-256)

fail() {
  echo "derivation-contract: $*" >&2
  exit 1
}

declare -a seen_drv_paths=()

for variant in "${variants[@]}"; do
  drv_path="$(nix-instantiate "$realize" --argstr digestFeature "$variant" 2>/dev/null)" ||
    fail "unevaluable-variant ${variant}"
  for previous in ${seen_drv_paths[@]+"${seen_drv_paths[@]}"}; do
    [ "$previous" = "$drv_path" ] &&
      fail "nondistinct-variant-derivation ${variant} shares ${drv_path}"
  done
  seen_drv_paths+=("$drv_path")

  # Newer Nix wraps the derivation map under `.derivations`; older Nix emits
  # the map itself. Normalize to the single derivation object either way.
  derivation="$(nix derivation show "$drv_path" | jq '(.derivations // .) | to_entries[0].value')"
  read_env() { printf '%s' "$derivation" | jq -r --arg key "$1" '.env[$key] // ""'; }

  read -r -a build_flags <<<"$(read_env cargoBuildFlags)"
  selected_packages=()
  selected_binaries=()
  selected_features=""
  defaults_disabled=false
  for ((flag_index = 0; flag_index < ${#build_flags[@]}; flag_index++)); do
    flag="${build_flags[$flag_index]}"
    case "$flag" in
    --package | --bin | --features)
      flag_index=$((flag_index + 1))
      [ "$flag_index" -lt "${#build_flags[@]}" ] ||
        fail "missing-build-flag-value ${variant}: ${flag}"
      value="${build_flags[$flag_index]}"
      case "$flag" in
      --package) selected_packages+=("$value") ;;
      --bin) selected_binaries+=("$value") ;;
      --features) selected_features="$value" ;;
      esac
      ;;
    --no-default-features) defaults_disabled=true ;;
    *) fail "unexpected-build-flag ${variant}: ${flag}" ;;
    esac
  done
  [ "$(printf '%s\n' "${selected_packages[@]}" | LC_ALL=C sort)" = "$(printf '%s\n' buoy chelis-provenance)" ] ||
    fail "wrong-selected-packages ${variant}"
  [ "$(printf '%s\n' "${selected_binaries[@]}" | LC_ALL=C sort)" = "$(printf '%s\n' buoy buoy-migrate chelis-provenance)" ] ||
    fail "wrong-selected-binaries ${variant}"
  [ "$defaults_disabled" = true ] ||
    fail "default-features-enabled ${variant}"
  [ "$(printf '%s' "$selected_features" | tr ',' '\n' | LC_ALL=C sort)" = "$(printf '%s\n' "buoy/${variant}" "chelis-provenance/${variant}")" ] ||
    fail "wrong-selected-features ${variant}"
  [ "$(read_env BUOY_PACKAGE_DIGEST_FEATURE)" = "$variant" ] ||
    fail "wrong-embedded-package-identity ${variant}"
  [ -n "$(read_env BUOY_PACKAGE_SOURCE_REVISION)" ] ||
    fail "missing-embedded-source-revision ${variant}"
  [ -n "$(read_env BUOY_PACKAGE_TOOLCHAIN)" ] ||
    fail "missing-embedded-toolchain-identity ${variant}"

  # PBD-004: a non-fixed-output derivation with no impure environment builds
  # inside the Nix sandbox and therefore cannot fetch a source, query a
  # registry, or discover a package at build time.
  fixed_output="$(printf '%s' "$derivation" | jq -r '.outputs.out.hash // ""')"
  [ -z "$fixed_output" ] ||
    fail "network-capable-package-derivation ${variant}: declares outputHash ${fixed_output}"
  [ -z "$(read_env impureEnvVars)" ] ||
    fail "impure-package-derivation ${variant}"
  [ -z "$(read_env __impure)" ] ||
    fail "impure-package-derivation ${variant}"

  # M13/PBD-001: the reviewed source verification must really gate this
  # build, so its derivation has to be one of this derivation's inputs.
  verified_source="$(read_env BUOY_VERIFIED_SOURCE)"
  case "$verified_source" in
  /nix/store/*buoy-verified-source*) ;;
  *) fail "unverified-source-input ${variant}: BUOY_VERIFIED_SOURCE=${verified_source}" ;;
  esac
  # `inputDrvs` on older Nix, `inputs.drvs` on newer Nix: either way the
  # verified-source derivation must be a declared build input.
  printf '%s' "$derivation" |
    jq -e --arg needle "buoy-verified-source" \
      '[(.inputDrvs // .inputs.drvs) | keys[] | select(contains($needle))] | length > 0' >/dev/null ||
    fail "source-verification-not-a-build-input ${variant}"

  echo "derivation-contract: ${variant} ok drv=${drv_path}"
done

# PBD-002 negative: an undeclared digest member is not a variant of this
# interface and must not even evaluate to a derivation.
if nix-instantiate "$realize" --argstr digestFeature "md5-128" >/dev/null 2>&1; then
  fail "undeclared-variant-evaluated md5-128"
fi
echo "derivation-contract: undeclared variant md5-128 rejected"

# M13 negative: the reviewed source-content digest is a committed constant,
# and a value that is not the declared source closure's real content hash
# must fail realization closed rather than build whatever `src` contains.
#
# The mutated value below is derived from the committed constant, so it is
# wrong no matter what the working tree currently contains, and the check
# requires Nix to reject it *for that reason* -- a hash mismatch naming the
# mutated value -- rather than for any other build failure. That keeps this
# control discriminating without depending on the committed constant being
# current: whether the committed constant is itself current is a separate,
# separately reported check (`derive-source-content-digest.sh --verify`,
# run by `verify-offline-realization.sh` before any realization).
reviewed="$("$repo_root/nix/tools/derive-source-content-digest.sh" --show)"
reviewed_payload="${reviewed#sha256-}"
# Flip one base64 character of the payload, keeping the SRI encoding and its
# padding valid, so Nix rejects the value for a hash mismatch and not for a
# malformed hash.
if [ "${reviewed_payload:0:1}" = "A" ]; then flipped="B"; else flipped="A"; fi
mutated_digest="sha256-${flipped}${reviewed_payload:1}"
if mutated_output="$(nix-build "$realize" -A passthru.verifiedSource \
  --argstr sourceContentHash "$mutated_digest" --no-out-link 2>&1)"; then
  fail "unreviewed-source-content-accepted ${mutated_digest}"
fi
case "$mutated_output" in
*"hash mismatch in fixed-output derivation"*"$mutated_digest"*) ;;
*)
  fail "unreviewed-source-content-rejected-for-the-wrong-reason: ${mutated_output}"
  ;;
esac
echo "derivation-contract: unreviewed source content digest rejected"

# M13/PBD-005: the declared source closure, and so the reviewed digest, must
# be the same in a fresh checkout, a Devenv shell, and a fuzzed worktree.
# Devenv writes `.pre-commit-config.yaml` and the fcis `rust-toolchain.toml`
# as symlinks into the Nix store, and cargo-fuzz creates empty
# `fuzz/artifacts/<target>` directories; the filter must reject all three.
# Committed paths with the same names must stay in the closure.
filter_verdict() {
  printf '(source.filter "%s/%s" "%s") ' "$repo_root" "$1" "$2"
}
verdicts="$(nix-instantiate --eval --strict --json -E "
  let source = (import $repo_root/nix/tools/declared-source.nix { }).source; in [
    $(filter_verdict .pre-commit-config.yaml symlink)
    $(filter_verdict dylint_lints/fcis_core/rust-toolchain.toml symlink)
    $(filter_verdict fuzz/artifacts directory)
    $(filter_verdict rust-toolchain.toml regular)
    $(filter_verdict fuzz/corpus directory)
    $(filter_verdict fixtures/logical-mount-artifact-bindings/v1/artifacts directory)
  ]")" || fail "unevaluable-declared-source-filter"
[ "$verdicts" = "[false,false,false,true,true,true]" ] ||
  fail "declared-source-admits-uncommitted-paths ${verdicts}"
echo "derivation-contract: declared source excludes generated paths"

echo "derivation-contract: ok variants=${variants[*]}"
