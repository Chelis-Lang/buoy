#![doc = "Buoy quality-assurance registry validator and deterministic report generator."]

use std::env;
use std::path::Path;
use std::process::ExitCode;

fn main() -> ExitCode {
    let arguments = env::args().skip(1).collect::<Vec<_>>();
    match buoy::quality::run_cli(&arguments, Path::new(".")) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::from(1)
        }
    }
}
