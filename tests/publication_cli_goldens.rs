#![doc = "Exact byte-for-byte goldens captured from the real `buoy publication` CLI command."]
#![allow(missing_docs, clippy::expect_used)]

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::LazyLock;

fn publication_binary() -> &'static Path {
    static BINARY: LazyLock<PathBuf> = LazyLock::new(|| {
        let target =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("target/publication-cli-blake3-fixture");
        let mut command = Command::new(env!("CARGO"));
        for variable in [
            "CARGO_LLVM_COV",
            "CARGO_LLVM_COV_SHOW_ENV",
            "LLVM_PROFILE_FILE",
            "RUSTC_WRAPPER",
            "RUSTFLAGS",
            "RUSTDOCFLAGS",
            "__CARGO_LLVM_COV_RUSTC_WRAPPER",
            "__CARGO_LLVM_COV_RUSTC_WRAPPER_CRATE_NAMES",
            "__CARGO_LLVM_COV_RUSTC_WRAPPER_PRE_EXISTING",
            "__CARGO_LLVM_COV_RUSTC_WRAPPER_RUSTFLAGS",
        ] {
            command.env_remove(variable);
        }
        let output = command
            .args([
                "build",
                "--quiet",
                "--locked",
                "--offline",
                "--package",
                "buoy",
                "--bin",
                "buoy",
                "--no-default-features",
                "--features",
                "blake3-256",
                "--target-dir",
            ])
            .arg(&target)
            .output()
            .expect("BLAKE3 publication fixture build");
        assert!(
            output.status.success(),
            "BLAKE3 publication fixture build: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        target
            .join("debug")
            .join(format!("buoy{}", std::env::consts::EXE_SUFFIX))
    });
    BINARY.as_path()
}

/// Copies the frozen, self-consistent BLAKE3 binding fixture into a fresh
/// temporary root, so the real CLI runs against fully deterministic,
/// content-addressed bytes.
fn copy_binding_repository() -> tempfile::TempDir {
    let source = Path::new("fixtures/content-addressed-bindings/v1/positive");
    let target = tempfile::TempDir::new().expect("binding repository root");
    for path in [
        "buoy.toml",
        "openspec/specs/demo/spec.md",
        "src/lib.rs",
        "buoy/objects/v1/blake3-256/0e857e33e387325fbf457e426bc42286d9d99926b19f3efa3c203795cfeeb6c4.toml",
    ] {
        let destination = target.path().join(path);
        fs::create_dir_all(destination.parent().expect("fixture parent"))
            .expect("fixture directory");
        fs::copy(source.join(path), &destination).expect("copied fixture file");
    }
    target
}

fn plan_path(root: &Path) -> PathBuf {
    let plan_path = root.join("plan.json");
    fs::write(
        &plan_path,
        r#"{"schema":"buoy/trace-publication-plan/v1","publication_id":"golden-example","selector":{"requirement_id":"BGLC-003"},"expected_rows":1,"source_set":["openspec/specs/demo/spec.md","src/lib.rs","buoy/objects/v1/blake3-256/0e857e33e387325fbf457e426bc42286d9d99926b19f3efa3c203795cfeeb6c4.toml"],"outputs":[{"id":"page","mount":"repository","path":"docs/{publication_basis_path}.md","kind":"page","lifecycle":"replaceable"}]}"#,
    )
    .expect("plan");
    plan_path
}

