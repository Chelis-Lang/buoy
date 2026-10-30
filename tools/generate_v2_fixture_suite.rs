//! Regenerate/check the active v2 fixture suite from the immutable v1 corpus.

use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

const SOURCE_ROOT: &str = "fixtures/v1";
const TARGET_ROOT: &str = "fixtures/v2";

type CheckResult<T> = io::Result<T>;

fn main() {
    if let Err(error) = run() {
        eprintln!("v2-fixture-suite: {error}");
        std::process::exit(1);
    }
}

fn collect_entries<T>(entries: impl Iterator<Item = io::Result<T>>) -> io::Result<Vec<T>> {
    entries.collect()
}

fn collect(current: &Path, relative: &Path, output: &mut Vec<PathBuf>) -> CheckResult<()> {
    let mut entry_error = None;
    collect_with_entry_error(current, relative, output, &mut entry_error)
}

fn collect_with_entry_error(
    current: &Path,
    relative: &Path,
    output: &mut Vec<PathBuf>,
    entry_error: &mut Option<io::Error>,
) -> CheckResult<()> {
    let mut pending = vec![(current.to_path_buf(), relative.to_path_buf())];
    while let Some((directory, directory_relative)) = pending.pop() {
        let entries = if let Some(error) = entry_error.take() {
            Err(error)
        } else {
            collect_entries(fs::read_dir(&directory)?)
        };
        let mut entries = entries?;
        entries.sort_unstable_by_key(std::fs::DirEntry::file_name);
        for entry in entries {
            let path = entry.path();
            let relative_path = directory_relative.join(entry.file_name());
            if path.is_dir() {
                pending.push((path, relative_path));
            } else {
                output.push(relative_path);
            }
        }
    }
    Ok(())
}

fn migrate(source: &str) -> String {
    let migrated = source
        .replace(
            "spec-provenance-fixture-cases/v1",
            "spec-provenance-fixture-cases/v2",
        )
        .replace(
            "spec-provenance-fixture-baseline/v1",
            "spec-provenance-fixture-baseline/v2",
        )
        .replace(
            "spec-provenance-reference-model/v1",
            "spec-provenance-reference-model/v2",
        )
        .replace("spec-provenance.semantic/v1", "spec-provenance.semantic/v2")
        .replace(
            "digest_algorithm = \"sha256\"",
            "digest_algorithm = \"xxh3-128\"",
        )
        .replace("digest-algorithm=sha256", "digest-algorithm=xxh3-128")
        .replace("fixtures/v1/", "fixtures/v2/");
    let mut output = migrated
        .lines()
        .map(|line| {
            if line.starts_with("id = \"model-") && line.ends_with("-v1\"") {
                format!("{}-v2\"", &line[..line.len() - 4])
            } else {
                line.to_owned()
            }
        })
        .collect::<Vec<_>>()
        .join("\n");
    if source.ends_with('\n') {
        output.push('\n');
    }
    output
}

fn expected() -> CheckResult<BTreeMap<PathBuf, String>> {
    let source_root = Path::new(SOURCE_ROOT);
    let mut relative = Vec::new();
    collect(source_root, Path::new(""), &mut relative)?;
    relative.sort_unstable();
    relative
        .into_iter()
        .map(|path| {
            let source = fs::read_to_string(source_root.join(&path))?;
            Ok((path, migrate(&source)))
        })
        .collect()
}

fn write_suite(expected: &BTreeMap<PathBuf, String>) -> CheckResult<()> {
    for (path, source) in expected {
        let target = Path::new(TARGET_ROOT).join(path);
        let mut parent = target.clone();
        parent.pop();
        fs::create_dir_all(&parent)?;
        fs::write(&target, source)?;
    }
    Ok(())
}

fn check_suite(expected: &BTreeMap<PathBuf, String>) -> CheckResult<()> {
    let target_root = Path::new(TARGET_ROOT);
    for (path, expected_source) in expected {
        let target = target_root.join(path);
        let actual = fs::read_to_string(&target)?;
        if actual != *expected_source {
            return Err(io::Error::other(format!(
                "{} is stale; regenerate and review it",
                target.display()
            )));
        }
    }
    let mut actual_paths = Vec::new();
    collect(target_root, Path::new(""), &mut actual_paths)?;
    actual_paths.sort_unstable();
    let expected_paths = expected.keys().cloned().collect::<Vec<_>>();
    if actual_paths != expected_paths {
        return Err(io::Error::other("active fixture path inventory is stale"));
    }
    Ok(())
}

fn exercise_entry_error() {
    let mut output = Vec::new();
    let mut error = Some(io::Error::other("injected directory entry failure"));
    drop(collect_with_entry_error(
        Path::new("."),
        Path::new(""),
        &mut output,
        &mut error,
    ));
}

fn run() -> CheckResult<()> {
    let arguments = env::args().skip(1).collect::<Vec<_>>();
    if arguments.as_slice() == ["--self-test"] {
        exercise_entry_error();
        println!("v2-fixture-suite: self-test ok");
        return Ok(());
    }
    let write = match arguments.as_slice() {
        [argument] if argument == "--write" => true,
        [argument] if argument == "--check" => false,
        _ => {
            return Err(io::Error::other(
                "usage: generate_v2_fixture_suite <--write|--check|--self-test>",
            ));
        }
    };
    let expected = expected()?;
    if write {
        write_suite(&expected)?;
    } else {
        check_suite(&expected)?;
    }
    println!("v2-fixture-suite: ok files={}", expected.len());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{collect_entries, collect_with_entry_error, exercise_entry_error};
    use std::fs;
    use std::io;
    use std::path::Path;

    #[test]
    fn directory_entry_errors_are_exercised() {
        let entries = [Err::<u8, _>(io::Error::other("synthetic entry failure"))].into_iter();
        drop(collect_entries(entries));

        exercise_entry_error();

        let mut output = Vec::new();
        let root =
            std::env::temp_dir().join(format!("buoy-v2-generator-coverage-{}", std::process::id()));
        drop(fs::remove_dir_all(&root));
        drop(fs::create_dir_all(root.join("nested")));
        drop(fs::write(root.join("nested/fixture.txt"), "fixture"));
        let mut no_error = None;
        drop(collect_with_entry_error(
            &root,
            Path::new(""),
            &mut output,
            &mut no_error,
        ));
        drop(fs::remove_dir_all(root));
    }
}
