//! Verifies the committed Unicode case-fold source through the sole semantic hasher.

#![allow(clippy::expect_used)]

use buoy::core::{CanonicalValue, semantic_digest};
use std::env;
use std::fs;
use std::process::ExitCode;

const SOURCE: &str = "unicode/16.0.0/CaseFolding.txt";
const EXPECTED: &str = "xxh3-128:8c226c107f489afc04368356a654616e";

fn check(source: &str, expected: &str) -> Result<(), String> {
    let bytes = fs::read(source).map_err(|error| format!("cannot read {source}: {error}"))?;
    let digest = semantic_digest(
        "raw-artifact",
        vec![
            (1, CanonicalValue::Text(String::from("text/plain"))),
            (2, CanonicalValue::Bytes(bytes)),
        ],
    )
    .expect("closed raw Unicode artifact fields canonically encode")
    .canonical_text();
    if digest != expected {
        return Err(format!("expected {expected}, found {digest}"));
    }
    Ok(())
}

fn execute(arguments: &[String]) -> Result<(), String> {
    let source = if arguments == ["--error-control"] {
        "missing-unicode-source"
    } else {
        SOURCE
    };
    if !arguments.is_empty() && arguments != ["--error-control"] {
        return Err(String::from(
            "usage: check_unicode_source [--coverage-control]",
        ));
    }
    check(source, EXPECTED)?;
    println!("unicode-source: ok digest={EXPECTED}");
    Ok(())
}

fn finish(result: Result<(), String>) -> ExitCode {
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("DET-PATH-UNICODE-DATA: {error}");
            ExitCode::from(1)
        }
    }
}

fn run(arguments: &[String]) -> Result<(), String> {
    if arguments == ["--coverage-control"] {
        check("missing-unicode-source", EXPECTED).expect_err("missing source must reject");
        check(SOURCE, "xxh3-128:00000000000000000000000000000000")
            .expect_err("wrong digest must reject");
        assert_ne!(
            finish(execute(&[String::from("--error-control")])),
            ExitCode::SUCCESS
        );
        assert_ne!(
            finish(execute(&[String::from("invalid")])),
            ExitCode::SUCCESS
        );
        return run(&[]);
    }
    execute(arguments)
}

fn main() -> ExitCode {
    finish(run(&env::args().skip(1).collect::<Vec<_>>()))
}
