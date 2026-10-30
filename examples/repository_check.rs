#![doc = "Run one Buoy static repository check through the Rust API."]

use buoy::check::{
    CheckError, MachineReport, RequirementSubjectError, check_snapshot, render_machine,
    requirement_subject_facts,
};
use buoy::repository::{RepositoryError, load_config, snapshot_repository};
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::process::ExitCode;

const USAGE: &str = "usage: cargo run --example repository_check -- --root ROOT --config FILE";

struct Arguments {
    root: PathBuf,
    config: PathBuf,
}

fn parse_arguments(arguments: &[String]) -> Result<Option<Arguments>, &'static str> {
    if arguments == ["--help"] {
        return Ok(None);
    }
    let [root_flag, root, config_flag, config] = arguments else {
        return Err("EXAMPLE-USAGE");
    };
    if root_flag != "--root" || config_flag != "--config" || root.is_empty() || config.is_empty() {
        return Err("EXAMPLE-USAGE");
    }
    Ok(Some(Arguments {
        root: PathBuf::from(root),
        config: PathBuf::from(config),
    }))
}

fn run(arguments: &Arguments) -> Result<(String, bool), &'static str> {
    let configuration = load_config(&arguments.config).map_err(RepositoryError::code)?;
    let snapshot =
        snapshot_repository(&arguments.root, &configuration).map_err(RepositoryError::code)?;
    let result = check_snapshot(&snapshot, &configuration).map_err(CheckError::code)?;
    let requirement_subjects = if result.diagnostics.is_empty() {
        requirement_subject_facts(&result).map_err(RequirementSubjectError::code)?
    } else {
        Vec::new()
    };
    let report = MachineReport {
        command: String::from("static"),
        graph: result.graph_digest.canonical_text(),
        diagnostics: result.diagnostics,
        migration: result.migration,
        verdicts: BTreeMap::new(),
        impacts: Vec::new(),
        mounts: Vec::new(),
        requirement_subjects,
        opaque_subjects: Vec::new(),
        artifact_attachments: Vec::new(),
    };
    let valid = report.diagnostics.is_empty();
    render_machine(&report)
        .map(|output| (output, valid))
        .map_err(|_| "EXAMPLE-REPORT")
}

fn main() -> ExitCode {
    let raw = std::env::args().skip(1).collect::<Vec<_>>();
    let arguments = match parse_arguments(&raw) {
        Ok(Some(arguments)) => arguments,
        Ok(None) => {
            println!("{USAGE}");
            return ExitCode::SUCCESS;
        }
        Err(code) => {
            eprintln!("{code}: {USAGE}");
            return ExitCode::from(64);
        }
    };
    match run(&arguments) {
        Ok((output, valid)) => {
            println!("{output}");
            if valid {
                ExitCode::SUCCESS
            } else {
                ExitCode::from(2)
            }
        }
        Err(code) => {
            eprintln!("{code}");
            ExitCode::from(70)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn static_diagnostics_preserve_the_report_and_omit_requirement_subjects() {
        let (output, valid) = run(&Arguments {
            root: PathBuf::from("examples/repositories/stale-external-link"),
            config: PathBuf::from("examples/repositories/stale-external-link/buoy.toml"),
        })
        .expect("diagnostic static report");
        assert!(!valid);
        let report: serde_json::Value = serde_json::from_str(&output).expect("machine report");
        assert!(
            report["diagnostics"]
                .as_array()
                .is_some_and(|rows| !rows.is_empty())
        );
        assert!(report.get("requirement_subjects").is_none());
    }
}
