//! Blocking structural contract check for the Buoy-owned consumer package.
//!
//! This check runs offline and without Nix. It parses the real formal
//! argument set of `nix/consumer-package-v1.nix` -- not a substring scan of
//! the whole file -- and requires that closed contract to declare every
//! reviewed input (including `src`) with no default and no ellipsis, that
//! every declared input is actually consumed, that the source content is
//! verified against a committed reviewed digest, that every explicit digest
//! variant is declared, and that the `buoy/package-identity/v1` machine
//! value stays well formed.
//!
//! Nix realization is a separate step of the same blocking task owner
//! (`nix/tools/package-contract.sh`), which additionally checks the real
//! derivation contract and realizes every variant offline.
//!
//! Usage: `check-package-contract [--package-file PATH] [--lock-file PATH]`.
//! The optional paths exist so negative controls can run this exact gate
//! against a mutated copy of the committed file.

#![allow(clippy::expect_used)]

#[cfg(not(test))]
use std::env;
use std::fs;
use std::path::Path;
#[cfg(not(test))]
use std::path::PathBuf;
#[cfg(not(test))]
use std::process::ExitCode;

const PACKAGE_FILE: &str = "nix/consumer-package-v1.nix";
const SOURCE_CONTENT_LOCK: &str = "nix/consumer-package-v1.source-content.json";

/// Every input the reviewed closed contract must require. `src` is required
/// exactly like the pinned revision and toolchain: a package function that
/// can build without an explicitly declared source is the ambient-source
/// leak PBD-001 forbids.
const REQUIRED_INPUTS: [&str; 6] = [
    "src",
    "sourceRevision",
    "sourceContentHash",
    "rustToolchain",
    "system",
    "digestFeature",
];

/// Inputs a caller must never be able to supply: accepting a `rustPlatform`
/// (or a bare `cargo`/`rustc`) lets a forgetful caller silently resolve
/// nixpkgs' ambient default compiler instead of the pinned toolchain.
const FORBIDDEN_INPUTS: [&str; 3] = ["rustPlatform", "cargo", "rustc"];

const REQUIRED_VARIANTS: [&str; 3] = ["xxh3-128", "blake3-256", "sha-256"];

/// One declared formal argument of the package function.
struct FormalInput {
    name: String,
    has_default: bool,
}

