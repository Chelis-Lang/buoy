{
  lib,
  stdenv,
  makeRustPlatform,
  src,
  sourceRevision,
  sourceContentHash,
  rustToolchain,
  system,
  digestFeature,
}:

assert sourceRevision != "";
assert sourceContentHash != "";
assert rustToolchain != null;
assert system != "";
assert builtins.elem digestFeature [
  "xxh3-128"
  "blake3-256"
  "sha-256"
];

let
  # The blocking package function MUST construct its own `rustPlatform` from
  # the declared pinned `rustToolchain` -- never accept a caller-suppliable
  # `rustPlatform` that could silently resolve to nixpkgs' ambient default
  # Rust compiler if a caller forgot to override it. `makeRustPlatform` binds
  # both `cargo` and `rustc` to the exact pinned toolchain derivation this
  # function was already given.
  rustPlatform = makeRustPlatform {
    cargo = rustToolchain;
    rustc = rustToolchain;
  };

  # The blocking package function MUST genuinely verify `src`'s content
  # against the declared `sourceContentHash` -- not merely assert it is
  # nonempty text and otherwise ignore it, and never by deriving the
  # expected value from the very tree it is about to check. The caller
  # declares one committed, reviewed constant
  # (`nix/consumer-package-v1.source-content.json`); this fixed-output
  # derivation copies `src` unchanged and Nix itself derives the realized
  # tree's recursive content hash and rejects the build when it differs, so
  # an arbitrary or stale reviewed value fails realization instead of
  # silently building whatever `src` happens to contain.
  #
  # This verification is wired into the package below as a build-time input
  # (`BUOY_VERIFIED_SOURCE` plus the `preBuild` assertion), so `nix-build`
  # cannot produce an executable unless the reviewed digest matched. It is
  # deliberately NOT the `src`/`cargoLock.lockFile` path, so that merely
  # *evaluating* the package contract (`nix derivation show`) needs no
  # import-from-derivation and stays independent of digest freshness while
  # realization stays fail-closed.
  verifiedSource = stdenv.mkDerivation {
    pname = "buoy-verified-source";
    version = sourceRevision;
    inherit src;
    dontUnpack = true;
    dontConfigure = true;
    dontBuild = true;
    # This step only verifies and relocates the exact source tree; it
    # must not run any packaging phase that alters, patches, or validates
    # a "finished package" shape (shebangs, broken-symlink checks,
    # reference scrubbing). The source tree's own dangling development
    # symlinks (unrelated to the Rust build) are real, expected content,
    # not a defect this copy step should reject.
    dontFixup = true;
    dontPatchShebangs = true;
    # `cp -a` reproduces the source tree's Nix archive exactly: symbolic
    # links stay links and executable bits stay set, so the realized content
    # hash is the declared source closure's own content hash and not the
    # hash of a silently normalized copy.
    installPhase = ''
      cp -a "$src" "$out"
      chmod -R u+w "$out"
    '';
    outputHashMode = "recursive";
    outputHashAlgo = "sha256";
    outputHash = sourceContentHash;
  };
in
rustPlatform.buildRustPackage {
  # Match nixpkgs build-rust-crate and exclude compiler source paths from the closure.
  RUSTFLAGS = [ "--remap-path-prefix=${rustToolchain}=/rustc" ];
  disallowedReferences = [ rustToolchain ];
  pname = "buoy";
  version = sourceRevision;
  inherit src;
  cargoLock.lockFile = src + "/Cargo.lock";

  # The declared source closure only becomes an executable after the
  # reviewed-digest verification above realizes successfully: referencing
  # `verifiedSource` here makes it a build input of this derivation, and
  # the assertion documents that dependency in the build log.
  BUOY_VERIFIED_SOURCE = verifiedSource;
  preBuild = ''
    test -d "$BUOY_VERIFIED_SOURCE"
  '';
  # Install only the public commands. Repository tools remain Cargo targets.
  # Keep the digest feature selection explicit for both packages.
  cargoBuildFlags = [
    "--package"
    "buoy"
    "--package"
    "chelis-provenance"
    "--bin"
    "buoy"
    "--bin"
    "buoy-migrate"
    "--bin"
    "chelis-provenance"
    "--no-default-features"
    "--features"
    ("buoy/" + digestFeature + ",chelis-provenance/" + digestFeature)
  ];
  nativeBuildInputs = [ rustToolchain ];
  doCheck = false;

  # Task 10.6: embed operational package identity through these exact
  # reproducible derivation inputs, so `buoy package-identity` reports the
  # real values this derivation was built from rather than a build-time
  # placeholder. `src/cli.rs` reads these through `option_env!` at compile
  # time and falls back to development defaults outside this derivation.
  BUOY_PACKAGE_SOURCE_REVISION = sourceRevision;
  BUOY_PACKAGE_DIGEST_FEATURE = digestFeature;
  BUOY_PACKAGE_TOOLCHAIN = rustToolchain.name or "unknown-toolchain";

  # The declared, machine-inspectable package contract surface. A contract
  # check can read these exact values through `nix eval` without realizing
  # the package, so the reviewed inputs this function was actually applied
  # to are verifiable facts rather than a text scan of this file.
  passthru = {
    inherit
      verifiedSource
      sourceRevision
      sourceContentHash
      digestFeature
      ;
    declaredVariants = [
      "xxh3-128"
      "blake3-256"
      "sha-256"
    ];
  };

  meta = {
    description = "Pinned Buoy executable";
    homepage = "https://github.com/Chelis-Lang/buoy";
    license = lib.licenses.mit;
    platforms = [ system ];
  };

  postInstall = ''
    install -Dm644 LICENSE "$out/share/licenses/buoy/LICENSE"
    install -Dm644 crates/chelis-surf-sitter/LICENSE-CHELIS "$out/share/licenses/buoy/LICENSE-CHELIS"
    install -Dm644 crates/chelis-surf-sitter/LICENSE-TREE-SITTER "$out/share/licenses/buoy/LICENSE-TREE-SITTER"
    install -Dm644 unicode/16.0.0/LICENSE "$out/share/licenses/buoy/LICENSE-UNICODE"
  '';
}
