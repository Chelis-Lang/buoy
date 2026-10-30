{ config, ... }:

let
  # Vendored build scripts copy read-only Nix-store files into Cargo build
  # directories. A later feature-specific rebuild can then fail with EACCES.
  # Restore user write on generated build outputs before every task.
  writableVendorOutputs = "find target -path '*/build/*' -exec chmod u+w {} + 2>/dev/null || true; ";
  repoTask =
    task:
    {
      cwd = config.git.root;
    }
    // task
    // (if task ? exec then { exec = writableVendorOutputs + task.exec; } else { });
  taskDefinitions = {
    "buoy:check" = {
      description = "Check every production Cargo target";
      exec = "cargo check --workspace --all-targets --all-features";
    };
    "buoy:compile-fail" = {
      description = "Check frozen public core API compile-fail tripwires";
      exec = "buoy-compile-fail";
    };
    "buoy:fuzz-campaign" = {
      description = "Run the bounded initial coverage-guided fuzz campaign";
      exec = "buoy-fuzz-campaign";
    };
    "buoy:fuzz-replay" = {
      description = "Replay every committed fuzz corpus under deterministic bounds";
      exec = "buoy-fuzz-replay";
    };
    "buoy:mutation-campaign" = {
      description = "Manually run the selected deterministic cargo-mutants baseline";
      exec = "buoy-mutation-campaign";
    };
    "buoy:miri" = {
      description = "Run the declared pure-core Miri scope";
      exec = "buoy-miri";
    };
    "buoy:sanitizer" = {
      description = "Run the declared address-sanitizer scope";
      exec = "buoy-sanitizer";
    };
    "buoy:faults" = {
      description = "Run deterministic bounded fault injection";
      exec = "buoy-faults";
    };
    "buoy:platform-corpus" = {
      description = "Emit and repeat-check the local portable canonical corpus";
      exec = "buoy-platform-corpus";
    };
    "buoy:platform-result" = {
      description = "Materialize a host-detected local platform result";
      exec = "buoy-platform-result";
    };
    "buoy:adversarial-evidence" = {
      description = "Reject stale, expired, discharged, or failing campaign artifacts";
      exec = "buoy-adversarial-evidence";
    };

    "buoy:versions" = {
      description = "Check every reported tool identity";
      after = [ "devenv:rustup-links" ];
      exec = "buoy-versions";
    };
    "buoy:devenv" = {
      description = "Validate the resolved Devenv workflow and negative controls";
      after = [ "buoy:versions" ];
      exec = "buoy-devenv";
    };
    "buoy:digest-member-oracle" = {
      description = "Validate the selected digest member feature set";
      after = [ "buoy:devenv" ];
      exec = "buoy-digest-member-oracle";
    };
    "buoy:format" = {
      description = "Check all reviewed Rust and Nix formatting";
      after = [ "buoy:digest-member-oracle" ];
      exec = "buoy-format";
    };
    "buoy:clippy" = {
      description = "Run Clippy for every workspace target and feature";
      after = [ "buoy:format" ];
      exec = "cargo clippy --workspace --all-targets --all-features -- -D warnings";
    };
    "buoy:test" = {
      description = "Run the current production Cargo test surface";
      after = [ "buoy:clippy" ];
      exec = "cargo test --workspace --all-targets --all-features";
    };
    # `buoy:clippy` and `buoy:test` run `--all-features`, which activates
    # `xxh3-128` and makes it the build default. Compact bindings refuse a
    # change-detection member outright, so their positive suites compile out
    # of that build. This task lints and tests both collision-resistant builds.
    "buoy:chelis-compact-bindings" = {
      description = "Run collision-resistant Chelis compact-binding lints and suites";
      after = [ "buoy:test" ];
      exec = "buoy-chelis-compact-bindings";
    };
    "buoy:docs" = {
      description = "Build workspace documentation without dependencies";
      after = [ "buoy:chelis-compact-bindings" ];
      exec = "cargo doc --workspace --no-deps";
    };
    "buoy:product-identity" = {
      description = "Validate Buoy naming, compatibility, canonical parity, and migration fixtures";
      after = [ "buoy:docs" ];
      exec = "buoy-product-identity";
    };

    "buoy:schema" = {
      description = "Validate the active schema v3 inventory";
      after = [ "buoy:product-identity" ];
      exec = "buoy-schema";
    };
    "buoy:reference" = {
      description = "Validate dependency-independent executable authorities";
      after = [ "buoy:schema" ];
      exec = "buoy-reference";
    };
    "buoy:fixtures" = {
      description = "Validate the frozen fixture manifest and independent models";
      after = [ "buoy:reference" ];
      exec = "buoy-fixtures";
    };
    "buoy:properties" = {
      description = "Run frozen properties, exhaustive models, and formal spikes";
      after = [ "buoy:fixtures" ];
      exec = "buoy-properties";
    };
    "buoy:coverage-evidence" = {
      description = "Manually reject stale or inexact coverage evidence";
      exec = "buoy-coverage-evidence";
    };
    "buoy:coverage" = {
      description = "Manually require exact Cargo production coverage";
      exec = "buoy-coverage";
    };
    "buoy:p0" = {
      description = "P0: reject false-green assurance claims";
      after = [
        "buoy:schema"
        "buoy:reference"
        "buoy:fixtures"
        "buoy:properties"
      ];
      exec = "buoy-p0";
    };

    "buoy:hashing-policy" = {
      description = "Validate the active digest registry, migration, and repository policy";
      after = [ "buoy:p0" ];
      exec = "buoy-hashing-policy";
    };
    "buoy:identifier-policy" = {
      description = "Validate the typed stable-ID boundary";
      after = [ "buoy:hashing-policy" ];
      exec = "buoy-identifier-policy";
    };
    "buoy:core" = {
      description = "Check deterministic core contracts and selected proofs";
      after = [ "buoy:identifier-policy" ];
      exec = "buoy-core";
    };
    "buoy:fcis" = {
      description = "Check the structural functional-core boundary";
      after = [ "buoy:core" ];
      exec = "buoy-fcis";
    };
    "buoy:mutation-replay" = {
      description = "Manually run named mutation tripwires and artifact validation";
      exec = "buoy-mutation-replay";
    };
    "buoy:core-proofs" = {
      description = "Replay selected Verus integrity-kernel proofs";
      after = [ "buoy:fcis" ];
      exec = "buoy-core-proofs";
    };
    "buoy:p1" = {
      description = "P1: complete pure-core assurance";
      after = [
        "buoy:hashing-policy"
        "buoy:identifier-policy"
        "buoy:core"
        "buoy:fcis"
        "buoy:core-proofs"
      ];
      exec = "buoy-p1";
    };

    "buoy:unicode" = {
      description = "Validate pinned Unicode normalization and case-fold data";
      after = [ "buoy:p1" ];
      exec = "buoy-unicode";
    };
    "buoy:adapters" = {
      description = "Check total Markdown and Rust syntax adapters";
      after = [ "buoy:unicode" ];
      exec = "buoy-adapters";
    };
    "buoy:integration" = {
      description = "Check repository, CLI, oracle, and frontend integration";
      after = [ "buoy:adapters" ];
      exec = "buoy-integration";
    };
    "buoy:examples" = {
      description = "Verify public Rust, repository, change, and OpenSpec examples";
      after = [ "buoy:integration" ];
      exec = "buoy-examples";
    };
    "buoy:deny" = {
      description = "Enforce the pinned dependency advisory, license, source, and duplicate policy";
      after = [ "buoy:examples" ];
      exec = "buoy-deny";
    };
    "buoy:offline-acceptance" = {
      description = "Run the clean network-disabled sandbox acceptance";
      after = [ "buoy:deny" ];
      exec = "buoy-offline-acceptance";
    };
    "buoy:closure-size" = {
      description = "Report deterministic closure component sizes under the size policy";
      after = [ "buoy:offline-acceptance" ];
      exec = "buoy-closure-size";
    };
    "buoy:adversarial" = {
      description = "Run the complete bounded deterministic adversarial gate";
      after = [ "buoy:closure-size" ];
      exec = "buoy-adversarial";
    };
    "buoy:p2" = {
      description = "P2: complete adversarial systems assurance";
      after = [
        "buoy:unicode"
        "buoy:adapters"
        "buoy:integration"
        "buoy:examples"
        "buoy:deny"
        "buoy:offline-acceptance"
        "buoy:closure-size"
        "buoy:adversarial"
      ];
      exec = "buoy-p2";
    };

    "buoy:obligations" = {
      description = "Validate the exact fail-closed assurance-obligation ledger";
      after = [ "buoy:p2" ];
      exec = "buoy-obligations";
    };
    "buoy:gate-wiring" = {
      description = "Validate the resolved acceptance task graph";
      after = [ "buoy:obligations" ];
      exec = "buoy-gate-wiring";
    };
    "buoy:quality" = {
      description = "Validate the QA registry, bindings, and generated status report";
      after = [ "buoy:gate-wiring" ];
      exec = "buoy-quality";
    };
    "buoy:ledger-green" = {
      description = "Require a closed green assurance ledger";
      after = [ "buoy:quality" ];
      exec = "buoy-ledger-green";
    };
    "buoy:p3" = {
      description = "P3: close the assurance ledger";
      after = [
        "buoy:obligations"
        "buoy:gate-wiring"
        "buoy:quality"
        "buoy:ledger-green"
      ];
      exec = "buoy-p3";
    };

    "buoy:specs" = {
      description = "Strictly validate all OpenSpec artifacts and the project workflow";
      after = [ "buoy:p3" ];
      exec = ''
        openspec validate --all --strict --no-interactive
        openspec schema validate buoy-bridge
        cargo test -p buoy --all-features --test openspec_workflow
      '';
    };
    "buoy:package-contract" = {
      description = "Blocking offline structural contract check plus real local-source Nix realization for the Buoy-owned consumer package";
      after = [ "buoy:specs" ];
      # Task 10.11/10.12, audit M17: this task declares no step of its own.
      # `nix/tools/package-contract.sh` is the single registered owner of
      # every consumer-package task body, and the advisory downstream task
      # below invokes that same owner in its own mode, so neither task can
      # drift from the gate it claims to run.
      exec = "nix/tools/package-contract.sh --mode blocking";
    };
    "buoy:gate" = {
      description = "Complete the deterministic offline acceptance graph";
      after = [ "buoy:package-contract" ];
      before = [ "devenv:enterTest" ];
      exec = "buoy-gate";
    };

    # Advisory downstream task. It has no `before`/`after` edge into the
    # buoy:p0..p3 -> buoy:specs -> buoy:package-contract -> buoy:gate chain,
    # so it never runs as part of `devenv test` or any other automatic task.
    "buoy:consumer-package" = {
      description = "Advisory pinned private git+ssh consumer-package integration check";
      exec = "nix/tools/package-contract.sh --mode advisory";
    };
  };
  aliases = builtins.listToAttrs (
    map
      (name: {
        name = "spec-provenance:${name}";
        value = repoTask {
          description = "Deprecated compatibility alias for buoy:${name}";
          exec = "devenv tasks run buoy:${name}";
        };
      })
      [
        "check"
        "test"
        "fcis"
        "compile-fail"
        "specs"
        "schema"
        "reference"
        "unicode"
        "fixtures"
        "hashing-policy"
        "identifier-policy"
        "quality"
        "properties"
        "coverage"
        "core"
        "adapters"
        "integration"
        "fuzz-replay"
        "fuzz-campaign"
        "mutation-replay"
        "mutation-campaign"
        "miri"
        "sanitizer"
        "faults"
        "platform-corpus"
        "platform-result"
        "adversarial-evidence"
        "adversarial"
        "obligations"
        "p0"
        "p1"
        "p2"
        "p3"
        "gate"
        "product-identity"
      ]
  );
in
{
  tasks = builtins.mapAttrs (_: task: repoTask task) taskDefinitions // aliases;
}
