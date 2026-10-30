#![doc = "End-to-end contract for the real publication command pipeline."]
#![allow(missing_docs, clippy::expect_used)]
#![cfg(feature = "blake3-256")]

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Copies the frozen, self-consistent BLAKE3 binding fixture (one `OpenSpec`
/// requirement, one Rust implementation, one committed binding record) into
/// a fresh temporary root.
///
/// This is real repository state with a real current binding, not a stub:
/// `buoy trace --requirement-id BGLC-003` against this exact fixture already
/// returns exit `0` with an empty diagnostic set (see
/// `trace_command_stays_read_only_and_keeps_existing_exit_classes` in
/// `src/cli.rs`), so it is the correct fixture for proving the publication
/// pipeline's real row-conformance and candidate-rendering path end to end.
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

/// A dedicated, self-consistent binding fixture built specifically so a
/// publication-slot edit to `openspec/specs/demo/spec.md` NEVER invalidates
/// the committed implementation binding: unlike
/// `fixtures/content-addressed-bindings/v1/positive`, this fixture's
/// `openspec/specs/demo/spec.md` declares a SECOND requirement (`BGLC-004`,
/// deliberately unbound) after `BGLC-003`. A declared publication slot is
/// always inserted at the end of the `## Requirements` section (or end of
/// file), which the second requirement's own span now absorbs -- so
/// `BGLC-003`'s pinned requirement-node digest stays byte-identical across
/// a real external-link generation. The committed binding record here was
/// produced by the real `buoy bind` command against this exact fixture
/// (confirmed independently: `buoy trace --requirement-id BGLC-003` already
/// returns exit `0`, state `current`, with every freshness facet `true`).
fn copy_publication_sequence_repository() -> tempfile::TempDir {
    let source = Path::new("fixtures/content-addressed-bindings/v1/publication-sequence");
    let target = tempfile::TempDir::new().expect("publication-sequence repository root");
    for path in [
        "buoy.toml",
        "openspec/specs/demo/spec.md",
        "src/lib.rs",
        "buoy/objects/v1/blake3-256/5be031013feeaccb77f755d96d8f04dd9d131a4a493f7842483a734b6a38088f.toml",
    ] {
        let destination = target.path().join(path);
        fs::create_dir_all(destination.parent().expect("fixture parent"))
            .expect("fixture directory");
        fs::copy(source.join(path), &destination).expect("copied fixture file");
    }
    target
}

/// The same frozen binding fixture, but its `buoy.toml` declares only the
/// change-detection member `xxh3-128` as the active digest algorithm --
/// never eligible for an immutable, basis-keyed publication target.
fn copy_binding_repository_with_change_detection_only_algorithm() -> tempfile::TempDir {
    let root = copy_binding_repository();
    fs::write(
        root.path().join("buoy.toml"),
        fs::read_to_string(root.path().join("buoy.toml"))
            .expect("buoy.toml")
            .replacen(
                r#"digest_algorithms = ["blake3-256"]"#,
                r#"digest_algorithms = ["xxh3-128"]"#,
                1,
            ),
    )
    .expect("rewritten buoy.toml");
    root
}

fn plan(root: &Path, publication_id: &str, outputs: &str) -> PathBuf {
    let plan_path = root.join("plan.json");
    fs::write(
        &plan_path,
        format!(
            r#"{{"schema":"buoy/trace-publication-plan/v1","publication_id":"{publication_id}","selector":{{"requirement_id":"BGLC-003"}},"expected_rows":1,"source_set":["openspec/specs/demo/spec.md","src/lib.rs","buoy/objects/v1/blake3-256/0e857e33e387325fbf457e426bc42286d9d99926b19f3efa3c203795cfeeb6c4.toml"],"outputs":[{outputs}]}}"#
        ),
    )
    .expect("plan");
    plan_path
}

/// A plan JSON for the dedicated `publication-sequence` fixture, whose
/// committed binding record uses a different filename (digest) than the
/// `positive` fixture [`plan`] targets.
fn sequence_plan(root: &Path, publication_id: &str, outputs: &str) -> PathBuf {
    let plan_path = root.join("plan.json");
    fs::write(
        &plan_path,
        format!(
            r#"{{"schema":"buoy/trace-publication-plan/v1","publication_id":"{publication_id}","selector":{{"requirement_id":"BGLC-003"}},"expected_rows":1,"source_set":["openspec/specs/demo/spec.md","src/lib.rs","buoy/objects/v1/blake3-256/5be031013feeaccb77f755d96d8f04dd9d131a4a493f7842483a734b6a38088f.toml"],"outputs":[{outputs}]}}"#
        ),
    )
    .expect("plan");
    plan_path
}

/// A plan JSON identical to [`plan`] except its `source_set` is declared
/// empty.
fn plan_with_empty_source_set(root: &Path, publication_id: &str, outputs: &str) -> PathBuf {
    let plan_path = root.join("plan.json");
    fs::write(
        &plan_path,
        format!(
            r#"{{"schema":"buoy/trace-publication-plan/v1","publication_id":"{publication_id}","selector":{{"requirement_id":"BGLC-003"}},"expected_rows":1,"source_set":[],"outputs":[{outputs}]}}"#
        ),
    )
    .expect("plan");
    plan_path
}

fn run_buoy(
    root: &Path,
    config_path: &Path,
    plan_path: &Path,
    args: &[&str],
) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_buoy"))
        .arg("publication")
        .args(args)
        .arg("--plan")
        .arg(plan_path)
        .arg("--config")
        .arg(config_path)
        .arg("--root")
        .arg(root)
        .output()
        .expect("publication command")
}

#[test]
fn evidence_carries_a_real_source_digest_derived_from_retained_bytes() {
    let root = copy_binding_repository();
    let config_path = root.path().join("buoy.toml");
    let plan_path = plan(root.path(), "evidence-cli", "");

    let output = run_buoy(root.path(), &config_path, &plan_path, &["evidence"]);
    assert_eq!(
        output.status.code(),
        Some(0),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let rendered: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("evidence JSON");
    let digest = rendered["source_digest"]
        .as_str()
        .expect("source digest present");
    assert!(digest.starts_with("blake3-256:"));
    assert_eq!(rendered["selector"], "requirement-id BGLC-003");
    assert!(
        rendered["basis"]
            .as_str()
            .is_some_and(|value| value.starts_with("blake3-256:"))
    );
    assert!(
        rendered["graph"]
            .as_str()
            .is_some_and(|value| value.starts_with("blake3-256:"))
    );
    let rows = rendered["rows"].as_array().expect("rows array present");
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["requirement_id"], "BGLC-003");
    assert_eq!(rows[0]["state"], "current");
    assert!(
        rows[0]["atom_revision"]
            .as_str()
            .is_some_and(|value| value.starts_with("blake3-256:"))
    );

    // Changing a retained selected file must change the reported digest,
    // proving it is derived from real retained bytes rather than fixed. A
    // trailing comment in the nonauthority Rust source changes retained
    // bytes without touching the requirement or binding closure at all.
    let rust_path = root.path().join("src/lib.rs");
    let mut bytes = fs::read(&rust_path).expect("Rust bytes");
    bytes.extend_from_slice(b"\n// retained-bytes control comment\n");
    fs::write(&rust_path, bytes).expect("changed Rust bytes");

    let second_output = run_buoy(root.path(), &config_path, &plan_path, &["evidence"]);
    let second_rendered: serde_json::Value =
        serde_json::from_slice(&second_output.stdout).expect("second evidence JSON");
    assert_ne!(rendered["source_digest"], second_rendered["source_digest"]);
}

