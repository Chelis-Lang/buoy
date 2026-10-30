{
  config,
  inputs,
  pkgs,
  ...
}:

let
  rustPkgs = pkgs.extend inputs.rust-overlay.overlays.default;
  rustTarget = pkgs.stdenv.hostPlatform.rust.rustcTarget;
  rustToolchainName = "buoy-${rustTarget}";
  rustToolchain = config.languages.rust.toolchainPackage;
  dylintTools = pkgs.dylint-tools;
  formalTools = pkgs.callPackage ./nix/formal-tools.nix { inherit inputs rustPkgs; };
  supplyChain = pkgs.callPackage ./nix/supply-chain.nix {
    inherit
      dylintTools
      formalTools
      inputs
      rustTarget
      rustToolchain
      ;
  };
  cargoHome = "${toString ./.}/.devenv/state/cargo-home";
  dylintDriverPath = "${toString ./.}/.devenv/state/dylint-drivers";
  rustupHome = "${toString ./.}/.devenv/state/rustup";
in
{
  imports = [
    ./nix/devenv/environment.nix
    ./nix/devenv/hooks.nix
    ./nix/devenv/tasks.nix
    ./nix/devenv/scripts/core.nix
    ./nix/devenv/scripts/assurance.nix
    ./nix/devenv/scripts/adversarial.nix
    ./nix/devenv/scripts/acceptance.nix
  ];

  _module.args = {
    inherit
      cargoHome
      dylintDriverPath
      dylintTools
      formalTools
      rustTarget
      rustToolchainName
      rustupHome
      supplyChain
      ;
  };
}
