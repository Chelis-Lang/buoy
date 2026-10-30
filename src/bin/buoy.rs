#![doc = "Primary Buoy command-line entry point."]

use std::env;
use std::process::ExitCode;

fn main() -> ExitCode {
    let arguments = env::args().skip(1).collect::<Vec<_>>();
    ExitCode::from(buoy::cli::run(&arguments))
}