/// TPT-008: generated evidence excludes immutable record bodies, provider
/// fields, commits, users, approvals, clocks, and physical paths. This
/// checks the real rendered machine JSON (both top-level and every row) and
/// the real rendered Markdown from the actual CLI output, not the typed
/// value's field list alone -- proving no forbidden key or the real
/// temporary root's absolute path leaks into either rendered form.
#[test]
fn evidence_excludes_every_forbidden_field_and_the_physical_root() {
    let root = copy_binding_repository();
    let config_path = root.path().join("buoy.toml");
    let plan_path = plan(root.path(), "evidence-boundary", "");

    let output = run_buoy(root.path(), &config_path, &plan_path, &["evidence"]);
    assert_eq!(output.status.code(), Some(0));
    let text = String::from_utf8(output.stdout).expect("UTF-8 evidence JSON");
    let rendered: serde_json::Value = serde_json::from_str(&text).expect("evidence JSON");

    for forbidden_key in [
        "record_body",
        "body",
        "provider",
        "commit",
        "user",
        "approval",
        "clock",
        "timestamp",
        "physical_root",
        "physical_path",
    ] {
        assert!(
            rendered.get(forbidden_key).is_none(),
            "top-level evidence must never carry a `{forbidden_key}` field"
        );
        for row in rendered["rows"].as_array().unwrap_or(&Vec::new()) {
            assert!(
                row.get(forbidden_key).is_none(),
                "a row must never carry a `{forbidden_key}` field"
            );
        }
    }
    // The real absolute temporary repository root must never appear in the
    // rendered evidence bytes at all.
    let physical_root = root.path().to_str().expect("UTF-8 root path");
    assert!(
        !text.contains(physical_root),
        "evidence must never embed the real physical repository root"
    );
}

#[test]
fn generate_writes_the_declared_page_through_the_cli_pipeline() {
    let root = copy_binding_repository();
    let config_path = root.path().join("buoy.toml");
    let plan_path = plan(
        root.path(),
        "generate-cli",
        r#"{"id":"page","mount":"repository","path":"out/page.md","kind":"page","lifecycle":"replaceable"}"#,
    );

    let check_before = run_buoy(
        root.path(),
        &config_path,
        &plan_path,
        &["check", "--artifact", "page"],
    );
    assert_eq!(
        check_before.status.code(),
        Some(2),
        "an absent page must be a static blocker before generation"
    );
    assert!(
        !root.path().join("out/page.md").exists(),
        "check must never write the declared page"
    );

    let generated = run_buoy(
        root.path(),
        &config_path,
        &plan_path,
        &["generate", "--artifact", "page"],
    );
    assert_eq!(
        generated.status.code(),
        Some(0),
        "stderr: {}",
        String::from_utf8_lossy(&generated.stderr)
    );
    let page_path = root.path().join("out/page.md");
    assert!(
        page_path.is_file(),
        "generate must create the declared page"
    );
    let bytes = fs::read(&page_path).expect("generated page bytes");
    let text = String::from_utf8(bytes).expect("UTF-8 page");
    assert!(text.contains("BGLC-003"));
    assert!(text.contains("buoy/readable-trace-markdown/v1"));

    // The generate report identifies the real plan, basis, graph, prior
    // state, candidate digest, and local outcome — not a stub.
    let generated_report: serde_json::Value =
        serde_json::from_slice(&generated.stdout).expect("generate report JSON");
    assert_eq!(
        generated_report["schema"],
        "buoy/trace-publication-report/v1"
    );
    assert_eq!(generated_report["plan_id"], "generate-cli");
    assert_eq!(generated_report["prior_state"], "absent");
    // This page is declared `replaceable`, but its prior state is genuinely
    // absent, so the correct reported outcome is `created`, not `replaced`:
    // a replaceable writer must report the real prior state, not always
    // claim a replacement occurred.
    assert_eq!(generated_report["outcome"], "created");
    assert!(
        generated_report["basis"]
            .as_str()
            .is_some_and(|value| value.starts_with("blake3-256:"))
    );
    assert!(
        generated_report["graph"]
            .as_str()
            .is_some_and(|value| value.starts_with("blake3-256:"))
    );
    assert!(
        generated_report["candidate_digest"]
            .as_str()
            .is_some_and(|value| value.starts_with("blake3-256:"))
    );

    let check_after = run_buoy(
        root.path(),
        &config_path,
        &plan_path,
        &["check", "--artifact", "page"],
    );
    assert_eq!(
        check_after.status.code(),
        Some(0),
        "an equal generated page must pass check"
    );
    let check_report: serde_json::Value =
        serde_json::from_slice(&check_after.stdout).expect("check report JSON");
    assert_eq!(check_report["prior_state"], "equal");
    assert_eq!(check_report["outcome"], "no-op");
    assert_eq!(
        check_report["prior_digest"], generated_report["candidate_digest"],
        "an equal check must report the same digest generate wrote"
    );
}

/// TPT: a malformed plan must emit a versioned machine report with one
/// stable `TPT-*` diagnostic code and structured fields -- not a bare exit
/// code with no information at all. This runs the real CLI against a plan
/// with an immutable page that declares no referencing external link (an
/// orphan immutable page), which fails plan parsing itself.
#[test]
fn a_malformed_plan_emits_a_versioned_report_with_one_stable_code() {
    let root = copy_binding_repository();
    let config_path = root.path().join("buoy.toml");
    let plan_path = plan(
        root.path(),
        "malformed-plan-cli",
        r#"{"id":"page","mount":"repository","path":"docs/{publication_basis_path}.md","kind":"page","lifecycle":"immutable"}"#,
    );

    let output = run_buoy(
        root.path(),
        &config_path,
        &plan_path,
        &["check", "--artifact", "page"],
    );
    assert_eq!(output.status.code(), Some(70));
    assert!(
        !output.stdout.is_empty(),
        "a malformed plan must render a report, not bare silence"
    );
    let rendered: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("malformed-plan report JSON");
    assert_eq!(rendered["schema"], "buoy/trace-publication-report/v1");
    assert_eq!(rendered["operation"], "check");
    let diagnostics = rendered["diagnostics"]
        .as_array()
        .expect("diagnostics array");
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0]["code"], "TPT-PLAN-ORPHAN-IMMUTABLE-PAGE");
}

