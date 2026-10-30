## Task 10.10: advisory pinned private non-flake `git+ssh` consumer-package
## source fixture.
##
## This file is never imported by `devenv.nix` and is never realized as part
## of any automatic Buoy or consumer task. It documents, as one real,
## reviewable Nix expression, how a private downstream consumer pins Buoy
## through a non-flake `git+ssh` source: an exact `rev`, `ref`, and
## `narHash`, with an `sshKey` obtained only from an externally supplied
## file (never a repository-committed secret).
##
## A maintainer who has set `BUOY_CONSUMER_PACKAGE_SSH_KEY` to a filesystem
## path (per `tools/check_consumer_package.rs`) may realize this to confirm
## the pin resolves. Realization is advisory and requires read-only SSH
## access to the named private remote; it stores no credential and this
## expression commits none.
{
  url ? "git+ssh://git@example.invalid/buoy-private-mirror.git",
  rev,
  ref ? "main",
  narHash,
  sshKeyFile ? builtins.getEnv "BUOY_CONSUMER_PACKAGE_SSH_KEY",
}:

assert rev != "";
assert narHash != "";

builtins.fetchGit {
  inherit
    url
    rev
    ref
    narHash
    ;
  # `submodules` stays false: a private consumer pin names exactly one
  # source tree, not a transitively resolved submodule graph.
  submodules = false;
}
// {
  # Advisory metadata only: this attribute is not consumed by
  # `builtins.fetchGit`, and no repository-tracked file reads it.
  sshKeyFileDeclaredExternally = sshKeyFile != "";
}
