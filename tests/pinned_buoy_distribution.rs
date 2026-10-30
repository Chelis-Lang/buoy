#![doc = "Real production-path controls for PBD-001, PBD-002, and PBD-004."]
//!
//! Every negative control in this file mutates the real committed artifact
//! and then RUNS THE REAL GATE against the mutation, requiring a nonzero
//! exit and the real diagnostic. No control appends text to a file and then
//! asserts that the text it just appended is present.

// A control that cannot run must fail loudly rather than pass silently, so
// this file deliberately panics on a missing prerequisite (audit round 5,
// high finding 9).
#![allow(
    missing_docs,
    clippy::expect_used,
    clippy::panic,
    clippy::too_many_lines
)]

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::{Mutex, MutexGuard, PoisonError};

static NIX_EXECUTION_LOCK: Mutex<()> = Mutex::new(());

fn nix_execution_guard() -> MutexGuard<'static, ()> {
    NIX_EXECUTION_LOCK
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
}

const PACKAGE_FILE: &str = "nix/consumer-package-v1.nix";
const SOURCE_CONTENT_LOCK: &str = "nix/consumer-package-v1.source-content.json";
const MEASUREMENTS: &str = "nix/measurements/consumer-package-cost-v1.tsv";
const DERIVATION_CONTRACT: &str = "nix/tools/check-derivation-contract.sh";
const REALIZE: &str = "nix/tools/realize-consumer-package.nix";
const DECLARED_SOURCE: &str = "nix/tools/declared-source.nix";
const PACKAGE_FIXTURES: &str = "fixtures/trace-publication/v1/package/variants.json";

/// The ONE narrow, documented condition under which a Nix-backed control here
/// may decline to run: inside a Nix build sandbox there is no daemon, no
/// network, and no access to the pinned flake inputs the package expression
/// needs, so `nix-instantiate` exists on `PATH` but every evaluation is
/// structurally impossible. That is an environment fact, not a contract
/// result.
///
/// Audit round 5, high finding 9: outside that sandbox there is NO skip.
/// A missing `nix` is a hard failure, not a silent pass, so the normal
/// developer and CI path can never assert nothing. The package gate runs
/// `nix/tools/package-contract.sh --mode blocking` outside this helper.
fn nix_bin_directory() -> Option<PathBuf> {
    if std::env::var_os("NIX_BUILD_TOP").is_some() {
        eprintln!(
            "skipping inside a Nix build sandbox; the blocking task owner runs the same checks outside it"
        );
        return None;
    }
    match locate_nix() {
        Some(directory) => Some(directory),
        None => panic!(
            "no-nix-on-path: this control must run for real outside a Nix build sandbox; \
             install Nix or run it inside the devenv shell"
        ),
    }
}

/// Runs a Nix command with the located Nix directory prepended to `PATH`.
fn nix_command(nix: &Path, program: &str, arguments: &[&str]) -> Output {
    Command::new(nix.join(program))
        .args(arguments)
        .env(
            "PATH",
            format!(
                "{}:{}",
                nix.display(),
                std::env::var("PATH").unwrap_or_default()
            ),
        )
        .output()
        .unwrap_or_else(|error| panic!("{program} invocation: {error}"))
}

/// Runs a realized executable exactly as `verify-offline-realization.sh`
/// does: a cleared environment with empty Cargo, Rustup, and home
/// directories and black-holed proxies, so no ambient developer tool state
/// and no network can contribute to the result.
fn run_offline(binary: &Path, arguments: &[&str], empty_roots: &Path) -> Output {
    Command::new(binary)
        .args(arguments)
        .env_clear()
        .env("PATH", "/usr/bin:/bin")
        .env("HOME", empty_roots.join("home"))
        .env("CARGO_HOME", empty_roots.join("cargo"))
        .env("RUSTUP_HOME", empty_roots.join("rustup"))
        .env("http_proxy", "http://127.0.0.1:1")
        .env("https_proxy", "http://127.0.0.1:1")
        .env("all_proxy", "http://127.0.0.1:1")
        .env("HTTP_PROXY", "http://127.0.0.1:1")
        .env("HTTPS_PROXY", "http://127.0.0.1:1")
        .env("ALL_PROXY", "http://127.0.0.1:1")
        .output()
        .expect("realized executable invocation")
}

