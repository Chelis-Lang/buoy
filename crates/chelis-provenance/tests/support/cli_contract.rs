use super::run;
use chelis_provenance::command::{
    EXIT_CHANGE, EXIT_EXECUTION, EXIT_INPUT, EXIT_OK, EXIT_STATIC, EXIT_USAGE,
};

fn arguments(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| (*value).to_owned()).collect()
}

const CLEAN: &[u8] = b"-- chelis:provenance/v1 authority\n-- id = CLI-ATOM\n-- kind = behavioral\n-- scopes = cli\n-- statement = The command MUST preserve saved bytes.\ndef clean(x: f32) -> f32 = x\n";

fn repository() -> tempfile::TempDir {
    let root = tempfile::tempdir().unwrap_or_else(|_| unreachable!("temp root"));
    std::fs::create_dir(root.path().join("chelis"))
        .unwrap_or_else(|_| unreachable!("selector root"));
    std::fs::write(root.path().join("chelis/clean.ch"), CLEAN)
        .unwrap_or_else(|_| unreachable!("source write"));
    std::fs::write(
        root.path().join("config.toml"),
        b"schema = \"chelis-adapter-config/v1\"\nbinding-store = \"chelis/objects\"\n\n[[selector]]\nroot = \"chelis\"\ndialect = \"surf\"\n",
    )
    .unwrap_or_else(|_| unreachable!("config write"));
    root
}

fn command(root: &std::path::Path, name: &str, extra: &[&str]) -> Vec<String> {
    let mut values = vec![
        name.to_owned(),
        "--root".to_owned(),
        root.to_string_lossy().into_owned(),
        "--config".to_owned(),
        root.join("config.toml").to_string_lossy().into_owned(),
    ];
    values.extend(arguments(extra));
    values
}

#[test]
fn advisory_subcommand_preserves_the_exit_classes() {
    let root = repository();
    for name in ["static", "advisory", "trace"] {
        let values = command(root.path(), name, &["--id", "CLI-ATOM"]);
        assert_eq!(run(&values), EXIT_OK, "{name}");
        assert_eq!(
            std::fs::read(root.path().join("chelis/clean.ch"))
                .ok()
                .as_deref(),
            Some(CLEAN)
        );
        std::fs::write(root.path().join("chelis/clean.ch"), b"def broken( =\n")
            .unwrap_or_else(|_| unreachable!("invalid source"));
        let expected = if name == "trace" {
            EXIT_INPUT
        } else {
            EXIT_STATIC
        };
        assert_eq!(run(&values), expected, "{name}");
        std::fs::write(root.path().join("chelis/clean.ch"), CLEAN)
            .unwrap_or_else(|_| unreachable!("restore source"));
    }
}

#[test]
fn missing_arguments_answer_with_the_usage_exit_class() {
    for name in [
        "static", "change", "advisory", "trace", "bind", "rebind", "execute",
    ] {
        assert_eq!(run(&arguments(&[name])), EXIT_USAGE, "{name}");
    }
    let root = repository();
    for name in ["static", "advisory", "trace", "bind", "rebind", "execute"] {
        let values = command(
            root.path(),
            name,
            &[
                "--id",
                "CLI-ATOM",
                "--link",
                "CLI-LINK",
                "--oracles",
                "oracles.toml",
            ],
        );
        for option in ["--root", "--config"] {
            let mut missing = values.clone();
            let index = missing
                .iter()
                .position(|value| value == option)
                .unwrap_or_else(|| unreachable!("required option"));
            missing.drain(index..=index + 1);
            assert_eq!(run(&missing), EXIT_USAGE, "{name}: {option}");
        }
    }
    for (name, option) in [
        ("trace", "--id"),
        ("bind", "--link"),
        ("execute", "--oracles"),
    ] {
        let mut values = command(root.path(), name, &[]);
        values.push(option.to_owned());
        assert_eq!(run(&values), EXIT_USAGE, "{name}: missing value");
    }
    assert_eq!(run(&arguments(&["unknown"])), EXIT_USAGE);
    assert_eq!(run(&arguments(&[])), EXIT_USAGE);
}

#[test]
fn invalid_configuration_and_roots_preserve_the_input_exit_class() {
    let root = repository();
    let missing = root.path().join("absent").to_string_lossy().into_owned();
    let invalid_root = root.path().join("invalid-root");
    std::fs::create_dir(&invalid_root).unwrap_or_else(|_| unreachable!("invalid root"));
    std::fs::write(invalid_root.join("chelis"), b"not a directory")
        .unwrap_or_else(|_| unreachable!("invalid selector"));
    for name in [
        "static", "advisory", "trace", "bind", "rebind", "execute", "change",
    ] {
        let mut values = command(
            root.path(),
            name,
            &[
                "--id",
                "CLI-ATOM",
                "--link",
                "CLI-LINK",
                "--oracles",
                &missing,
                "--base",
                &missing,
                "--head",
                &missing,
                "--manifest",
                &missing,
            ],
        );
        let index = values
            .iter()
            .position(|value| value == "--config")
            .unwrap_or_else(|| unreachable!("config option"))
            + 1;
        values[index] = missing.clone();
        assert_eq!(run(&values), EXIT_INPUT, "{name}: absent configuration");
        values[index] = root
            .path()
            .join("config.toml")
            .to_string_lossy()
            .into_owned();
        let config =
            std::fs::read(&values[index]).unwrap_or_else(|_| unreachable!("saved configuration"));
        std::fs::write(&values[index], b"schema = 7\n")
            .unwrap_or_else(|_| unreachable!("invalid configuration"));
        assert_eq!(run(&values), EXIT_INPUT, "{name}: invalid configuration");
        std::fs::write(&values[index], config)
            .unwrap_or_else(|_| unreachable!("restore configuration"));
        values[2] = invalid_root.to_string_lossy().into_owned();
        if name == "change" {
            let base = values
                .iter()
                .position(|value| value == "--base")
                .unwrap_or_else(|| unreachable!("base option"))
                + 1;
            values[base] = invalid_root.to_string_lossy().into_owned();
        }
        assert_eq!(run(&values), EXIT_INPUT, "{name}: invalid root");
    }
}

