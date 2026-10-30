## Single owner of the pinned Nix evaluation context and the declared source
## closure for `nix/consumer-package-v1.nix`.
##
## Every consumer-package entry point (`realize-consumer-package.nix`, the
## derivation-contract check, and the offline realization script) imports this
## one file, so the pinned `nixpkgs`/`rust-overlay` revisions, the pinned Rust
## toolchain, the declared source filter, and the reviewed source-content
## digest each have exactly one definition in the repository.
##
## Read the current root pins from `devenv.lock`.
## Consumer builds and the development shell use the same reviewed inputs.
## Initial source acquisition can need network access.
{ }:

let
  sourceRoot = builtins.findFile [
    {
      prefix = "";
      path = ../..;
    }
  ] "";
  lock = builtins.fromJSON (builtins.readFile ../../devenv.lock);
  rootInputs = lock.nodes.${lock.root}.inputs;
  nixpkgsSrc = builtins.fetchTree lock.nodes.${rootInputs.nixpkgs}.locked;
  rustOverlaySrc = builtins.fetchTree lock.nodes.${rootInputs.rust-overlay}.locked;
  pkgs = import nixpkgsSrc {
    overlays = [ (import rustOverlaySrc) ];
  };
  lib = pkgs.lib;

  ## Task 10.5 / M13: the source-content digest is a committed, reviewed
  ## constant. It is NEVER derived from the tree that is about to be built
  ## and then compared against itself; realization declares this reviewed
  ## value to Nix, and Nix's own fixed-output check derives the realized
  ## tree's content hash and fails closed when the two differ.
  ##
  ## `nix/tools/derive-source-content-digest.sh --update` is the single
  ## maintainer-invoked path that rewrites this reviewed constant, and it is
  ## deliberately separate from every verification path.
  sourceContentLock = builtins.fromJSON (
    builtins.readFile ../consumer-package-v1.source-content.json
  );
in
{
  inherit pkgs lib;

  rustToolchain = pkgs.rust-bin.fromRustupToolchainFile ../../rust-toolchain.toml;

  ## The declared source excludes local state and credential files.
  ## Example environment files remain source inputs.
  ## The reviewed digest lock stays outside its own source closure.
  ## Its content hash cannot include itself.
  source = lib.cleanSourceWith {
    name = "buoy-declared-source";
    src = sourceRoot;
    filter =
      path: _type:
      let
        base = baseNameOf (toString path);
        isLocalEnvironment =
          (base == ".env" || lib.hasPrefix ".env." base) && !(lib.hasSuffix ".example" base);
      in
      toString path != toString (sourceRoot + "/nix/consumer-package-v1.source-content.json")
      # Task 10.16: the retained closure-size and build-cost measurements are
      # environment-dependent output, never a build input and never part of
      # any identity, so they stay outside the declared source closure that
      # the reviewed content digest covers.
      && toString path != toString (sourceRoot + "/nix/measurements")
      # Local credentials are not source inputs.
      && !isLocalEnvironment
      # Audit round 5, high finding 10: `.drain-*` files are transient
      # orchestration artifacts (drain prompts, drain reports, drain audit
      # rounds), never package source. Including them would make the reviewed
      # source-content digest change whenever an orchestration file is added
      # or cleaned up, staling the reviewed constant and breaking the blocking
      # gate for a reason that has nothing to do with the package. This one
      # filter is the single definition both the digest derivation
      # (`derive-source-content-digest.sh`) and the package `src` use, so the
      # two can never disagree about what the declared closure contains.
      && !(lib.hasPrefix ".drain-" base)
      # The exact-coverage process creates this isolated test repository below
      # the worktree. It is transient test state, not package source.
      && !(lib.hasPrefix ".buoy-product-check-" base)
      # Instrumented child processes can emit raw profiles at the worktree
      # root. Raw coverage profiles are runtime output, not package source.
      && !(lib.hasSuffix ".profraw" base)
      # Generated hooks, toolchain links, and fuzz output are local state.
      && toString path != toString (sourceRoot + "/.pre-commit-config.yaml")
      && toString path != toString (sourceRoot + "/dylint_lints/fcis_core/rust-toolchain.toml")
      && toString path != toString (sourceRoot + "/fuzz/artifacts")
      && !(builtins.elem base [
        ".claude"
        ".devenv"
        ".direnv"
        ".git"
        ".pi"
        ".worktrees"
        "target"
        "result"
      ]);
  };

  reviewedSourceContentLock = sourceContentLock;
  reviewedSourceContentHash = sourceContentLock.reviewed_source_content_hash;
}
