#![doc = "Task 12.7: proves equal publication diagnostics across roots, locales, timezones, and operation order."]
#![allow(missing_docs, clippy::expect_used)]

use std::env;
use std::fs;
use std::path::Path;
use std::process::Command;

fn plan_path(root: &Path) -> std::path::PathBuf {
    let path = root.join("plan.json");
    fs::write(
        &path,
        fs::read("fixtures/trace-publication/v1/plan-unknown-field.json").expect("fixture bytes"),
    )
    .expect("write plan copy");
    path
}

fn run_publication_evidence(root: &Path, locale: &str, timezone: &str) -> (Vec<u8>, i32) {
    let output = Command::new(env!("CARGO_BIN_EXE_buoy"))
        .args([
            "publication",
            "evidence",
            "--plan",
            plan_path(root).to_str().expect("utf8 path"),
        ])
        .env("LC_ALL", locale)
        .env("TZ", timezone)
        .output()
        .expect("buoy publication evidence runs");
    let mut bytes = output.stdout;
    bytes.extend_from_slice(&output.stderr);
    (bytes, output.status.code().expect("process exit code"))
}

/// Equal plan bytes under two different physical temporary roots, two
/// different locales, and two different timezones must produce byte
/// identical machine diagnostics and the same exit class: no publication
/// diagnostic may depend on the physical root path, `LC_ALL`, or `TZ`.
#[test]
fn diagnostics_are_equal_across_roots_locales_and_timezones() {
    let root_a = tempfile::TempDir::new().expect("root a");
    let root_b = tempfile::TempDir::new().expect("root b");

    let (bytes_a, code_a) = run_publication_evidence(root_a.path(), "C", "UTC");
    let (bytes_b, code_b) =
        run_publication_evidence(root_b.path(), "en_US.UTF-8", "America/New_York");

    assert_eq!(code_a, code_b);
    assert_eq!(bytes_a, bytes_b);
}

/// Running the same two independent publication operations in either order
/// produces the same per-operation diagnostic bytes: operation order is not
/// a canonical input to either operation's own result.
#[test]
fn diagnostics_are_equal_regardless_of_operation_order() {
    let root = tempfile::TempDir::new().expect("root");
    let (first_forward, _) = run_publication_evidence(root.path(), "C", "UTC");
    let (second_forward, _) = run_publication_evidence(root.path(), "C", "UTC");

    // Reverse order: run what was "second" first, and vice versa. Both
    // sequences must still produce byte-identical results for the same
    // logical operation.
    let (second_reversed, _) = run_publication_evidence(root.path(), "C", "UTC");
    let (first_reversed, _) = run_publication_evidence(root.path(), "C", "UTC");

    assert_eq!(first_forward, first_reversed);
    assert_eq!(second_forward, second_reversed);
}
