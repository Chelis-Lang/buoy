//! Chelis provenance command adapter with separate static, change, advisory,
//! trace, bind, rebind, and effectful execution modes that preserve the core
//! exit classes.

use chelis_provenance::advisory::advisory_report;
use chelis_provenance::bind::{BindError, bind};
use chelis_provenance::command::{
    ChangeManifest, EXIT_INPUT, EXIT_OK, EXIT_STATIC, EXIT_USAGE, change_check, static_check,
};
use chelis_provenance::execution::execute;
use chelis_provenance::rebind::{RebindError, rebind};
use chelis_provenance::snapshot::{ChelisConfiguration, ChelisSnapshot};
use chelis_provenance::trace::trace_projection;
use serde::Deserialize;
use std::path::Path;

fn option_value<'a>(arguments: &'a [String], name: &str) -> Option<&'a str> {
    arguments
        .windows(2)
        .find(|window| window[0] == name)
        .map(|window| window[1].as_str())
}

fn load_configuration(path: &str) -> Result<ChelisConfiguration, u8> {
    let bytes = std::fs::read(path).map_err(|_| EXIT_INPUT)?;
    ChelisConfiguration::parse(&bytes).map_err(|_| EXIT_INPUT)
}

fn load_snapshot(root: &str, configuration: &ChelisConfiguration) -> Result<ChelisSnapshot, u8> {
    ChelisSnapshot::from_directory(Path::new(root), configuration).map_err(|_| EXIT_INPUT)
}

fn static_command(arguments: &[String]) -> u8 {
    let (Some(root), Some(config)) = (
        option_value(arguments, "--root"),
        option_value(arguments, "--config"),
    ) else {
        return EXIT_USAGE;
    };
    let Ok(configuration) = load_configuration(config) else {
        return EXIT_INPUT;
    };
    let Ok(snapshot) = load_snapshot(root, &configuration) else {
        return EXIT_INPUT;
    };
    match static_check(&snapshot, &configuration) {
        Ok(outcome) => {
            println!("{}", outcome.report);
            outcome.exit
        }
        Err(_) => EXIT_INPUT,
    }
}

fn change_command(arguments: &[String]) -> u8 {
    let (Some(base), Some(head), Some(config), Some(manifest)) = (
        option_value(arguments, "--base"),
        option_value(arguments, "--head"),
        option_value(arguments, "--config"),
        option_value(arguments, "--manifest"),
    ) else {
        return EXIT_USAGE;
    };
    let Ok(configuration) = load_configuration(config) else {
        return EXIT_INPUT;
    };
    let (Ok(base_snapshot), Ok(head_snapshot)) = (
        load_snapshot(base, &configuration),
        load_snapshot(head, &configuration),
    ) else {
        return EXIT_INPUT;
    };
    let Ok(manifest_bytes) = std::fs::read(manifest) else {
        return EXIT_INPUT;
    };
    let Ok(manifest) = ChangeManifest::parse(&manifest_bytes) else {
        return EXIT_INPUT;
    };
    let (Ok(base_outcome), Ok(head_outcome)) = (
        static_check(&base_snapshot, &configuration),
        static_check(&head_snapshot, &configuration),
    ) else {
        return EXIT_INPUT;
    };
    match change_check(&base_outcome, &head_outcome, &manifest) {
        Ok(outcome) => {
            println!("{}", outcome.report);
            outcome.exit
        }
        Err(_) => EXIT_INPUT,
    }
}

fn trace_command(arguments: &[String]) -> u8 {
    let (Some(root), Some(config), Some(subject)) = (
        option_value(arguments, "--root"),
        option_value(arguments, "--config"),
        option_value(arguments, "--id"),
    ) else {
        return EXIT_USAGE;
    };
    let Ok(configuration) = load_configuration(config) else {
        return EXIT_INPUT;
    };
    let Ok(snapshot) = load_snapshot(root, &configuration) else {
        return EXIT_INPUT;
    };
    match trace_projection(&snapshot, &configuration, subject) {
        Ok(outcome) => {
            println!("{}", outcome.report);
            outcome.exit
        }
        Err(_) => EXIT_INPUT,
    }
}

fn rebind_command(arguments: &[String]) -> u8 {
    let (Some(root), Some(config)) = (
        option_value(arguments, "--root"),
        option_value(arguments, "--config"),
    ) else {
        return EXIT_USAGE;
    };
    let write = arguments.iter().any(|argument| argument == "--write");
    let Ok(configuration) = load_configuration(config) else {
        return EXIT_INPUT;
    };
    match rebind(Path::new(root), &configuration, write) {
        Ok(outcome) => {
            println!("{}", outcome.report);
            EXIT_OK
        }
        Err(RebindError::ParseRejected(path)) => {
            eprintln!("rebind: parse rejected: {path}");
            EXIT_STATIC
        }
        Err(RebindError::ObjectCollision(path)) => {
            eprintln!("rebind: the destination object holds different bytes: {path}");
            EXIT_INPUT
        }
        Err(RebindError::Diverged) => {
            eprintln!("rebind: saved bytes changed before the plan converged");
            EXIT_INPUT
        }
        Err(_) => EXIT_INPUT,
    }
}

