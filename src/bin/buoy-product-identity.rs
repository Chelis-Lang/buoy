#![doc = "Buoy product-identity migration and stale-name checker."]

use std::env;
use std::path::Path;
use std::process::ExitCode;

fn main() -> ExitCode {
    let arguments = env::args().skip(1).collect::<Vec<_>>();
    match buoy::product_identity::run_cli(&arguments, Path::new(".")) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::from(1)
        }
    }
}
