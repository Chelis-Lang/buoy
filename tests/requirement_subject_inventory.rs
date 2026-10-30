#![doc = "CAB-014 controls for the current requirement-subject static inventory."]
#![allow(clippy::expect_used)]

use buoy::check::RequirementSubjectFact;
use hegel::generators as gs;
use hegel::{Hegel, TestCase};
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use tempfile::TempDir;

#[path = "support/hegel_settings.rs"]
mod hegel_settings;
use hegel_settings::settings;

const FIXTURE: &str = "fixtures/requirement-subjects/v1";

fn fixture_path(relative: &str) -> PathBuf {
    Path::new(FIXTURE).join(relative)
}

fn stage_repository(reverse_creation_order: bool) -> TempDir {
    let root = TempDir::new().expect("requirement-subject repository");
    let specification = root.path().join("openspec/specs/demo/spec.md");
    let change = root
        .path()
        .join("openspec/changes/ignored/specs/demo/spec.md");
    let artifact = root.path().join("artifact.bin");
    let binding_root = root.path().join("buoy");

    fs::create_dir_all(specification.parent().expect("specification parent"))
        .expect("specification directory");
    fs::create_dir_all(change.parent().expect("change parent")).expect("change directory");
    fs::create_dir_all(&binding_root).expect("binding directory");

    let writes = if reverse_creation_order {
        vec![
            (
                change,
                fixture_path("repository/openspec/changes/ignored/specs/demo/spec.md"),
            ),
            (artifact, fixture_path("repository/artifact.bin")),
            (
                specification,
                fixture_path("repository/openspec/specs/demo/spec.md"),
            ),
        ]
    } else {
        vec![
            (
                specification,
                fixture_path("repository/openspec/specs/demo/spec.md"),
            ),
            (artifact, fixture_path("repository/artifact.bin")),
            (
                change,
                fixture_path("repository/openspec/changes/ignored/specs/demo/spec.md"),
            ),
        ]
    };
    for (target, source) in writes {
        fs::copy(source, target).expect("fixture copy");
    }
    root
}

fn run_mounted(root: &Path, command: &str, format: &str) -> Output {
    Command::new(env!("CARGO_BIN_EXE_buoy"))
        .args([
            command,
            "--mount",
            &format!("repository={}", root.display()),
            "--config",
            fixture_path("buoy.toml")
                .to_str()
                .expect("UTF-8 fixture path"),
            "--format",
            format,
        ])
        .output()
        .expect("Buoy command")
}

fn successful_json(output: &Output) -> Value {
    assert_eq!(
        output.status.code(),
        Some(0),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).expect("machine report JSON")
}

#[test]
fn static_machine_and_human_reports_match_frozen_bytes() {
    let root = fixture_path("repository");
    let machine = run_mounted(&root, "static", "machine");
    assert_eq!(machine.status.code(), Some(0));
    assert_eq!(
        machine.stdout,
        fs::read(fixture_path("expected-machine.json")).expect("machine golden")
    );

    let human = run_mounted(&root, "static", "human");
    assert_eq!(human.status.code(), Some(0));
    assert_eq!(
        human.stdout,
        fs::read(fixture_path("expected-human.txt")).expect("human golden")
    );
}

#[test]
fn requirement_subjects_ignore_creation_order_and_physical_roots() {
    let first = stage_repository(false);
    let second = stage_repository(true);
    let first_output = run_mounted(first.path(), "static", "machine");
    let second_output = run_mounted(second.path(), "static", "machine");
    assert_eq!(first_output.status.code(), Some(0));
    assert_eq!(second_output.status.code(), Some(0));
    assert_eq!(first_output.stdout, second_output.stdout);

    let report = successful_json(&first_output);
    let rows = report["requirement_subjects"]
        .as_array()
        .expect("requirement-subject rows");
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0]["id"], "RQS-001");
    assert_eq!(rows[1]["id"], "RQS-002");
    assert!(
        report
            .to_string()
            .find(first.path().to_str().expect("UTF-8 root"))
            .is_none()
    );
    assert!(
        report
            .to_string()
            .find(second.path().to_str().expect("UTF-8 root"))
            .is_none()
    );
}

#[test]
fn active_change_documents_do_not_enter_the_inventory() {
    let report = successful_json(&run_mounted(
        &fixture_path("repository"),
        "static",
        "machine",
    ));
    let rows = report["requirement_subjects"]
        .as_array()
        .expect("requirement-subject rows");
    assert!(rows.iter().all(|row| row["id"] != "RQS-999"));
    assert!(rows.iter().all(|row| {
        !row["path"]
            .as_str()
            .expect("logical path")
            .contains("openspec/changes")
    }));
}