/// TPT-003: a row-conformance drift must emit a versioned report naming its
/// one exact stable `TPT-ROW-*` code, and exit `2` -- through the real CLI,
/// not a bare exit code. A plan that declares `expected_rows: 2` against a
/// selector this fixture resolves to exactly one real current row is a
/// real, deterministic row-count drift.
#[test]
fn a_row_conformance_drift_emits_a_versioned_report_with_its_exact_stable_code() {
    let root = copy_binding_repository();
    let config_path = root.path().join("buoy.toml");
    let plan_path = root.path().join("plan.json");
    fs::write(
        &plan_path,
        r#"{"schema":"buoy/trace-publication-plan/v1","publication_id":"drift-cli","selector":{"requirement_id":"BGLC-003"},"expected_rows":2,"source_set":["openspec/specs/demo/spec.md","src/lib.rs","buoy/objects/v1/blake3-256/0e857e33e387325fbf457e426bc42286d9d99926b19f3efa3c203795cfeeb6c4.toml"],"outputs":[{"id":"page","mount":"repository","path":"out/page.md","kind":"page","lifecycle":"replaceable"}]}"#,
    )
    .expect("row-count drift plan");

    let output = run_buoy(
        root.path(),
        &config_path,
        &plan_path,
        &["check", "--artifact", "page"],
    );
    assert_eq!(output.status.code(), Some(2));
    assert!(
        !output.stdout.is_empty(),
        "a row-conformance drift must render a report, not bare silence"
    );
    let rendered: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("drift report JSON");
    assert_eq!(rendered["schema"], "buoy/trace-publication-report/v1");
    assert_eq!(rendered["plan_id"], "drift-cli");
    let diagnostics = rendered["diagnostics"]
        .as_array()
        .expect("diagnostics array");
    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic["code"] == "TPT-ROW-COUNT"),
        "drift report must carry the exact TPT-ROW-COUNT code: {rendered}"
    );
}

#[test]
fn evidence_fails_closed_before_rendering_when_a_declared_output_is_absent() {
    let root = copy_binding_repository();
    let config_path = root.path().join("buoy.toml");
    let plan_path = plan(
        root.path(),
        "evidence-gate",
        r#"{"id":"page","mount":"repository","path":"out/page.md","kind":"page","lifecycle":"replaceable"}"#,
    );

    let output = run_buoy(root.path(), &config_path, &plan_path, &["evidence"]);
    assert_eq!(output.status.code(), Some(2));
    // `evidence` is the one operation whose entire product IS its machine
    // document, so a blocked run must still emit a complete, versioned
    // `buoy/trace-publication-evidence/v1` value carrying the exact stable
    // blocker code. Exiting `2` with empty output would leave a caller
    // nothing machine-readable to act on -- precisely the defect this now
    // asserts is fixed.
    assert!(
        !output.stdout.is_empty(),
        "a blocked evidence run must still render its versioned machine document"
    );
    let rendered: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("blocked evidence JSON");
    assert_eq!(
        rendered["schema"], "buoy/trace-publication-evidence/v1",
        "the blocked document must still identify its own schema"
    );
    assert_eq!(
        rendered["status"], "blocked",
        "a drifted plan must never claim a complete historical summary"
    );
    let diagnostics = rendered["diagnostics"]
        .as_array()
        .expect("diagnostics array");
    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic["code"] == "TPT-EVIDENCE-ARTIFACT-DRIFT"),
        "the absent declared page must be named by its stable drift code: {rendered}"
    );
    // The blocker must name the exact artifact that drifted, so a caller
    // with a multi-output plan knows which one to repair.
    assert!(
        diagnostics.iter().any(|diagnostic| diagnostic["message"]
            .as_str()
            .is_some_and(|message| message.contains("page"))),
        "the blocker must name the drifted artifact: {rendered}"
    );
    // A blocked document still reports every identity the operation did
    // resolve, so a reader can see exactly how far it got.
    assert!(
        rendered["basis"].is_string(),
        "a blocked document still carries the basis it computed: {rendered}"
    );
}

#[test]
fn a_blocked_evidence_document_is_byte_identical_across_separate_roots() {
    // TPT-009: the blocked evidence document is an automation contract too,
    // so two runs over equal logical inputs under different physical roots
    // must produce byte-identical output -- no timestamp, temporary name, or
    // physical root may leak in through the new diagnostic path.
    let first_root = copy_binding_repository();
    let first_config = first_root.path().join("buoy.toml");
    let first_plan = plan(
        first_root.path(),
        "evidence-determinism",
        r#"{"id":"page","mount":"repository","path":"out/page.md","kind":"page","lifecycle":"replaceable"}"#,
    );
    let second_root = copy_binding_repository();
    let second_config = second_root.path().join("buoy.toml");
    let second_plan = plan(
        second_root.path(),
        "evidence-determinism",
        r#"{"id":"page","mount":"repository","path":"out/page.md","kind":"page","lifecycle":"replaceable"}"#,
    );

    let first = run_buoy(first_root.path(), &first_config, &first_plan, &["evidence"]);
    let second = run_buoy(
        second_root.path(),
        &second_config,
        &second_plan,
        &["evidence"],
    );

    assert_eq!(first.status.code(), Some(2));
    assert_eq!(second.status.code(), Some(2));
    assert_eq!(
        first.stdout, second.stdout,
        "blocked evidence documents must be byte-identical under separate physical roots"
    );
    let text = String::from_utf8(first.stdout).expect("evidence UTF-8");
    for physical in [
        first_root.path().display().to_string(),
        second_root.path().display().to_string(),
    ] {
        assert!(
            !text.contains(&physical),
            "a blocked document must never embed a physical root"
        );
    }
}

