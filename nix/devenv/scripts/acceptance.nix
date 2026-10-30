{
  dylintTools,
  formalTools,
  pkgs,
  supplyChain,
  ...
}:

{
  scripts.buoy-versions = {
    description = "Check every reported tool identity";
    packages = [
      dylintTools
      formalTools.aeneas
      formalTools.leanAeneas
      formalTools.verus
      formalTools.z3
      pkgs.cargo-llvm-cov
      pkgs.cargo-fuzz
      pkgs.cargo-mutants
      pkgs.cargo-deny
      supplyChain.cleanCargoRunner
      supplyChain.closureReportRunner
    ];
    exec = ''
      set -euo pipefail
      rustc --version
      cargo --version
      cargo clippy --version
      rust-analyzer --version
      cargo dylint --version
      cargo llvm-cov --version
      cargo fuzz --version
      cargo mutants --version
      cargo deny --version
      rustfmt --version
      cargo nextest --version
      verus --version
      z3 --version
      openspec --version
      devenv version
    '';
  };

  scripts.buoy-devenv = {
    description = "Validate the resolved Devenv workflow and all negative controls";
    exec = ''
      set -euo pipefail
      cargo run --quiet --locked --offline --bin check-devenv-task-graph -- --self-test
      cargo run --quiet --locked --offline --bin check-devenv-task-graph -- \
        --check --tasks "$DEVENV_TASK_FILE" --policy "$BUOY_DEVENV_POLICY" --root "$DEVENV_ROOT"
    '';
  };

  scripts.buoy-examples = {
    description = "Verify public Rust, repository, change, and OpenSpec examples";
    packages = [ pkgs.jq ];
    exec = ''
      set -euo pipefail
      scratch="$(mktemp -d)"
      trap 'rm -rf "$scratch"' EXIT

      cargo test --quiet --locked --offline --all-features --test usage_examples
      cargo run --quiet --locked --offline --example core_graph >"$scratch/core.json"
      cmp examples/expected/core-graph.json "$scratch/core.json"
      set +e
      cargo run --quiet --locked --offline --example core_graph -- --invalid \
        >"$scratch/core-invalid.json"
      core_invalid_status="$?"
      set -e
      test "$core_invalid_status" -eq 2
      cmp examples/expected/core-graph-invalid.json "$scratch/core-invalid.json"

      cargo run --quiet --locked --offline --example repository_check -- \
        --root examples/repositories/openspec-authority \
        --config examples/repositories/openspec-authority/buoy.toml \
        >"$scratch/repository.json"
      cmp examples/expected/openspec-static.json "$scratch/repository.json"
      set +e
      cargo run --quiet --locked --offline --example repository_check -- \
        --root examples/repositories/openspec-authority \
        --config examples/repositories/invalid-config.toml \
        >"$scratch/repository-invalid.json" \
        2>"$scratch/repository-invalid.err"
      repository_invalid_status="$?"
      set -e
      test "$repository_invalid_status" -eq 70
      grep -F 'REPO-CONFIG-SCHEMA' "$scratch/repository-invalid.err" >/dev/null

      cargo build --quiet --locked --offline --bin buoy
      target/debug/buoy static \
        --root examples/repositories/openspec-authority \
        --config examples/repositories/openspec-authority/buoy.toml \
        --format machine >"$scratch/static.json"
      cmp examples/expected/openspec-static.json "$scratch/static.json"

      set +e
      target/debug/buoy static \
        --root examples/repositories/stale-external-link \
        --config examples/repositories/stale-external-link/buoy.toml \
        --format machine >"$scratch/stale.json"
      stale_status="$?"
      set -e
      test "$stale_status" -eq 2
      cmp examples/expected/stale-static.json "$scratch/stale.json"

      target/debug/buoy change \
        --base examples/repositories/change-check/base \
        --head examples/repositories/change-check/head \
        --config examples/repositories/change-check/buoy.toml \
        --manifest examples/repositories/change-check/complete.toml \
        --format machine >"$scratch/change-complete.json"
      cmp examples/expected/change-complete.json "$scratch/change-complete.json"
      set +e
      target/debug/buoy change \
        --base examples/repositories/change-check/base \
        --head examples/repositories/change-check/head \
        --config examples/repositories/change-check/buoy.toml \
        --manifest examples/repositories/change-check/incomplete.toml \
        --format machine >"$scratch/change-incomplete.json"
      incomplete_status="$?"
      set -e
      test "$incomplete_status" -eq 3
      cmp examples/expected/change-incomplete.json "$scratch/change-incomplete.json"

      workflow="$scratch/workflow"
      mkdir -p "$workflow"
      cp -R examples/openspec-workflow/. "$workflow/"
      (
        cd "$workflow"
        openspec schema validate buoy-traceability
        openspec new change flow-control
        openspec status --change flow-control --json >"$scratch/flow-initial.json"
        jq -e '.artifacts[] | select(.id == "proposal" and .status == "ready")' \
          "$scratch/flow-initial.json" >/dev/null
        openspec instructions proposal --change flow-control --json \
          >"$scratch/flow-proposal-instructions.json"
        jq -e '.template | contains("## Non-Goals")' \
          "$scratch/flow-proposal-instructions.json" >/dev/null

        change="openspec/changes/flow-control"
        printf '%s\n' '## Why' 'Control.' >"$change/proposal.md"
        openspec status --change flow-control --json >"$scratch/flow-specs.json"
        jq -e '.artifacts[] | select(.id == "specs" and .status == "ready")' \
          "$scratch/flow-specs.json" >/dev/null
        openspec instructions specs --change flow-control --json \
          >"$scratch/flow-specs-instructions.json"
        jq -e '.template | contains("### Requirement: DEMO-001")' \
          "$scratch/flow-specs-instructions.json" >/dev/null
        mkdir -p "$change/specs/demo"
        printf '%s\n' \
          '## ADDED Requirements' \
          '### Requirement: FLOW-001 Control' \
          'The control MUST remain deterministic.' \
          '#### Scenario: Positive - control' \
          '- **GIVEN** saved input' \
          '- **WHEN** validation runs' \
          '- **THEN** the result is stable' \
          '#### Scenario: Negative - control' \
          '- **GIVEN** invalid input' \
          '- **WHEN** validation runs' \
          '- **THEN** a stable diagnostic occurs' \
          >"$change/specs/demo/spec.md"

        openspec status --change flow-control --json >"$scratch/flow-design.json"
        jq -e '.artifacts[] | select(.id == "design" and .status == "ready")' \
          "$scratch/flow-design.json" >/dev/null
        openspec instructions design --change flow-control --json \
          >"$scratch/flow-design-instructions.json"
        jq -e '.template | contains("Repository configuration owns authority")' \
          "$scratch/flow-design-instructions.json" >/dev/null
        printf '%s\n' '## Context' 'Control.' >"$change/design.md"

        openspec status --change flow-control --json >"$scratch/flow-trace.json"
        jq -e '.artifacts[] | select(.id == "traceability" and .status == "ready")' \
          "$scratch/flow-trace.json" >/dev/null
        jq -e '.artifacts[] | select(.id == "tasks" and .status == "blocked")' \
          "$scratch/flow-trace.json" >/dev/null
        openspec instructions traceability --change flow-control --json \
          >"$scratch/flow-trace-instructions.json"
        jq -e '.template | contains("## Requirement Actions")' \
          "$scratch/flow-trace-instructions.json" >/dev/null
        printf '%s\n' '## Requirement Actions' 'FLOW-001' >"$change/traceability.md"

        openspec status --change flow-control --json >"$scratch/flow-tasks.json"
        jq -e '.artifacts[] | select(.id == "tasks" and .status == "ready")' \
          "$scratch/flow-tasks.json" >/dev/null
        openspec instructions tasks --change flow-control --json \
          >"$scratch/flow-tasks-instructions.json"
        jq -e '.template | contains("- [ ]")' \
          "$scratch/flow-tasks-instructions.json" >/dev/null
        printf '%s\n' '## Tasks' '- [x] Verify the control.' >"$change/tasks.md"
        openspec instructions apply --change flow-control --json \
          >"$scratch/flow-complete.json"
        jq -e '.state == "all_done"' "$scratch/flow-complete.json" >/dev/null
      )
    '';
  };

  scripts.buoy-format = {
    description = "Check all reviewed Rust and Nix formatting";
    packages = [ pkgs.nixfmt ];
    exec = ''
      set -euo pipefail
      cargo fmt --all -- --check
      rustfmt --edition 2024 --check tools/check_adversarial_evidence.rs tools/check_devenv_task_graph.rs tools/check_fixture_manifest.rs tools/check_hardening_manifest.rs tools/hardening_ledger.rs tools/check_schema_inventory.rs tools/check_property_evidence.rs tools/check_exact_coverage.rs tools/check_reference_independence.rs tools/check_digest_fixtures.rs tools/check_digest_correspondence.rs tools/check_xxh3_vectors.rs tools/check_xxh3_migration_model.rs tools/digest_binding.rs tools/execution_evidence.rs tools/generate_casefold_table.rs tools/generate_coverage_fixtures.rs tools/generate_property_evidence.rs tools/generate_property_fixtures.rs tools/generate_v2_fixture_suite.rs
      cargo fmt --manifest-path dylint_lints/fcis_core/Cargo.toml -- --check
      cargo fmt --manifest-path proofs/spikes/aeneas/rust/Cargo.toml -- --check
      for fixture_manifest in fixtures/dylint/*/Cargo.toml; do
        cargo fmt --manifest-path "$fixture_manifest" -- --check
      done
      nixfmt --check devenv.nix nix/*.nix nix/devenv/*.nix nix/devenv/scripts/*.nix
    '';
  };

  scripts.buoy-phase-result = {
    description = "Validate one independently produced source-bound phase-result record";
    exec = ''
      set -euo pipefail
      phase="''${1:-}"
      case "$phase" in
        P0) command='devenv tasks run buoy:p0' ;;
        P1) command='devenv tasks run buoy:p1' ;;
        P2) command='devenv tasks run buoy:p2' ;;
        P3) command='devenv tasks run buoy:p3' ;;
        FINAL) command='devenv test' ;;
        *) echo "PHASE-ID: expected P0, P1, P2, P3, or FINAL" >&2; exit 2 ;;
      esac
      result="assurance/v1/phase-results/$phase.toml"
      if [ ! -f "$result" ]; then
        echo "$phase-MISSING: $result" >&2
        exit 1
      fi
      for exact in \
        'schema = "spec-provenance-phase-result/v2"' \
        "phase = \"$phase\"" \
        "command = \"$command\"" \
        'status = "green"' \
        'execution = "pass"' \
        'provenance = "independent-external-execution"'; do
        if ! grep -Fx "$exact" "$result" >/dev/null; then
          echo "$phase-STALE: missing exact result binding: $exact" >&2
          exit 1
        fi
      done
      for pattern in \
        '^provider = "[^\"]\+"$' \
        '^run_id = "[^\"]\+"$' \
        '^run_url = "https://[^\"]\+"$' \
        '^revision = "[0-9a-f]\{40\}"$' \
        '^phase_handoffs_digest = "sha-256:[0-9a-f]\{64\}"$' \
        '^gate_source_digest = "sha-256:[0-9a-f]\{64\}"$'; do
        if ! grep -x "$pattern" "$result" >/dev/null; then
          echo "$phase-STALE: missing independent provenance or source binding" >&2
          exit 1
        fi
      done
    '';
  };

  scripts.buoy-p0 = {
    description = "Report completion of the P0 task dependency slice";
    exec = "printf '%s\\n' 'P0: dependency slice complete'";
  };

  scripts.buoy-p1 = {
    description = "Report completion of the P1 task dependency slice";
    exec = "printf '%s\\n' 'P1: dependency slice complete'";
  };

  scripts.buoy-p2 = {
    description = "Report completion of the P2 task dependency slice";
    exec = "printf '%s\\n' 'P2: dependency slice complete'";
  };

  scripts.buoy-p3 = {
    description = "Report completion of the P3 task dependency slice";
    exec = "printf '%s\\n' 'P3: dependency slice complete'";
  };

  scripts.buoy-ledger-green = {
    description = "Require a closed green assurance ledger";
    exec = ''
      set -euo pipefail
      cargo run --quiet --locked --offline --bin check-hardening-manifest -- --ledger-green
    '';
  };

  scripts.buoy-gate = {
    description = "Report completion of the authoritative acceptance task graph";
    exec = "printf '%s\\n' 'acceptance graph complete'";
  };
}