#[test]
fn a_repository_without_openspec_authority_omits_the_field() {
    let root = stage_repository(false);
    let output = Command::new(env!("CARGO_BIN_EXE_buoy"))
        .args([
            "static",
            "--mount",
            &format!("repository={}", root.path().display()),
            "--config",
            fixture_path("no-authority.toml")
                .to_str()
                .expect("UTF-8 fixture path"),
            "--format",
            "machine",
        ])
        .output()
        .expect("Buoy static command");
    let report = successful_json(&output);
    assert!(report.get("requirement_subjects").is_none());
}

#[test]
fn version_two_root_mode_exposes_current_requirement_subjects() {
    let root = stage_repository(false);
    let output = Command::new(env!("CARGO_BIN_EXE_buoy"))
        .args([
            "static",
            "--root",
            root.path().to_str().expect("UTF-8 root"),
            "--config",
            fixture_path("v2.toml")
                .to_str()
                .expect("UTF-8 fixture path"),
            "--format",
            "machine",
        ])
        .output()
        .expect("Buoy static command");
    let report = successful_json(&output);
    let rows = report["requirement_subjects"]
        .as_array()
        .expect("requirement-subject rows");
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0]["id"], "RQS-001");
    assert_eq!(rows[0]["path"], "openspec/specs/demo/spec.md");
}

#[test]
fn the_reported_subject_bootstraps_the_first_artifact_binding() {
    let root = stage_repository(false);
    let static_output = run_mounted(root.path(), "static", "machine");
    let report = successful_json(&static_output);
    let subject = report["requirement_subjects"]
        .as_array()
        .expect("requirement-subject rows")
        .iter()
        .find(|row| row["id"] == "RQS-001")
        .and_then(|row| row["subject"].as_str())
        .expect("RQS-001 requirement subject");

    let bind = Command::new(env!("CARGO_BIN_EXE_buoy"))
        .args([
            "bind",
            "--mount",
            &format!("repository={}", root.path().display()),
            "--config",
            fixture_path("buoy.toml")
                .to_str()
                .expect("UTF-8 fixture path"),
            "--requirement",
            subject,
            "--artifact",
            "repository/artifact.bin",
            "--relation",
            "RQS-TO-ARTIFACT",
            "--format",
            "machine",
        ])
        .output()
        .expect("Buoy bind command");
    let bind_report = successful_json(&bind);
    assert_eq!(bind_report["requirement_id"], "RQS-001");
    assert!(
        bind_report["nodes"]
            .as_array()
            .expect("binding tree nodes")
            .iter()
            .any(|node| node["digest"] == subject)
    );
}

#[test]
fn execution_reports_do_not_claim_binding_bootstrap_inventory() {
    let output = run_mounted(&fixture_path("repository"), "execute", "machine");
    let report = successful_json(&output);
    assert!(report.get("requirement_subjects").is_none());
}

fn ordering_facts() -> Vec<RequirementSubjectFact> {
    vec![
        RequirementSubjectFact {
            id: String::from("RQS-002"),
            subject: String::from("blake3-256:44"),
            atom_revision: String::from("blake3-256:55"),
            path: String::from("repository/z/spec.md"),
            scopes: vec![String::from("scope.z")],
        },
        RequirementSubjectFact {
            id: String::from("RQS-001"),
            subject: String::from("blake3-256:33"),
            atom_revision: String::from("blake3-256:66"),
            path: String::from("repository/b/spec.md"),
            scopes: vec![String::from("scope.a")],
        },
        RequirementSubjectFact {
            id: String::from("RQS-001"),
            subject: String::from("blake3-256:11"),
            atom_revision: String::from("blake3-256:77"),
            path: String::from("repository/a/spec.md"),
            scopes: vec![String::from("scope.a"), String::from("scope.b")],
        },
        RequirementSubjectFact {
            id: String::from("RQS-001"),
            subject: String::from("blake3-256:22"),
            atom_revision: String::from("blake3-256:00"),
            path: String::from("repository/a/spec.md"),
            scopes: vec![String::from("scope.a"), String::from("scope.b")],
        },
    ]
}

#[test]
fn hegel_requirement_subject_order_is_permutation_invariant() {
    Hegel::new(|test_case: TestCase| {
        let mut actual = ordering_facts();
        for _ in 0..16 {
            let left = usize::from(test_case.draw(gs::integers::<u8>())) % actual.len();
            let right = usize::from(test_case.draw(gs::integers::<u8>())) % actual.len();
            actual.swap(left, right);
        }
        let mut expected = actual.clone();
        expected.sort_by(|left, right| {
            (
                &left.id,
                &left.scopes,
                &left.path,
                &left.subject,
                &left.atom_revision,
            )
                .cmp(&(
                    &right.id,
                    &right.scopes,
                    &right.path,
                    &right.subject,
                    &right.atom_revision,
                ))
        });
        actual.sort_unstable();
        assert_eq!(actual, expected);
    })
    .settings(settings(916))
    .run();
}
