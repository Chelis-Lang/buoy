//! Dependency and source-boundary checker for independent assurance authorities.

#![allow(clippy::expect_used)]

use std::env;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

const ROOT: &str = "crates/buoy-reference";
const FORBIDDEN: [&str; 13] = [
    "buoy-core",
    "buoy_core",
    "buoy =",
    "buoy::",
    "../buoy-core",
    "spec-provenance-core",
    "spec_provenance_core",
    "spec-provenance =",
    "spec_provenance::",
    "../spec-provenance-core",
    "crate::value",
    "crate::algorithms",
    "include!(",
];

fn main() {
    if let Err(error) = run() {
        eprintln!("reference-independence: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let arguments = env::args().skip(1).collect::<Vec<_>>();
    if arguments.as_slice() == ["--self-test"] {
        return self_test().map(|()| println!("reference-independence self-test: ok"));
    }
    if !arguments.is_empty() {
        return Err(String::from(
            "usage: check_reference_independence [--self-test]",
        ));
    }
    let files = check_root(Path::new(ROOT))?;
    println!("reference-independence: ok files={files}");
    Ok(())
}

fn check_text(name: &str, text: &str) -> Result<(), String> {
    for forbidden in FORBIDDEN {
        if text.contains(forbidden) {
            return Err(format!(
                "HEGEL-REFERENCE-NOT-INDEPENDENT [{name}]: forbidden dependency/API marker `{forbidden}`"
            ));
        }
    }
    Ok(())
}

fn check_root(root: &Path) -> Result<usize, String> {
    let mut paths = vec![root.join("Cargo.toml")];
    collect_rs(&root.join("src"), &mut paths)?;
    paths.sort();
    for path in &paths {
        let text =
            fs::read_to_string(path).map_err(|error| format!("{}: {error}", path.display()))?;
        check_text(&path.display().to_string(), &text)?;
    }
    Ok(paths.len())
}

fn collect_results<T>(
    entries: impl Iterator<Item = io::Result<T>>,
    root: &Path,
) -> Result<Vec<T>, String> {
    entries
        .collect::<io::Result<Vec<_>>>()
        .map_err(|error| format!("{}: {error}", root.display()))
}

fn collect_rs(root: &Path, output: &mut Vec<PathBuf>) -> Result<(), String> {
    let mut entry_error = None;
    collect_rs_with_error(root, output, &mut entry_error)
}

fn collect_rs_with_error(
    root: &Path,
    output: &mut Vec<PathBuf>,
    entry_error: &mut Option<io::Error>,
) -> Result<(), String> {
    let mut pending = vec![root.to_path_buf()];
    while let Some(directory) = pending.pop() {
        let entries = fs::read_dir(&directory)
            .map_err(|error| format!("{}: {error}", directory.display()))?;
        let entries = if let Some(error) = entry_error.take() {
            collect_results([Err::<fs::DirEntry, _>(error)].into_iter(), &directory)
        } else {
            collect_results(entries, &directory)
        };
        for entry in entries? {
            let path = entry.path();
            if path.is_dir() {
                pending.push(path);
            } else if path.extension().and_then(|value| value.to_str()) == Some("rs") {
                output.push(path);
            }
        }
    }
    Ok(())
}

fn self_test() -> Result<(), String> {
    for forbidden in FORBIDDEN {
        let first_match = FORBIDDEN
            .iter()
            .find(|candidate| forbidden.contains(**candidate))
            .expect("every reviewed marker contains itself");
        assert_eq!(
            check_text("negative", forbidden)
                .expect_err("forbidden reference marker must be rejected"),
            format!(
                "HEGEL-REFERENCE-NOT-INDEPENDENT [negative]: forbidden dependency/API marker `{first_match}`"
            )
        );
    }
    collect_results(
        [Err::<u8, _>(io::Error::other("synthetic entry failure"))].into_iter(),
        Path::new("synthetic"),
    )
    .expect_err("synthetic directory entry failure must propagate");
    let mut output = Vec::new();
    let mut entry_error = Some(io::Error::other("injected entry failure"));
    collect_rs_with_error(Path::new("."), &mut output, &mut entry_error)
        .expect_err("injected directory entry failure must propagate");

    let root = env::temp_dir().join(format!(
        "buoy-reference-independence-{}",
        std::process::id()
    ));
    drop(fs::remove_dir_all(&root));
    fs::create_dir_all(root.join("src/nested")).expect("control source tree must construct");
    fs::write(root.join("Cargo.toml"), "[package]\nname='reference'\n")
        .expect("control manifest must write");
    fs::write(root.join("src/lib.rs"), "pub fn reference() {}\n")
        .expect("control library must write");
    fs::write(root.join("src/nested/data.txt"), "ignored\n")
        .expect("ignored control source must write");
    fs::write(root.join("src/nested/value.rs"), "pub fn value() {}\n")
        .expect("nested control source must write");
    let positive = check_root(&root).map(|_| ());

    fs::remove_file(root.join("Cargo.toml")).expect("control manifest must remove");
    check_root(&root).expect_err("missing manifest must reject");
    fs::write(root.join("Cargo.toml"), "buoy_core = '0'\n")
        .expect("negative control manifest must write");
    assert!(
        check_root(&root)
            .expect_err("forbidden dependency must reject")
            .starts_with("HEGEL-REFERENCE-NOT-INDEPENDENT")
    );
    fs::remove_dir_all(root.join("src")).expect("control source tree must remove");
    check_root(&root).expect_err("missing source root must reject");
    fs::remove_dir_all(root).expect("control root must remove");
    positive
}
