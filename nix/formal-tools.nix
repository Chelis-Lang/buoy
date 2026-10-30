{
  pkgs,
  inputs,
  rustPkgs,
}:

let
  verusVersion = "0.2026.07.12.0b42f4c";
  verusTag = "release/${verusVersion}";
  verusRustTarget = pkgs.stdenv.hostPlatform.rust.rustcTarget;
  verusRustToolchainName = "1.96.0-${verusRustTarget}";
  verusRustToolchain = rustPkgs.rust-bin.stable."1.96.0".minimal;
  verusAssets = {
    aarch64-darwin = {
      name = "verus-${verusVersion}-arm64-macos.zip";
      hash = "sha256-IS2hScB6RiHBPQMrCmr1CyvLODMi09ZdASDsQ2qGS28=";
    };
    x86_64-darwin = {
      name = "verus-${verusVersion}-x86-macos.zip";
      hash = "sha256-wiZmWEhGN7uA+cCa3KBCpVaPjIzyktQF08ed4uXSpWs=";
    };
    x86_64-linux = {
      name = "verus-${verusVersion}-x86-linux.zip";
      hash = "sha256-9vT10I4H0+Gtch13W9pbqWud0Mc7SPwX8uBxhm+9AcA=";
    };
  };
  verusAsset =
    verusAssets.${pkgs.system}
      or (throw "Verus ${verusVersion} has no pinned upstream archive for ${pkgs.system}");

  verus = pkgs.stdenvNoCC.mkDerivation {
    pname = "verus";
    version = verusVersion;
    src = pkgs.fetchurl {
      url = "https://github.com/verus-lang/verus/releases/download/${verusTag}/${verusAsset.name}";
      inherit (verusAsset) hash;
    };
    nativeBuildInputs = [
      pkgs.makeWrapper
      pkgs.unzip
    ];
    dontUnpack = true;
    installPhase = ''
      runHook preInstall
      mkdir -p "$out/libexec" "$out/bin" unpacked
      unzip -q "$src" -d unpacked
      distribution="$(find unpacked -mindepth 1 -maxdepth 1 -type d -print -quit)"
      if [ -z "$distribution" ] || [ ! -x "$distribution/verus" ] || [ ! -x "$distribution/z3" ]; then
        echo "Verus archive has an unsupported layout" >&2
        exit 1
      fi
      cp -R "$distribution" "$out/libexec/verus"
      makeWrapper "$out/libexec/verus/verus" "$out/bin/verus" \
        --set RUSTUP_TOOLCHAIN "${verusRustToolchainName}"
      makeWrapper "$out/libexec/verus/cargo-verus" "$out/bin/cargo-verus" \
        --set RUSTUP_TOOLCHAIN "${verusRustToolchainName}"
      runHook postInstall
    '';
    passthru = {
      sourceTag = verusTag;
      sourceCommit = "0b42f4cee92a178937608cf55e512371d2fd8cd4";
      archiveName = verusAsset.name;
      archiveHash = verusAsset.hash;
      bundledZ3Relative = "libexec/verus/z3";
    };
    meta = {
      description = "Pinned Verus verifier release distribution";
      homepage = "https://github.com/verus-lang/verus";
      license = pkgs.lib.licenses.mit;
      platforms = builtins.attrNames verusAssets;
      mainProgram = "verus";
    };
  };

  # The Aeneas nightly-2026.07.01 Lean backend needs Lean 4.30.0. Nixpkgs
  # already pins that exact release and the binary cache serves it prebuilt.
  # Overriding the source to the v4.30.0-rc2 tag changed the derivation hash,
  # which made the package unsubstitutable and forced a source build of about
  # two and three quarter hours on every clean machine.
  leanAeneas = pkgs.lean4;

  charonFlake = inputs.aeneas.inputs.charon;
  charonToolchain = charonFlake.packages.${pkgs.system}.rustToolchain;
  charonUnwrapped =
    (charonFlake.packages.${pkgs.system}.charon-unwrapped.override {
      miriSysroots = null;
    }).overrideAttrs
      (_: {
        doCheck = false;
        checkPhase = "true";
        checkPhaseCargoCommand = "true";
      });
  charon =
    pkgs.runCommand "charon-without-full-mir-sysroots"
      {
        nativeBuildInputs = [
          pkgs.makeWrapper
        ]
        ++ pkgs.lib.optionals pkgs.stdenv.isDarwin [ pkgs.stdenv.cc.bintools ];
      }
      (
        ''
          cp -r ${charonUnwrapped} $out
          chmod -R u+w $out
          wrapProgram $out/bin/charon \
            --set CHARON_TOOLCHAIN_IS_IN_PATH 1 \
            --prefix LD_LIBRARY_PATH : "${pkgs.lib.makeLibraryPath [ charonToolchain ]}" \
            --prefix PATH : "${pkgs.lib.makeBinPath [ charonToolchain ]}"
        ''
        + pkgs.lib.optionalString pkgs.stdenv.isDarwin ''
          install_name_tool -add_rpath "${charonToolchain}/lib" "$out/bin/charon-driver"
        ''
      );
  charonMl = inputs.aeneas.packages.${pkgs.system}.charon-ml.override {
    inherit charon;
  };
  aeneas = inputs.aeneas.packages.${pkgs.system}.aeneas.override {
    inherit charon;
    charon-ml = charonMl;
  };
in
{
  inherit
    aeneas
    charon
    leanAeneas
    verus
    verusRustToolchain
    verusRustToolchainName
    ;
  z3 = pkgs.z3;
  aeneasSource = inputs.aeneas;
}