/// Runs the real, committed `check-package-contract` gate binary -- the same
/// one the blocking `buoy:package-contract` task owner invokes -- against a
/// given package file and reviewed source-content lock.
fn run_real_gate(package_file: &Path, lock_file: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_check-package-contract"))
        .args([
            "--package-file",
            &package_file.display().to_string(),
            "--lock-file",
            &lock_file.display().to_string(),
        ])
        .output()
        .expect("real check-package-contract invocation")
}

fn temporary_dir(label: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!("buoy-pbd-{label}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&path);
    fs::create_dir_all(&path).expect("temporary directory");
    path
}

/// PBD-001 positive: the real gate accepts the real committed package file.
#[test]
fn the_real_gate_accepts_the_real_committed_package_file() {
    let output = run_real_gate(Path::new(PACKAGE_FILE), Path::new(SOURCE_CONTENT_LOCK));
    assert!(
        output.status.success(),
        "exit={:?} stdout={} stderr={}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    for expected in [
        "src",
        "sourceContentHash",
        "xxh3-128",
        "blake3-256",
        "sha-256",
    ] {
        assert!(stdout.contains(expected), "stdout={stdout}");
    }
}

/// PBD-001 negative, and audit H8/M14: each row mutates the REAL committed
/// `nix/consumer-package-v1.nix` on disk (in a temporary copy) and runs the
/// REAL gate binary against that mutation, requiring exit 1 and the real
/// diagnostic. The unmutated committed file is re-checked above, so the
/// gate is proven discriminating rather than merely loud.
#[test]
fn the_real_gate_rejects_every_mutation_of_the_real_committed_package_file() {
    let text = fs::read_to_string(PACKAGE_FILE).expect("real committed package file");
    let directory = temporary_dir("package-mutation");
    let lock_file = Path::new(SOURCE_CONTENT_LOCK)
        .canonicalize()
        .expect("committed source-content lock");

    let mutations: Vec<(&str, &str, String)> = vec![
        // The closed input contract omits the required `src` input: a
        // package function that can build without an explicitly declared
        // source is the ambient-source leak PBD-001 forbids.
        (
            "omitted-src",
            "missing-required-input src",
            text.replacen("  src,\n", "", 1),
        ),
        // `src` present but defaulted: a caller who forgets it silently
        // gets an ambient tree.
        (
            "defaulted-src",
            "defaulted-required-input src",
            text.replacen("  src,\n", "  src ? ../..,\n", 1),
        ),
        // PBD-001's own negative scenario: a live branch fetch.
        (
            "live-branch-fetch",
            "live-source-fetch",
            text.replacen(
                "  inherit src;",
                "  src = fetchFromGitHub { owner = \"buoy-project\"; repo = \"buoy\"; ref = \"main\"; };",
                1,
            ),
        ),
        // PBD-004's own negative scenario: a launcher that clones at run
        // time.
        (
            "runtime-clone",
            "runtime-source-fetch",
            text.replacen(
                "  preBuild = ''",
                "  postInstall = ''\n    echo 'git clone https://example.invalid/buoy' >> \"$out/bin/buoy\"\n  '';\n  preBuild = ''",
                1,
            ),
        ),
        // An ambient Rust platform input defeats the pinned toolchain.
        (
            "ambient-rust-platform",
            "ambient-toolchain-input rustPlatform",
            text.replacen("  src,\n", "  src,\n  rustPlatform,\n", 1),
        ),
        // An open contract accepts undeclared ambient inputs.
        (
            "open-contract",
            "open-input-contract",
            text.replacen("  src,\n", "  src,\n  ...\n", 1),
        ),
        // The declared source-content digest stops gating the build.
        (
            "unverified-source-content",
            "unverified-source-content",
            text.replacen("outputHash = sourceContentHash;", "outputHash = null;", 1),
        ),
        // The verified source is no longer a build input of the executable.
        (
            "unenforced-source-verification",
            "unenforced-source-verification",
            text.replacen("BUOY_VERIFIED_SOURCE = verifiedSource;", "", 1),
        ),
        // Default Cargo features leak an undeclared digest member in.
        (
            "default-features",
            "default-features-enabled",
            text.replacen("\"--no-default-features\"", "\"--offline\"", 1),
        ),
        // A declared variant disappears from the interface.
        (
            "dropped-variant",
            "missing-declared-variant sha-256",
            text.replace("\"sha-256\"", "\"sha-512\""),
        ),
    ];

    for (label, expected_diagnostic, mutated) in mutations {
        assert_ne!(mutated, text, "mutation `{label}` changed nothing");
        let mutated_file = directory.join(format!("{label}.nix"));
        fs::write(&mutated_file, &mutated).expect("mutated package file");
        let output = run_real_gate(&mutated_file, &lock_file);
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert_eq!(
            output.status.code(),
            Some(1),
            "mutation `{label}` must make the real gate exit 1; stderr={stderr}"
        );
        assert!(
            stderr.contains(expected_diagnostic),
            "mutation `{label}` must produce `{expected_diagnostic}`; stderr={stderr}"
        );
        // The identical gate invocation accepts the unmutated committed file
        // from the same temporary directory, so the failure above is caused
        // by the mutation and not by the copy's location.
        let control_file = directory.join(format!("{label}-control.nix"));
        fs::write(&control_file, &text).expect("unmutated control copy");
        assert!(
            run_real_gate(&control_file, &lock_file).status.success(),
            "the unmutated committed file must still pass the same gate"
        );
    }
    let _ = fs::remove_dir_all(&directory);
}

/// Audit M13 negative: the reviewed source-content digest is a committed
/// constant. A mutated lock -- a self-derived placeholder, a wrong
/// algorithm, or an absent digest -- must make the real gate fail closed
/// against the unchanged committed package file.
#[test]
fn the_real_gate_rejects_a_mutated_reviewed_source_content_digest() {
    let package_file = Path::new(PACKAGE_FILE)
        .canonicalize()
        .expect("committed package file");
    let lock = fs::read_to_string(SOURCE_CONTENT_LOCK).expect("committed source-content lock");
    let directory = temporary_dir("lock-mutation");
    let mutations = [
        (
            "derive-me",
            lock.replace(
                lock.split("\"reviewed_source_content_hash\": \"")
                    .nth(1)
                    .and_then(|rest| rest.split('"').next())
                    .expect("committed digest"),
                "derive-me",
            ),
        ),
        ("wrong-algorithm", lock.replace("\"sha256\"", "\"md5\"")),
        ("wrong-mode", lock.replace("recursive-nar", "flat")),
    ];
    for (label, mutated) in mutations {
        assert_ne!(mutated, lock, "mutation `{label}` changed nothing");
        let mutated_lock = directory.join(format!("{label}.json"));
        fs::write(&mutated_lock, &mutated).expect("mutated lock");
        let output = run_real_gate(&package_file, &mutated_lock);
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert_eq!(
            output.status.code(),
            Some(1),
            "mutation `{label}` must fail closed; stderr={stderr}"
        );
        assert!(
            stderr.contains("source-content-lock") || stderr.contains("reviewed-source-content"),
            "mutation `{label}` must name the reviewed digest; stderr={stderr}"
        );
    }
    let _ = fs::remove_dir_all(&directory);
}

/// PBD-002 and PBD-004, audit M11/M12: the real Nix package, evaluated.
///
/// This runs the committed `nix/tools/check-derivation-contract.sh`, the
/// same script the blocking task owner runs, which asserts values Nix itself
/// computed from `nix/consumer-package-v1.nix`: three distinct real
/// derivations with per-variant `cargoBuildFlags` and embedded identity, a
/// rejected undeclared member, a non-fixed-output (sandboxed, network-free)
/// package derivation, the reviewed source verification as a real build
/// input, and a rejected unreviewed source-content digest.
///
/// The full offline command matrix stays in
/// `nix/tools/verify-offline-realization.sh`, which the blocking task owner
/// also runs; real per-variant realization is bound below.
#[test]
fn the_real_nix_package_satisfies_the_derivation_contract() {
    let _guard = nix_execution_guard();
    let Some(nix) = nix_bin_directory() else {
        return;
    };
    let output = Command::new("bash")
        .arg(DERIVATION_CONTRACT)
        .env(
            "PATH",
            format!(
                "{}:{}",
                nix.display(),
                std::env::var("PATH").unwrap_or_default()
            ),
        )
        .output()
        .expect("derivation contract invocation");
    assert!(
        output.status.success(),
        "exit={:?} stdout={} stderr={}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    for expected in [
        "xxh3-128 ok",
        "blake3-256 ok",
        "sha-256 ok",
        "undeclared variant md5-128 rejected",
        "unreviewed source content digest rejected",
        "ok variants=xxh3-128 blake3-256 sha-256",
    ] {
        assert!(stdout.contains(expected), "stdout={stdout}");
    }
}

/// Audit round 5, high finding 9, PBD-002 and PBD-004 bound to REAL package
/// realization and REAL command output rather than to a Rust helper.
///
/// Task 1.11's package fixture file enumerates the digest features and the
/// missing-source-identity and ambient-tool-leakage rejections; every case in
/// it is driven here against the real Nix package:
///
///   * each declared digest feature is REALIZED (`nix-build`) and the
///     resulting executable's REAL `package-identity` output must report that
///     exact feature -- so a variant that builds the wrong feature set fails
///     (PBD-002 positive);
///   * the realized launcher is a compiled executable, embeds no
///     host-specific source path, and runs with a cleared environment and
///     empty Cargo/Rustup/home directories (PBD-004 positive);
///   * a case with no source identity must not even evaluate, because the
///     package function asserts a nonempty `sourceRevision`; and
///   * ambient Cargo/Rustup state must not change the derivation, so the
///     instantiated derivation path is identical with and without polluted
///     ambient tool homes on the evaluating environment.
#[test]
fn every_package_fixture_case_is_bound_to_the_real_realized_package() {
    let _guard = nix_execution_guard();
    let Some(nix) = nix_bin_directory() else {
        return;
    };
    let fixtures: serde_json::Value =
        serde_json::from_slice(&fs::read(PACKAGE_FIXTURES).expect("package fixture file"))
            .expect("package fixture JSON");
    let cases = fixtures["cases"].as_array().expect("fixture cases").clone();
    let empty_roots = temporary_dir("realized-offline");
    for name in ["home", "cargo", "rustup"] {
        fs::create_dir_all(empty_roots.join(name)).expect("empty root");
    }
    let repository_root = std::env::current_dir().expect("repository root");

    let mut accepted = 0_usize;
    let mut rejected = 0_usize;
    for case in &cases {
        let name = case["name"].as_str().expect("case name");
        let features: Vec<&str> = case["features"]
            .as_array()
            .expect("case features")
            .iter()
            .filter_map(serde_json::Value::as_str)
            .collect();
        let feature = *features.first().expect("one declared digest feature");

        if case["expected"] == "accept" {
            accepted += 1;
            let build = nix_command(
                &nix,
                "nix-build",
                &[
                    REALIZE,
                    "--argstr",
                    "digestFeature",
                    feature,
                    "--no-out-link",
                ],
            );
            assert!(
                build.status.success(),
                "case `{name}`: realization failed; stderr={}",
                String::from_utf8_lossy(&build.stderr)
            );
            let store_path = PathBuf::from(
                String::from_utf8_lossy(&build.stdout)
                    .lines()
                    .last()
                    .expect("realized store path")
                    .trim(),
            );
            let binary = store_path.join("bin/buoy");
            assert!(binary.is_file(), "case `{name}`: {binary:?} is not a file");

            // PBD-004: the launcher is the compiled executable itself, never
            // a script wrapper that could clone or fetch before running.
            let bytes = fs::read(&binary).expect("realized launcher");
            assert_ne!(
                bytes.get(..2),
                Some(b"#!".as_slice()),
                "case `{name}`: the launcher must be a compiled executable"
            );

            // PBD-002 through the REAL command: the realized executable's own
            // machine report must name the feature it was realized with.
            let identity = run_offline(&binary, &["package-identity"], &empty_roots);
            assert!(
                identity.status.success(),
                "case `{name}`: package-identity failed offline; stderr={}",
                String::from_utf8_lossy(&identity.stderr)
            );
            let report: serde_json::Value =
                serde_json::from_slice(&identity.stdout).expect("package-identity JSON");
            assert_eq!(
                report["features"],
                serde_json::json!([feature]),
                "case `{name}`: realized package reports {report}"
            );
            assert_eq!(report["schema"], "buoy/package-identity/v1");

            // PBD-004: no host-specific source path is embedded, so the
            // realized package cannot reach back into a developer worktree.
            let root = repository_root.display().to_string();
            assert!(
                !bytes
                    .windows(root.len())
                    .any(|window| window == root.as_bytes()),
                "case `{name}`: the realized launcher embeds {root}"
            );
            continue;
        }

        rejected += 1;
        match name {
            "missing-source-identity" => {
                let expression = format!(
                    "let d = import ./{DECLARED_SOURCE} {{ }}; in d.pkgs.callPackage ./{PACKAGE_FILE} {{ \
                     src = d.source; sourceRevision = \"\"; \
                     sourceContentHash = d.reviewedSourceContentHash; \
                     inherit (d) rustToolchain; \
                     system = d.pkgs.stdenv.hostPlatform.system; \
                     digestFeature = \"{feature}\"; }}"
                );
                let output = nix_command(&nix, "nix-instantiate", &["-E", &expression]);
                assert!(
                    !output.status.success(),
                    "case `{name}`: a package with no source identity must not evaluate"
                );
                let stderr = String::from_utf8_lossy(&output.stderr);
                assert!(
                    stderr.contains("assertion") && stderr.contains("sourceRevision"),
                    "case `{name}`: expected the sourceRevision assertion; stderr={stderr}"
                );
            }
            "ambient-tool-leakage" => {
                let clean = nix_command(
                    &nix,
                    "nix-instantiate",
                    &[REALIZE, "--argstr", "digestFeature", feature],
                );
                assert!(clean.status.success(), "case `{name}`: clean instantiate");
                let ambient_home = case["ambient_cargo_home"]
                    .as_str()
                    .unwrap_or("/ambient/.cargo");
                let polluted = Command::new(nix.join("nix-instantiate"))
                    .args([REALIZE, "--argstr", "digestFeature", feature])
                    .env(
                        "PATH",
                        format!(
                            "{}:{}",
                            nix.display(),
                            std::env::var("PATH").unwrap_or_default()
                        ),
                    )
                    .env("CARGO_HOME", ambient_home)
                    .env("RUSTUP_HOME", "/ambient/.rustup")
                    .env("CARGO", "/ambient/bin/cargo")
                    .env("RUSTC", "/ambient/bin/rustc")
                    .env("RUSTFLAGS", "--cfg ambient_leak")
                    .output()
                    .expect("polluted instantiate");
                assert!(
                    polluted.status.success(),
                    "case `{name}`: polluted instantiate"
                );
                assert_eq!(
                    String::from_utf8_lossy(&clean.stdout).trim(),
                    String::from_utf8_lossy(&polluted.stdout).trim(),
                    "case `{name}`: ambient Rust tool state changed the derivation"
                );
            }
            other => panic!("unhandled package fixture rejection case `{other}`"),
        }
    }
    assert!(
        accepted >= 3,
        "every declared digest feature must be realized"
    );
    assert!(rejected >= 2, "both rejection cases must be exercised");
    let _ = fs::remove_dir_all(&empty_roots);
}

/// Audit round 5, medium finding 6: the fetch deny list is derived from what
/// this repository's pinned Nix evaluation context actually reaches, not
/// guessed.
///
/// The reachable set is enumerated HERE, from the live pinned `nixpkgs` that
/// `nix/tools/declared-source.nix` resolves, plus every `builtins` fetcher.
/// Each name is then written into a mutation of the REAL committed
/// `nix/consumer-package-v1.nix`, and the REAL gate binary must reject that
/// mutation with its exact `live-source-fetch <name>` diagnostic. An
/// unmutated copy of the same file in the same directory must exit 0, so a
/// rejection is attributable to the mutation and not to the copy's location.
#[test]
fn the_real_gate_rejects_every_fetch_primitive_reachable_from_the_pinned_nixpkgs() {
    let _guard = nix_execution_guard();
    let Some(nix) = nix_bin_directory() else {
        return;
    };
    let expression = format!(
        "let d = import ./{DECLARED_SOURCE} {{ }}; in \
         builtins.filter (n: (builtins.match \"(fetch|requireFile).*\" n) != null) \
         (builtins.attrNames d.pkgs) \
         ++ builtins.filter (n: (builtins.match \"fetch.*\" n) != null) (builtins.attrNames builtins)"
    );
    let output = nix_command(
        &nix,
        "nix-instantiate",
        &["--eval", "--json", "-E", &expression],
    );
    assert!(
        output.status.success(),
        "enumerating reachable fetch primitives failed; stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let mut primitives: Vec<String> =
        serde_json::from_slice(&output.stdout).expect("reachable fetch primitive list");
    primitives.sort_unstable();
    primitives.dedup();
    // A guessed list would be short; the real pinned nixpkgs reaches dozens.
    assert!(
        primitives.len() >= 50,
        "the derived reachable set looks truncated: {primitives:?}"
    );
    for expected in [
        "fetchurl",
        "fetchTarball",
        "fetchGit",
        "fetchTree",
        "fetchFromGitHub",
        "fetchFromGitLab",
        "fetchFromBitbucket",
        "fetchFromSourcehut",
        "fetchzip",
        "fetchsvn",
        "fetchhg",
        "fetchCrate",
        "fetchgit",
        "requireFile",
    ] {
        assert!(
            primitives.iter().any(|value| value == expected),
            "the derived reachable set omits `{expected}`"
        );
    }

    let text = fs::read_to_string(PACKAGE_FILE).expect("real committed package file");
    let directory = temporary_dir("fetch-denial");
    let lock_file = Path::new(SOURCE_CONTENT_LOCK)
        .canonicalize()
        .expect("committed source-content lock");
    let control_file = directory.join("unmutated-control.nix");
    fs::write(&control_file, &text).expect("unmutated control copy");

    for primitive in &primitives {
        // `fetch-scm` and friends are attribute names, not identifiers; a
        // caller reaches them as a quoted attribute path.
        let call = if primitive
            .chars()
            .all(|value| value.is_alphanumeric() || value == '_')
        {
            format!("  src = {primitive} {{ }};")
        } else {
            format!("  src = pkgs.\"{primitive}\" {{ }};")
        };
        let mutated = text.replacen("  inherit src;", &call, 1);
        assert_ne!(mutated, text, "mutation `{primitive}` changed nothing");
        let mutated_file = directory.join("mutated.nix");
        fs::write(&mutated_file, &mutated).expect("mutated package file");
        let output = run_real_gate(&mutated_file, &lock_file);
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert_eq!(
            output.status.code(),
            Some(1),
            "`{primitive}` must make the real gate exit 1; stderr={stderr}"
        );
        let leading = primitive
            .split(|value: char| !(value.is_alphanumeric() || value == '_'))
            .next()
            .unwrap_or(primitive);
        assert!(
            stderr.contains(&format!("live-source-fetch {leading}")),
            "`{primitive}` must produce `live-source-fetch {leading}`; stderr={stderr}"
        );
        // Same gate, same directory, unmutated bytes: exit 0.
        assert!(
            run_real_gate(&control_file, &lock_file).status.success(),
            "the unmutated control must pass the same gate from the same directory"
        );
    }
    let _ = fs::remove_dir_all(&directory);
}

fn locate_nix() -> Option<PathBuf> {
    std::env::var("PATH").ok().and_then(|path| {
        std::env::split_paths(&path)
            .chain([
                PathBuf::from("/nix/var/nix/profiles/default/bin"),
                PathBuf::from("/run/current-system/sw/bin"),
            ])
            .find(|directory| directory.join("nix-instantiate").exists())
    })
}

/// Task 10.16: closure size and build cost are recorded separately, in a
/// retained artifact, and are explicitly noncanonical.
#[test]
fn closure_size_and_build_cost_are_retained_separately_and_marked_noncanonical() {
    let text = fs::read_to_string(MEASUREMENTS).expect("retained measurement artifact");
    let mut observed = std::collections::BTreeSet::new();
    for row in text
        .lines()
        .filter(|line| !line.starts_with('#') && !line.starts_with("measurement\t"))
    {
        let fields: Vec<&str> = row.split('\t').collect();
        assert_eq!(fields.len(), 5, "row={row}");
        assert!(
            observed.insert((fields[0], fields[1])),
            "duplicate measurement and variant: {row}"
        );
        assert!(
            fields[2].parse::<u64>().is_ok(),
            "measurement must be numeric: {row}"
        );
        let unit = match fields[0] {
            "output-nar-size" | "closure-size" => "bytes",
            "build-cost" => "seconds",
            other => panic!("unknown measurement: {other}"),
        };
        assert_eq!(fields[3], unit, "row={row}");
        assert_eq!(fields[4], "noncanonical-measurement", "row={row}");
    }
    let expected: std::collections::BTreeSet<_> = ["xxh3-128", "blake3-256", "sha-256"]
        .into_iter()
        .flat_map(|variant| {
            ["output-nar-size", "closure-size", "build-cost"]
                .into_iter()
                .map(move |measurement| (measurement, variant))
        })
        .collect();
    assert_eq!(observed, expected, "missing or undeclared measurement rows");
}
