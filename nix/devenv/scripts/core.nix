{ dylintTools, ... }:
{
  scripts.buoy-fcis = {
    description = "Enforce the structural functional-core boundary";
    packages = [ dylintTools ];
    exec = ''
      set -euo pipefail

      cargo check --package buoy-core --lib --target thumbv7em-none-eabi

      fcis_target="$PWD/target/fcis-focused"
      CARGO_TARGET_DIR="$fcis_target" cargo build --release --locked --offline \
        --manifest-path dylint_lints/fcis_core/Cargo.toml --lib
      library_dir="$(mktemp -d)"
      dylint_home="$(mktemp -d)"
      driver_log="$(mktemp)"
      negative_log="$(mktemp)"
      trap 'rm -rf "$library_dir" "$dylint_home"; rm -f "$driver_log" "$negative_log"' EXIT
      cat >"$dylint_home/config.toml" <<EOF
      [source.crates-io]
      replace-with = "dylint-vendor"
      [source.dylint-vendor]
      directory = "$BUOY_DYLINT_DRIVER_VENDOR"
      [patch.crates-io]
      dylint_driver = { path = "$BUOY_DYLINT_DRIVER_SOURCE" }
      [net]
      offline = true
      EOF
      toolchain="$(rustup show active-toolchain | awk '{print $1}')"
      case "$(uname -s)" in
        Darwin) extension="dylib" ;;
        Linux) extension="so" ;;
        *) echo "unsupported Dylint platform" >&2; exit 1 ;;
      esac
      cp "$fcis_target/release/libfcis_core.$extension" \
        "$library_dir/libfcis_core@$toolchain.$extension"

      run_fcis() {
        DYLINT_LIBRARY_PATH="$library_dir" \
          cargo dylint --no-metadata --no-build --lib fcis_core -- "$@"
      }

      CARGO_HOME="$dylint_home" DYLINT_LIBRARY_PATH="$library_dir" \
        cargo dylint --no-metadata --no-build --lib fcis_core -- \
          --manifest-path fixtures/dylint/missing-no-std/Cargo.toml \
          >"$driver_log" 2>&1 || true
      if [ ! -x "$DYLINT_DRIVER_PATH/$toolchain/dylint-driver" ]; then
        cat "$driver_log" >&2
        exit 1
      fi

      run_fcis --package buoy-core --lib
      run_fcis --manifest-path fixtures/dylint/allowed-boundaries/Cargo.toml

      expect_fcis_rejection() {
        manifest="$1"
        expected="$2"
        expected_count="$3"

        if run_fcis --manifest-path "$manifest" --message-format=json >"$negative_log" 2>&1; then
          echo "FCIS negative fixture unexpectedly passed: $manifest" >&2
          exit 1
        fi

        match_count="$(grep -F -c "$expected" "$negative_log" || true)"
        if [ "$match_count" -ne "$expected_count" ]; then
          echo "FCIS negative fixture emitted $match_count/$expected_count expected diagnostics: $manifest" >&2
          cat "$negative_log" >&2
          exit 1
        fi

        if grep -E '"suggested_replacement":"[^"]+"' "$negative_log" >/dev/null; then
          echo "FCIS architecture lint unexpectedly emitted an automatic fix: $manifest" >&2
          cat "$negative_log" >&2
          exit 1
        fi
      }

      expect_fcis_rejection \
        fixtures/dylint/missing-no-std/Cargo.toml \
        'this functional core crate does not declare `#![no_std]`' \
        1
      expect_fcis_rejection \
        fixtures/dylint/ambient-state/Cargo.toml \
        'this static gives the functional core ambient mutable state' \
        2
      expect_fcis_rejection \
        fixtures/dylint/callable-public-interface/Cargo.toml \
        'this exported functional-core interface exposes callable behavior' \
        6
    '';
  };

  scripts.buoy-compile-fail = {
    description = "Run pinned public core API compile-fail tripwires";
    exec = ''
      set -euo pipefail
      cargo test --locked --offline -p buoy-core --test compile_fail_contract
    '';
  };

  scripts.buoy-product-identity = {
    description = "Check Buoy naming, compatibility facades, and frozen canonical parity";
    exec = ''
      set -euo pipefail
      cargo run --quiet --bin buoy-product-identity -- --self-test
      cargo run --quiet --bin buoy-product-identity -- --check
      cargo test --package buoy --test product_identity
      cargo test --package spec-provenance --package spec-provenance-core
      cargo check --package buoy-core --lib --target thumbv7em-none-eabi

      negative_log="$(mktemp)"
      primary_stdout="$(mktemp)"
      primary_stderr="$(mktemp)"
      alias_stdout="$(mktemp)"
      alias_stderr="$(mktemp)"
      alias_without_warning="$(mktemp)"
      trap 'rm -f "$negative_log" "$primary_stdout" "$primary_stderr" "$alias_stdout" "$alias_stderr" "$alias_without_warning"' EXIT
      if cargo check \
        --manifest-path fixtures/product-identity/v1/negative/buoy-core-effect/Cargo.toml \
        --target thumbv7em-none-eabi >"$negative_log" 2>&1; then
        echo "Buoy core-effect negative fixture unexpectedly compiled" >&2
        exit 1
      fi
      grep -E 'cannot find crate|std|rustix|platform' "$negative_log" >/dev/null

      cargo build --workspace --bins
      set +e
      target/debug/buoy >"$primary_stdout" 2>"$primary_stderr"
      primary_status="$?"
      target/debug/spec-provenance >"$alias_stdout" 2>"$alias_stderr"
      alias_status="$?"
      set -e
      test "$primary_status" -eq 64
      test "$alias_status" -eq 64
      cmp "$primary_stdout" "$alias_stdout"
      grep -Fx 'warning: `spec-provenance` is deprecated; use `buoy`; compatibility expires at 0.3.0' "$alias_stderr" >/dev/null
      tail -n +2 "$alias_stderr" >"$alias_without_warning"
      cmp "$primary_stderr" "$alias_without_warning"

      target/debug/buoy static \
        --root fixtures/product-identity/v1/cli/repository \
        --config fixtures/product-identity/v1/cli/config.toml \
        --format machine >"$primary_stdout" 2>"$primary_stderr"
      target/debug/spec-provenance static \
        --root fixtures/product-identity/v1/cli/repository \
        --config fixtures/product-identity/v1/cli/config.toml \
        --format machine >"$alias_stdout" 2>"$alias_stderr"
      cmp "$primary_stdout" "$alias_stdout"
      test ! -s "$primary_stderr"
      test "$(wc -l < "$alias_stderr")" -eq 1
      grep -Fx 'warning: `spec-provenance` is deprecated; use `buoy`; compatibility expires at 0.3.0' "$alias_stderr" >/dev/null
    '';
  };

  scripts.buoy-quality = {
    description = "Check the QA registry fixtures, live claims, bindings, and generated report";
    exec = ''
      set -euo pipefail
      export CARGO_NET_OFFLINE=true
      cargo run --quiet --locked --offline --bin buoy-quality -- --self-test
      cargo run --quiet --locked --offline --bin buoy-quality -- --check
      if [ -n "''${BUOY_QUALITY_OUTPUT:-}" ]; then
        cargo run --quiet --locked --offline --bin buoy-quality -- \
          --generate --output "$BUOY_QUALITY_OUTPUT"
      fi
      cargo test --quiet --locked --offline --test quality_assurance
    '';
  };

  scripts.buoy-obligations = {
    description = "Validate ledger closure and digest-only proof negative controls";
    exec = ''
      set -euo pipefail
      cargo run --quiet --locked --offline --bin check-hardening-manifest -- --ledger-self-test
      cargo run --quiet --locked --offline --bin check-hardening-manifest -- --ledger
    '';
  };

  scripts.buoy-package-contract = {
    description = "Run the blocking structural contract check and the real local-source Nix realization for the Buoy-owned consumer package";
    # Audit M17: this gate invocation delegates to the single registered
    # consumer-package task owner instead of restating its steps.
    exec = ''
      set -euo pipefail
      nix/tools/package-contract.sh --mode blocking
    '';
  };

  scripts.buoy-schema = {
    description = "Check the machine-readable schema v3 inventory";
    exec = ''
      set -euo pipefail
      cargo run --quiet --bin check-schema-inventory -- --self-test
      cargo run --quiet --bin check-schema-inventory
      cargo test --quiet -p buoy-core --test core_contract complete_atom_surface_parsed_item_and_source_map_values_are_canonical
      cargo test --quiet -p buoy-core --test core_contract complete_task_3_4_values_derive_references_and_ignore_set_input_order
      cargo test --quiet -p buoy-core --test core_contract complete_task_3_5_values_are_canonical_and_derive_exact_references
      cargo test --quiet -p buoy-core --test core_contract graph_and_artifact_index_encoders_match_frozen_field_tables
      cargo test --quiet -p buoy-core --test core_contract newly_registered_nested_values_match_frozen_field_tables
      cargo test --quiet -p buoy-core --test core_contract model_constructors_reject_each_public_invalid_shape
      cargo test --quiet -p buoy-core --test core_contract constructor_diagnostics_cover_grammar_nfc_kind_scope_options_and_local_invariants
      cargo test --quiet -p buoy-core --test core_report_coverage_controls every_report_fact_round_trips_through_complete_reports
      cargo test --quiet -p buoy-core --test core_report_coverage_controls strict_report_decoders_reject_wrong_shapes_and_malformed_bytes
      cargo test --quiet --test integration repository_configuration_identity_is_complete_and_order_independent
      cargo test --quiet --test integration snapshot_encoders_match_frozen_field_tables
      cargo run --quiet --bin generate-property-evidence -- --check
      cargo run --quiet --bin check-property-evidence -- --self-test
      cargo run --quiet --bin check-property-evidence
    '';
  };

  scripts.buoy-unicode = {
    description = "Check Unicode 16 source identity, generated table, and conformance";
    exec = ''
      set -euo pipefail
      cargo run --quiet --example check_unicode_source
      cargo run --quiet --bin generate-casefold-table -- --self-test
      cargo run --quiet --bin generate-casefold-table -- --check
      cargo test --test unicode_conformance
    '';
  };

  scripts.buoy-reference = {
    description = "Check independent reference dependencies and canonical differential tests";
    exec = ''
      set -euo pipefail
      cargo run --quiet --bin check-reference-independence -- --self-test
      cargo run --quiet --bin check-reference-independence
      cargo run --quiet --example generate_canonical_vectors -- --check
      cargo run --quiet --example generate_malformed_vectors -- --check
      cargo test --test canonical_differential --test canonical_vectors --test malformed_canonical_vectors
    '';
  };

  scripts.buoy-fixtures = {
    description = "Check fixture contracts and independent finite models";
    exec = ''
      set -euo pipefail
      cargo run --quiet --bin generate-coverage-fixtures -- --check
      cargo run --quiet --bin generate-property-fixtures -- --check
      cargo run --quiet --bin generate-v2-fixture-suite -- --check
      cargo run --quiet --bin check-hardening-manifest -- --self-test
      cargo run --quiet --bin check-hardening-manifest
      cargo run --quiet --bin check-fixture-manifest
      cargo test --quiet --test fixture_truthfulness
      cargo test --quiet --all-features --test integration frozen_repository_fixture_manifest_executes_every_task_2_12_category -- --exact
    '';
  };

  scripts.buoy-hashing-policy = {
    description = "Check XXH3 fixtures, migration model, external boundaries, and repository policy";
    exec = ''
      set -euo pipefail
      migration_report="$(mktemp)"
      migration_replay="$(mktemp)"
      model_report="$(mktemp)"
      model_control="$(mktemp)"
      trap 'rm -f "$migration_report" "$migration_replay" "$model_report" "$model_control"' EXIT
      cargo run --quiet --bin check-digest-fixtures -- --self-test
      cargo run --quiet --bin check-digest-fixtures
      cargo run --quiet --all-features --bin check-digest-correspondence -- --self-test
      cargo run --quiet --all-features --bin check-digest-correspondence
      cargo run --quiet --bin check-digest-transition-model -- --self-test
      cargo run --quiet --bin check-digest-transition-model >"$model_report"
      grep -F 'exhaustively-checked states=4172 accepted=20 rejected=4152' "$model_report" >/dev/null
      grep -F 'transition-accepted=16 transition-rejected=4080 fixed-points=16 fixed-point=true iterations=1' "$model_report" >/dev/null
      if cargo run --quiet --bin check-digest-transition-model -- \
        --fault model-index-authority >"$model_control" 2>&1; then
        echo "digest transition model index-authority control unexpectedly passed" >&2
        exit 1
      fi
      grep -F 'DIG-MODEL-INDEX-AUTHORITY: path=model state=0x' "$model_control" >/dev/null
      if cargo run --quiet --bin check-digest-transition-model -- \
        --cutoff 4095 >"$model_control" 2>&1; then
        echo "digest transition model cutoff control unexpectedly passed" >&2
        exit 1
      fi
      grep -F 'DIG-MODEL-CUTOFF: explored=4095 declared=4096' "$model_control" >/dev/null
      cargo test --test external_integrity --test migration
      cargo test --quiet --test integration \
        correspondence_index_is_non_authoritative_for_static_identity -- --exact
      cargo run --quiet --bin buoy-migrate -- complete \
        fixtures/xxh3/v2/migration/complete-v1-input.toml >"$migration_report"
      cargo run --quiet --bin buoy-migrate -- complete \
        fixtures/xxh3/v2/migration/complete-v1-input.toml >"$migration_replay"
      cmp "$migration_report" "$migration_replay"
      grep -q '^spec-provenance/v3' "$migration_report"
      cargo run --quiet --bin buoy-hashing-policy -- --self-test
      cargo run --quiet --bin buoy-hashing-policy
    '';
  };

  scripts.buoy-identifier-policy = {
    description = "Check the typed stable-ID boundary fixtures and repository source";
    exec = ''
      set -euo pipefail
      cargo run --quiet --bin buoy-identifier-policy -- --self-test
      cargo run --quiet --bin buoy-identifier-policy
      cargo test --quiet --test identifier_policy
    '';
  };

  scripts.buoy-digest-member-oracle = {
    description = "Validate one digest member feature set with its matching repository configuration";
    exec = ''
      set -euo pipefail
      member="''${BUOY_DIGEST_MEMBER:-xxh3-128}"
      case "$member" in
        xxh3-128) feature=xxh3-128 ;;
        blake3-256) feature=blake3-256 ;;
        sha-256) feature=sha-256 ;;
        *)
          echo "DIG-MEMBER-ORACLE-UNKNOWN: $member" >&2
          exit 1
          ;;
      esac

      committed_config="fixtures/product-identity/v1/cli/config.toml"
      scratch="$(mktemp -d)"
      trap 'rm -rf "$scratch"' EXIT
      if [ "$member" = xxh3-128 ]; then
        cp "$committed_config" "$scratch/config.toml"
        cmp "$committed_config" "$scratch/config.toml"
      else
        config_source="$(<"$committed_config")"
        printf '%s\n' "''${config_source/xxh3-128/$member}" >"$scratch/config.toml"
      fi

      cargo check --workspace --all-targets --no-default-features --features "$feature"
      cargo run --quiet --locked --offline --package buoy \
        --no-default-features --features "$feature" --bin buoy -- static \
        --root fixtures/product-identity/v1/cli/repository \
        --config "$scratch/config.toml" --format machine >"$scratch/result.json"
      grep -F "\"graph\":\"$member:" "$scratch/result.json" >/dev/null
      echo "digest-member-oracle: member=$member feature=$feature configuration=matching"
    '';
  };

}
