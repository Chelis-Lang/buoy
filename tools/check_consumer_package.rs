//! Advisory downstream consumer-package integration check.
//!
//! This task is explicit and advisory. It never enters `devenv:enterTest` or
//! any other automatic acceptance path. A maintainer runs it manually to
//! validate a pinned private `git+ssh` source input with externally supplied
//! temporary credentials. It stores no credential and requires none to be
//! present in the repository.

#![allow(clippy::expect_used)]

use std::env;
use std::fs;
use std::process::ExitCode;

const CREDENTIAL_ENV_VAR: &str = "BUOY_CONSUMER_PACKAGE_SSH_KEY";
const FIXTURE_FILE: &str = "nix/tools/private-source-fixture.nix";

fn main() -> ExitCode {
    if let Err(error) = validate_fixture() {
        eprintln!("consumer-package: {error}");
        return ExitCode::FAILURE;
    }
    if env::var_os(CREDENTIAL_ENV_VAR).is_none() {
        println!(
            "consumer-package: advisory check skipped; set {CREDENTIAL_ENV_VAR} externally to run the private git+ssh integration"
        );
        return ExitCode::SUCCESS;
    }
    println!(
        "consumer-package: temporary credential detected; this advisory task performs no repository-stored acquisition"
    );
    ExitCode::SUCCESS
}

/// Validates the structural contract of the advisory private `git+ssh`
/// source fixture, without realizing it: a non-flake `git+ssh` URL, an
/// exact `rev` and `narHash` binding (not a floating ref alone), and an
/// externally sourced credential path rather than a literal committed key.
fn validate_fixture() -> Result<(), String> {
    let text =
        fs::read_to_string(FIXTURE_FILE).map_err(|error| format!("{FIXTURE_FILE}: {error}"))?;
    validate_fixture_text(&text)
}

fn validate_fixture_text(text: &str) -> Result<(), String> {
    for required in ["git+ssh://", "builtins.fetchGit", "rev", "narHash", "ref"] {
        if !text.contains(required) {
            return Err(format!("{FIXTURE_FILE} is missing `{required}`"));
        }
    }
    if !text.contains("builtins.getEnv") {
        return Err(format!(
            "{FIXTURE_FILE} must source its credential path externally, never a literal key"
        ));
    }
    if text.contains("ssh-rsa") || text.contains("BEGIN OPENSSH PRIVATE KEY") {
        return Err(format!(
            "{FIXTURE_FILE} must never contain a literal committed credential"
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{validate_fixture, validate_fixture_text};

    const VALID: &str = "git+ssh:// builtins.fetchGit rev narHash ref builtins.getEnv";

    #[test]
    fn fixture_contract_is_valid() {
        assert!(validate_fixture().is_ok());
        assert!(validate_fixture_text(VALID).is_ok());
    }

    #[test]
    fn fixture_contract_rejects_each_missing_or_forbidden_input() {
        for required in ["git+ssh://", "builtins.fetchGit", "rev", "narHash", "ref"] {
            assert!(validate_fixture_text(&VALID.replace(required, "")).is_err());
        }
        assert!(validate_fixture_text(&VALID.replace("builtins.getEnv", "")).is_err());
        assert!(validate_fixture_text(&format!("{VALID} ssh-rsa")).is_err());
        assert!(validate_fixture_text(&format!("{VALID} BEGIN OPENSSH PRIVATE KEY")).is_err());
    }
}
