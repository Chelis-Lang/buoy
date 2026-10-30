{ pkgs, ... }:
{
  scripts.buoy-core = {
    description = "Run the non-duplicative pure-core parity, canonical, and governance slice";
    exec = ''
      set -euo pipefail
      cargo test --test core_contract --test report_encoding --test canonical_differential --test canonical_vectors --test malformed_canonical_vectors
      cargo test -p buoy-core --test graph_validation_contract
      cargo test -p buoy-core --test schema_impact_contract
      cargo test -p buoy-core --test policy_ratchet_contract
      cargo test -p buoy-core --test disposition_protocol_contract
    '';
  };

  scripts.buoy-adapters = {
    description = "Run parser fixtures, mutations, parity, and root-independence checks";
    exec = ''
      set -euo pipefail
      cargo test --test adapters
      cargo test --all-features --test openspec_traceability
    '';
  };

  scripts.buoy-integration = {
    description = "Run isolated repository and exact CLI integration contracts";
    exec = ''
      set -euo pipefail
      cargo test --test integration
      cargo test --all-features --test requirement_subject_inventory
    '';
  };

  scripts.buoy-fuzz-replay = {
    description = "Replay committed coverage-guided fuzz seeds without live repository effects";
    packages = [ pkgs.cargo-fuzz ];
    exec = ''
      set -euo pipefail
      test "$(cargo fuzz --version)" = "cargo-fuzz 0.13.2"
      seed_count=0
      for target in canonical_values paths_globs adapters config_manifests graphs_governance reports publication; do
        for seed in "fuzz/corpus/$target"/*; do
          test -f "$seed"
          cargo fuzz run "$target" "$seed" -- \
            -runs=1 -seed=20260722 -max_len=16384 -timeout=5
          seed_count=$((seed_count + 1))
        done
      done
      test "$seed_count" -eq 14
      echo "fuzz-replay: targets=7 committed-seeds=14 scope=bounded-deterministic"
    '';
  };

  scripts.buoy-fuzz-campaign = {
    description = "Run a bounded deterministic-seed coverage-guided fuzz smoke campaign";
    packages = [ pkgs.cargo-fuzz ];
    exec = ''
      set -euo pipefail
      test "$(cargo fuzz --version)" = "cargo-fuzz 0.13.2"
      scratch="$(mktemp -d)"
      trap 'rm -rf "$scratch"' EXIT
      for target in canonical_values paths_globs adapters config_manifests graphs_governance reports publication; do
        cp -R "fuzz/corpus/$target" "$scratch/$target"
        cargo fuzz run "$target" "$scratch/$target" -- \
          -runs=64 -seed=20260722 -max_len=16384 -timeout=5
      done
      echo "fuzz-campaign: targets=7 runs-per-target=64 findings=0"
    '';
  };

  scripts.buoy-mutation-replay = {
    description = "Run every named high-consequence mutation tripwire";
    exec = ''
      set -euo pipefail
      cargo test --locked --offline --all-features --test mutations
      cargo run --quiet --locked --offline --bin check-adversarial-evidence -- \
        --check "''${BUOY_CAMPAIGN_AS_OF:-$(date -u +%F)}"
    '';
  };

  scripts.buoy-mutation-campaign = {
    description = "Run the pinned explicitly selected automated mutation baseline";
    packages = [ pkgs.cargo-mutants ];
    exec = ''
      set -euo pipefail
      test "$(cargo mutants --version)" = "cargo-mutants 27.1.0"
      export CARGO_PROFILE_DEV_DEBUG=0
      export CARGO_PROFILE_TEST_DEBUG=0
      export CARGO_INCREMENTAL=0
      unset RUSTC_WRAPPER RUSTC_WORKSPACE_WRAPPER CARGO_BUILD_RUSTC_WRAPPER
      if [ "''${BUOY_LIST_MUTANTS:-0}" = 1 ]; then
        cargo mutants --config .cargo/mutants.toml --workspace --list
        exit 0
      fi
      mkdir -p target
      rm -rf target/mutants-initial
      mkdir -p target
      cargo mutants \
        --config .cargo/mutants.toml \
        --workspace \
        `# The 19 selected mutants live only in buoy and buoy-core. Test those` \
        `# two packages so unrelated workspace binaries do not dominate each` \
        `# mutant build. The all-features behavioral controls remain included.` \
        --test-workspace false \
        --test-package buoy \
        --test-package buoy-core \
        --baseline run \
        `# Round-5: the selected test suite's baseline phase runs real CLI` \
        `# invocations. At the` \
        `# previous --timeout 900 (1.5x baseline) most mutants returned` \
        `# TIMEOUT, which is an INCONCLUSIVE verdict, not a caught one -- the` \
        `# campaign reported success while never deciding them. 2400s matches` \
        `# the configured timeout_multiplier of 4.0 against that baseline.` \
        `# Mutant builds can exceed five minutes when the shared build host is` \
        `# busy. Keep build pressure distinct from an inconclusive test result.` \
        --timeout 2400 \
        --build-timeout 1200 \
        --minimum-test-timeout 120 \
        --jobs 1 \
        --no-shuffle \
        --no-times \
        --output target/mutants-initial \
        --caught \
        --unviable
    '';
  };

  scripts.buoy-miri = {
    description = "Run Miri only over the declared deterministic pure-core scope";
    exec = ''
      set -euo pipefail
      cargo miri --version
      cargo miri setup
      MIRIFLAGS="-Zmiri-strict-provenance" \
        cargo miri test --locked --offline -p buoy-core --test canonical_differential
      MIRIFLAGS="-Zmiri-strict-provenance" \
        cargo miri test --locked --offline -p buoy-core --test schema_impact_contract \
          assumption_property_proof_chain_reaches_the_fixed_point_in_canonical_order -- --exact
    '';
  };

  scripts.buoy-sanitizer = {
    description = "Run address sanitizer over the declared Linux/macOS focused scope";
    exec = ''
      set -euo pipefail
      case "$(uname -s)" in
        Darwin) asan_options="detect_leaks=0:halt_on_error=1" ;;
        Linux) asan_options="detect_leaks=1:halt_on_error=1" ;;
        *) echo "ADV-SANITIZER-PLATFORM: address sanitizer scope is Linux/macOS" >&2; exit 1 ;;
      esac
      host="$(rustc -vV | awk '/^host:/ { print $2 }')"
      CARGO_BUILD_JOBS=1 \
      ASAN_OPTIONS="$asan_options" \
      RUSTFLAGS="-Zsanitizer=address" \
        cargo test --locked --offline --target "$host" \
          -p buoy-core --test canonical_differential
    '';
  };

  scripts.buoy-faults = {
    description = "Run bounded filesystem parser allocation process pipe result timeout and cancellation faults";
    exec = ''
      set -euo pipefail
      cargo test --locked --offline --all-features --test adversarial_faults
    '';
  };

  scripts.buoy-platform-corpus = {
    description = "Repeat the local platform corpus and retain scoped effect diagnostics";
    exec = ''
      set -euo pipefail
      case "$(uname -s)" in
        Darwin) platform=macos ;;
        Linux) platform=linux ;;
        *) platform=windows ;;
      esac
      first="$(mktemp -d)"
      second="$(mktemp -d)"
      trap 'rm -rf "$first" "$second"' EXIT
      cargo run --quiet --locked --offline --bin check-adversarial-evidence -- \
        --platform-corpus "$platform" "$first"
      cargo run --quiet --locked --offline --bin check-adversarial-evidence -- \
        --platform-corpus "$platform" "$second"
      cmp "$first/portable-corpus.tsv" "$second/portable-corpus.tsv"
      cmp "$first/platform-effects.tsv" "$second/platform-effects.tsv"
    '';
  };

  scripts.buoy-platform-result = {
    description = "Materialize one current host-detected platform observation";
    exec = ''
      set -euo pipefail
      cargo test --quiet --locked --offline --all-features --test adversarial_faults
      cargo run --quiet --locked --offline --bin check-adversarial-evidence -- \
        --platform-result assurance/adversarial/v1/platform-results
    '';
  };

  scripts.buoy-adversarial-evidence = {
    description = "Validate campaign source tool configuration expiry and non-discharge bindings";
    exec = ''
      set -euo pipefail
      cargo run --quiet --locked --offline --bin check-adversarial-evidence -- --self-test
      cargo run --quiet --locked --offline --bin check-adversarial-evidence -- \
        --check "''${BUOY_CAMPAIGN_AS_OF:-$(date -u +%F)}"
    '';
  };

  scripts.buoy-adversarial = {
    description = "Run bounded replay named mutations faults dynamic tools platform corpus and freshness";
    exec = ''
      set -euo pipefail
      buoy-fuzz-replay
      buoy-faults
      buoy-miri
      buoy-sanitizer
      buoy-compile-fail
      buoy-platform-corpus
      buoy-adversarial-evidence
    '';
  };
}
