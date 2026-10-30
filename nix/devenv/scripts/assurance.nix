{
  dylintTools,
  formalTools,
  pkgs,
  supplyChain,
  ...
}:
{
  scripts.buoy-deny = {
    description = "Enforce the pinned dependency advisory, license, source, and duplicate policy offline";
    packages = [ pkgs.cargo-deny ];
    exec =
      assert
        (import ../../tools/advisory-exceptions.nix ../../../deny.toml)
        && !(import ../../tools/advisory-exceptions.nix ../../../fixtures/supply-chain/v1/negative/wildcard-advisory.toml);
      ''
        set -euo pipefail
        metadata="$(mktemp)"
        dylint_metadata="$(mktemp)"
        hegel_metadata="$(mktemp)"
        database_root="$(mktemp -d)"
        policy="$(mktemp)"
        policy_home="$(mktemp -d)"
        workspace_policy_log="$(mktemp)"
        dylint_policy_log="$(mktemp)"
        hegel_policy_log="$(mktemp)"
        trap 'rm -f "$metadata" "$dylint_metadata" "$hegel_metadata" "$policy" "$workspace_policy_log" "$dylint_policy_log" "$hegel_policy_log"; rm -rf "$database_root" "$policy_home"' EXIT

        run_deny() {
          log="$1"
          shift
          set +e
          CARGO_HOME="$policy_home" cargo deny "$@" 2>&1 | tee "$log"
          status="''${PIPESTATUS[0]}"
          set -e
          if [ "$status" -ne 0 ] || grep -Eq '(^error\[|\[ERROR\]| FAILED)' "$log"; then
            echo "deny: cargo-deny failed closed" >&2
            exit 1
          fi
        }

        cargo metadata --locked --offline --all-features --format-version=1 >"$metadata"
        cargo metadata --locked --offline --all-features --format-version=1 \
          --manifest-path dylint_lints/fcis_core/Cargo.toml >"$dylint_metadata"
        hegel_manifest="$BUOY_CARGO_VENDOR/vendor/hegeltest-c-0.29.0/Cargo.toml"
        cargo metadata --locked --offline --all-features --format-version=1 \
          --manifest-path "$hegel_manifest" >"$hegel_metadata"

        database="$database_root/advisory-db-3157b0e258782691"
        mkdir -p "$database" "$policy_home/registry/index/index.crates.io-1949cf8c6b5b557f"
        cp -R "$BUOY_ADVISORY_DB"/. "$database"/
        cp -R assurance/supply-chain/v1/registry-index/. \
          "$policy_home/registry/index/index.crates.io-1949cf8c6b5b557f"/
        cp ${supplyChain.cargoConfig} "$policy_home/config.toml"
        chmod -R u+w "$database_root" "$policy_home"
        (
          cd "$database"
          git init --quiet
          git config user.name Buoy
          git config user.email supply-chain.invalid
          git add .
          GIT_AUTHOR_DATE=2026-10-29T14:05:38Z \
            GIT_COMMITTER_DATE=2026-10-29T14:05:38Z \
            git commit --quiet -m pinned-advisory-db
          git remote add origin https://github.com/RustSec/advisory-db
          printf "%s\t\tbranch 'main' of https://github.com/RustSec/advisory-db\n" \
            "$(git rev-parse HEAD)" >.git/FETCH_HEAD
        )
        sed "s|@advisoryDb@|$database_root|g" ${../../../deny.toml} >"$policy"

        run_deny "$workspace_policy_log" --workspace --offline --locked \
          --config "$policy" --metadata-path "$metadata" check all
        run_deny "$dylint_policy_log" \
          --manifest-path dylint_lints/fcis_core/Cargo.toml --offline --locked \
          --config "$policy" --metadata-path "$dylint_metadata" check all
        run_deny "$hegel_policy_log" --manifest-path "$hegel_manifest" \
          --offline --locked --config "$policy" \
          --metadata-path "$hegel_metadata" check all

        echo "deny: workspace=green dylint=green hegel=green wildcard-exemption=rejected"
      '';
  };

  scripts.buoy-offline-acceptance = {
    description = "Run the clean network-disabled sandbox acceptance from the declared closure";
    packages = [ supplyChain.cleanCargoRunner ];
    exec = ''
      set -euo pipefail
      buoy-clean-cargo-acceptance
      echo "offline-acceptance: green exact-final-oracle=not-run"
    '';
  };

  scripts.buoy-closure-size = {
    description = "Report deterministic closure component sizes under the size policy";
    packages = [ supplyChain.closureReportRunner ];
    exec = ''
      set -euo pipefail
      sizes="$(mktemp)"
      repeat="$(mktemp)"
      trap 'rm -f "$sizes" "$repeat"' EXIT
      buoy-closure-report >"$sizes"
      buoy-closure-report >"$repeat"
      if ! cmp -s "$sizes" "$repeat"; then
        echo "closure-size: repeated report drifted" >&2
        exit 1
      fi
      awk -F'\t' '
        NR == FNR {
          if (FNR > 1 && NF == 3) {
            budget[$1] = $2
          }
          next
        }
        FNR == 1 {
          if ($0 != "component\tplatform\tnar_bytes\tclosure_bytes\tsemantic_identity_effect\tassurance_effect") {
            print "closure-size: unexpected report header" > "/dev/stderr"
            failed = 1
            exit 1
          }
          next
        }
        {
          if (NF != 6 || $5 != "none" || $6 != "none") {
            print "closure-size: " $1 " claims a semantic or assurance effect" > "/dev/stderr"
            failed = 1
            exit 1
          }
          if (!($1 in budget)) {
            print "closure-size: " $1 " has no size budget" > "/dev/stderr"
            failed = 1
            exit 1
          }
          if ($4 + 0 > budget[$1] + 0) {
            print "closure-size: " $1 " exceeds its budget: " $4 " > " budget[$1] > "/dev/stderr"
            failed = 1
            exit 1
          }
          seen[$1] = 1
        }
        END {
          if (failed) {
            exit 1
          }
          for (component in budget) {
            if (!(component in seen)) {
              print "closure-size: " component " missing from the report" > "/dev/stderr"
              exit 1
            }
          }
        }
      ' assurance/supply-chain/v1/closure-size-policy.tsv "$sizes"
      cat "$sizes"
      echo "closure-size: deterministic=true budget=green"
    '';
  };

  scripts.buoy-formal-spikes = {
    description = "Replay equivalent Verus and Aeneas/Lean formal spikes";
    packages = [
      formalTools.aeneas
      formalTools.leanAeneas
      formalTools.verus
      formalTools.z3
    ];
    exec = ''
      set -euo pipefail
      root="$PWD"
      scratch="$(mktemp -d)"
      negative_log="$(mktemp)"
      trap 'rm -rf "$scratch" "$negative_log"' EXIT

      if grep -E -n '(^|[[:space:]])(sorry|admit)([[:space:]]|$)' \
        proofs/spikes/aeneas/lean/*.lean >"$negative_log"; then
        echo "Aeneas/Lean spike contains an unproved admission" >&2
        cat "$negative_log" >&2
        exit 1
      fi

      verus proofs/spikes/verus/integrity.rs
      verus proofs/spikes/verus/source_item_kind.rs
      verus proofs/spikes/verus/bounded_traversal.rs
      verus proofs/spikes/verus/logical_mounts.rs
      if verus proofs/spikes/verus/integrity_negative.rs >"$negative_log" 2>&1; then
        echo "Verus negative control unexpectedly verified" >&2
        exit 1
      fi
      grep -F "postcondition not satisfied" "$negative_log" >/dev/null
      if verus proofs/spikes/verus/logical_mounts_negative.rs >"$negative_log" 2>&1; then
        echo "Verus logical-mount negative control unexpectedly verified" >&2
        exit 1
      fi
      grep -F "postcondition not satisfied" "$negative_log" >/dev/null

      (
        cd proofs/spikes/aeneas/rust
        charon cargo --preset=aeneas --dest-file="$scratch/integrity.llbc"
      )
      aeneas -backend lean "$scratch/integrity.llbc" \
        -dest "$scratch" -namespace Integrity -no-progress-bar
      awk \
        '!/^@\[discriminant isize\]$/ && !/^set_option linter\.dupNamespace false$/ && !/^set_option linter\.hashCommand false$/' \
        "$scratch/Integrity.lean" >"$scratch/Integrity.normalized.lean"
      if [ "''${BUOY_WRITE_AENEAS_GOLDEN:-0}" = 1 ]; then
        cp "$scratch/Integrity.normalized.lean" \
          proofs/spikes/aeneas/generated/Integrity.lean
      else
        cmp proofs/spikes/aeneas/generated/Integrity.lean \
          "$scratch/Integrity.normalized.lean"
      fi

      cp proofs/spikes/aeneas/lean/Aeneas.lean "$scratch/Aeneas.lean"
      cp proofs/spikes/aeneas/lean/IntegrityProofs.lean "$scratch/IntegrityProofs.lean"
      cp proofs/spikes/aeneas/lean/IntegrityNegative.lean "$scratch/IntegrityNegative.lean"
      cp "$scratch/Integrity.normalized.lean" "$scratch/Integrity.lean"
      export LEAN_PATH="$scratch"
      lean --root="$scratch" -o "$scratch/Aeneas.olean" "$scratch/Aeneas.lean"
      lean --root="$scratch" -o "$scratch/Integrity.olean" "$scratch/Integrity.lean"
      lean --root="$scratch" "$scratch/IntegrityProofs.lean"
      if lean --root="$scratch" "$scratch/IntegrityNegative.lean" \
        >"$negative_log" 2>&1; then
        echo "Lean negative control unexpectedly verified" >&2
        exit 1
      fi
      grep -F "is not definitionally equal" "$negative_log" >/dev/null

      echo "formal-spikes: verus=27/27 aeneas-lean=13/13 negative-controls=3/3"
    '';
  };

  scripts.buoy-properties = {
    description = "Run the complete deterministic Hegel lane, exhaustive models, and formal checks";
    exec = ''
      set -euo pipefail
      export CARGO_NET_OFFLINE=true

      cargo run --quiet --locked --offline --bin generate-property-fixtures -- --check
      cargo run --quiet --locked --offline --bin check-property-evidence -- --self-test
      cargo run --quiet --locked --offline --bin check-reference-independence -- --self-test
      cargo run --quiet --locked --offline --bin check-reference-independence

      cargo test --locked --offline \
        --test hegel_controls \
        --test hegel_properties \
        --test logical_mount_hegel_properties \
        --test xxh3_hegel_properties
      cargo test --locked --offline --test bounded_mount_binding_model
      cargo test --locked --offline --test logical_mount_proof_bindings
      cargo test --locked --offline -p buoy-core --test xxh3_hegel_envelope
      cargo test --locked --offline --all-features \
        --test requirement_subject_inventory \
        hegel_requirement_subject_order_is_permutation_invariant -- --exact

      cargo run --quiet --locked --offline --bin generate-property-evidence
      cargo run --quiet --locked --offline --bin check-property-evidence
      echo "property-evidence: current normalized results emitted after all Hegel controls passed"

      buoy-formal-spikes
    '';
  };

  scripts.buoy-gate-wiring = {
    description = "Validate resolved phase ownership and lifecycle wiring";
    exec = ''
      set -euo pipefail
      cargo run --quiet --locked --offline --bin check-devenv-task-graph -- \
        --check --tasks "$DEVENV_TASK_FILE" --policy "$BUOY_DEVENV_POLICY" --root "$DEVENV_ROOT"
    '';
  };

  scripts.buoy-coverage-evidence = {
    description = "Reject stale or inexact recorded coverage evidence without LLVM instrumentation";
    exec = ''
      set -euo pipefail
      cargo run --quiet --locked --offline --bin check-exact-coverage -- --check-inventory
      cargo run --quiet --locked --offline --bin check-exact-coverage -- \
        --check-result assurance/v1/coverage-result.tsv
      cargo run --quiet --locked --offline --bin check-hardening-manifest -- \
        --coverage-evidence
    '';
  };

  scripts.buoy-coverage = {
    description = "Instrument the complete reviewed source inventory and require exact four-metric closure";
    packages = [
      pkgs.cargo-llvm-cov
      dylintTools
    ];
    exec = ''
      set -euo pipefail
      export CARGO_NET_OFFLINE=true
      # Every root-manifest coverage run must reuse the all-feature library image
      # produced by the workspace pass; a default-feature rebuild creates a
      # second partially observed copy of shared library functions.
      cargo() {
        if [ "$#" -ge 2 ] && [ "$1" = "llvm-cov" ] && [ "$2" = "run" ]; then
          shift 2
          command cargo llvm-cov run --all-features "$@"
        else
          command cargo "$@"
        fi
      }
      workspace_report="$(mktemp)"
      workspace_binding="$(mktemp)"
      examples_dir="$(mktemp -d)"
      standalone_dir="$(mktemp -d)"
      dylint_report="$(mktemp)"
      dylint_binding="$(mktemp)"
      normalized_result="$(mktemp)"
      trap 'rm -f "$workspace_report" "$workspace_binding" "$dylint_report" "$dylint_binding" "$normalized_result"; rm -rf "$examples_dir" "$standalone_dir"' EXIT

      test "$(cargo llvm-cov --version)" = "cargo-llvm-cov 0.8.5"
      cargo run --quiet --bin generate-coverage-fixtures -- --check
      cargo run --quiet --bin check-exact-coverage -- --self-test
      cargo run --quiet --bin check-exact-coverage -- --check-inventory

      cargo llvm-cov clean --workspace
      cargo llvm-cov clean --manifest-path dylint_lints/fcis_core/Cargo.toml
      cargo llvm-cov --workspace --all-targets --all-features --branch \
        --json --output-path "$workspace_report" --locked --offline -- \
        --skip committed_fuzz_and_mutation_campaigns_are_current_in_their_exact_scopes \
        --skip stale_expired_or_discharged_campaign_evidence_cannot_satisfy_an_obligation
      chelis_target="$standalone_dir/chelis-target"
      RUSTC_WRAPPER= CARGO_BUILD_RUSTC_WRAPPER= \
        CARGO_LLVM_COV_TARGET_DIR="$chelis_target" \
        CARGO_LLVM_COV_BUILD_DIR="$chelis_target" \
        cargo llvm-cov test --branch --all-features --locked --offline \
        -p chelis-provenance -p chelis-surf-sitter --all-targets --no-report
      chelis_controls="$standalone_dir/chelis-cli-controls"
      mkdir -p "$chelis_controls/chelis"
      printf '%s\n' \
        'schema = "chelis-adapter-config/v1"' \
        'binding-store = "chelis/objects"' \
        '[[selector]]' 'root = "chelis"' 'dialect = "surf"' \
        >"$chelis_controls/config.toml"
      printf '%s\n' 'def clean(x: f32) -> f32 = x' >"$chelis_controls/chelis/control.ch"
      RUSTC_WRAPPER= CARGO_BUILD_RUSTC_WRAPPER= \
        CARGO_LLVM_COV_TARGET_DIR="$chelis_target" \
        CARGO_LLVM_COV_BUILD_DIR="$chelis_target" \
        command cargo llvm-cov run --branch --all-features --locked --offline \
        -p chelis-provenance --bin chelis-provenance --no-report -- \
        static --root "$chelis_controls" --config "$chelis_controls/config.toml"
      for feature in blake3-256 sha-256; do
        find "$chelis_target" -path '*/build/*' -exec chmod u+w {} +
        RUSTC_WRAPPER= CARGO_BUILD_RUSTC_WRAPPER= \
          CARGO_LLVM_COV_TARGET_DIR="$chelis_target" \
          CARGO_LLVM_COV_BUILD_DIR="$chelis_target" \
          cargo llvm-cov test --branch --no-default-features \
          --features "$feature" --locked --offline -p chelis-provenance \
          --test bind_conversion --test binding_regeneration \
          --test compact_bindings --test trace_projection --no-report
        RUSTC_WRAPPER= CARGO_BUILD_RUSTC_WRAPPER= \
          CARGO_LLVM_COV_TARGET_DIR="$chelis_target" \
          CARGO_LLVM_COV_BUILD_DIR="$chelis_target" \
          command cargo llvm-cov run --branch --no-default-features \
          --features "$feature" --locked --offline -p chelis-provenance \
          --bin chelis-provenance --no-report -- static --root "$chelis_controls" \
          --config "$chelis_controls/config.toml"
        chelis_zero="$feature:$(printf '%064d' 0)"
        cp "$chelis_controls/config.toml" "$chelis_controls/configured.toml"
        for chelis_case in navigation atoms stale store; do
          chelis_atoms="CLI-ATOM@$chelis_zero"
          chelis_navigation=implements
          case "$chelis_case" in
            navigation)
              chelis_navigation=consumes
              chelis_reason='bind: the compact form carries only implements, not consumes'
              ;;
            atoms)
              chelis_atoms="CLI-ATOM@$chelis_zero;OTHER-ATOM@$chelis_zero"
              chelis_reason='bind: the compact form binds exactly one atom: CLI-LINK:2'
              ;;
            stale)
              chelis_reason="bind: the link's item digest is not current:"
              ;;
            store)
              chelis_reason=
              printf '%s\n' 'schema = "chelis-adapter-config/v1"' \
                '[[selector]]' 'root = "chelis"' 'dialect = "surf"' \
                >"$chelis_controls/config.toml"
              ;;
          esac
          printf '%s\n' '-- chelis:provenance/v1 implementation-link' \
            '-- id = CLI-LINK' "-- atoms = $chelis_atoms" \
            "-- item-digest = $chelis_zero" "-- navigation = $chelis_navigation" \
            'def clean(x: f32) -> f32 = x' >"$chelis_controls/chelis/control.ch"
          cp "$chelis_controls/chelis/control.ch" "$chelis_controls/before.ch"
          chelis_status=0
          RUSTC_WRAPPER= CARGO_BUILD_RUSTC_WRAPPER= \
            CARGO_LLVM_COV_TARGET_DIR="$chelis_target" \
            CARGO_LLVM_COV_BUILD_DIR="$chelis_target" \
            command cargo llvm-cov run --branch --no-default-features \
            --features "$feature" --locked --offline -p chelis-provenance \
            --bin chelis-provenance --no-report -- bind --root "$chelis_controls" \
            --config "$chelis_controls/config.toml" --link CLI-LINK --write \
            >"$chelis_controls/$chelis_case.stdout" \
            2>"$chelis_controls/$chelis_case.stderr" || chelis_status="$?"
          test "$chelis_status" -eq 70
          if [ -n "$chelis_reason" ]; then
            grep -F "$chelis_reason" "$chelis_controls/$chelis_case.stderr" >/dev/null
          fi
          test ! -s "$chelis_controls/$chelis_case.stdout"
          cmp "$chelis_controls/before.ch" "$chelis_controls/chelis/control.ch"
          test ! -e "$chelis_controls/chelis/objects"
        done
        cp "$chelis_controls/configured.toml" "$chelis_controls/config.toml"
        printf '%s\n' '-- chelis:provenance/v1 authority' '-- id = CLI-ATOM' \
          '-- kind = behavioral' '-- scopes = cli' \
          '-- statement = The command MUST preserve saved bytes.' \
          'def subject(x: f32) -> f32 = x' \
          '-- chelis:provenance/v1 implementation-link' '-- id = CLI-LINK' \
          "-- atoms = CLI-ATOM@$chelis_zero" "-- item-digest = $chelis_zero" \
          '-- navigation = implements' 'def clean(x: f32) -> f32 = x' \
          >"$chelis_controls/chelis/control.ch"
        chelis_arguments=(--root "$chelis_controls" --config "$chelis_controls/config.toml" --write)
        for chelis_command in rebind bind; do
          if [ "$chelis_command" = bind ]; then
            chelis_arguments+=(--link CLI-LINK)
            cp "$chelis_controls/chelis/control.ch" "$chelis_controls/before-bind.ch"
          fi
          RUSTC_WRAPPER= CARGO_BUILD_RUSTC_WRAPPER= \
            CARGO_LLVM_COV_TARGET_DIR="$chelis_target" \
            CARGO_LLVM_COV_BUILD_DIR="$chelis_target" \
            command cargo llvm-cov run --branch --no-default-features \
            --features "$feature" --locked --offline -p chelis-provenance \
            --bin chelis-provenance --no-report -- "$chelis_command" \
            "''${chelis_arguments[@]}"
        done
        test -d "$chelis_controls/chelis/objects"
        cp "$chelis_controls/before-bind.ch" "$chelis_controls/chelis/control.ch"
        chmod u-w "$chelis_controls/chelis/control.ch"
        chelis_status=0
        RUSTC_WRAPPER= CARGO_BUILD_RUSTC_WRAPPER= \
          CARGO_LLVM_COV_TARGET_DIR="$chelis_target" \
          CARGO_LLVM_COV_BUILD_DIR="$chelis_target" \
          command cargo llvm-cov run --branch --no-default-features \
          --features "$feature" --locked --offline -p chelis-provenance \
          --bin chelis-provenance --no-report -- bind --root "$chelis_controls" \
          --config "$chelis_controls/config.toml" --link CLI-LINK --write \
          >"$chelis_controls/read-only.stdout" \
          2>"$chelis_controls/read-only.stderr" || chelis_status="$?"
        chmod u+w "$chelis_controls/chelis/control.ch"
        test "$chelis_status" -eq 70
        cmp "$chelis_controls/before-bind.ch" "$chelis_controls/chelis/control.ch"
        chelis_object="$(find "$chelis_controls/chelis/objects" -type f)"
        printf '%s\n' occupied >"$chelis_controls/collision.bytes"
        cp "$chelis_controls/collision.bytes" "$chelis_object"
        chelis_status=0
        RUSTC_WRAPPER= CARGO_BUILD_RUSTC_WRAPPER= \
          CARGO_LLVM_COV_TARGET_DIR="$chelis_target" \
          CARGO_LLVM_COV_BUILD_DIR="$chelis_target" \
          command cargo llvm-cov run --branch --no-default-features \
          --features "$feature" --locked --offline -p chelis-provenance \
          --bin chelis-provenance --no-report -- bind --root "$chelis_controls" \
          --config "$chelis_controls/config.toml" --link CLI-LINK --write \
          >"$chelis_controls/collision.stdout" \
          2>"$chelis_controls/collision.stderr" || chelis_status="$?"
        test "$chelis_status" -eq 70
        grep -F 'bind: the destination object holds different bytes:' \
          "$chelis_controls/collision.stderr" >/dev/null
        cmp "$chelis_controls/before-bind.ch" "$chelis_controls/chelis/control.ch"
        cmp "$chelis_controls/collision.bytes" "$chelis_object"
        rm -rf "$chelis_controls/chelis/objects"
        printf '%s\n' 'def clean(x: f32) -> f32 = x' >"$chelis_controls/chelis/control.ch"
      done
      chelis_state_controls="$standalone_dir/chelis-state-controls"
      mkdir -p "$chelis_state_controls"
      chelis_cli() {
        local log="$1" expected="$2" status=0
        shift 2
        printf '%q ' env RUSTC_WRAPPER= CARGO_BUILD_RUSTC_WRAPPER= \
          "CARGO_LLVM_COV_TARGET_DIR=$chelis_target" "CARGO_LLVM_COV_BUILD_DIR=$chelis_target" \
          cargo llvm-cov run --branch "''${chelis_image[@]}" --locked --offline \
          -p chelis-provenance --bin chelis-provenance --no-report -- "$@" >"$log.command"
        printf '\n' >>"$log.command"
        RUSTC_WRAPPER= CARGO_BUILD_RUSTC_WRAPPER= \
          CARGO_LLVM_COV_TARGET_DIR="$chelis_target" \
          CARGO_LLVM_COV_BUILD_DIR="$chelis_target" \
          command cargo llvm-cov run --branch "''${chelis_image[@]}" --locked --offline \
          -p chelis-provenance --bin chelis-provenance --no-report -- "$@" \
          >"$log.stdout" 2>"$log.stderr" || status="$?"
        printf '%s\n' "$status" >"$log.exit"
        if [ "$status" -ne "$expected" ]; then
          printf 'chelis: %s expected exit %s, observed %s\n' "$log" "$expected" "$status" >&2
          return 1
        fi
      }
      chelis_authority() {
        printf '%s\n' '-- chelis:provenance/v1 authority' '-- id = CLI-ATOM' \
          '-- kind = behavioral' '-- scopes = cli' "-- statement = $2" \
          'def subject(x: f32) -> f32 = x' >"$1/chelis/authority.ch"
      }
      for feature in blake3-256 sha-256; do
        chelis_image=(--no-default-features --features "$feature")
        seed="$chelis_state_controls/$feature-seed"
        mkdir -p "$seed/chelis"
        printf '%s\n' 'schema = "chelis-adapter-config/v1"' \
          'binding-store = "chelis/objects"' '[[selector]]' 'root = "chelis"' \
          'dialect = "surf"' >"$seed/config.toml"
        chelis_authority "$seed" 'The command MUST preserve saved bytes.'
        zero="$feature:$(printf '%064d' 0)"
        printf '%s\n' '-- chelis:provenance/v1 implementation-link' '-- id = CLI-LINK' \
          "-- atoms = CLI-ATOM@$zero" "-- item-digest = $zero" \
          '-- navigation = implements' 'def clean(x: f32) -> f32 = x' \
          >"$seed/chelis/control.ch"
        chelis_cli "$seed/repair" 0 rebind --root "$seed" --config "$seed/config.toml" --write
        cp "$seed/chelis/control.ch" "$seed/inline.ch"
        chelis_cli "$seed/convert" 0 bind --root "$seed" --config "$seed/config.toml" --link CLI-LINK --write
        cp "$seed/chelis/control.ch" "$seed/compact.ch"
        object="$(find "$seed/chelis/objects" -type f)"
        cp "$object" "$seed/object.bytes"
        case_root="$chelis_state_controls/$feature-identical-bind"
        cp -R "$seed" "$case_root"
        cp "$case_root/inline.ch" "$case_root/chelis/control.ch"
        chelis_cli "$case_root/write" 0 bind --root "$case_root" --config "$case_root/config.toml" --link CLI-LINK --write
        cmp "$case_root/compact.ch" "$case_root/chelis/control.ch"
        object="$(find "$case_root/chelis/objects" -type f)"
        cmp "$case_root/object.bytes" "$object"
        for control in successor collision read-only blocked-store identical-rebind missing-authority malformed-authority; do
          case_root="$chelis_state_controls/$feature-$control"
          cp -R "$seed" "$case_root"
          if [ "$control" = missing-authority ]; then
            printf '%s\n' 'def subject(x: f32) -> f32 = x' >"$case_root/chelis/authority.ch"
          elif [ "$control" = malformed-authority ]; then
            printf '%s\n' '-- chelis:provenance/v1 authority' '-- id = CLI-ATOM' \
              '-- kind = behavioral' '-- scopes = cli' 'def subject(x: f32) -> f32 = x' \
              >"$case_root/chelis/authority.ch"
          else
            chelis_authority "$case_root" 'The command MUST preserve all saved bytes.'
          fi
          cp "$case_root/chelis/control.ch" "$case_root/before.ch"
          cp "$case_root/chelis/authority.ch" "$case_root/authority.before.ch"
          old_object="$(find "$case_root/chelis/objects" -type f)"
          cp "$old_object" "$case_root/old-object.bytes"
          chelis_cli "$case_root/plan" 0 rebind --root "$case_root" --config "$case_root/config.toml"
          if [ "$control" = missing-authority ] || [ "$control" = malformed-authority ]; then
            ${pkgs.python3}/bin/python3 -c 'import json,sys; assert json.load(open(sys.argv[1]))["entries"] == []' "$case_root/plan.stdout"
            chelis_cli "$case_root/static" 2 static --root "$case_root" --config "$case_root/config.toml"
            grep -F CHELIS-PROV-BINDING-REQUIREMENT-STALE "$case_root/static.stdout" >/dev/null
            grep -F 'current=absent' "$case_root/static.stdout" >/dev/null
            if [ "$control" = malformed-authority ]; then
              grep -F 'missing-field:statement' "$case_root/static.stdout" >/dev/null
            fi
            chelis_cli "$case_root/write" 0 rebind --root "$case_root" --config "$case_root/config.toml" --write
            cmp "$case_root/before.ch" "$case_root/chelis/control.ch"
            cmp "$case_root/authority.before.ch" "$case_root/chelis/authority.ch"
            cmp "$case_root/old-object.bytes" "$old_object"
            continue
          fi
          successor="$(${pkgs.python3}/bin/python3 -c 'import json,sys; rows=json.load(open(sys.argv[1]))["entries"]; print(next(r["path"] for r in rows if r["action"] == "object-write"))' "$case_root/plan.stdout")"
          case "$control" in
            successor)
              chelis_cli "$case_root/write" 0 rebind --root "$case_root" --config "$case_root/config.toml" --write
              test ! -e "$old_object"
              test -f "$case_root/$successor"
              if cmp -s "$case_root/before.ch" "$case_root/chelis/control.ch"; then exit 1; fi
              chelis_cli "$case_root/converged" 0 rebind --root "$case_root" --config "$case_root/config.toml"
              ${pkgs.python3}/bin/python3 -c 'import json,sys; assert json.load(open(sys.argv[1]))["entries"] == []' "$case_root/converged.stdout"
              ;;
            collision)
              printf '%s\n' occupied >"$case_root/collision.bytes"
              cp "$case_root/collision.bytes" "$case_root/$successor"
              chelis_cli "$case_root/write" 70 rebind --root "$case_root" --config "$case_root/config.toml" --write
              grep -F 'rebind: the destination object holds different bytes:' "$case_root/write.stderr" >/dev/null
              cmp "$case_root/collision.bytes" "$case_root/$successor"
              ;;
            read-only)
              chmod u-w "$case_root/chelis/control.ch"
              chelis_cli "$case_root/write" 70 rebind --root "$case_root" --config "$case_root/config.toml" --write
              chmod u+w "$case_root/chelis/control.ch"
              test -f "$case_root/$successor"
              ;;
            blocked-store)
              chmod u-w "$(dirname "$old_object")"
              chelis_cli "$case_root/write" 70 rebind --root "$case_root" --config "$case_root/config.toml" --write
              chmod u+w "$(dirname "$old_object")"
              test ! -e "$case_root/$successor"
              ;;
            identical-rebind)
              donor="$chelis_state_controls/$feature-successor"
              cp "$donor/$successor" "$case_root/$successor"
              cp "$case_root/$successor" "$case_root/successor.bytes"
              chelis_cli "$case_root/write" 0 rebind --root "$case_root" --config "$case_root/config.toml" --write
              cmp "$case_root/successor.bytes" "$case_root/$successor"
              cmp "$donor/chelis/control.ch" "$case_root/chelis/control.ch"
              test ! -e "$old_object"
              ;;
          esac
          cmp "$case_root/authority.before.ch" "$case_root/chelis/authority.ch"
          if [ "$control" != successor ] && [ "$control" != identical-rebind ]; then
            test ! -s "$case_root/write.stdout"
            cmp "$case_root/before.ch" "$case_root/chelis/control.ch"
            cmp "$case_root/old-object.bytes" "$old_object"
          fi
        done
        case_root="$chelis_state_controls/$feature-diverged"
        mkdir -p "$case_root/chelis"
        cp "$seed/config.toml" "$case_root/config.toml"
        printf '%s\n' '-- chelis:provenance/v1 oracle' '-- id = CLI-ORACLE' \
          '-- oracle-kind = test' '-- runner-id = chelis-test' '-- runner-version = 1.0.0' \
          "-- runner-digest = $zero" "-- runner-configuration-digest = $zero" \
          '-- runner-config-paths = chelis/control.ch' '-- selection-schema = chelis-oracle-selection/v1' \
          '-- selection = selection' '-- normalization-schema = chelis-oracle-normalization/v1' \
          '-- normalization = normalization' "-- configuration-digest = $zero" \
          '-- scope-schema = chelis-oracle-scope/v1' '-- scope = cli' \
          'def clean(x: f32) -> f32 = x' >"$case_root/chelis/control.ch"
        cp "$case_root/chelis/control.ch" "$case_root/before.ch"
        chelis_cli "$case_root/write" 70 rebind --root "$case_root" --config "$case_root/config.toml" --write
        grep -F 'rebind: saved bytes changed before the plan converged' "$case_root/write.stderr" >/dev/null
        test ! -s "$case_root/write.stdout"
        if cmp -s "$case_root/before.ch" "$case_root/chelis/control.ch"; then exit 1; fi
        chelis_cli "$case_root/remaining" 0 rebind --root "$case_root" --config "$case_root/config.toml"
        ${pkgs.python3}/bin/python3 -c 'import json,sys; rows=json.load(open(sys.argv[1]))["entries"]; assert any(r["field"] == "runner-digest" and r["bound"] != r["current"] for r in rows)' "$case_root/remaining.stdout"
      done
      for feature in blake3-256 sha-256; do
        chelis_image=(--no-default-features --features "$feature")
        other=sha-256
        if [ "$feature" = sha-256 ]; then other=blake3-256; fi
        case_root="$chelis_state_controls/$feature-cross-image"
        cp -R "$chelis_state_controls/$other-seed" "$case_root"
        cp "$case_root/chelis/control.ch" "$case_root/before.ch"
        chelis_cli "$case_root/plan" 0 rebind --root "$case_root" --config "$case_root/config.toml" --write
        ${pkgs.python3}/bin/python3 -c 'import json,sys; report=json.load(open(sys.argv[1])); assert report["entries"] == [] and not report["written"]' "$case_root/plan.stdout"
        cmp "$case_root/before.ch" "$case_root/chelis/control.ch"
        object="$(find "$case_root/chelis/objects" -type f)"
        cmp "$case_root/object.bytes" "$object"
      done
      chelis_image=(--all-features)
      case_root="$chelis_state_controls/oversized"
      mkdir -p "$case_root/chelis"
      printf '%s\n' 'def clean(x: f32) -> f32 = x' >"$case_root/chelis/control.ch"
      printf '%s\n' 'schema = "chelis-change-manifest/v1"' >"$case_root/manifest.toml"
      printf '%s\n' 'oracle = []' >"$case_root/oracles.toml"
      ${pkgs.python3}/bin/python3 -c 'import pathlib,sys; pathlib.Path(sys.argv[1]).write_text("schema = \"chelis-adapter-config/v1\"\n[[selector]]\nroot = \"chelis\"\ndialect = \"surf\"\n#" + "x" * 1048576 + "\n")' "$case_root/config.toml"
      cp "$case_root/config.toml" "$case_root/config.before.toml"
      cp "$case_root/chelis/control.ch" "$case_root/before.ch"
      for chelis_command in static change trace advisory execute; do
        chelis_arguments=(--root "$case_root" --config "$case_root/config.toml")
        case "$chelis_command" in
          change) chelis_arguments+=(--base "$case_root" --head "$case_root" --manifest "$case_root/manifest.toml");;
          trace) chelis_arguments+=(--id CLI-ATOM);;
          execute) chelis_arguments+=(--oracles "$case_root/oracles.toml");;
        esac
        chelis_cli "$case_root/$chelis_command" 70 "$chelis_command" "''${chelis_arguments[@]}"
        test ! -s "$case_root/$chelis_command.stdout"
        cmp "$case_root/config.before.toml" "$case_root/config.toml"
        cmp "$case_root/before.ch" "$case_root/chelis/control.ch"
      done
      chelis_image=(--no-default-features)
      for control in selected artifact-only; do
        case_root="$chelis_state_controls/no-feature-$control"
        mkdir -p "$case_root/chelis"
        cp "$chelis_state_controls/blake3-256-seed/config.toml" "$case_root/config.toml"
        if [ "$control" = selected ]; then
          printf '%s\n' 'def clean(x: f32) -> f32 = x' >"$case_root/chelis/control.ch"
        else
          printf '%s\n' artifact >"$case_root/chelis/control.bin"
        fi
        cp -R "$case_root/chelis" "$case_root/before"
        chelis_cli "$case_root/rebind" 70 rebind --root "$case_root" --config "$case_root/config.toml" --write
        test ! -s "$case_root/rebind.stdout"
        diff -r "$case_root/before" "$case_root/chelis"
        test ! -e "$case_root/chelis/objects"
      done
      unset -f chelis_cli chelis_authority
      printf '%s\n' \
        '-- chelis:provenance/v1 implementation-link' '-- id = CLI-LINK' \
        'def clean(x: f32) -> f32 = x' >"$chelis_controls/chelis/control.ch"
      cp "$chelis_controls/chelis/control.ch" "$chelis_controls/before.ch"
      chelis_missing_member_status=0
      RUSTC_WRAPPER= CARGO_BUILD_RUSTC_WRAPPER= \
        CARGO_LLVM_COV_TARGET_DIR="$chelis_target" \
        CARGO_LLVM_COV_BUILD_DIR="$chelis_target" \
        command cargo llvm-cov run --branch --no-default-features \
        --locked --offline -p chelis-provenance --bin chelis-provenance \
        --no-report -- bind --root "$chelis_controls" \
        --config "$chelis_controls/config.toml" --link CLI-LINK --write \
        >"$chelis_controls/missing-member.stdout" \
        2>"$chelis_controls/missing-member.stderr" || chelis_missing_member_status="$?"
      test "$chelis_missing_member_status" -eq 70
      grep -F 'bind: this build cannot address content: algorithm-unavailable:DIG-ALGORITHM-UNAVAILABLE' \
        "$chelis_controls/missing-member.stderr" >/dev/null
      test ! -s "$chelis_controls/missing-member.stdout"
      cmp "$chelis_controls/before.ch" "$chelis_controls/chelis/control.ch"
      test ! -e "$chelis_controls/chelis/objects"
      RUSTC_WRAPPER= CARGO_BUILD_RUSTC_WRAPPER= \
        CARGO_LLVM_COV_TARGET_DIR="$chelis_target" \
        CARGO_LLVM_COV_BUILD_DIR="$chelis_target" \
        cargo llvm-cov report --branch --include-build-script --json \
        -p chelis-provenance -p chelis-surf-sitter \
        --output-path "$standalone_dir/chelis-feature-matrix.json" --locked --offline
      migration_target="$standalone_dir/migration-target"
      CARGO_LLVM_COV_TARGET_DIR="$migration_target" \
        CARGO_LLVM_COV_BUILD_DIR="$migration_target" \
        cargo llvm-cov test --branch --no-clean --no-default-features \
        --features xxh3-128 --locked --offline --test migration \
        migration_rejects_a_registered_target_that_is_unavailable_in_the_build -- --exact
      CARGO_LLVM_COV_TARGET_DIR="$migration_target" \
        CARGO_LLVM_COV_BUILD_DIR="$migration_target" \
        cargo llvm-cov test --branch --no-clean --no-default-features \
        --locked --offline --test assurance_coverage_controls \
        repository_default_digest_paths_fail_when_the_build_has_no_member -- --exact
      # The isolated target prevents feature-matrix profiles from changing later exports.
      CARGO_LLVM_COV_TARGET_DIR="$migration_target" \
        CARGO_LLVM_COV_BUILD_DIR="$migration_target" \
        cargo llvm-cov report --branch --json \
        --ignore-filename-regex 'src/(adapter_sdk\.rs|adapters/.*|check\.rs|cli\.rs)' \
        --output-path "$standalone_dir/migration-feature-matrix.json" --locked --offline
      CARGO_LLVM_COV_TARGET_DIR=target/llvm-cov-target \
        CARGO_LLVM_COV_BUILD_DIR=target/llvm-cov-target \
        cargo llvm-cov report --manifest-path crates/buoy-core/Cargo.toml \
        --branch --json --output-path "$standalone_dir/core-workspace.json" \
        --locked --offline
      core_matrix_target="$standalone_dir/core-matrix-target"
      CARGO_LLVM_COV_TARGET_DIR="$core_matrix_target" \
        CARGO_LLVM_COV_BUILD_DIR="$core_matrix_target" \
        cargo llvm-cov test --branch --manifest-path crates/buoy-core/Cargo.toml \
        --all-features --json --output-path "$standalone_dir/core-features-all.json" \
        --locked --offline --test digest_registry_contract \
        feature_profiles_cover_default_dispatch_and_error_translation -- --exact
      for feature in blake3-256 sha-256; do
        CARGO_LLVM_COV_TARGET_DIR="$core_matrix_target" \
          CARGO_LLVM_COV_BUILD_DIR="$core_matrix_target" \
          cargo llvm-cov test --branch --no-clean \
          --manifest-path crates/buoy-core/Cargo.toml --no-default-features \
          --features "coverage-controls,$feature" --json \
          --output-path "$standalone_dir/core-features-$feature.json" \
          --locked --offline --test digest_registry_contract \
          feature_profiles_cover_default_dispatch_and_error_translation -- --exact
      done
      CARGO_LLVM_COV_TARGET_DIR="$core_matrix_target" \
        CARGO_LLVM_COV_BUILD_DIR="$core_matrix_target" \
        cargo llvm-cov test --branch --no-clean \
        --manifest-path crates/buoy-core/Cargo.toml --no-default-features \
        --features coverage-controls --json \
        --output-path "$standalone_dir/core-features-none.json" \
        --locked --offline --test digest_registry_contract \
        feature_profiles_cover_default_dispatch_and_error_translation -- --exact

      cargo llvm-cov run --branch --no-report --locked --offline \
        --example check_unicode_source -- --coverage-control
      unicode_binary="target/llvm-cov-target/debug/examples/check_unicode_source"
      if LLVM_PROFILE_FILE="$PWD/target/llvm-cov-target/unicode-%p-%m.profraw" \
        "$unicode_binary" --error-control >/dev/null 2>&1; then
        echo "Unicode source error control unexpectedly passed" >&2
        exit 1
      fi
      if LLVM_PROFILE_FILE="$PWD/target/llvm-cov-target/unicode-%p-%m.profraw" \
        "$unicode_binary" invalid >/dev/null 2>&1; then
        echo "Unicode source usage control unexpectedly passed" >&2
        exit 1
      fi
      LLVM_PROFILE_FILE="$PWD/target/llvm-cov-target/unicode-%p-%m.profraw" \
        "$unicode_binary"
      cargo llvm-cov report --branch --no-default-ignore-filename-regex \
        --json --output-path "$examples_dir/check-unicode.json" --locked --offline
      cargo llvm-cov run --branch --no-clean --no-default-ignore-filename-regex \
        --json --output-path "$examples_dir/report-goldens.json" \
        --locked --offline --example generate_report_goldens >/dev/null
      cargo llvm-cov run --branch --no-report --locked --offline \
        --example generate_presentation_goldens -- json >/dev/null
      presentation_binary="target/llvm-cov-target/debug/examples/generate_presentation_goldens"
      if LLVM_PROFILE_FILE="$PWD/target/llvm-cov-target/presentation-%p-%m.profraw" \
        "$presentation_binary" invalid >/dev/null 2>&1; then
        echo "presentation golden usage control unexpectedly passed" >&2
        exit 1
      fi
      LLVM_PROFILE_FILE="$PWD/target/llvm-cov-target/presentation-%p-%m.profraw" \
        "$presentation_binary" human >/dev/null
      cargo llvm-cov report --branch --no-default-ignore-filename-regex \
        --json --output-path "$examples_dir/presentation-goldens.json" --locked --offline

      cargo llvm-cov run --branch --no-report --locked --offline \
        --example generate_canonical_vectors -- --coverage-control
      canonical_binary="target/llvm-cov-target/debug/examples/generate_canonical_vectors"
      if LLVM_PROFILE_FILE="$PWD/target/llvm-cov-target/canonical-%p-%m.profraw" \
        "$canonical_binary" invalid >/dev/null 2>&1; then
        echo "canonical vector usage control unexpectedly passed" >&2
        exit 1
      fi
      LLVM_PROFILE_FILE="$PWD/target/llvm-cov-target/canonical-%p-%m.profraw" \
        "$canonical_binary" --check
      cargo llvm-cov report --branch --no-default-ignore-filename-regex \
        --json --output-path "$examples_dir/canonical-vectors.json" --locked --offline

      cargo llvm-cov run --branch --no-report --locked --offline \
        --example generate_malformed_vectors -- --coverage-control
      malformed_binary="target/llvm-cov-target/debug/examples/generate_malformed_vectors"
      if LLVM_PROFILE_FILE="$PWD/target/llvm-cov-target/malformed-%p-%m.profraw" \
        "$malformed_binary" invalid >/dev/null 2>&1; then
        echo "malformed vector usage control unexpectedly passed" >&2
        exit 1
      fi
      LLVM_PROFILE_FILE="$PWD/target/llvm-cov-target/malformed-%p-%m.profraw" \
        "$malformed_binary" --check
      cargo llvm-cov report --branch --no-default-ignore-filename-regex \
        --json --output-path "$examples_dir/malformed-vectors.json" --locked --offline

      cargo llvm-cov run --branch --no-clean --locked --offline --bin check-fixture-manifest -- --self-test
      cargo llvm-cov run --branch --no-clean --locked --offline --bin check-fixture-manifest
      cargo llvm-cov run --branch --no-clean --locked --offline --bin check-schema-inventory -- --self-test
      # Run write coverage only after the parallel workspace test process exits.
      # This order prevents transient repository bytes during Nix source snapshots.
      cargo llvm-cov run --branch --no-clean --locked --offline --bin check-schema-inventory -- --write-construction-fixtures
      cargo llvm-cov run --branch --no-clean --locked --offline --bin check-schema-inventory -- --write-execution-evidence
      cargo llvm-cov run --branch --no-clean --locked --offline --bin check-schema-inventory
      cargo llvm-cov run --branch --no-clean --locked --offline --bin check-property-evidence -- --self-test
      cargo llvm-cov run --branch --no-clean --locked --offline --bin check-property-evidence
      cargo llvm-cov run --branch --no-clean --locked --offline --bin check-reference-independence -- --self-test
      cargo llvm-cov run --branch --no-clean --locked --offline --bin check-reference-independence
      cargo llvm-cov run --branch --no-clean --locked --offline --bin check-digest-transition-model -- --self-test
      cargo llvm-cov run --branch --no-clean --locked --offline --bin check-digest-transition-model
      cargo llvm-cov run --branch --no-clean --locked --offline --bin check-digest-transition-model -- --cutoff 4096
      if cargo llvm-cov run --branch --no-clean --locked --offline --bin check-digest-transition-model -- --cutoff invalid >/dev/null 2>&1; then
        echo "digest transition model invalid-cutoff control unexpectedly passed" >&2
        exit 1
      fi
      if cargo llvm-cov run --branch --no-clean --locked --offline --bin check-digest-transition-model -- --invalid >/dev/null 2>&1; then
        echo "digest transition model usage control unexpectedly passed" >&2
        exit 1
      fi
      if cargo llvm-cov run --branch --no-clean --locked --offline --bin check-digest-transition-model -- --invalid 1 >/dev/null 2>&1; then
        echo "digest transition model paired-usage control unexpectedly passed" >&2
        exit 1
      fi
      dtm_binary="$PWD/target/llvm-cov-target/debug/check-digest-transition-model"
      dtm_profile="$PWD/target/llvm-cov-target/dtm-%p-%m.profraw"
      if LLVM_PROFILE_FILE="$dtm_profile" \
        "$dtm_binary" --fault model-index-authority >/dev/null 2>&1; then
        echo "digest transition model fault control unexpectedly passed" >&2
        exit 1
      fi
      dtm_control="$(mktemp -d)"
      if (
        cd "$dtm_control"
        LLVM_PROFILE_FILE="$dtm_profile" "$dtm_binary" >/dev/null 2>&1
      ); then
        echo "digest transition model missing-contract control unexpectedly passed" >&2
        exit 1
      fi
      rm -rf "$dtm_control"
      cargo llvm-cov run --branch --no-clean --locked --offline --bin check-digest-fixtures -- --self-test
      cargo llvm-cov run --branch --no-clean --locked --offline --bin check-digest-fixtures
      cargo llvm-cov run --branch --no-clean --locked --offline --bin check-digest-fixtures -- --write-execution-evidence
      if cargo llvm-cov run --branch --no-clean --locked --offline --bin check-digest-fixtures -- --invalid >/dev/null 2>&1; then
        echo "digest fixture usage control unexpectedly passed" >&2
        exit 1
      fi
      cargo llvm-cov run --branch --no-clean --locked --offline --bin check-xxh3-vectors -- --self-test
      cargo llvm-cov run --branch --no-clean --locked --offline --bin check-digest-correspondence -- --self-test
      cargo llvm-cov run --branch --no-clean --locked --offline --bin check-digest-correspondence -- --write
      cargo llvm-cov run --branch --no-clean --locked --offline --bin check-digest-correspondence
      if cargo llvm-cov run --branch --no-clean --locked --offline --bin check-digest-correspondence -- --invalid >/dev/null 2>&1; then
        echo "digest correspondence usage control unexpectedly passed" >&2
        exit 1
      fi
      cargo llvm-cov run --branch --no-clean --locked --offline --bin generate-casefold-table -- --self-test
      cargo llvm-cov run --branch --no-clean --locked --offline --bin generate-casefold-table -- --check
      cargo llvm-cov run --branch --no-clean --locked --offline --bin generate-coverage-fixtures -- --check
      cargo llvm-cov run --branch --no-clean --locked --offline --bin generate-property-evidence -- --check
      cargo llvm-cov run --branch --no-clean --locked --offline --bin generate-property-fixtures -- --check
      cargo llvm-cov run --branch --no-clean --locked --offline --bin generate-v2-fixture-suite -- --check
      cargo llvm-cov run --branch --no-clean --locked --offline --bin buoy-quality -- --self-test
      cargo llvm-cov run --branch --no-clean --locked --offline --bin buoy-quality -- --check
      quality_output="$standalone_dir/quality-output"
      cargo llvm-cov run --branch --no-clean --locked --offline --bin buoy-quality -- \
        --generate --output "$quality_output"
      cargo llvm-cov run --branch --no-clean --locked --offline --bin buoy-quality -- \
        --check-execution-evidence
      cargo llvm-cov run --branch --no-clean --locked --offline --bin buoy-quality -- \
        --write-execution-evidence
      cargo llvm-cov run --branch --no-clean --json \
        --output-path "$standalone_dir/hashing-policy-self-test.json" \
        --locked --offline --bin buoy-hashing-policy -- --self-test
      cargo llvm-cov run --branch --no-clean --json \
        --output-path "$standalone_dir/hashing-policy-check.json" \
        --locked --offline --bin buoy-hashing-policy
      cargo llvm-cov run --branch --no-clean --json \
        --output-path "$standalone_dir/product-identity-self-test.json" \
        --locked --offline --bin buoy-product-identity -- --self-test
      quality_binary="target/llvm-cov-target/debug/buoy-quality"
      for missing_value in --root --output; do
        if LLVM_PROFILE_FILE="$PWD/target/llvm-cov-target/quality-invalid-%p-%m.profraw" \
          "$quality_binary" --check "$missing_value" >/dev/null 2>&1; then
          echo "quality CLI missing-value control unexpectedly passed: $missing_value" >&2
          exit 1
        fi
      done
      for execution_evidence_action in --check-execution-evidence --write-execution-evidence; do
        if LLVM_PROFILE_FILE="$PWD/target/llvm-cov-target/quality-invalid-%p-%m.profraw" \
          "$quality_binary" "$execution_evidence_action" --output "$quality_output" \
          >/dev/null 2>&1; then
          echo "quality CLI execution-evidence output-usage control unexpectedly passed: $execution_evidence_action" >&2
          exit 1
        fi
      done

      cargo llvm-cov run --branch --no-report --locked --offline \
        --bin check-package-contract
      package_contract_binary="$PWD/target/llvm-cov-target/debug/check-package-contract"
      for arguments in "--package-file" "--lock-file" "--unknown" \
        "--package-file missing.nix" \
        "--lock-file missing-source-content.json"; do
        if LLVM_PROFILE_FILE="$PWD/target/llvm-cov-target/package-contract-%p-%m.profraw" \
          "$package_contract_binary" $arguments >/dev/null 2>&1; then
          echo "package contract negative control unexpectedly passed: $arguments" >&2
          exit 1
        fi
      done

      cargo llvm-cov run --branch --no-report --locked --offline \
        --bin check-consumer-package
      consumer_package_binary="$PWD/target/llvm-cov-target/debug/check-consumer-package"
      BUOY_CONSUMER_PACKAGE_SSH_KEY=coverage-control \
        LLVM_PROFILE_FILE="$PWD/target/llvm-cov-target/consumer-package-%p-%m.profraw" \
        "$consumer_package_binary" >/dev/null
      consumer_missing_root="$standalone_dir/consumer-missing-root"
      consumer_profile="$PWD/target/llvm-cov-target/consumer-package-%p-%m.profraw"
      mkdir -p "$consumer_missing_root"
      if (
        cd "$consumer_missing_root"
        LLVM_PROFILE_FILE="$consumer_profile" \
          "$consumer_package_binary" >/dev/null 2>&1
      ); then
        echo "consumer package missing-fixture control unexpectedly passed" >&2
        exit 1
      fi

      trace_projection_binary="$PWD/target/llvm-cov-target/debug/check_trace_projection"
      cargo llvm-cov run --branch --no-report --locked --offline \
        --bin check_trace_projection -- --input examples/expected/trace-page.md
      for arguments in "" "--wrong" "--input" \
        "--input missing-trace-page.md" \
        "--input examples/expected/trace-page.md extra"; do
        if LLVM_PROFILE_FILE="$PWD/target/llvm-cov-target/trace-projection-%p-%m.profraw" \
          "$trace_projection_binary" $arguments >/dev/null 2>&1; then
          echo "trace projection negative control unexpectedly passed: $arguments" >&2
          exit 1
        fi
      done
      invalid_projection="$standalone_dir/invalid-trace-projection.md"
      printf '%s\n' 'Source graph: missing-contract-fields' >"$invalid_projection"
      if LLVM_PROFILE_FILE="$PWD/target/llvm-cov-target/trace-projection-%p-%m.profraw" \
        "$trace_projection_binary" --input "$invalid_projection" >/dev/null 2>&1; then
        echo "invalid trace projection control unexpectedly passed" >&2
        exit 1
      fi

      cargo llvm-cov run --branch --no-report --locked --offline \
        --bin generate-publication-goldens >/dev/null
      cargo llvm-cov run --branch --no-clean --locked --offline \
        --bin check-exact-coverage -- --self-test
      cargo llvm-cov run --branch --no-clean --json \
        --output-path "$standalone_dir/generator-controls-all-features.json" \
        --locked --offline --example generator_coverage_controls
      cargo llvm-cov run --branch --no-clean --json \
        --output-path "$standalone_dir/fixture-checker.json" --locked --offline \
        --bin check-fixture-manifest
      cargo llvm-cov run --branch --no-report --locked --offline \
        --features coverage-controls --bin check-hardening-manifest -- --self-test
      cargo llvm-cov run --branch --no-report --locked --offline \
        --features coverage-controls --bin check-hardening-manifest
      cargo llvm-cov run --branch --no-report --locked --offline \
        --features coverage-controls --bin check-hardening-manifest -- --write-execution-evidence
      hardening_binary="$PWD/target/llvm-cov-target/debug/check-hardening-manifest"
      hardening_profile="$PWD/target/llvm-cov-target/hardening-%p-%m.profraw"
      if LLVM_PROFILE_FILE="$hardening_profile" \
        "$hardening_binary" --invalid >/dev/null 2>&1; then
        echo "hardening manifest usage control unexpectedly passed" >&2
        exit 1
      fi
      hardening_control="$standalone_dir/hardening-missing-repository"
      mkdir -p "$hardening_control"
      if (
        cd "$hardening_control"
        LLVM_PROFILE_FILE="$hardening_profile" \
          "$hardening_binary" >/dev/null 2>&1
      ); then
        echo "hardening manifest missing-repository control unexpectedly passed" >&2
        exit 1
      fi
      cargo llvm-cov run --branch --no-report --locked --offline \
        --features coverage-controls --bin check-hardening-manifest -- --ledger-self-test
      cargo llvm-cov run --branch --no-clean --json \
        --output-path "$standalone_dir/hardening-manifest-checker.json" --locked --offline \
        --features coverage-controls --bin check-hardening-manifest -- --ledger
      cp "$standalone_dir/hardening-manifest-checker.json" \
        "$standalone_dir/hardening-ledger-library.json"
      cp "$standalone_dir/hardening-manifest-checker.json" \
        "$standalone_dir/execution-evidence-library.json"
      cargo llvm-cov run --branch --no-report --locked --offline \
        --bin check-adversarial-evidence -- --self-test
      adversarial_binary="target/llvm-cov-target/debug/check-adversarial-evidence"
      adversarial_control="$standalone_dir/adversarial-controls"
      mkdir -p "$adversarial_control/platform-linux" \
        "$adversarial_control/platform-macos" "$adversarial_control/platform-windows"
      run_adversarial_control() {
        LLVM_PROFILE_FILE="$PWD/target/llvm-cov-target/adversarial-%p-%m.profraw" \
          "$adversarial_binary" "$@"
      }
      run_adversarial_control --bindings fuzz >/dev/null
      run_adversarial_control --bindings mutation >/dev/null
      run_adversarial_control --platform-corpus linux \
        "$adversarial_control/platform-linux" >/dev/null
      run_adversarial_control --platform-corpus macos \
        "$adversarial_control/platform-macos" >/dev/null
      run_adversarial_control --platform-corpus windows \
        "$adversarial_control/platform-windows" >/dev/null
      for arguments in "" "invalid" "invalid value" "invalid value value" \
        "--bindings invalid" "--check invalid" \
        "--expect-error-control" "--mutation-document-control" \
        "--self-test-platform-result-fault invalid"; do
        if LLVM_PROFILE_FILE="$PWD/target/llvm-cov-target/adversarial-%p-%m.profraw" \
          "$adversarial_binary" $arguments >/dev/null 2>&1; then
          echo "adversarial negative control unexpectedly passed: $arguments" >&2
          exit 1
        fi
      done
      mkdir -p "$adversarial_control/missing-policy" \
        "$adversarial_control/missing-artifact/assurance/adversarial/v1" \
        "$adversarial_control/binding-failure/assurance/adversarial/v1"
      cp assurance/adversarial/v1/fuzz-policy.toml \
        "$adversarial_control/missing-artifact/assurance/adversarial/v1/fuzz-policy.toml"
      cp assurance/adversarial/v1/{fuzz-policy,fuzz-campaign,mutation-policy,mutation-campaign}.toml \
        "$adversarial_control/binding-failure/assurance/adversarial/v1/"
      for root in "$adversarial_control/missing-policy" \
        "$adversarial_control/missing-artifact" \
        "$adversarial_control/binding-failure"; do
        if run_adversarial_control --self-test-root "$root" >/dev/null 2>&1; then
          echo "adversarial root control unexpectedly passed: $root" >&2
          exit 1
        fi
      done
      if run_adversarial_control --bindings-root fuzz \
        "$adversarial_control/binding-failure" >/dev/null 2>&1; then
        echo "adversarial binding-root control unexpectedly passed" >&2
        exit 1
      fi
      for fault in corpus effects; do
        if run_adversarial_control --platform-fault "$fault" \
          "$adversarial_control/platform-linux" >/dev/null 2>&1; then
          echo "adversarial platform fault unexpectedly passed: $fault" >&2
          exit 1
        fi
      done
      for fault in platform bound-input os read remove-file create-dir cleanup; do
        if run_adversarial_control --self-test-platform-result-fault "$fault" >/dev/null 2>&1; then
          echo "adversarial platform-result self-test fault unexpectedly passed: $fault" >&2
          exit 1
        fi
      done
      current_platform="$(run_adversarial_control --bindings fuzz >/dev/null; uname -s)"
      case "$current_platform" in
        Darwin) current_platform_id=macos ;;
        Linux) current_platform_id=linux ;;
        *) current_platform_id="$current_platform" ;;
      esac
      mkdir -p "$adversarial_control/platform-result-blocked"
      printf '%s\n' blocked >"$adversarial_control/platform-result-blocked/$current_platform_id"
      if run_adversarial_control --platform-result "$adversarial_control/platform-result-blocked" >/dev/null 2>&1; then
        echo "adversarial platform-result corpus-write control unexpectedly passed" >&2
        exit 1
      fi
      run_adversarial_control --platform-result "$adversarial_control/platform-result" >/dev/null
      printf '%s\n' blocked >"$adversarial_control/create-failure"
      mkdir -p "$adversarial_control/portable-failure/portable-corpus.tsv" \
        "$adversarial_control/effects-failure/platform-effects.tsv"
      for output in "$adversarial_control/create-failure" \
        "$adversarial_control/portable-failure" "$adversarial_control/effects-failure"; do
        if run_adversarial_control --platform-corpus linux "$output" >/dev/null 2>&1; then
          echo "adversarial output control unexpectedly passed: $output" >&2
          exit 1
        fi
      done
      run_adversarial_control --check "''${BUOY_CAMPAIGN_AS_OF:-$(date -u +%F)}"
      cargo llvm-cov report --branch --json \
        --output-path "$standalone_dir/adversarial-evidence-checker.json" --locked --offline
      cargo llvm-cov run --branch --no-report --locked --offline \
        --bin check-devenv-task-graph -- --self-test
      cargo llvm-cov run --branch --no-clean --locked --offline \
        --bin check-devenv-task-graph -- --write-execution-evidence
      devenv_graph_binary="target/llvm-cov-target/debug/check-devenv-task-graph"
      for arguments in "" "invalid" "--write-execution-evidence invalid" "--check --tasks" "--check --policy" "--check --root"; do
        if LLVM_PROFILE_FILE="$PWD/target/llvm-cov-target/devenv-graph-%p-%m.profraw" \
          "$devenv_graph_binary" $arguments >/dev/null 2>&1; then
          echo "Devenv graph usage control unexpectedly passed: $arguments" >&2
          exit 1
        fi
      done
      if env -u DEVENV_TASK_FILE \
        LLVM_PROFILE_FILE="$PWD/target/llvm-cov-target/devenv-graph-%p-%m.profraw" \
        "$devenv_graph_binary" --check >/dev/null 2>&1; then
        echo "Devenv graph missing-task-file control unexpectedly passed" >&2
        exit 1
      fi
      LLVM_PROFILE_FILE="$PWD/target/llvm-cov-target/devenv-graph-%p-%m.profraw" \
        "$devenv_graph_binary" --check --tasks "$DEVENV_TASK_FILE" \
        --policy "$BUOY_DEVENV_POLICY" --root "$DEVENV_ROOT"
      cargo llvm-cov run --branch --no-clean --json \
        --output-path "$standalone_dir/devenv-task-graph-checker.json" --locked --offline \
        --bin check-devenv-task-graph -- --check
      cargo llvm-cov run --branch --no-clean --json \
        --output-path "$standalone_dir/schema-checker.json" --locked --offline \
        --bin check-schema-inventory
      cargo llvm-cov run --branch --no-clean --json \
        --output-path "$standalone_dir/property-evidence-checker.json" --locked --offline \
        --bin check-property-evidence
      cargo llvm-cov run --branch --no-clean --json \
        --output-path "$standalone_dir/reference-independence.json" --locked --offline \
        --bin check-reference-independence
      cargo llvm-cov run --branch --no-clean --json \
        --output-path "$standalone_dir/digest-transition-model.json" --locked --offline \
        --bin check-digest-transition-model
      cargo llvm-cov run --branch --no-clean --json \
        --output-path "$standalone_dir/digest-fixtures.json" --locked --offline \
        --bin check-digest-fixtures
      cargo llvm-cov run --branch --no-clean --json \
        --output-path "$standalone_dir/digest-correspondence.json" --locked --offline \
        --bin check-digest-correspondence -- --self-test
      cargo llvm-cov run --branch --no-clean --json \
        --output-path "$standalone_dir/xxh3-vector-compatibility.json" --locked --offline \
        --bin check-xxh3-vectors -- --self-test
      cargo llvm-cov run --branch --no-clean --json \
        --output-path "$standalone_dir/casefold.json" --locked --offline \
        --bin generate-casefold-table -- --check
      cargo llvm-cov run --branch --no-clean --json \
        --output-path "$standalone_dir/coverage-fixtures.json" --locked --offline \
        --bin generate-coverage-fixtures -- --check
      cargo llvm-cov run --branch --no-clean --json \
        --output-path "$standalone_dir/property-evidence-generator.json" --locked --offline \
        --bin generate-property-evidence -- --check
      cargo llvm-cov run --branch --no-clean --json \
        --output-path "$standalone_dir/property-fixtures.json" --locked --offline \
        --bin generate-property-fixtures -- --check
      cargo llvm-cov run --branch --no-clean --json \
        --output-path "$standalone_dir/v2-fixtures.json" --locked --offline \
        --bin generate-v2-fixture-suite -- --check
      cargo llvm-cov run --branch --no-clean --json \
        --output-path "$standalone_dir/digest-binding.json" --locked --offline \
        --bin check-exact-coverage -- --self-test
      cargo llvm-cov run --branch --no-clean --json \
        --output-path "$standalone_dir/quality-registry.json" --locked --offline \
        --bin buoy-quality -- --check
      cargo llvm-cov run --branch --no-clean --json \
        --output-path "$standalone_dir/package-contract-checker.json" --locked --offline \
        --bin check-package-contract
      cargo llvm-cov run --branch --no-clean --json \
        --output-path "$standalone_dir/consumer-package-checker.json" --locked --offline \
        --bin check-consumer-package
      cargo llvm-cov run --branch --no-clean --json \
        --output-path "$standalone_dir/trace-projection-checker.json" --locked --offline \
        --bin check_trace_projection -- --input examples/expected/trace-page.md
      cargo llvm-cov run --branch --no-clean --json \
        --output-path "$standalone_dir/publication-golden-generator.json" --locked --offline \
        --bin generate-publication-goldens >/dev/null

      (
        eval "$(cargo llvm-cov show-env --branch \
          --manifest-path dylint_lints/fcis_core/Cargo.toml --sh)"
        covtarget="$PWD/dylint_lints/fcis_core/target/llvm-cov-target"
        rm -rf "$covtarget/debug"
        rm -f "$covtarget"/*.profraw
        CARGO_TARGET_DIR="$covtarget" cargo build \
          --manifest-path dylint_lints/fcis_core/Cargo.toml --lib
        library_dir="$(mktemp -d)"
        dylint_home="$(mktemp -d)"
        driver_log="$(mktemp)"
        trap 'rm -rf "$library_dir" "$dylint_home"; rm -f "$driver_log"' EXIT
        printf '%s\n' \
          '[source.crates-io]' \
          'replace-with = "dylint-vendor"' \
          '[source.dylint-vendor]' \
          "directory = \"$BUOY_DYLINT_DRIVER_VENDOR\"" \
          '[patch.crates-io]' \
          "dylint_driver = { path = \"$BUOY_DYLINT_DRIVER_SOURCE\" }" \
          '[net]' \
          'offline = true' >"$dylint_home/config.toml"
        toolchain="$(rustup show active-toolchain | awk '{print $1}')"
        case "$(uname -s)" in
          Darwin) extension="dylib" ;;
          Linux) extension="so" ;;
          *) echo "unsupported Dylint coverage platform" >&2; exit 1 ;;
        esac
        cp "$covtarget/debug/libfcis_core.$extension" \
          "$library_dir/libfcis_core@$toolchain.$extension"
        run_instrumented_fcis() {
          LLVM_PROFILE_FILE="$covtarget/fcis-%p-%m.profraw" \
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
        run_instrumented_fcis --package buoy-core --lib
        run_instrumented_fcis \
          --manifest-path fixtures/dylint/allowed-boundaries/Cargo.toml
        run_instrumented_fcis \
          --manifest-path fixtures/dylint/missing-no-std/Cargo.toml \
          --message-format=json >/dev/null 2>&1 || true
        run_instrumented_fcis \
          --manifest-path fixtures/dylint/ambient-state/Cargo.toml \
          --message-format=json >/dev/null 2>&1 || true
        run_instrumented_fcis \
          --manifest-path fixtures/dylint/callable-public-interface/Cargo.toml \
          --message-format=json >/dev/null 2>&1 || true
      )
      buoy-fcis
      cargo llvm-cov report --manifest-path dylint_lints/fcis_core/Cargo.toml \
        --branch --json --output-path "$dylint_report" --locked --offline
      # Re-export every workspace package after all explicit controls.
      # The package list preserves the non-root workspace objects.
      compatibility_package="spec""-provenance"
      cargo llvm-cov report \
        -p buoy -p buoy-core -p buoy-reference \
        -p "$compatibility_package" -p "$compatibility_package-core" \
        --branch --json --output-path "$workspace_report" --locked --offline

      checker="target/debug/check-exact-coverage"
      test -x "$checker"
      "$checker" --binding workspace portable\;linux-macos \
        "$(command -v cargo-llvm-cov)" assurance/v1/coverage-command.toml \
        assurance/v1/source-inventory.tsv >"$workspace_binding"
      normalize_inputs=("$workspace_binding" "$workspace_report")
      chelis_binding="$standalone_dir/chelis-feature-matrix.toml"
      "$checker" --binding chelis-feature-matrix portable\;linux-macos \
        "$(command -v cargo-llvm-cov)" assurance/v1/coverage-command.toml \
        assurance/v1/source-inventory.tsv >"$chelis_binding"
      normalize_inputs+=("$chelis_binding" "$standalone_dir/chelis-feature-matrix.json")
      for example_id in check-unicode report-goldens presentation-goldens canonical-vectors malformed-vectors; do
        example_binding="$examples_dir/$example_id.toml"
        "$checker" --binding "examples-$example_id" portable \
          "$(command -v cargo-llvm-cov)" assurance/v1/coverage-command.toml \
          assurance/v1/source-inventory.tsv >"$example_binding"
        normalize_inputs+=("$example_binding" "$examples_dir/$example_id.json")
      done
      for standalone_id in \
        core-workspace \
        fixture-checker hardening-manifest-checker hardening-ledger-library \
        execution-evidence-library \
        adversarial-evidence-checker devenv-task-graph-checker \
        schema-checker property-evidence-checker exact-coverage-checker \
        reference-independence digest-transition-model digest-fixtures \
        digest-correspondence xxh3-vector-compatibility casefold coverage-fixtures \
        property-evidence-generator property-fixtures v2-fixtures \
        digest-binding quality-registry generator-controls-all-features \
        package-contract-checker consumer-package-checker \
        trace-projection-checker publication-golden-generator; do
        standalone_binding="$standalone_dir/$standalone_id.toml"
        "$checker" --binding "standalone-$standalone_id" portable \
          "$(command -v cargo-llvm-cov)" assurance/v1/coverage-command.toml \
          assurance/v1/source-inventory.tsv >"$standalone_binding"
        normalize_inputs+=("$standalone_binding" "$standalone_dir/$standalone_id.json")
      done
      "$checker" --binding dylint portable \
        "$(command -v cargo-llvm-cov)" assurance/v1/coverage-command.toml \
        assurance/v1/source-inventory.tsv >"$dylint_binding"
      normalize_inputs+=("$dylint_binding" "$dylint_report")
      # The checker artifact records one real ownership run over every export binding.
      ownership_inputs=()
      for ((index = 0; index < ''${#normalize_inputs[@]}; index += 2)); do
        ownership_inputs+=("''${normalize_inputs[index]}")
      done
      cargo llvm-cov run --branch --no-clean --json \
        --output-path "$standalone_dir/exact-coverage-checker.json" --locked --offline \
        --bin check-exact-coverage -- --check-ownership assurance/v1/source-inventory.tsv \
        "''${ownership_inputs[@]}"
      "$checker" --normalize . assurance/v1/source-inventory.tsv \
        "''${normalize_inputs[@]}" >"$normalized_result"
      "$checker" --check-result "$normalized_result"
      cp "$normalized_result" assurance/v1/coverage-result.tsv
      "$checker" --check-current . assurance/v1/source-inventory.tsv \
        assurance/v1/coverage-result.tsv \
        "''${normalize_inputs[@]}"
      echo "exact-coverage: current normalized result emitted after all controls passed"
    '';
  };

  scripts.buoy-chelis-compact-bindings = {
    description = "Check collision-resistant Chelis compact bindings";
    exec = ''
      set -euo pipefail
      fix_build_outputs() {
        find target -path '*/build/*' -exec chmod u+w {} + 2>/dev/null || true
      }
      cargo clippy -p chelis-provenance --all-targets --no-default-features --features blake3-256 -- -D warnings
      fix_build_outputs
      cargo test -p chelis-provenance --no-default-features --features blake3-256 --test compact_bindings
      fix_build_outputs
      cargo test -p chelis-provenance --no-default-features --features blake3-256 --test bind_conversion
      fix_build_outputs
      cargo test -p chelis-provenance --no-default-features --features blake3-256 --test binding_regeneration
      fix_build_outputs
      cargo test -p chelis-provenance --no-default-features --features blake3-256 --test trace_projection
      fix_build_outputs
      cargo clippy -p chelis-provenance --all-targets --no-default-features --features sha-256 -- -D warnings
      fix_build_outputs
      cargo test -p chelis-provenance --no-default-features --features sha-256 --test compact_bindings
      fix_build_outputs
      cargo test -p chelis-provenance --no-default-features --features sha-256 --test bind_conversion
      fix_build_outputs
      cargo test -p chelis-provenance --no-default-features --features sha-256 --test binding_regeneration
      fix_build_outputs
      cargo test -p chelis-provenance --no-default-features --features sha-256 --test trace_projection
    '';
  };

  scripts.buoy-core-proofs = {
    description = "Replay selected Verus integrity-kernel proofs";
    packages = [ formalTools.verus ];
    exec = ''
      set -euo pipefail
      negative_log="$(mktemp)"
      trap 'rm -f "$negative_log"' EXIT
      verus proofs/core/integrity.rs
      if verus proofs/core/integrity_negative.rs >"$negative_log" 2>&1; then
        echo "Production Verus negative control unexpectedly verified" >&2
        exit 1
      fi
      grep -F "postcondition not satisfied" "$negative_log" >/dev/null
      cargo test --test proof_bindings
    '';
  };

}
