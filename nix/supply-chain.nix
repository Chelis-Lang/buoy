{
  pkgs,
  inputs,
  dylintTools,
  formalTools,
  rustToolchain,
  rustTarget,
}:

let
  lib = pkgs.lib;
  clippyRevision = "f6d310692116e9a527ce6d0b3526c965d9c5d7b9";

  workspaceCargoSources = pkgs.rustPlatform.importCargoLock {
    lockFile = ../Cargo.lock;
  };

  dylintCargoSources = pkgs.rustPlatform.importCargoLock {
    lockFile = ../dylint_lints/fcis_core/Cargo.lock;
    outputHashes = {
      "clippy_utils-0.1.97" = "sha256-0uKEpH0UqfRkRq3y7ooVk1g74N5xMJLLfM26Cpxxj6Y=";
    };
  };

  fuzzCargoSources = pkgs.rustPlatform.importCargoLock {
    lockFile = ../fuzz/Cargo.lock;
  };

  hegelBuildCargoSources = pkgs.rustPlatform.importCargoLock {
    lockFile = ./locks/hegeltest-c-0.29.0.Cargo.lock;
  };

  # `cargo miri setup` builds a custom sysroot from rust-src, so the vendor must
  # also carry the standard library's own locked dependencies.
  sysrootCargoSources = pkgs.rustPlatform.importCargoLock {
    lockFile = ./locks/rust-sysroot-nightly-2026-04-16.Cargo.lock;
  };

  cargoVendor = pkgs.runCommand "buoy-cargo-vendor-v1" { } ''
    set -euo pipefail
    mkdir -p "$out/vendor"

    for source in ${workspaceCargoSources} ${dylintCargoSources} ${fuzzCargoSources} ${hegelBuildCargoSources} ${sysrootCargoSources}; do
      for package in "$source"/*; do
        if [ ! -d "$package" ]; then
          continue
        fi
        name="$(basename "$package")"
        if [ -e "$out/vendor/$name" ]; then
          cmp "$package/.cargo-checksum.json" "$out/vendor/$name/.cargo-checksum.json"
        else
          ln -s "$package" "$out/vendor/$name"
        fi
      done
    done

    # Hegel header generation reads the package's own lockfile.
    # Use the reviewed lock for the offline dependency closure.
    hegel_package="$out/vendor/hegeltest-c-0.29.0"
    hegel_upstream="$(readlink "$hegel_package")"
    rm "$hegel_package"
    cp -RL "$hegel_upstream" "$hegel_package"
    chmod u+w "$hegel_package" "$hegel_package/Cargo.lock"
    cp ${./locks/hegeltest-c-0.29.0.Cargo.lock} "$hegel_package/Cargo.lock"

    test -e "$out/vendor/hegeltest-0.28.0/.cargo-checksum.json"
    test -e "$out/vendor/clippy_utils-0.1.97/.cargo-checksum.json"
    test -e "$out/vendor/libfuzzer-sys-0.4.10/.cargo-checksum.json"
    test -e "$out/vendor/hashbrown-0.17.0/.cargo-checksum.json"
  '';

  cargoConfig = pkgs.writeText "buoy-cargo-config-v1.toml" ''
    [net]
    offline = true

    [source.crates-io]
    replace-with = "buoy-nix-vendor"

    [source.buoy-nix-vendor]
    directory = "${cargoVendor}/vendor"

    [source."git+https://github.com/rust-lang/rust-clippy?rev=${clippyRevision}#${clippyRevision}"]
    git = "https://github.com/rust-lang/rust-clippy"
    rev = "${clippyRevision}"
    replace-with = "buoy-nix-vendor"
  '';

  repositorySource = lib.cleanSourceWith {
    src = ../.;
    filter =
      path: type:
      let
        base = baseNameOf (toString path);
        isLocalEnvironment =
          (base == ".env" || lib.hasPrefix ".env." base) && !(lib.hasSuffix ".example" base);
      in
      !(builtins.elem base [
        ".claude"
        ".devenv"
        ".direnv"
        ".git"
        ".pi"
        ".pre-commit-config.yaml"
        ".worktrees"
        "result"
        "target"
      ])
      && !isLocalEnvironment
      && !(lib.hasPrefix ".drain-" base)
      && !lib.hasSuffix ".profraw" base
      && toString path != toString ../dylint_lints/fcis_core/rust-toolchain.toml;
  };

  sourceManifestRunner = pkgs.writeShellApplication {
    name = "buoy-source-manifest";
    runtimeInputs = [
      pkgs.coreutils
      pkgs.findutils
      pkgs.git
      pkgs.nix
    ];
    text = ''
      set -euo pipefail
      paths="$(mktemp)"
      hashes="$(mktemp)"
      trap 'rm -f "$paths" "$hashes"' EXIT
      git ls-files -z --cached --others --exclude-standard \
        | tr '\0' '\n' \
        | LC_ALL=C sort -u \
        | while IFS= read -r path; do
            if [ -f "$path" ]; then
              printf '%s\n' "$path"
            fi
          done >"$paths"
      if [ ! -s "$paths" ]; then
        echo "SUPPLY-SOURCE-FILES-EMPTY" >&2
        exit 1
      fi
      tr '\n' '\0' <"$paths" | xargs -0 nix hash file --type sha256 --sri >"$hashes"
      printf 'path\tsha256\n'
      paste "$paths" "$hashes"
    '';
  };

  cleanCargoAcceptance = pkgs.stdenv.mkDerivation {
    pname = "buoy-clean-cargo-acceptance";
    version = "1";
    src = repositorySource;

    nativeBuildInputs = [
      rustToolchain
      pkgs.git
      pkgs.nix
      pkgs.pkg-config
      sourceManifestRunner
    ];
    buildInputs = [
      pkgs.openssl
      pkgs.zlib
    ];

    configurePhase = ''
      runHook preConfigure
      export HOME="$TMPDIR/empty-home"
      export CARGO_HOME="$TMPDIR/empty-cargo-home"
      export RUSTUP_HOME="$TMPDIR/empty-rustup-home"
      export CARGO_NET_OFFLINE=true
      export CARGO_TARGET_DIR="$TMPDIR/cargo-target"
      mkdir -p "$HOME" "$CARGO_HOME" "$RUSTUP_HOME" "$CARGO_TARGET_DIR"
      ln -s ${cargoConfig} "$CARGO_HOME/config.toml"
      test ! -e "$CARGO_HOME/registry"
      test ! -e "$CARGO_HOME/git"
      runHook postConfigure
    '';

    buildPhase = ''
      runHook preBuild
      git init --quiet
      git config user.name Buoy
      git config user.email supply-chain.invalid
      git add -f .
      GIT_AUTHOR_DATE=2026-07-22T17:32:51Z \
        GIT_COMMITTER_DATE=2026-07-22T17:32:51Z \
        git commit --quiet -m clean-sandbox-source
      if command -v cargo-deny >/dev/null 2>&1; then
        echo "SUPPLY-AMBIENT-TOOL: cargo-deny is absent from this sandbox input closure but resolved anyway" >&2
        exit 1
      fi
      undeclared_control="$TMPDIR/undeclared-dependency-control"
      developer_cargo_home="$TMPDIR/developer-cargo-home"
      developer_package="$developer_cargo_home/cache/buoy-undeclared-dependency-control"
      mkdir -p "$undeclared_control/src" "$developer_package/src"
      printf '%s\n' \
        '[package]' \
        'name = "undeclared-dependency-control"' \
        'version = "0.0.0"' \
        'edition = "2021"' \
        '[dependencies]' \
        'buoy-undeclared-dependency-control = "1"' >"$undeclared_control/Cargo.toml"
      printf 'fn main() {}\n' >"$undeclared_control/src/main.rs"
      printf '%s\n' \
        '[package]' \
        'name = "buoy-undeclared-dependency-control"' \
        'version = "1.0.0"' \
        'edition = "2021"' >"$developer_package/Cargo.toml"
      printf 'pub fn cached() {}\n' >"$developer_package/src/lib.rs"
      cat >"$developer_cargo_home/config.toml" <<CONFIG
      [patch.crates-io]
      buoy-undeclared-dependency-control = { path = "$developer_package" }
      [net]
      offline = true
      CONFIG
      if ! (
        cd "$undeclared_control"
        CARGO_HOME="$developer_cargo_home" cargo metadata --offline --format-version=1 >/dev/null 2>&1
      ); then
        echo "SUPPLY-CACHE-CONTROL: the developer Cargo cache did not supply its control package" >&2
        exit 1
      fi
      if (
        cd "$undeclared_control"
        CARGO_HOME="$TMPDIR/empty-cargo-home" cargo metadata --offline --format-version=1 >/dev/null 2>&1
      ); then
        echo "SUPPLY-UNDECLARED-DEPENDENCY: the developer Cargo cache supplied an undeclared dependency" >&2
        exit 1
      fi
      cargo metadata --locked --offline --format-version=1 >/dev/null
      quality_artifacts="$TMPDIR/quality-artifacts"
      cargo run --quiet --locked --offline --bin buoy-quality -- \
        --generate --output "$quality_artifacts"
      for binding in \
        "assurance/v1/quality-assurance-bindings.tsv:quality-assurance-bindings.tsv" \
        "docs/assurance/quality-assurance-status.md:quality-assurance-status.md"; do
        committed="''${binding%%:*}"
        generated="$quality_artifacts/''${binding#*:}"
        if ! cmp -s "$committed" "$generated"; then
          echo "SUPPLY-QUALITY-DRIFT: clean source changed $committed" >&2
          diff -u "$committed" "$generated" >&2 || true
          exit 1
        fi
      done
      # Round-5: the scenario-manifest execution harness runs each manifest
      # row's DECLARED COMMAND for real. PBD-004's row invokes
      # `check-derivation-contract.sh`, which asks Nix to evaluate the
      # consumer package -- structurally impossible INSIDE a Nix builder,
      # which has no daemon and no network, so it exits 1 with
      # `unevaluable-variant`. That is a property of the sandbox, not a
      # defect in the row.
      #
      # This is a narrow exclusion of ONE test, not a skip branch inside a
      # gate: the same test runs unsandboxed in `buoy:test` (which this
      # graph also requires) where every one of its 78 rows executes its
      # real command. Excluding it here keeps the sandbox lane honest about
      # what it can actually verify instead of asserting a false expectation.
      cargo test --locked --offline --workspace --all-targets --all-features \
        -- --skip every_manifest_row_executes_its_declared_command_in_an_isolated_fixture
      cargo metadata --locked --offline --format-version=1 \
        --manifest-path dylint_lints/fcis_core/Cargo.toml >/dev/null
      cargo check --locked --offline --manifest-path dylint_lints/fcis_core/Cargo.toml
      runHook postBuild
    '';

    installPhase = ''
      runHook preInstall
      mkdir -p "$out"
      buoy-source-manifest >"$out/source-manifest.tsv"
      cat > "$out/result.toml" <<'RESULT'
      schema = "buoy-clean-cargo-acceptance/v1"
      status = "green"
      network = "disabled-by-nix-build-sandbox"
      user_home = "empty"
      cargo_home = "empty-except-declared-nix-vendor-config"
      rustup_home = "empty"
      ambient_executables = "excluded-by-nix-build-input-closure"
      scope = "Cargo workspace tests and Dylint build only"
      exact_final_oracle = "not-run"
      assurance_effect = "no-class-upgrade"
      RESULT
      runHook postInstall
    '';
  };

  cleanCargoDrvPath = builtins.unsafeDiscardStringContext cleanCargoAcceptance.drvPath;

  cleanCargoRunner = pkgs.writeShellApplication {
    name = "buoy-clean-cargo-acceptance";
    runtimeInputs = [
      pkgs.coreutils
      pkgs.nix
      sourceManifestRunner
    ];
    text = ''
      set -euo pipefail
      sandbox="$(nix config show sandbox 2>/dev/null || nix show-config sandbox)"
      case "$sandbox" in
        true|"sandbox = true") ;;
        *)
          echo "SUPPLY-SANDBOX-DISABLED: Nix sandbox must be enabled" >&2
          exit 1
          ;;
      esac
      current_source_manifest="$(mktemp)"
      trap 'rm -f "$current_source_manifest"' EXIT
      buoy-source-manifest >"$current_source_manifest"
      started="$SECONDS"
      result="$(nix-store --realise ${cleanCargoDrvPath})"
      test -f "$result/result.toml"
      cmp "$current_source_manifest" "$result/source-manifest.tsv"
      grep -Fx 'status = "green"' "$result/result.toml" >/dev/null
      grep -Fx 'exact_final_oracle = "not-run"' "$result/result.toml" >/dev/null
      printf 'clean-cargo-acceptance: green scope=cargo-and-dylint-builds build_seconds=%s noncanonical=true\n' "$((SECONDS - started))"
    '';
  };

  closureComponents = {
    advisory-db = inputs.advisory-db;
    aeneas = formalTools.aeneas;
    aeneas-source = inputs.aeneas;
    bash = pkgs.bash;
    cargo-deny = pkgs.cargo-deny;
    cargo-fuzz = pkgs.cargo-fuzz;
    cargo-llvm-cov = pkgs.cargo-llvm-cov;
    cargo-mutants = pkgs.cargo-mutants;
    cargo-nextest = pkgs.cargo-nextest;
    cargo-vendor = cargoVendor;
    coreutils = pkgs.coreutils;
    dylint-driver-source = dylintTools.dylintDriverSource;
    dylint-driver-vendor = dylintTools.dylintDriverVendor;
    dylint-tools = dylintTools;
    git = pkgs.git;
    git-hooks-source = inputs.git-hooks;
    gnugrep = pkgs.gnugrep;
    gnused = pkgs.gnused;
    jq = pkgs.jq;
    lean-aeneas = formalTools.leanAeneas;
    native-cc = pkgs.stdenv.cc;
    nix = pkgs.nix;
    nixfmt = pkgs.nixfmt;
    nixpkgs-source = inputs.nixpkgs;
    openspec = pkgs.openspec;
    prek = pkgs.prek;
    openssl = pkgs.openssl;
    pkg-config = pkgs.pkg-config;
    rust-clippy-source = inputs.rust-clippy;
    rust-overlay-source = inputs.rust-overlay;
    rust-toolchain = rustToolchain;
    rustup = pkgs.rustup;
    unicode-casefold-data = pkgs.runCommand "unicode-casefold-data-16.0.0" { } ''
      mkdir -p "$out/share/unicode/16.0.0"
      cp ${../unicode/16.0.0/CaseFolding.txt} "$out/share/unicode/16.0.0/CaseFolding.txt"
    '';
    verus = formalTools.verus;
    verus-rust-toolchain = formalTools.verusRustToolchain;
    z3-standalone = formalTools.z3;
    zlib = pkgs.zlib;
  };

  declaredClosure = pkgs.linkFarm "buoy-declared-supply-chain-closure-${rustTarget}" (
    lib.mapAttrsToList (name: path: { inherit name path; }) closureComponents
  );

  closureReportRunner = pkgs.writeShellApplication {
    name = "buoy-closure-report";
    runtimeInputs = [
      pkgs.coreutils
      pkgs.jq
      pkgs.nix
    ];
    text = ''
      set -euo pipefail
      rows="$(mktemp)"
      errors="$(mktemp)"
      trap 'rm -f "$rows" "$errors"' EXIT
      report_component() {
        component="$1"
        path="$2"
        if ! information="$(nix path-info --json --json-format 1 -S "$path" 2>"$errors")"; then
          cat "$errors" >&2
          exit 1
        fi
        nar_bytes="$(printf '%s' "$information" | jq -r 'to_entries[0].value.narSize')"
        closure_bytes="$(printf '%s' "$information" | jq -r 'to_entries[0].value.closureSize')"
        printf '%s\t%s\t%s\t%s\tnone\tnone\n' \
          "$component" ${lib.escapeShellArg rustTarget} "$nar_bytes" "$closure_bytes" >>"$rows"
      }
      report_component aggregate ${lib.escapeShellArg (toString declaredClosure)}
      ${lib.concatStringsSep "\n" (
        lib.mapAttrsToList (
          name: path: "report_component ${lib.escapeShellArg name} ${lib.escapeShellArg (toString path)}"
        ) closureComponents
      )}
      printf 'component\tplatform\tnar_bytes\tclosure_bytes\tsemantic_identity_effect\tassurance_effect\n'
      LC_ALL=C sort "$rows"
    '';
  };
in
{
  inherit
    cargoConfig
    cargoVendor
    cleanCargoAcceptance
    cleanCargoRunner
    closureComponents
    closureReportRunner
    declaredClosure
    dylintCargoSources
    hegelBuildCargoSources
    sourceManifestRunner
    workspaceCargoSources
    ;
  advisoryDb = inputs.advisory-db;
}
