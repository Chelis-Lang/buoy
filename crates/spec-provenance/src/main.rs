#![doc = "Deprecated compatibility entry point for the Buoy command."]

use std::env;
use std::process::ExitCode;

fn main() -> ExitCode {
    eprintln!(
        "warning: `spec-provenance` is deprecated; use `buoy`; compatibility expires at 0.3.0"
    );
    let arguments = env::args().skip(1).collect::<Vec<_>>();
    ExitCode::from(buoy::cli::run(&arguments))
}