/// This is the exact machine `buoy/trace-publication-report/v1` output the
/// real `buoy publication generate` command emits against the frozen
/// fixture, not an isolated `PublicationReport::new(...)` construction: the
/// committed golden proves the production CLI pipeline's real output shape,
/// including its basis, graph, prior-state, and candidate-digest fields.
#[test]
fn committed_generate_report_golden_matches_the_real_cli_output() {
    let root = copy_binding_repository();
    let config_path = root.path().join("buoy.toml");
    let plan = plan_path(root.path());

    let output = Command::new(publication_binary())
        .args([
            "publication",
            "generate",
            "--plan",
            plan.to_str().expect("plan path"),
            "--artifact",
            "page",
            "--config",
            config_path.to_str().expect("config path"),
            "--root",
            root.path().to_str().expect("root path"),
        ])
        .output()
        .expect("publication generate command");
    assert_eq!(
        output.status.code(),
        Some(0),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let committed = fs::read("examples/expected/publication-cli-generate-report.json")
        .expect("committed generate-report golden");
    assert_eq!(
        committed, output.stdout,
        "regenerate by running the exact command this test issues and committing its stdout \
         (see this test's source for the frozen fixture and plan)"
    );
}

/// This is the exact machine `buoy/trace-publication-evidence/v1` output the
/// real `buoy publication evidence` command emits against the frozen
/// fixture, captured end to end through the production pipeline.
#[test]
fn committed_evidence_golden_matches_the_real_cli_output() {
    let root = copy_binding_repository();
    let config_path = root.path().join("buoy.toml");
    let plan_path = root.path().join("plan.json");
    fs::write(
        &plan_path,
        r#"{"schema":"buoy/trace-publication-plan/v1","publication_id":"golden-example","selector":{"requirement_id":"BGLC-003"},"expected_rows":1,"source_set":["openspec/specs/demo/spec.md","src/lib.rs","buoy/objects/v1/blake3-256/0e857e33e387325fbf457e426bc42286d9d99926b19f3efa3c203795cfeeb6c4.toml"],"outputs":[]}"#,
    )
    .expect("evidence plan");

    let output = Command::new(publication_binary())
        .args([
            "publication",
            "evidence",
            "--plan",
            plan_path.to_str().expect("plan path"),
            "--config",
            config_path.to_str().expect("config path"),
            "--root",
            root.path().to_str().expect("root path"),
        ])
        .output()
        .expect("publication evidence command");
    assert_eq!(
        output.status.code(),
        Some(0),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let committed = fs::read("examples/expected/publication-cli-evidence.json")
        .expect("committed CLI evidence golden");
    assert_eq!(
        committed, output.stdout,
        "regenerate by running the exact command this test issues and committing its stdout"
    );
}

/// Task 13.7: the exact generated *page* bytes the real `buoy publication
/// generate` command writes to disk, captured byte-for-byte -- not the
/// operation report describing the write, and not an isolated call to the
/// Markdown renderer.
#[test]
fn committed_page_golden_matches_the_real_cli_output() {
    let root = copy_binding_repository();
    let config_path = root.path().join("buoy.toml");
    let plan = plan_path(root.path());

    let output = Command::new(publication_binary())
        .args([
            "publication",
            "generate",
            "--plan",
            plan.to_str().expect("plan path"),
            "--artifact",
            "page",
            "--config",
            config_path.to_str().expect("config path"),
            "--root",
            root.path().to_str().expect("root path"),
        ])
        .output()
        .expect("publication generate command");
    assert_eq!(
        output.status.code(),
        Some(0),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let docs_dir = root.path().join("docs/blake3-256");
    let mut generated_pages = fs::read_dir(&docs_dir)
        .expect("docs/blake3-256 directory")
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|extension| extension == "md"))
        .collect::<Vec<_>>();
    assert_eq!(
        generated_pages.len(),
        1,
        "exactly one basis-keyed page must exist"
    );
    let generated_page = generated_pages.remove(0);
    let generated_bytes = fs::read(&generated_page).expect("generated page bytes");

    let committed =
        fs::read("examples/expected/publication-page.md").expect("committed page golden");
    assert_eq!(
        committed, generated_bytes,
        "regenerate by running the exact command this test issues and committing the \
         generated docs/blake3-256/*.md bytes as examples/expected/publication-page.md"
    );
}

/// Task 13.7: the exact generated `OpenSpec` document bytes the real `buoy
/// publication generate` command writes for a declared external-link
/// artifact, captured byte-for-byte through the real CLI dispatch.
#[test]
fn committed_external_link_golden_matches_the_real_cli_output() {
    let root = copy_binding_repository();
    let config_path = root.path().join("buoy.toml");
    let plan_path = root.path().join("plan.json");
    fs::write(
        &plan_path,
        r#"{"schema":"buoy/trace-publication-plan/v1","publication_id":"golden-link-example","selector":{"requirement_id":"BGLC-003"},"expected_rows":1,"source_set":["openspec/specs/demo/spec.md","src/lib.rs","buoy/objects/v1/blake3-256/0e857e33e387325fbf457e426bc42286d9d99926b19f3efa3c203795cfeeb6c4.toml"],"outputs":[{"id":"page","mount":"repository","path":"docs/{publication_basis_path}.md","kind":"page","lifecycle":"immutable","target_base":"https://example.invalid/trace/"},{"id":"link","mount":"repository","path":"openspec/specs/demo/spec.md","kind":"external-link","lifecycle":"replaceable","relation":"documents","page":"page"}]}"#,
    )
    .expect("link plan");

    let output = Command::new(publication_binary())
        .args([
            "publication",
            "generate",
            "--plan",
            plan_path.to_str().expect("plan path"),
            "--artifact",
            "link",
            "--config",
            config_path.to_str().expect("config path"),
            "--root",
            root.path().to_str().expect("root path"),
        ])
        .output()
        .expect("publication generate command");
    assert_eq!(
        output.status.code(),
        Some(0),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let generated_bytes =
        fs::read(root.path().join("openspec/specs/demo/spec.md")).expect("generated document");
    let committed = fs::read("examples/expected/publication-external-link-document.md")
        .expect("committed external-link document golden");
    assert_eq!(
        committed, generated_bytes,
        "regenerate by running the exact command this test issues and committing the \
         generated openspec/specs/demo/spec.md bytes as \
         examples/expected/publication-external-link-document.md"
    );
}