#[test]
fn binding_commands_reject_invalid_saved_source_without_writes() {
    let root = repository();
    assert_eq!(
        run(&command(
            root.path(),
            "bind",
            &["--link", "ABSENT", "--write"]
        )),
        EXIT_INPUT
    );
    assert_eq!(
        std::fs::read(root.path().join("chelis/clean.ch"))
            .ok()
            .as_deref(),
        Some(CLEAN)
    );
    let invalid = b"def broken( =\n";
    std::fs::write(root.path().join("chelis/clean.ch"), invalid)
        .unwrap_or_else(|_| unreachable!("invalid source"));
    for name in ["bind", "rebind"] {
        assert_eq!(
            run(&command(
                root.path(),
                name,
                &["--link", "ABSENT", "--write"]
            )),
            EXIT_STATIC
        );
        assert_eq!(
            std::fs::read(root.path().join("chelis/clean.ch"))
                .ok()
                .as_deref(),
            Some(invalid.as_slice())
        );
        assert!(!root.path().join("chelis/objects").exists());
    }
}

#[test]
fn change_reads_both_saved_trees_and_rejects_invalid_manifests() {
    let base = repository();
    let head = repository();
    let manifest = head.path().join("manifest.toml");
    let config = head.path().join("config.toml");
    let values = arguments(&[
        "change",
        "--base",
        &base.path().to_string_lossy(),
        "--head",
        &head.path().to_string_lossy(),
        "--config",
        &config.to_string_lossy(),
        "--manifest",
        &manifest.to_string_lossy(),
    ]);
    assert_eq!(run(&values), EXIT_INPUT, "absent manifest");
    std::fs::write(&manifest, b"schema = 7\n").unwrap_or_else(|_| unreachable!("invalid manifest"));
    assert_eq!(run(&values), EXIT_INPUT, "invalid manifest");
    std::fs::write(&manifest, b"schema = \"chelis-change-manifest/v1\"\n")
        .unwrap_or_else(|_| unreachable!("valid manifest"));
    assert_eq!(run(&values), EXIT_OK);
    std::fs::write(head.path().join("chelis/clean.ch"), b"def broken( =\n")
        .unwrap_or_else(|_| unreachable!("invalid head"));
    assert_eq!(run(&values), EXIT_CHANGE);
    assert_eq!(
        std::fs::read(base.path().join("chelis/clean.ch"))
            .ok()
            .as_deref(),
        Some(CLEAN)
    );
    for option in ["--base", "--head", "--config", "--manifest"] {
        let mut missing = values.clone();
        let index = missing
            .iter()
            .position(|value| value == option)
            .unwrap_or_else(|| unreachable!("required option"));
        missing.drain(index..=index + 1);
        assert_eq!(run(&missing), EXIT_USAGE, "{option}");
    }
}

#[test]
fn execute_reads_saved_oracles_and_preserves_effects_and_exit_classes() {
    let root = repository();
    let oracles = root.path().join("oracles.toml");
    let values = command(
        root.path(),
        "execute",
        &["--oracles", &oracles.to_string_lossy()],
    );
    assert_eq!(run(&values), EXIT_INPUT, "absent oracle file");
    std::fs::write(&oracles, b"unknown = true\n")
        .unwrap_or_else(|_| unreachable!("invalid oracle file"));
    assert_eq!(run(&values), EXIT_INPUT, "unknown oracle field");
    for (status, expected) in [(0, EXIT_OK), (1, EXIT_EXECUTION)] {
        let text = format!(
            "[[oracle]]\nid = \"CLI-ORACLE\"\nprogram = \"/bin/sh\"\nargs = [\"-c\", \"printf saved > receipt.txt; printf 'spec-provenance-result/v1\\\\tpass\\\\n'; exit {status}\"]\ntimeout_ms = 5000\n"
        );
        std::fs::write(&oracles, text).unwrap_or_else(|_| unreachable!("oracle file"));
        assert_eq!(run(&values), expected, "oracle status {status}");
        assert_eq!(
            std::fs::read(root.path().join("receipt.txt"))
                .ok()
                .as_deref(),
            Some(b"saved".as_slice())
        );
        std::fs::remove_file(root.path().join("receipt.txt"))
            .unwrap_or_else(|_| unreachable!("remove receipt"));
        assert_eq!(
            std::fs::read(root.path().join("chelis/clean.ch"))
                .ok()
                .as_deref(),
            Some(CLEAN)
        );
    }
}