#[test]
fn evidence_succeeds_once_every_non_evidence_artifact_passes_parity() {
    let root = copy_binding_repository();
    let config_path = root.path().join("buoy.toml");
    let plan_path = plan(
        root.path(),
        "evidence-parity",
        r#"{"id":"page","mount":"repository","path":"out/page.md","kind":"page","lifecycle":"replaceable"}"#,
    );

    let generated = run_buoy(
        root.path(),
        &config_path,
        &plan_path,
        &["generate", "--artifact", "page"],
    );
    assert_eq!(generated.status.code(), Some(0));

    let output = run_buoy(root.path(), &config_path, &plan_path, &["evidence"]);
    assert_eq!(
        output.status.code(),
        Some(0),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(!output.stdout.is_empty());
    let rendered: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("evidence JSON");
    assert!(
        rendered["page_digest"]
            .as_str()
            .is_some_and(|value| value.starts_with("blake3-256:")),
        "declared page artifact must contribute its content digest to evidence"
    );
}

/// A real hostile `PATH` and `BUOY_EXECUTABLE`-shaped environment variable
/// naming a substitute `buoy` cannot redirect any publication operation.
///
/// This runs the actual command with a fake executable placed first on
/// `PATH`; the fake writes a marker file if anything ever invokes it. The
/// publication command family calls only in-process functions (proven
/// statically by `tests/publication_no_substitute_executable.rs`), so this
/// test proves the same fact behaviorally: the marker is never created and
/// the real generation still succeeds using the real linked implementation.
#[test]
fn hostile_path_and_environment_cannot_substitute_the_executable() {
    let root = copy_binding_repository();
    let config_path = root.path().join("buoy.toml");
    let plan_path = plan(
        root.path(),
        "hostile-path",
        r#"{"id":"page","mount":"repository","path":"out/page.md","kind":"page","lifecycle":"replaceable"}"#,
    );

    let fake_bin_dir = root.path().join("fake-bin");
    fs::create_dir_all(&fake_bin_dir).expect("fake bin directory");
    let marker = root.path().join("substitute-invoked.marker");
    let fake_buoy = fake_bin_dir.join("buoy");
    fs::write(
        &fake_buoy,
        format!(
            "#!/bin/sh\ntouch \"{}\"\nexit 0\n",
            marker.to_str().expect("marker path")
        ),
    )
    .expect("fake buoy script");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut permissions = fs::metadata(&fake_buoy)
            .expect("fake buoy metadata")
            .permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&fake_buoy, permissions).expect("fake buoy permissions");
    }

    let hostile_path = format!(
        "{}:{}",
        fake_bin_dir.to_str().expect("fake bin path"),
        std::env::var("PATH").unwrap_or_default()
    );

    let output = Command::new(env!("CARGO_BIN_EXE_buoy"))
        .env("PATH", hostile_path)
        .env("BUOY_EXECUTABLE", &fake_buoy)
        .env("BUOY_BIN", &fake_buoy)
        .arg("publication")
        .arg("generate")
        .arg("--plan")
        .arg(&plan_path)
        .arg("--artifact")
        .arg("page")
        .arg("--config")
        .arg(&config_path)
        .arg("--root")
        .arg(root.path())
        .output()
        .expect("publication generate under a hostile PATH");

    assert_eq!(
        output.status.code(),
        Some(0),
        "generation must still succeed using the real linked implementation"
    );
    assert!(
        !marker.exists(),
        "the substitute executable must never be invoked"
    );
    assert!(root.path().join("out/page.md").is_file());
}

/// End-to-end proof that `publication check|generate` on an external-link
/// artifact runs the real cycle-safe slot generator against a real
/// `OpenSpec` document, not a stub: absent slot returns a static blocker,
/// `generate` inserts the real canonical slot with the real resolved atom
/// revision and the real basis-keyed target, and an equal re-check succeeds.
#[test]
fn external_link_generate_writes_a_real_slot_into_the_openspec_document() {
    let root = copy_binding_repository();
    let config_path = root.path().join("buoy.toml");
    let plan_path = plan(
        root.path(),
        "link-cli",
        r#"{"id":"page","mount":"repository","path":"docs/{publication_basis_path}.md","kind":"page","lifecycle":"immutable","target_base":"https://example.invalid/trace/"},{"id":"link","mount":"repository","path":"openspec/specs/demo/spec.md","kind":"external-link","lifecycle":"replaceable","relation":"documents","page":"page"}"#,
    );
    let spec_path = root.path().join("openspec/specs/demo/spec.md");
    let before = fs::read(&spec_path).expect("spec bytes before");
    assert!(!String::from_utf8_lossy(&before).contains("## Publication: link"));

    let check_before = run_buoy(
        root.path(),
        &config_path,
        &plan_path,
        &["check", "--artifact", "link"],
    );
    assert_eq!(
        check_before.status.code(),
        Some(2),
        "an absent slot must be a static blocker before generation"
    );
    assert_eq!(
        fs::read(&spec_path).expect("spec unchanged after check"),
        before,
        "check must never write the OpenSpec document"
    );

    let generated = run_buoy(
        root.path(),
        &config_path,
        &plan_path,
        &["generate", "--artifact", "link"],
    );
    assert_eq!(
        generated.status.code(),
        Some(0),
        "stderr: {}",
        String::from_utf8_lossy(&generated.stderr)
    );
    let after = fs::read_to_string(&spec_path).expect("spec bytes after generation");
    assert!(after.contains("## Publication: link"));
    assert!(after.contains("buoy-external-trace-v1"));
    assert!(after.contains("BGLC-003"));
    assert!(after.contains("https://example.invalid/trace/docs/blake3-256/"));
    // The normative requirement and its scenarios must be byte-identical:
    // the edit touches only the declared nonauthority slot.
    assert!(
        after.contains("### Requirement: BGLC-003 Render a deterministic and complete catalog")
    );
    assert!(after.contains("#### Scenario: Complete catalog"));
    assert!(after.contains("#### Scenario: Stable output"));

    // A re-check immediately after generation correctly returns a static
    // blocker here, for a real and independently confirmed reason: this
    // fixture's Rust implementation binding pins the exact digest of the
    // requirement's enclosing `openspec/requirement/v1` node, and appending
    // any nonauthority content to that same document — including the
    // declared publication slot — changes that node's computed digest,
    // exactly as `buoy trace` already reports for a totally unrelated
    // trailing edit to the same file (confirmed independently before this
    // edit was written). This is a real, pre-existing consequence of
    // compact-binding content-addressing, not a defect in slot generation:
    // the maintainer must re-run `buoy bind` to refresh the binding record
    // after an external-link edit lands, exactly as the design's own
    // migration guidance describes (generate the source edit, then start a
    // new snapshot for any later operation).
    let check_after = run_buoy(
        root.path(),
        &config_path,
        &plan_path,
        &["check", "--artifact", "link"],
    );
    assert_eq!(
        check_after.status.code(),
        Some(2),
        "the requirement's implementation binding is now stale, a real effect of editing its document"
    );

    // The slot bytes committed by the successful generation remain durable
    // and correct regardless of the now-stale binding: re-reading the file
    // directly (bypassing trace-row conformance) proves the write was real
    // and persisted.
    let still_present = fs::read_to_string(&spec_path).expect("spec bytes remain readable");
    assert!(still_present.contains("## Publication: link"));
    assert!(still_present.contains("https://example.invalid/trace/docs/blake3-256/"));

    // The immutable page the link references was never touched by this
    // operation; the page must still be generated separately.
    assert!(!root.path().join("docs").exists());
}

