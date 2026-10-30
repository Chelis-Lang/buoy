#![doc = "Offline deterministic Buoy migration driver."]

use buoy::migration::{MigrationError, migrate_complete, migrate_leaf_bindings};
use std::env;
use std::fs;
use std::process::ExitCode;

fn run() -> Result<(), MigrationError> {
    let arguments = env::args().skip(1).collect::<Vec<_>>();
    let [phase, path] = arguments.as_slice() else {
        return Err(MigrationError::Input);
    };
    let source = fs::read_to_string(path).map_err(|_| MigrationError::Input)?;
    match phase.as_str() {
        "leaves" => {
            let migration = migrate_leaf_bindings(&source)?;
            println!("schema\trole\tstable_id\tauthoritative_path\tmedia_type\tdigest");
            for binding in migration.bindings {
                println!(
                    "{}\t{}\t{}\t{}\t{}\t{}",
                    migration.schema,
                    binding.role.as_str(),
                    binding.stable_id,
                    binding.authoritative_path,
                    binding.media_type,
                    binding.digest,
                );
            }
        }
        "complete" => {
            let migration = migrate_complete(&source)?;
            println!("schema\torder\tstable_id\tkind\tlifecycle\treferences\tdigest");
            for object in migration.objects {
                println!(
                    "{}\t{}\t{}\t{}\t{}\t{}\t{}",
                    migration.schema,
                    object.order,
                    object.stable_id,
                    object.kind,
                    object.lifecycle,
                    object.references.join(","),
                    object.digest,
                );
            }
            println!("impact_schema\tstable_id\tdisposition\told_digest\tnew_digest");
            for impact in migration.impacts {
                println!(
                    "spec-provenance-migration-impact/v2\t{}\t{}\t{}\t{}",
                    impact.stable_id,
                    impact.disposition.as_str(),
                    impact.old_digest,
                    impact.new_digest,
                );
            }
        }
        _ => return Err(MigrationError::Input),
    }
    Ok(())
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{}", error.code());
            ExitCode::from(1)
        }
    }
}
