{
  cargoHome,
  config,
  dylintDriverPath,
  dylintTools,
  formalTools,
  pkgs,
  rustTarget,
  rustToolchainName,
  rustupHome,
  supplyChain,
  ...
}:

let
  rustToolchain = config.languages.rust.toolchainPackage;
in
{
  languages.rust = {
    enable = true;
    toolchainFile = ../../rust-toolchain.toml;
  };

  env.CARGO_HOME = cargoHome;
  env.CARGO_NET_OFFLINE = "true";
  env.BUOY_ADVISORY_DB = toString supplyChain.advisoryDb;
  env.BUOY_CARGO_VENDOR = toString supplyChain.cargoVendor;
  env.BUOY_DECLARED_CLOSURE = toString supplyChain.declaredClosure;
  env.BUOY_DEVENV_POLICY = toString ./workflow-policy.toml;
  env.DYLINT_DRIVER_PATH = dylintDriverPath;
  env.BUOY_DYLINT_DRIVER_SOURCE = toString dylintTools.dylintDriverSource;
  env.BUOY_DYLINT_DRIVER_VENDOR = toString dylintTools.dylintDriverVendor;
  env.RUSTUP_HOME = rustupHome;
  env.RUSTUP_TOOLCHAIN = rustToolchainName;
  env.AENEAS_SOURCE = toString formalTools.aeneasSource;
  env.AENEAS_LEAN_BACKEND = "${toString formalTools.aeneasSource}/backends/lean";

  files."dylint_lints/fcis_core/rust-toolchain.toml".text = ''
    [toolchain]
    channel = "${rustToolchainName}"
    profile = "minimal"
  '';

  packages = [
    pkgs.rustup
    pkgs.cargo-nextest
    pkgs.git
    pkgs.nixfmt
    pkgs.openspec
    pkgs.openssl
    pkgs.pkg-config
    pkgs.prek
    pkgs.zlib
  ];

  tasks."devenv:rustup-links" = {
    description = "Create pinned Rustup aliases for Buoy, Dylint, and Verus";
    after = [ "devenv:files" ];
    before = [
      "devenv:enterShell"
      "devenv:enterTest"
    ];
    cwd = config.git.root;
    exec = ''
      set -euo pipefail
      mkdir -p "$CARGO_HOME" "$DYLINT_DRIVER_PATH" "$RUSTUP_HOME/toolchains"
      ln -sfn ${supplyChain.cargoConfig} "$CARGO_HOME/config.toml"
      ln -sfn ${rustToolchain} "$RUSTUP_HOME/toolchains/${rustToolchainName}"
      ln -sfn ${formalTools.verusRustToolchain} "$RUSTUP_HOME/toolchains/${formalTools.verusRustToolchainName}"
      ln -sfn ${rustToolchain} "$RUSTUP_HOME/toolchains/stable-${rustTarget}"
    '';
  };
}