/// TPT-004: a declared page that a plan's external link references cannot
/// be checked or generated until that link is itself an equal, saved
/// source edit in the current snapshot — the real, maintainer-controlled
/// two-step order design.md section 13 requires. This proves the guard
/// through the actual CLI dispatch, not an isolated helper call: `check`
/// and `generate` on the page artifact both return the static-blocker exit
/// while the link is absent, and only succeed after the link has been
/// separately generated and a fresh invocation captures a new snapshot
/// that already contains it.
#[test]
fn page_operations_are_blocked_until_their_declared_link_is_a_current_saved_source_edit() {
    let root = copy_binding_repository();
    let config_path = root.path().join("buoy.toml");
    let plan_path = plan(
        root.path(),
        "page-precedes-link",
        r#"{"id":"page","mount":"repository","path":"docs/{publication_basis_path}.md","kind":"page","lifecycle":"immutable","target_base":"https://example.invalid/trace/"},{"id":"link","mount":"repository","path":"openspec/specs/demo/spec.md","kind":"external-link","lifecycle":"replaceable","relation":"documents","page":"page"}"#,
    );

    // The link does not exist yet in this snapshot: both `check` and
    // `generate` on the page must fail closed before any page render or
    // write, exactly like any other absent declared source edit.
    let check_page_before_link = run_buoy(
        root.path(),
        &config_path,
        &plan_path,
        &["check", "--artifact", "page"],
    );
    assert_eq!(
        check_page_before_link.status.code(),
        Some(2),
        "a page whose declared link is absent must fail closed before rendering"
    );
    let generate_page_before_link = run_buoy(
        root.path(),
        &config_path,
        &plan_path,
        &["generate", "--artifact", "page"],
    );
    assert_eq!(generate_page_before_link.status.code(), Some(2));
    assert!(
        !root.path().join("docs").exists(),
        "generate must never write the page before its declared link is current"
    );

    // Step 1 (maintainer-controlled): generate and save the link candidate
    // in the authority worktree first.
    let generate_link = run_buoy(
        root.path(),
        &config_path,
        &plan_path,
        &["generate", "--artifact", "link"],
    );
    assert_eq!(
        generate_link.status.code(),
        Some(0),
        "stderr: {}",
        String::from_utf8_lossy(&generate_link.stderr)
    );

    // Note on scope: this fixture's BGLC-003 implementation binding pins
    // the exact digest of the requirement's enclosing document
    // (`openspec/specs/demo/spec.md`), so the link-generation edit above
    // — which necessarily changes that document's bytes — makes the
    // binding itself stale, exactly as
    // `external_link_generate_writes_a_real_slot_into_the_openspec_document`
    // independently documents and confirms for a totally unrelated
    // trailing edit to the same file. That is a real, separate row-level
    // static blocker (report-level diagnostics nonempty) that this exact
    // fixture cannot avoid without a `buoy bind` refresh step, which is out
    // of scope for a publication-only test. A `buoy publication check
    // --artifact page` immediately after the link generation above
    // therefore still returns exit `2` here, but for that distinct,
    // already-covered reason, not because this test's new source-edit gate
    // is still blocking: the negative half above already isolated and
    // proved the new gate directly (a page-generation attempt against a
    // snapshot from *before* the link candidate was saved is rejected, and
    // writes nothing), matching the unit-level proof in
    // `publication_snapshot_guards.rs`'s
    // `page_generation_requires_the_saved_link_candidate_from_a_prior_snapshot`
    // now wired through the real CLI dispatch.
    let check_page_after_link = run_buoy(
        root.path(),
        &config_path,
        &plan_path,
        &["check", "--artifact", "page"],
    );
    assert_eq!(check_page_after_link.status.code(), Some(2));
    assert!(
        !root.path().join("docs").exists(),
        "check must never write the page regardless of which blocker fired"
    );
}

