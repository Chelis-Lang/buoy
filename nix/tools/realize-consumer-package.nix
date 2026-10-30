## Standalone, on-demand realization of `nix/consumer-package-v1.nix`.
##
## This file is deliberately NOT imported by `devenv.nix`: embedding a real
## package build into any `scripts.*`/`tasks.*` derivation would force Nix to
## realize it every time a plain `devenv shell` assembles its environment,
## which is inappropriate for routine development. Invoke this expression
## explicitly (`nix-build nix/tools/realize-consumer-package.nix --argstr
## digestFeature blake3-256`) to obtain real, local, offline evidence for
## task 10.2/10.8/10.9 without adding that cost to every shell entry.
##
## The pinned evaluation context, the pinned Rust toolchain, the declared
## source closure, and the reviewed source-content digest all come from the
## single owner `nix/tools/declared-source.nix`.
{
  digestFeature ? "xxh3-128",
  ## Task 10.5 / M13: by default the reviewed, committed source-content
  ## constant from `nix/consumer-package-v1.source-content.json` is declared
  ## to Nix, which then derives the realized tree's own content hash and
  ## fails closed when the two differ. A caller may override this only to
  ## exercise the failure path (negative controls) or to bootstrap a new
  ## reviewed value through `derive-source-content-digest.sh --update`;
  ## no verification path may compute it from the tree under test.
  sourceContentHash ? null,
}:

let
  declared = import ./declared-source.nix { };
in
declared.pkgs.callPackage ../consumer-package-v1.nix {
  # `makeRustPlatform` is auto-wired by `callPackage` from ambient `pkgs`;
  # the package function itself constructs its active `rustPlatform` from
  # the declared pinned `rustToolchain` this file passes below, so this
  # caller no longer needs to (and cannot) override `rustPlatform` directly.
  src = declared.source;
  sourceRevision = "local-worktree-realization";
  sourceContentHash =
    if sourceContentHash != null then sourceContentHash else declared.reviewedSourceContentHash;
  inherit (declared) rustToolchain;
  system = declared.pkgs.stdenv.hostPlatform.system;
  inherit digestFeature;
}