fn bind_command(arguments: &[String]) -> u8 {
    let (Some(root), Some(config), Some(link)) = (
        option_value(arguments, "--root"),
        option_value(arguments, "--config"),
        option_value(arguments, "--link"),
    ) else {
        return EXIT_USAGE;
    };
    let write = arguments.iter().any(|argument| argument == "--write");
    let Ok(configuration) = load_configuration(config) else {
        return EXIT_INPUT;
    };
    match bind(Path::new(root), &configuration, link, write) {
        Ok(outcome) => {
            println!("{}", outcome.report);
            EXIT_OK
        }
        Err(BindError::ParseRejected(path)) => {
            eprintln!("bind: parse rejected: {path}");
            EXIT_STATIC
        }
        Err(BindError::UnknownLink(id)) => {
            eprintln!("bind: no inline implementation link carries the ID {id}");
            EXIT_INPUT
        }
        Err(BindError::NotCollisionResistant(reason)) => {
            eprintln!("bind: this build cannot address content: {reason}");
            EXIT_INPUT
        }
        Err(BindError::UnsupportedNavigation(navigation)) => {
            eprintln!("bind: the compact form carries only implements, not {navigation}");
            EXIT_INPUT
        }
        Err(BindError::NotExactlyOneAtom(context)) => {
            eprintln!("bind: the compact form binds exactly one atom: {context}");
            EXIT_INPUT
        }
        Err(BindError::StaleItem { bound, current }) => {
            eprintln!(
                "bind: the link's item digest is not current: bound={bound};current={current}"
            );
            EXIT_INPUT
        }
        Err(BindError::ObjectCollision(path)) => {
            eprintln!("bind: the destination object holds different bytes: {path}");
            EXIT_INPUT
        }
        Err(BindError::Diverged(path)) => {
            eprintln!("bind: saved source bytes changed before the write: {path}");
            EXIT_INPUT
        }
        Err(_) => EXIT_INPUT,
    }
}

fn advisory_command(arguments: &[String]) -> u8 {
    let (Some(root), Some(config)) = (
        option_value(arguments, "--root"),
        option_value(arguments, "--config"),
    ) else {
        return EXIT_USAGE;
    };
    let Ok(configuration) = load_configuration(config) else {
        return EXIT_INPUT;
    };
    let Ok(snapshot) = load_snapshot(root, &configuration) else {
        return EXIT_INPUT;
    };
    match static_check(&snapshot, &configuration) {
        Ok(outcome) => match advisory_report(&snapshot, &outcome) {
            Ok(report) => {
                println!("{report}");
                outcome.exit
            }
            Err(_) => EXIT_INPUT,
        },
        Err(_) => EXIT_INPUT,
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct OracleFile {
    #[serde(default)]
    oracle: Vec<buoy::repository::OracleCommand>,
}

fn execute_command(arguments: &[String]) -> u8 {
    let (Some(root), Some(config), Some(oracles)) = (
        option_value(arguments, "--root"),
        option_value(arguments, "--config"),
        option_value(arguments, "--oracles"),
    ) else {
        return EXIT_USAGE;
    };
    let Ok(configuration) = load_configuration(config) else {
        return EXIT_INPUT;
    };
    let Ok(snapshot) = load_snapshot(root, &configuration) else {
        return EXIT_INPUT;
    };
    let Ok(outcome) = static_check(&snapshot, &configuration) else {
        return EXIT_INPUT;
    };
    let Ok(oracle_text) = std::fs::read_to_string(oracles) else {
        return EXIT_INPUT;
    };
    let Ok(oracle_file) = toml::from_str::<OracleFile>(&oracle_text) else {
        return EXIT_INPUT;
    };
    match execute(outcome.identity, &oracle_file.oracle, Path::new(root)) {
        Ok(execution) => {
            println!("{}", execution.report);
            execution.exit
        }
        Err(_) => EXIT_INPUT,
    }
}

fn run(arguments: &[String]) -> u8 {
    match arguments.first().map(String::as_str) {
        Some("static") => static_command(arguments),
        Some("change") => change_command(arguments),
        Some("advisory") => advisory_command(arguments),
        Some("trace") => trace_command(arguments),
        Some("bind") => bind_command(arguments),
        Some("rebind") => rebind_command(arguments),
        Some("execute") => execute_command(arguments),
        _ => EXIT_USAGE,
    }
}

fn main() {
    let arguments = std::env::args().skip(1).collect::<Vec<_>>();
    std::process::exit(i32::from(run(&arguments)));
}

#[cfg(test)]
#[path = "../tests/support/cli_contract.rs"]
mod tests;