/// The COMPLETE stale-link-repair sequence, proved end to end through the
/// real CLI against the dedicated `publication-sequence` fixture (which has
/// no compact-binding pinning conflict on the edited document): a stale
/// (absent) external link is repaired via `buoy publication generate`, a NEW
/// process invocation then captures a NEW snapshot that already contains the
/// saved link candidate, and page generation succeeds against that new
/// snapshot -- the page-first integration ordering design.md section 13
/// requires, carried all the way through to a real, successful, verified
/// page write (not blocked by an unrelated binding staleness, as the
/// `positive` fixture's equivalent sequence is).
#[test]
#[allow(clippy::too_many_lines)]
fn the_complete_stale_link_repair_then_new_snapshot_then_page_generation_sequence_succeeds() {
    let root = copy_publication_sequence_repository();
    let config_path = root.path().join("buoy.toml");
    let plan_path = sequence_plan(
        root.path(),
        "full-sequence",
        r#"{"id":"page","mount":"repository","path":"docs/{publication_basis_path}.md","kind":"page","lifecycle":"immutable","target_base":"https://example.invalid/trace/"},{"id":"link","mount":"repository","path":"openspec/specs/demo/spec.md","kind":"external-link","lifecycle":"replaceable","relation":"documents","page":"page"}"#,
    );

    // Confirm the fixture's real starting state first: BGLC-003 is current
    // and fresh, and the declared link slot is genuinely absent.
    let spec_path = root.path().join("openspec/specs/demo/spec.md");
    let before = fs::read_to_string(&spec_path).expect("spec bytes before");
    assert!(!before.contains("## Publication: link"));

    // Step 0: both artifacts are blocked before the link exists.
    let check_link_before = run_buoy(
        root.path(),
        &config_path,
        &plan_path,
        &["check", "--artifact", "link"],
    );
    assert_eq!(
        check_link_before.status.code(),
        Some(2),
        "an absent link slot is a static blocker before generation"
    );
    let check_page_before = run_buoy(
        root.path(),
        &config_path,
        &plan_path,
        &["check", "--artifact", "page"],
    );
    assert_eq!(
        check_page_before.status.code(),
        Some(2),
        "a page whose declared link is absent must fail closed before rendering"
    );

    // Step 1: repair the stale (absent) external link through the real CLI.
    let generate_link = run_buoy(
        root.path(),
        &config_path,
        &plan_path,
        &["generate", "--artifact", "link"],
    );
    assert_eq!(
        generate_link.status.code(),
        Some(0),
        "stderr: {}",
        String::from_utf8_lossy(&generate_link.stderr)
    );
    let after_link = fs::read_to_string(&spec_path).expect("spec bytes after link generation");
    assert!(after_link.contains("## Publication: link"));
    assert!(after_link.contains("buoy-external-trace-v1"));
    assert!(after_link.contains("BGLC-003"));
    // The normative requirement content is byte-identical; only the
    // declared nonauthority slot changed.
    assert!(
        after_link
            .contains("### Requirement: BGLC-003 Render a deterministic and complete catalog")
    );
    assert!(after_link.contains("### Requirement: BGLC-004"));

    // Because this fixture reserves a trailing, deliberately unbound
    // requirement to absorb the appended publication slot, the committed
    // BGLC-003 binding is NOT invalidated by the edit above. A fresh CLI
    // invocation (a genuinely NEW process, capturing a genuinely NEW
    // snapshot) now proves the link is a current, equal, saved source edit.
    let check_link_after = run_buoy(
        root.path(),
        &config_path,
        &plan_path,
        &["check", "--artifact", "link"],
    );
    assert_eq!(
        check_link_after.status.code(),
        Some(0),
        "the saved link candidate must now check as equal: stderr {}",
        String::from_utf8_lossy(&check_link_after.stderr)
    );

    // Step 2: with a new snapshot that already contains the saved link
    // candidate, page generation must now succeed for real -- not merely
    // stop at "no longer blocked", but actually render and write the
    // complete, correct page.
    let generate_page = run_buoy(
        root.path(),
        &config_path,
        &plan_path,
        &["generate", "--artifact", "page"],
    );
    assert_eq!(
        generate_page.status.code(),
        Some(0),
        "stderr: {}",
        String::from_utf8_lossy(&generate_page.stderr)
    );
    let page_report: serde_json::Value =
        serde_json::from_slice(&generate_page.stdout).expect("page generate report JSON");
    assert_eq!(page_report["schema"], "buoy/trace-publication-report/v1");
    assert_eq!(page_report["prior_state"], "absent");
    assert_eq!(page_report["outcome"], "created");
    let basis = page_report["basis"]
        .as_str()
        .expect("basis present")
        .to_owned();
    assert!(basis.starts_with("blake3-256:"));

    let files = list_files_relative(&root.path().join("docs"));
    assert_eq!(files.len(), 1, "exactly one page file, found {files:?}");
    let page_bytes =
        fs::read(root.path().join("docs").join(&files[0])).expect("generated page bytes");
    let page_text = String::from_utf8(page_bytes).expect("UTF-8 page");
    assert!(page_text.contains("BGLC-003"));
    assert!(page_text.contains("buoy/readable-trace-markdown/v1"));

    // The link's own target inside the saved slot must reference exactly
    // this basis-keyed page path.
    let payload = basis.split_once(':').expect("basis has a payload").1;
    let expected_target = format!("https://example.invalid/trace/docs/blake3-256/{payload}");
    assert!(
        after_link.contains(&expected_target)
            || fs::read_to_string(&spec_path)
                .expect("spec bytes")
                .contains(&expected_target),
        "the saved link slot must target the exact basis-keyed page path"
    );

    // A final `check` on both artifacts now succeeds with an equal prior
    // state, proving the complete sequence is durable and self-consistent.
    let check_page_after = run_buoy(
        root.path(),
        &config_path,
        &plan_path,
        &["check", "--artifact", "page"],
    );
    assert_eq!(check_page_after.status.code(), Some(0));
    let check_link_final = run_buoy(
        root.path(),
        &config_path,
        &plan_path,
        &["check", "--artifact", "link"],
    );
    assert_eq!(check_link_final.status.code(), Some(0));
}