#[cfg(not(test))]
fn main() -> ExitCode {
    let mut package_file = PathBuf::from(PACKAGE_FILE);
    let mut lock_file = PathBuf::from(SOURCE_CONTENT_LOCK);
    let mut arguments = env::args().skip(1);
    while let Some(argument) = arguments.next() {
        match argument.as_str() {
            "--package-file" => {
                let Some(value) = arguments.next() else {
                    eprintln!("package-contract: missing-argument --package-file");
                    return ExitCode::FAILURE;
                };
                package_file = PathBuf::from(value);
            }
            "--lock-file" => {
                let Some(value) = arguments.next() else {
                    eprintln!("package-contract: missing-argument --lock-file");
                    return ExitCode::FAILURE;
                };
                lock_file = PathBuf::from(value);
            }
            other => {
                eprintln!("package-contract: unknown-argument {other}");
                return ExitCode::FAILURE;
            }
        }
    }
    match run(&package_file, &lock_file) {
        Ok(()) => {
            println!(
                "package-contract: ok inputs={REQUIRED_INPUTS:?} variants={REQUIRED_VARIANTS:?}"
            );
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("package-contract: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run(package_file: &Path, lock_file: &Path) -> Result<(), String> {
    let text = match fs::read_to_string(package_file) {
        Ok(text) => text,
        Err(error) => {
            return Err(format!(
                "unreadable-package-file {}: {error}",
                package_file.display()
            ));
        }
    };
    validate_contract(&text)?;
    let lock = match fs::read_to_string(lock_file) {
        Ok(lock) => lock,
        Err(error) => {
            return Err(format!(
                "unreadable-source-content-lock {}: {error}",
                lock_file.display()
            ));
        }
    };
    validate_source_content_lock(&lock)?;
    validate_package_identity()
}

/// Splits the package function's formal argument set from its body, then
/// checks both. The argument set is everything up to the `}:` line that
/// closes the function header.
fn split_header(text: &str) -> Result<(&str, &str), String> {
    let trimmed = text.trim_start();
    if !trimmed.starts_with('{') {
        return Err(String::from(
            "not-a-package-function: the contract must begin with a formal argument set",
        ));
    }
    let start = text.len() - trimmed.len();
    let end = text[start..].find("\n}:").ok_or_else(|| {
        String::from("unterminated-input-contract: no `}:` closes the argument set")
    })?;
    Ok((&text[start + 1..start + end], &text[start + end + 3..]))
}

fn parse_formal_inputs(header: &str) -> Vec<FormalInput> {
    header
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .filter_map(|line| {
            let entry = line.trim_end_matches(',').trim();
            let (name, has_default) = match entry.split_once('?') {
                Some((name, _)) => (name.trim(), true),
                None => (entry, false),
            };
            if name.is_empty()
                || !name
                    .chars()
                    .all(|value| value.is_alphanumeric() || value == '_')
            {
                return None;
            }
            Some(FormalInput {
                name: name.to_owned(),
                has_default,
            })
        })
        .collect()
}

fn validate_contract(text: &str) -> Result<(), String> {
    let (header, body) = split_header(text)?;
    if header.contains("...") {
        return Err(String::from(
            "open-input-contract: the package function must not accept an ellipsis",
        ));
    }
    let inputs = parse_formal_inputs(header);
    for required in REQUIRED_INPUTS {
        let Some(input) = inputs.iter().find(|input| input.name == required) else {
            return Err(format!("missing-required-input {required}"));
        };
        if input.has_default {
            return Err(format!(
                "defaulted-required-input {required}: a reviewed input must never fall back to an ambient value"
            ));
        }
        if !body.contains(required) {
            return Err(format!(
                "unused-required-input {required}: a declared input that is never consumed proves nothing"
            ));
        }
    }
    for forbidden in FORBIDDEN_INPUTS {
        if inputs.iter().any(|input| input.name == forbidden) {
            return Err(format!(
                "ambient-toolchain-input {forbidden}: the package must construct its Rust platform from the pinned toolchain"
            ));
        }
    }
    validate_source_verification(body)?;
    validate_variants(body)?;
    validate_no_ambient_resolution(body)
}

/// The declared `sourceContentHash` must genuinely gate the build: it must
/// be the fixed output hash of a recursive source copy, and that verified
/// copy must be a real build input of the produced executable.
fn validate_source_verification(body: &str) -> Result<(), String> {
    if !body.contains("outputHash = sourceContentHash;") {
        return Err(String::from(
            "unverified-source-content: `sourceContentHash` must be the fixed output hash of the source copy",
        ));
    }
    if !body.contains("outputHashMode = \"recursive\"") {
        return Err(String::from(
            "unverified-source-content: source verification must hash the recursive source tree",
        ));
    }
    if !body.contains("BUOY_VERIFIED_SOURCE = verifiedSource;") {
        return Err(String::from(
            "unenforced-source-verification: the verified source must be a build input of the executable",
        ));
    }
    Ok(())
}

fn validate_variants(body: &str) -> Result<(), String> {
    for variant in REQUIRED_VARIANTS {
        if !body.contains(variant) {
            return Err(format!("missing-declared-variant {variant}"));
        }
    }
    if !body.contains("--no-default-features") {
        return Err(String::from(
            "default-features-enabled: every explicit variant must disable default Cargo features",
        ));
    }
    Ok(())
}

/// Removes Nix comments so a sentence that merely *mentions* a fetch
/// primitive cannot trip the deny rule, and so a mutation cannot hide a real
/// fetch behind a comment marker. A `#` only opens a comment at the start of
/// a line or after whitespace, exactly as Nix itself treats it.
fn strip_nix_comments(text: &str) -> String {
    let characters: Vec<char> = text.chars().collect();
    let mut stripped = String::with_capacity(text.len());
    let mut index = 0;
    while index < characters.len() {
        let previous = if index == 0 {
            None
        } else {
            Some(characters[index - 1])
        };
        if characters[index] == '#'
            && previous.is_none_or(|value| value.is_whitespace() || value == '\n')
        {
            while index < characters.len() && characters[index] != '\n' {
                index += 1;
            }
        } else if characters[index] == '/' && characters.get(index + 1) == Some(&'*') {
            index += 2;
            while index < characters.len()
                && !(characters[index] == '*' && characters.get(index + 1) == Some(&'/'))
            {
                index += 1;
            }
            index = usize::min(index + 2, characters.len());
        } else {
            stripped.push(characters[index]);
            index += 1;
        }
    }
    stripped
}

/// Splits Nix code into identifier tokens. `.` is a separator, so
/// `builtins.fetchGit` yields `builtins` and `fetchGit`, and an attribute
/// path can never smuggle a denied primitive past the rule.
fn nix_identifier_tokens(text: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    for character in text.chars() {
        if character.is_alphanumeric() || character == '_' || character == '-' || character == '\''
        {
            current.push(character);
        } else if !current.is_empty() {
            tokens.push(std::mem::take(&mut current));
        }
    }
    if !current.is_empty() {
        tokens.push(current);
    }
    tokens
}

/// Vendoring inputs that make `buildRustPackage` resolve crates from a
/// remote registry instead of the committed `Cargo.lock` the reviewed
/// contract already pins.
const DENIED_VENDOR_INPUTS: [&str; 4] = ["cargoHash", "cargoSha256", "cargoDeps", "cargoVendorDir"];

/// Runtime and build-script fetch commands. Every one of these is reachable
/// from an ordinary `stdenv` build phase or from a launcher script, so each
/// is denied by literal text rather than by identifier shape.
const DENIED_FETCH_COMMANDS: [&str; 26] = [
    "git clone",
    "git fetch",
    "git pull",
    "git submodule",
    "git archive",
    "curl ",
    "wget ",
    "aria2c",
    "scp ",
    "sftp ",
    "rsync ",
    "nix-prefetch",
    "nix flake",
    "nix-channel",
    "nix registry",
    "nix copy",
    "nix-store --realise",
    "cargo fetch",
    "cargo update",
    "cargo install",
    "cargo vendor",
    "cargo publish",
    "npm install",
    "pnpm install",
    "yarn install",
    "pip install",
];

/// Denies EVERY fetch primitive reachable from this repository's pinned Nix
/// evaluation context, not a hand-picked handful.
///
/// The reachable set was derived, not guessed: the pinned `nixpkgs` this
/// repository resolves through `nix/tools/declared-source.nix` exposes 59
/// top-level `fetch*` attributes (`fetch`, `fetch-scm`, `fetchCrate`,
/// `fetchFromBitbucket`, `fetchFromCodeberg`, `fetchFromForgejo`,
/// `fetchFromGitHub`, `fetchFromGitLab`, `fetchFromGitea`,
/// `fetchFromSourcehut`, `fetchgit`, `fetchhg`, `fetchsvn`, `fetchzip`,
/// `fetchurl`, `fetchpatch`, `fetchPypi`, `fetchNpmDeps`, `fetchYarnDeps`,
/// ... ) plus `requireFile`, and `builtins` exposes `fetchGit`,
/// `fetchMercurial`, `fetchTarball`, `fetchTree`, and `fetchurl`. Enumerating
/// those 65 names literally would go stale the moment nixpkgs adds a
/// fetcher, so the rule is the shape they all share: any identifier whose
/// name begins with `fetch` (in any case) is denied. `requireFile`,
/// impure `<angle-bracket>` path lookups, and remote crate-vendoring inputs
/// are denied by name because they do not share that shape.
///
/// `tests/pinned_buoy_distribution.rs` re-derives the reachable list from the
/// live pinned nixpkgs and requires the real gate binary to reject every
/// derived name, so this rule cannot silently fall behind the primitive set
/// it claims to cover.
fn validate_no_ambient_resolution(body: &str) -> Result<(), String> {
    let code = strip_nix_comments(body);
    // Literal fetch COMMANDS are classified first. `git fetch` is a runtime
    // fetch, and classifying it by its bare `fetch` word would report the
    // wrong class for the right defect.
    for pattern in DENIED_FETCH_COMMANDS {
        if code.contains(pattern) {
            return Err(format!(
                "runtime-source-fetch {pattern:?}: the packaged launcher must never fetch at run time"
            ));
        }
    }
    for token in nix_identifier_tokens(&code) {
        if token.to_ascii_lowercase().starts_with("fetch") {
            return Err(format!(
                "live-source-fetch {token}: the package must not resolve a live source at build time"
            ));
        }
        if token == "requireFile" {
            return Err(String::from(
                "live-source-fetch requireFile: the package must not require an out-of-band downloaded source",
            ));
        }
        if DENIED_VENDOR_INPUTS.contains(&token.as_str()) {
            return Err(format!(
                "live-source-fetch {token}: crates must come from the committed Cargo.lock, never a remote vendor fetch"
            ));
        }
    }
    if let Some(lookup) = impure_path_lookup(&code) {
        return Err(format!(
            "live-source-fetch {lookup}: an angle-bracket lookup resolves an ambient channel at evaluation time"
        ));
    }
    Ok(())
}

/// Finds an impure `<channel>` path lookup such as `<nixpkgs>`.
fn impure_path_lookup(code: &str) -> Option<String> {
    let mut rest = code;
    while let Some(open) = rest.find('<') {
        let after = &rest[open + 1..];
        if let Some(close) = after.find('>') {
            let inner = &after[..close];
            if !inner.is_empty()
                && inner
                    .chars()
                    .all(|value| value.is_alphanumeric() || "._-/".contains(value))
            {
                return Some(format!("<{inner}>"));
            }
        }
        rest = &rest[open + 1..];
    }
    None
}

/// The reviewed source-content digest must be a committed constant with a
/// declared algorithm and encoding. Verification compares an independently
/// derived digest against this value; it never derives the expected value
/// from the tree it is checking.
fn validate_source_content_lock(lock: &str) -> Result<(), String> {
    let value: serde_json::Value = serde_json::from_str(lock)
        .map_err(|error| format!("malformed-source-content-lock: {error}"))?;
    if value["schema"] != "buoy/consumer-package-source-content/v1" {
        return Err(String::from(
            "malformed-source-content-lock: wrong or missing schema",
        ));
    }
    if value["algorithm"] != "sha256" || value["mode"] != "recursive-nar" {
        return Err(String::from(
            "malformed-source-content-lock: the reviewed digest must declare sha256 recursive-nar",
        ));
    }
    let digest = value["reviewed_source_content_hash"]
        .as_str()
        .ok_or_else(|| String::from("missing-reviewed-source-content-digest"))?;
    if !digest.starts_with("sha256-") || digest.len() < 20 {
        return Err(format!(
            "malformed-source-content-lock: `{digest}` is not an SRI sha256 digest"
        ));
    }
    Ok(())
}

fn validate_package_identity() -> Result<(), String> {
    validate_package_identity_with(&buoy::publication::PackageIdentity::new(
        env!("CARGO_PKG_VERSION"),
        "contract-check",
        "rust-toolchain",
        REQUIRED_VARIANTS.to_vec(),
    ))
}

fn validate_package_identity_with(
    identity: &buoy::publication::PackageIdentity,
) -> Result<(), String> {
    let rendered = match identity.render() {
        Ok(rendered) => rendered,
        Err(error) => return Err(format!("package identity did not render: {error:?}")),
    };
    validate_package_identity_bytes(&rendered)
}

fn validate_package_identity_bytes(rendered: &[u8]) -> Result<(), String> {
    let value: serde_json::Value = match serde_json::from_slice(rendered) {
        Ok(value) => value,
        Err(error) => return Err(format!("malformed identity JSON: {error}")),
    };
    validate_package_identity_value(&value)
}

fn validate_package_identity_value(value: &serde_json::Value) -> Result<(), String> {
    if value["schema"] != "buoy/package-identity/v1" {
        return Err(String::from("package identity carries the wrong schema"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{
        impure_path_lookup, nix_identifier_tokens, parse_formal_inputs, split_header,
        validate_contract, validate_package_identity, validate_package_identity_bytes,
        validate_package_identity_value, validate_package_identity_with,
        validate_source_content_lock,
    };

    fn committed_package() -> String {
        std::fs::read_to_string("nix/consumer-package-v1.nix").expect("consumer package file")
    }

    #[test]
    fn valid_contract_passes() {
        assert!(validate_contract(&committed_package()).is_ok());
        assert!(
            super::run(
                std::path::Path::new(super::PACKAGE_FILE),
                std::path::Path::new(super::SOURCE_CONTENT_LOCK),
            )
            .is_ok()
        );
        assert!(validate_package_identity().is_ok());
        let lock = std::fs::read_to_string("nix/consumer-package-v1.source-content.json")
            .expect("source content lock");
        assert!(validate_source_content_lock(&lock).is_ok());
    }

    #[test]
    fn the_committed_contract_declares_every_reviewed_input_without_a_default() {
        let text = committed_package();
        let (header, _) = split_header(&text).expect("formal argument set");
        let inputs = parse_formal_inputs(header);
        for required in super::REQUIRED_INPUTS {
            let input = inputs
                .iter()
                .find(|input| input.name == required)
                .expect("required input");
            assert!(!input.has_default, "{required} must have no default");
        }
    }

    /// Each mutation removes exactly one reviewed property of the real
    /// committed contract, so a rule that never fires cannot pass silently.
    #[test]
    fn each_single_property_mutation_of_the_real_contract_fails_closed() {
        let text = committed_package();
        let mutations: Vec<(&str, String)> = vec![
            (
                "missing-required-input src",
                text.replacen("  src,\n", "", 1),
            ),
            (
                "defaulted-required-input src",
                text.replacen("  src,\n", "  src ? ../..,\n", 1),
            ),
            (
                "ambient-toolchain-input rustPlatform",
                text.replacen("  src,\n", "  src,\n  rustPlatform,\n", 1),
            ),
            (
                "open-input-contract",
                text.replacen("  src,\n", "  src,\n  ...\n", 1),
            ),
            (
                "unverified-source-content",
                text.replacen("outputHash = sourceContentHash;", "outputHash = null;", 1),
            ),
            (
                "unenforced-source-verification",
                text.replacen("BUOY_VERIFIED_SOURCE = verifiedSource;", "", 1),
            ),
            (
                "default-features-enabled",
                text.replacen("\"--no-default-features\"", "\"--offline\"", 1),
            ),
            (
                "missing-declared-variant",
                text.replace("blake3-256", "blake2-256"),
            ),
            (
                "live-source-fetch",
                text.replacen("  inherit src;", "  src = fetchFromGitHub { };", 1),
            ),
        ];
        for (expected, mutated) in mutations {
            assert_ne!(mutated, text, "mutation `{expected}` changed nothing");
            let error = validate_contract(&mutated)
                .expect_err(&format!("mutation `{expected}` must fail closed"));
            assert!(
                error.starts_with(expected.split(' ').next().unwrap_or(expected)),
                "expected `{expected}`, got `{error}`"
            );
        }
    }

    /// Audit round 5, medium finding 6: the deny rule covers the *shape*
    /// every reachable nixpkgs fetcher shares, so a primitive that was never
    /// named individually is still rejected.
    #[test]
    fn every_reachable_fetch_primitive_shape_is_denied() {
        let text = committed_package();
        for primitive in [
            "fetchurl",
            "fetchTarball",
            "fetchGit",
            "fetchTree",
            "fetchMercurial",
            "fetchFromGitHub",
            "fetchFromGitLab",
            "fetchFromBitbucket",
            "fetchFromSourcehut",
            "fetchFromGitea",
            "fetchFromCodeberg",
            "fetchFromForgejo",
            "fetchFromSavannah",
            "fetchFromGitiles",
            "fetchFromRepoOrCz",
            "fetchzip",
            "fetchgit",
            "fetchsvn",
            "fetchhg",
            "fetchbzr",
            "fetchcvs",
            "fetchdarcs",
            "fetchfossil",
            "fetchpijul",
            "fetchmtn",
            "fetchipfs",
            "fetchs3",
            "fetchCrate",
            "fetchNpmDeps",
            "fetchYarnDeps",
            "fetchPnpmDeps",
            "fetchNuGet",
            "fetchHex",
            "fetchPypi",
            "fetchMavenArtifact",
            "fetchpatch",
            "fetchpatch2",
            "fetchDebianPatch",
            "fetchRepoProject",
            "fetchgitLocal",
            "fetchDockerLayer",
            "fetchFirefoxAddon",
            "requireFile",
            "cargoHash",
            "cargoSha256",
            "cargoDeps",
            "cargoVendorDir",
        ] {
            let mutated =
                text.replacen("  inherit src;", &format!("  src = {primitive} {{ }};"), 1);
            assert_ne!(mutated, text, "mutation `{primitive}` changed nothing");
            let error =
                validate_contract(&mutated).expect_err(&format!("`{primitive}` must be denied"));
            assert!(
                error.starts_with(&format!("live-source-fetch {primitive}")),
                "expected `live-source-fetch {primitive}`, got `{error}`"
            );
        }
    }

    #[test]
    fn every_runtime_fetch_command_is_denied() {
        let text = committed_package();
        for command in super::DENIED_FETCH_COMMANDS {
            let mutated = text.replacen(
                "  preBuild = ''",
                &format!(
                    "  postInstall = ''\n    {command} example-target\n  '';\n  preBuild = ''"
                ),
                1,
            );
            assert_ne!(mutated, text, "mutation `{command}` changed nothing");
            let error =
                validate_contract(&mutated).expect_err(&format!("`{command}` must be denied"));
            assert!(
                error.starts_with("runtime-source-fetch"),
                "expected `runtime-source-fetch` for `{command}`, got `{error}`"
            );
        }
    }

    /// An angle-bracket lookup resolves an ambient channel at evaluation
    /// time, which is an ambient-source leak even though it names no fetcher.
    #[test]
    fn an_impure_channel_lookup_is_denied() {
        let text = committed_package();
        let mutated = text.replacen("  inherit src;", "  src = import <nixpkgs> { };", 1);
        let error = validate_contract(&mutated).expect_err("channel lookup must be denied");
        assert!(
            error.starts_with("live-source-fetch <nixpkgs>"),
            "got `{error}`"
        );
    }

    /// The rule reads code, not prose: the committed file's own comments talk
    /// about fetching, and a comment must neither trip the rule nor hide a
    /// real fetch.
    #[test]
    fn comments_neither_trip_nor_mask_the_deny_rule() {
        let commented = "{ a }:\n# this mentions fetchurl and git clone in prose\n/* and fetchFromGitHub too */\n{ inherit a; }\n";
        assert_eq!(
            super::nix_identifier_tokens(&super::strip_nix_comments(commented))
                .iter()
                .filter(|token| token.starts_with("fetch"))
                .count(),
            0
        );
        // A `#` that does not open a comment cannot hide a real fetcher.
        let hidden = "{ a }:\n{ src = x#fetchurl { }; }\n";
        assert!(
            super::nix_identifier_tokens(&super::strip_nix_comments(hidden))
                .iter()
                .any(|token| token == "fetchurl")
        );
        assert_eq!(super::strip_nix_comments("# terminal comment"), "");
        assert_eq!(super::strip_nix_comments("/* unterminated"), "");
        assert_eq!(super::strip_nix_comments("/* *x */value"), "value");
    }

    #[test]
    fn a_self_derived_or_malformed_source_digest_fails_closed() {
        for malformed in [
            "{",
            r#"{"schema":"other/v1","algorithm":"sha256","mode":"recursive-nar","reviewed_source_content_hash":"sha256-AAAAAAAAAAAAAAAAAAAA"}"#,
            r#"{"schema":"buoy/consumer-package-source-content/v1","algorithm":"md5","mode":"recursive-nar","reviewed_source_content_hash":"sha256-AAAAAAAAAAAAAAAAAAAA"}"#,
            r#"{"schema":"buoy/consumer-package-source-content/v1","algorithm":"sha256","mode":"flat","reviewed_source_content_hash":"sha256-AAAAAAAAAAAAAAAAAAAA"}"#,
            r#"{"schema":"buoy/consumer-package-source-content/v1","algorithm":"sha256","mode":"recursive-nar"}"#,
            r#"{"schema":"buoy/consumer-package-source-content/v1","algorithm":"sha256","mode":"recursive-nar","reviewed_source_content_hash":"derive-me"}"#,
            r#"{"schema":"buoy/consumer-package-source-content/v1","algorithm":"sha256","mode":"recursive-nar","reviewed_source_content_hash":"sha256-short"}"#,
        ] {
            assert!(validate_source_content_lock(malformed).is_err());
        }
    }

    #[test]
    fn parser_and_contract_boundary_controls_fail_closed() {
        assert!(split_header("not a function").is_err());
        assert!(split_header("{ input,\nbody").is_err());
        assert!(validate_contract("not a function").is_err());

        let parsed =
            parse_formal_inputs("\n# ignored\n,\ninvalid-name,\nvalid,\nwith_default ? value,\n");
        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed[0].name, "valid");
        assert!(!parsed[0].has_default);
        assert!(parsed[1].has_default);

        let text = committed_package();
        let (header, body) = split_header(&text).expect("committed contract");
        let body_without_revision = body.replace("sourceRevision", "source_revision_removed");
        let unused = format!("{{{header}\n}}:{body_without_revision}");
        assert!(
            validate_contract(&unused)
                .expect_err("unused input")
                .starts_with("unused-required-input sourceRevision")
        );
        assert!(
            validate_contract(&text.replacen(
                "outputHashMode = \"recursive\"",
                "outputHashMode = \"flat\"",
                1,
            ))
            .expect_err("nonrecursive source verification")
            .starts_with("unverified-source-content")
        );

        assert_eq!(nix_identifier_tokens("terminal"), vec!["terminal"]);
        assert_eq!(
            impure_path_lookup("<> then <nixpkgs>"),
            Some(String::from("<nixpkgs>"))
        );
        assert_eq!(impure_path_lookup("<unterminated"), None);
        assert_eq!(impure_path_lookup("<bad!>"), None);
        assert!(
            validate_package_identity_value(&serde_json::json!({"schema": "wrong/v1"})).is_err()
        );
        assert!(validate_package_identity_bytes(b"not JSON").is_err());
        assert!(
            validate_package_identity_with(&buoy::publication::PackageIdentity::new(
                "",
                "revision",
                "toolchain",
                vec!["xxh3-128"],
            ))
            .is_err()
        );
    }
}
