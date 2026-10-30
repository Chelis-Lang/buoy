#![doc = "Validate a rendered readable trace projection against its versioned contract."]

use std::env;
use std::fs;
use std::process::ExitCode;

fn main() -> ExitCode {
    let mut args = env::args().skip(1);
    let Some(flag) = args.next() else {
        eprintln!("RTP-PROJECTION-INPUT");
        return ExitCode::from(2);
    };
    if flag != "--input" {
        eprintln!("RTP-PROJECTION-INPUT");
        return ExitCode::from(2);
    }
    let Some(path) = args.next() else {
        eprintln!("RTP-PROJECTION-INPUT");
        return ExitCode::from(2);
    };
    if args.next().is_some() {
        eprintln!("RTP-PROJECTION-INPUT");
        return ExitCode::from(2);
    }
    let Ok(text) = fs::read_to_string(&path) else {
        eprintln!("RTP-PROJECTION-INPUT");
        return ExitCode::from(2);
    };
    match buoy::publication::validate_markdown_projection(&text) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{}", error.stable_code());
            ExitCode::from(2)
        }
    }
}