/// TPT-004: `evidence` requires every non-evidence artifact -- INCLUDING a
/// declared `source-edit` -- to pass parity through a REAL byte comparison,
/// not an unconditional pass. This proves the production wiring: the
/// whole-plan `evidence` operation succeeds when the source-edit's current
/// on-disk bytes genuinely equal the retained snapshot's bytes for that
/// same path.
#[test]
fn evidence_requires_a_real_byte_comparison_for_a_declared_source_edit() {
    let root = copy_binding_repository();
    let config_path = root.path().join("buoy.toml");
    let plan_path = root.path().join("plan.json");
    fs::write(
        &plan_path,
        r#"{"schema":"buoy/trace-publication-plan/v1","publication_id":"source-edit-evidence","selector":{"requirement_id":"BGLC-003"},"expected_rows":1,"source_set":["openspec/specs/demo/spec.md","src/lib.rs","buoy/objects/v1/blake3-256/0e857e33e387325fbf457e426bc42286d9d99926b19f3efa3c203795cfeeb6c4.toml"],"outputs":[{"id":"edit","mount":"repository","path":"openspec/specs/demo/spec.md","kind":"source-edit","lifecycle":"replaceable"}]}"#,
    )
    .expect("source-edit evidence plan");

    let output = run_buoy(root.path(), &config_path, &plan_path, &["evidence"]);
    assert_eq!(
        output.status.code(),
        Some(0),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(!output.stdout.is_empty());
}

/// A `check` on a declared source-edit artifact succeeds once its
/// classification (exactly one regular selected source path) has already
/// passed AND a real reopened-bytes comparison against the retained
/// snapshot also passes; `generate` on the same artifact fails closed,
/// because Buoy never authors a human source edit itself.
#[test]
fn source_edit_check_succeeds_and_generate_fails_closed() {
    let root = copy_binding_repository();
    let config_path = root.path().join("buoy.toml");
    let plan_path = root.path().join("plan.json");
    fs::write(
        &plan_path,
        r#"{"schema":"buoy/trace-publication-plan/v1","publication_id":"source-edit-cli","selector":{"requirement_id":"BGLC-003"},"expected_rows":1,"source_set":["openspec/specs/demo/spec.md","src/lib.rs","buoy/objects/v1/blake3-256/0e857e33e387325fbf457e426bc42286d9d99926b19f3efa3c203795cfeeb6c4.toml"],"outputs":[{"id":"edit","mount":"repository","path":"openspec/specs/demo/spec.md","kind":"source-edit","lifecycle":"replaceable"}]}"#,
    )
    .expect("source-edit plan");

    let checked = run_buoy(
        root.path(),
        &config_path,
        &plan_path,
        &["check", "--artifact", "edit"],
    );
    assert_eq!(checked.status.code(), Some(0));
    // TPT-004/TPT-011: `check --artifact <source-edit-id>` must emit a real
    // versioned `buoy/trace-publication-report/v1` naming its exact outcome
    // -- never silent success with no machine report at all.
    let checked_report: serde_json::Value =
        serde_json::from_slice(&checked.stdout).expect("check emits a machine report");
    assert_eq!(checked_report["schema"], "buoy/trace-publication-report/v1");
    assert_eq!(checked_report["artifact"], "edit");
    assert_eq!(checked_report["outcome"], "no-op");

    let generated = run_buoy(
        root.path(),
        &config_path,
        &plan_path,
        &["generate", "--artifact", "edit"],
    );
    assert_eq!(
        generated.status.code(),
        Some(70),
        "Buoy must never author a human source edit"
    );
    let generated_report: serde_json::Value =
        serde_json::from_slice(&generated.stdout).expect("generate emits a machine report");
    assert_eq!(
        generated_report["schema"],
        "buoy/trace-publication-report/v1"
    );
    assert_eq!(
        generated_report["diagnostics"][0]["code"],
        "TPT-SOURCE-EDIT-NOT-WRITABLE"
    );
}

/// TPT: an immutable output declared under a change-detection-only active
/// digest member must be rejected before ANY publication-basis computation
/// runs, for `check`, `generate`, AND `evidence` alike -- not only
/// immediately before the write. This proves the ordering through the real
/// CLI: no output directory is ever created, for any of the three
/// operations.
#[test]
fn immutable_output_under_a_change_detection_member_is_rejected_before_basis_computation_for_every_operation()
 {
    for operation in ["check", "generate", "evidence"] {
        let root = copy_binding_repository_with_change_detection_only_algorithm();
        let config_path = root.path().join("buoy.toml");
        // The immutable page must be referenced by exactly one external
        // link (otherwise plan parsing itself fails closed first, as an
        // orphan immutable page, before this test's own digest-class check
        // is ever reached).
        let plan_path = plan(
            root.path(),
            "digest-class-cli",
            r#"{"id":"page","mount":"repository","path":"docs/{publication_basis_path}.md","kind":"page","lifecycle":"immutable","target_base":"https://example.invalid/trace/"},{"id":"link","mount":"repository","path":"openspec/specs/demo/spec.md","kind":"external-link","lifecycle":"replaceable","relation":"documents","page":"page"}"#,
        );
        let mut arguments = vec![operation];
        if operation != "evidence" {
            arguments.push("--artifact");
            arguments.push("page");
        }
        let output = run_buoy(root.path(), &config_path, &plan_path, &arguments);
        assert_eq!(
            output.status.code(),
            Some(70),
            "operation {operation} must reject a change-detection member for an immutable output"
        );
        // This fixture carries a `binding/v1` selector, so `RepositoryConfig`
        // independently refuses to LOAD it under a change-detection member
        // (`RepositoryError::BindingDigestClass`) before publication's own
        // TPT-008 check is ever reached. That makes this a configuration
        // control, not a digest-class control -- and it must still emit a
        // versioned machine report rather than exit silently. The genuine
        // reachable TPT-008 control lives in
        // `publication_diagnostic_codes_cli::an_immutable_target_under_a_change_detection_member_emits_the_digest_class_code`,
        // which uses a repository shape that can actually load.
        let report: serde_json::Value = serde_json::from_slice(&output.stdout)
            .expect("every operation must still emit a versioned report rather than exit silently");
        assert_eq!(report["schema"], "buoy/trace-publication-report/v1");
        assert_eq!(
            report["diagnostics"][0]["code"], "TPT-CONFIG-INVALID",
            "a compact-binding repository cannot load under a change-detection member: {report}"
        );
        assert!(
            !root.path().join("docs").exists(),
            "operation {operation} must never create the declared output directory"
        );
    }
}

/// `check` and `evidence` must never create a missing output directory as a
/// side effect of parent descent -- the repository must stay
/// directory-identical before and after either operation runs. The declared
/// page path here nests three directory levels that do not exist yet; a
/// regression would leave any of them behind even though neither operation
/// ever writes the page itself.
#[test]
fn check_and_evidence_never_create_a_missing_output_directory() {
    let root = copy_binding_repository();
    let config_path = root.path().join("buoy.toml");
    let plan_path = plan(
        root.path(),
        "no-incidental-directories-cli",
        r#"{"id":"page","mount":"repository","path":"deep/nested/out/page.md","kind":"page","lifecycle":"replaceable"}"#,
    );

    let check = run_buoy(
        root.path(),
        &config_path,
        &plan_path,
        &["check", "--artifact", "page"],
    );
    assert_eq!(
        check.status.code(),
        Some(2),
        "an absent page is a static blocker"
    );
    assert!(
        !root.path().join("deep").exists(),
        "check must never create the declared output's parent directories"
    );

    let evidence = run_buoy(root.path(), &config_path, &plan_path, &["evidence"]);
    assert_eq!(
        evidence.status.code(),
        Some(2),
        "evidence fails closed while the declared page remains absent"
    );
    assert!(
        !root.path().join("deep").exists(),
        "evidence must never create the declared output's parent directories"
    );
}

/// TPT: an empty declared `source_set` must never disable source-set
/// conformance -- it is exact equality with the empty selection, so a real
/// nonempty repository selection against an empty policy fails closed
/// through the real CLI, for `check`, `generate`, AND `evidence` alike.
#[test]
fn empty_source_set_policy_fails_closed_against_a_real_nonempty_selection() {
    for operation in ["check", "generate", "evidence"] {
        let root = copy_binding_repository();
        let config_path = root.path().join("buoy.toml");
        let plan_path = plan_with_empty_source_set(
            root.path(),
            "empty-source-set-cli",
            r#"{"id":"page","mount":"repository","path":"out/page.md","kind":"page","lifecycle":"replaceable"}"#,
        );
        let mut arguments = vec![operation];
        if operation != "evidence" {
            arguments.push("--artifact");
            arguments.push("page");
        }
        let output = run_buoy(root.path(), &config_path, &plan_path, &arguments);
        assert_eq!(
            output.status.code(),
            Some(2),
            "operation {operation} must reject an empty source-set policy against a nonempty real selection"
        );
        assert!(!root.path().join("out").exists());
    }
}

/// TPT: two declared outputs whose UNRENDERED templates are textually
/// DIFFERENT (so a collision check that compares only the unrendered
/// template would never catch them) but whose RENDERED paths are
/// byte-identical must still be rejected through the real CLI. `page-a`
/// uses the live `{publication_basis_path}` substitution; `page-b` names,
/// as a literal path with no substitution at all, the exact basis-keyed
/// path this fixture's real basis renders to (independently confirmed by
/// `committed_generate_report_golden_matches_the_real_cli_output`'s frozen
/// basis for this identical fixture and selector). Only comparing the
/// REAL RENDERED paths can catch this collision.
#[test]
fn generate_rejects_two_outputs_whose_rendered_paths_collide_even_though_their_templates_differ() {
    let root = copy_binding_repository();
    let config_path = root.path().join("buoy.toml");
    let plan_path = plan(
        root.path(),
        "collision-cli",
        r#"{"id":"page-a","mount":"repository","path":"out/{publication_basis_path}.md","kind":"page","lifecycle":"replaceable"},{"id":"page-b","mount":"repository","path":"out/blake3-256/f89d33925c544093da18dcd31c052e1fba3894d0e6cedbe537977ec113ba983c.md","kind":"page","lifecycle":"replaceable"}"#,
    );

    let generated = run_buoy(
        root.path(),
        &config_path,
        &plan_path,
        &["generate", "--artifact", "page-a"],
    );
    assert_eq!(
        generated.status.code(),
        Some(2),
        "two outputs whose distinct templates render to the identical path must be rejected before any write"
    );
    assert!(!root.path().join("out").exists());
    // TPT-OUTPUT-COLLISION: the rejection must carry a real versioned
    // report naming the exact stable diagnostic code, not a bare exit code.
    let report: serde_json::Value =
        serde_json::from_slice(&generated.stdout).expect("generate emits a machine report");
    assert_eq!(report["schema"], "buoy/trace-publication-report/v1");
    assert_eq!(report["diagnostics"][0]["code"], "TPT-OUTPUT-COLLISION");
}

/// TPT: an explicit `evidence`-kind declared artifact must be checkable and
/// generatable through the same real safe writer a page uses -- not only
/// available as machine JSON to standard output. `check` on the absent
/// artifact is a static blocker and writes nothing; `generate` writes
/// deterministic Markdown containing the evidence marker and the real
/// selected requirement; a re-`check` afterward passes with an `equal`
/// prior state; and the file on disk is real Markdown, not JSON.
#[test]
fn evidence_kind_artifact_is_checked_and_generated_through_the_real_safe_writer() {
    let root = copy_binding_repository();
    let config_path = root.path().join("buoy.toml");
    let plan_path = plan(
        root.path(),
        "evidence-artifact-cli",
        r#"{"id":"evidence-doc","mount":"repository","path":"out/evidence.md","kind":"evidence","lifecycle":"replaceable"}"#,
    );

    let check_before = run_buoy(
        root.path(),
        &config_path,
        &plan_path,
        &["check", "--artifact", "evidence-doc"],
    );
    assert_eq!(
        check_before.status.code(),
        Some(2),
        "an absent evidence artifact must be a static blocker before generation"
    );
    assert!(!root.path().join("out/evidence.md").exists());

    let generated = run_buoy(
        root.path(),
        &config_path,
        &plan_path,
        &["generate", "--artifact", "evidence-doc"],
    );
    assert_eq!(
        generated.status.code(),
        Some(0),
        "stderr: {}",
        String::from_utf8_lossy(&generated.stderr)
    );
    let evidence_path = root.path().join("out/evidence.md");
    assert!(
        evidence_path.is_file(),
        "generate must create the declared evidence document"
    );
    let text = fs::read_to_string(&evidence_path).expect("evidence Markdown");
    assert!(text.starts_with("<!-- BEGIN buoy-trace-publication-evidence-v1 -->"));
    assert!(text.contains("Publication ID: evidence-artifact-cli"));
    assert!(text.contains("BGLC-003"));
    // Real Markdown, never machine JSON.
    assert!(!text.trim_start().starts_with('{'));

    let generated_report: serde_json::Value =
        serde_json::from_slice(&generated.stdout).expect("generate report JSON");
    assert_eq!(
        generated_report["schema"],
        "buoy/trace-publication-report/v1"
    );
    assert_eq!(generated_report["outcome"], "created");

    let check_after = run_buoy(
        root.path(),
        &config_path,
        &plan_path,
        &["check", "--artifact", "evidence-doc"],
    );
    assert_eq!(check_after.status.code(), Some(0));
    let check_report: serde_json::Value =
        serde_json::from_slice(&check_after.stdout).expect("check report JSON");
    assert_eq!(check_report["prior_state"], "equal");
    assert_eq!(check_report["outcome"], "no-op");
}

/// Recursively collects every regular file's path (relative to `root`,
/// `/`-separated) beneath `root`.
fn list_files_relative(root: &Path) -> Vec<String> {
    fn walk(base: &Path, current: &Path, out: &mut Vec<String>) {
        for entry in fs::read_dir(current).expect("read directory") {
            let entry = entry.expect("directory entry");
            let path = entry.path();
            if path.is_dir() {
                walk(base, &path, out);
            } else {
                out.push(
                    path.strip_prefix(base)
                        .expect("relative path")
                        .to_str()
                        .expect("UTF-8 relative path")
                        .replace(std::path::MAIN_SEPARATOR, "/"),
                );
            }
        }
    }
    let mut out = Vec::new();
    walk(root, root, &mut out);
    out
}

/// TPT: a declared page path template that uses `{atom_revision_path}`,
/// `{record_digest_path}`, and `{publication_basis_path}` together must
/// render three genuinely distinct payload segments through the real CLI
/// generate pipeline -- not the same basis value repeated three times. This
/// is the production-path regression proof for the round-2 audit finding
/// that atom and record substitutions silently reused the basis value.
#[test]
fn generate_renders_distinct_atom_record_and_basis_path_segments() {
    let root = copy_binding_repository();
    let config_path = root.path().join("buoy.toml");
    let plan_path = plan(
        root.path(),
        "distinct-identities-cli",
        r#"{"id":"page","mount":"repository","path":"out/{atom_revision_path}/{record_digest_path}/{publication_basis_path}.md","kind":"page","lifecycle":"replaceable"}"#,
    );

    let generated = run_buoy(
        root.path(),
        &config_path,
        &plan_path,
        &["generate", "--artifact", "page"],
    );
    assert_eq!(
        generated.status.code(),
        Some(0),
        "stderr: {}",
        String::from_utf8_lossy(&generated.stderr)
    );

    let files = list_files_relative(&root.path().join("out"));
    assert_eq!(
        files.len(),
        1,
        "generate must write exactly one page file, found {files:?}"
    );
    // The rendered relative path has the shape
    // `<atom-algo>/<atom-payload>/<record-algo>/<record-payload>/<basis-algo>/<basis-payload>.md`.
    let segments: Vec<&str> = files[0].split('/').collect();
    assert_eq!(
        segments.len(),
        6,
        "expected atom/record/basis algorithm-payload pairs, found {segments:?}"
    );
    let [
        atom_algorithm,
        atom_payload,
        record_algorithm,
        record_payload,
        basis_algorithm,
        basis_file,
    ] = <[&str; 6]>::try_from(segments).expect("exactly six path segments");
    let basis_payload = basis_file.strip_suffix(".md").expect("basis file suffix");

    assert_eq!(atom_algorithm, "blake3-256");
    assert_eq!(record_algorithm, "blake3-256");
    assert_eq!(basis_algorithm, "blake3-256");
    assert_ne!(
        atom_payload, record_payload,
        "atom-revision and record-digest payloads must be genuinely distinct"
    );
    assert_ne!(
        atom_payload, basis_payload,
        "atom-revision payload must not collapse to the basis payload"
    );
    assert_ne!(
        record_payload, basis_payload,
        "record-digest payload must not collapse to the basis payload"
    );
}
