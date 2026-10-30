{ config, pkgs, ... }:

let
  # `cargo fmt` resolves its `cargo-fmt` subcommand through PATH, not through
  # the directory of the cargo binary. Invoking cargo by absolute store path
  # therefore fails with `no such command: fmt` unless the toolchain bin is on
  # PATH, which only happens inside the devenv shell. Wrap the check so it works
  # for any invocation, including `nix run nixpkgs#pre-commit` outside the shell.
  rustfmtCheck = pkgs.writeShellScriptBin "buoy-rustfmt-check" ''
    export PATH="${config.languages.rust.toolchainPackage}/bin:$PATH"
    exec cargo fmt --all -- --check
  '';
in
{
  git-hooks.hooks = {
    buoy-rustfmt = {
      enable = true;
      name = "Rust format";
      entry = "${rustfmtCheck}/bin/buoy-rustfmt-check";
      files = "\\.rs$";
      pass_filenames = false;
      package = rustfmtCheck;
    };

    buoy-nixfmt = {
      enable = true;
      name = "Nix format";
      entry = "${pkgs.nixfmt}/bin/nixfmt --check";
      files = "\\.nix$";
      package = pkgs.nixfmt;
    };

    buoy-openspec = {
      enable = true;
      name = "Strict OpenSpec validation";
      entry = "${pkgs.openspec}/bin/openspec validate --all --strict --no-interactive";
      files = "^openspec/";
      pass_filenames = false;
      package = pkgs.openspec;
    };
  };
}
