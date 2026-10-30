#![doc = "Real command-execution contract for `fixtures/trace-publication/v1/manifest.tsv`."]
#![allow(missing_docs, clippy::expect_used, clippy::panic)]

use std::process::Command;
use std::sync::{Mutex, MutexGuard, PoisonError};

static COMMAND_EXECUTION_LOCK: Mutex<()> = Mutex::new(());

fn command_execution_guard() -> MutexGuard<'static, ()> {
    COMMAND_EXECUTION_LOCK
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
}

#[path = "support/manifest_execution.rs"]
mod manifest_execution;
#[path = "support/manifest_invocation.rs"]
mod manifest_invocation;

use manifest_execution::ManifestRow;

/// Splits one `<target>::<path>::<name>` binding into the `cargo test`
/// target selector and the exact test path to run.
fn target_selector(focused_test: &str) -> Option<(Vec<String>, String)> {
    focused_test.split_once("::").map(|(head, rest)| {
        if head == "lib" {
            (Vec::from(["--lib".to_owned()]), rest.to_owned())
        } else {
            (
                Vec::from(["--test".to_owned(), head.to_owned()]),
                rest.to_owned(),
            )
        }
    })
}

/// Reads and parses the committed manifest.
fn committed_rows() -> Vec<ManifestRow> {
    let text = std::fs::read_to_string(manifest_execution::MANIFEST_PATH)
        .expect("trace-publication manifest");
    let rows = manifest_execution::parse_rows(&text);
    assert!(!rows.is_empty(), "manifest must declare at least one row");
    rows
}

/// Runs every manifest row's `focused_test` as a real `cargo test`
/// subprocess and requires it to pass.
///
/// This stays an INDEPENDENT ORACLE, never a substitute for running the
/// row's declared command: `every_manifest_row_executes_its_declared_command`
/// below drives the real executable in a fresh isolated fixture and observes
/// the real process exit status, while this control proves the same scenario
/// is also covered by one committed, currently passing Rust test.
#[test]
fn every_manifest_row_focused_test_passes() {
    let _guard = command_execution_guard();
    let rows = committed_rows();

    let mut failures = Vec::new();
    for row in &rows {
        let Some((target, test_name)) = target_selector(&row.focused_test) else {
            failures.push(format!(
                "{}: focused_test `{}` is not in `<target>::<test-name>` shape",
                row.key(),
                row.focused_test
            ));
            continue;
        };
        let mut command = Command::new(env!("CARGO"));
        command
            .arg("test")
            .arg("--locked")
            .arg("--all-features")
            .args(&target)
            .arg(&test_name)
            .arg("--")
            .arg("--exact");
        let output = command.output().expect("cargo test subprocess");
        let stdout = String::from_utf8_lossy(&output.stdout);
        if !output.status.success() || stdout.contains("0 passed") || !stdout.contains("1 passed") {
            failures.push(format!(
                "{} ({} {}): focused test `{}` did not pass\n{}",
                row.key(),
                row.polarity,
                row.exit_class,
                row.focused_test,
                stdout
            ));
        }
    }
    assert!(
        failures.is_empty(),
        "manifest rows with a non-passing focused test:\n{}",
        failures.join("\n\n")
    );
}

/// Every manifest row must name a registered, executable invocation.
///
/// This is the completeness half of the repair and needs no digest feature:
/// a scenario added to the delta specs (and therefore to the manifest) with
/// no registered invocation is a FAILURE, not a skip, so the registry cannot
/// silently fall behind the manifest.
#[test]
fn every_manifest_row_has_a_registered_executable_invocation() {
    let rows = committed_rows();
    let unregistered: Vec<String> = rows
        .iter()
        .filter(|row| !manifest_invocation::is_registered(&row.requirement, &row.scenario))
        .map(ManifestRow::key)
        .collect();
    assert!(
        unregistered.is_empty(),
        "these {} manifest scenarios have no registered real command invocation \
         (register one in tests/support/manifest_invocation.rs; a scenario with no \
         executable invocation is a failure, never a skip):\n{}",
        unregistered.len(),
        unregistered.join("\n")
    );
    // A registered key the manifest no longer declares is drift in the other
    // direction, and is equally a failure.
    let keys: std::collections::BTreeSet<String> = rows.iter().map(ManifestRow::key).collect();
    let stale: Vec<&str> = manifest_invocation::REGISTERED
        .iter()
        .filter(|key| !keys.contains(**key))
        .copied()
        .collect();
    assert!(
        stale.is_empty(),
        "these registered invocations name scenarios the manifest no longer declares: {stale:?}"
    );
    assert_eq!(
        manifest_invocation::REGISTERED.len(),
        rows.len(),
        "every manifest row must have exactly one registered real command invocation"
    );
    assert!(
        rows.len() >= 40,
        "the manifest must carry a substantial scenario set, found {}",
        rows.len()
    );
}

// ---------------------------------------------------------------------------
// Real execution
// ---------------------------------------------------------------------------

/// What one real invocation actually did.
#[cfg(all(feature = "blake3-256", feature = "xxh3-128"))]
struct Observed {
    exit_class: String,
    stdout: String,
    stderr: String,
    rendered: String,
}

#[cfg(all(feature = "blake3-256", feature = "xxh3-128"))]
impl Observed {
    /// Whether the real process wrote `token` to stdout or stderr.
    fn emitted(&self, token: &str) -> bool {
        self.stdout.contains(token) || self.stderr.contains(token)
    }

    fn transcript(&self) -> String {
        format!(
            "  argv: {}\n  stdout: {}\n  stderr: {}",
            self.rendered,
            self.stdout.trim(),
            self.stderr.trim()
        )
    }
}

/// Builds one row's isolated fixture, runs its real command, and records the
/// real exit status and real output bytes.
#[cfg(all(feature = "blake3-256", feature = "xxh3-128"))]
fn observe(row: &ManifestRow) -> Observed {
    let invocation = manifest_invocation::invocation_for(&row.requirement, &row.scenario)
        .unwrap_or_else(|| panic!("{}: no registered invocation", row.key()));
    let rendered = invocation.rendered();
    let output = invocation.run();
    Observed {
        exit_class: output
            .status
            .code()
            .map_or_else(|| String::from("signal"), |code| code.to_string()),
        stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        rendered,
    }
}

/// Compares one row's DECLARED columns against what the real command really
/// did. Nothing here reads Rust source text.
#[cfg(all(feature = "blake3-256", feature = "xxh3-128"))]
fn faults(row: &ManifestRow, observed: &Observed) -> Vec<String> {
    let mut faults = Vec::new();
    if observed.rendered.split_whitespace().next().is_some() && !observed.rendered.is_empty() {
        // The invocation registry reports the real family through the same
        // process object used for execution. The manifest command is not a
        // comment or a focused-test label.
        let invocation = manifest_invocation::invocation_for(&row.requirement, &row.scenario)
            .expect("registered invocation");
        if invocation.command_family() != row.command {
            faults.push(format!(
                "{}: declares command `{}` but the real invocation is `{}`\n{}",
                row.key(),
                row.command,
                invocation.command_family(),
                observed.transcript()
            ));
        }
    }
    if observed.exit_class != row.exit_class {
        faults.push(format!(
            "{}: declares exit class {} but `{}` really exited {}\n{}",
            row.key(),
            row.exit_class,
            row.command,
            observed.exit_class,
            observed.transcript()
        ));
    }
    if !row.diagnostic.is_empty() && !observed.emitted(&row.diagnostic) {
        faults.push(format!(
            "{}: declares diagnostic `{}` but the real invocation never emitted it\n{}",
            row.key(),
            row.diagnostic,
            observed.transcript()
        ));
    }
    faults
}

/// THE REPAIR (round-5 closure audit, finding "decorative manifest columns").
///
/// Every scenario row builds one fresh isolated temporary fixture, runs the
/// REAL executable its `command` column names, and has its declared
/// `exit_class` compared against the REAL process exit status and its
/// declared `diagnostic` compared against the REAL bytes that process wrote.
/// The previous control asserted both by scanning Rust source text for the
/// literal `Some(2)` and for the diagnostic string, so a row could claim any
/// exit class or code that merely appeared somewhere in a test body. That
/// source scanning is gone.
///
/// The digest-feature gate matches the acceptance build (`cargo test
/// --workspace --all-targets --all-features`); the frozen fixtures select a
/// collision-resistant member and one row exercises a change-detection
/// member, so both must be compiled in for the observed exit classes to be
/// the production ones.
#[test]
#[cfg(all(feature = "blake3-256", feature = "xxh3-128"))]
fn every_manifest_row_executes_its_declared_command_in_an_isolated_fixture() {
    let _guard = command_execution_guard();
    let rows = committed_rows();

    let mut failures = Vec::new();
    let mut executed = 0_usize;
    let mut checked_diagnostics = 0_usize;
    let mut nonzero_exit_classes = 0_usize;
    for row in &rows {
        if manifest_invocation::invocation_for(&row.requirement, &row.scenario).is_none() {
            failures.push(format!(
                "{}: no registered real command invocation",
                row.key()
            ));
            continue;
        }
        let observed = observe(row);
        executed += 1;
        if !row.diagnostic.is_empty() {
            checked_diagnostics += 1;
        }
        if row.exit_class != "0" {
            nonzero_exit_classes += 1;
        }
        failures.extend(faults(row, &observed));
    }

    assert!(
        failures.is_empty(),
        "manifest rows whose real invocation disagrees with the declared columns:\n\n{}",
        failures.join("\n\n")
    );
    assert_eq!(
        executed,
        rows.len(),
        "every manifest row must execute one real command"
    );
    // Non-vacuity: the committed manifest must really exercise both stable
    // tokens and nonzero exit classes on more than a token number of rows.
    assert!(
        checked_diagnostics >= 20,
        "the manifest must bind real emitted tokens on more than a token number of rows, \
         found {checked_diagnostics}"
    );
    assert!(
        nonzero_exit_classes >= 20,
        "the manifest must bind real nonzero exit classes on more than a token number of rows, \
         found {nonzero_exit_classes}"
    );
}

/// NON-VACUITY CONTROL for the harness above.
///
/// Three real committed rows -- one exit-class-zero row, one nonzero
/// exit-class row, and one token-bearing row -- are executed for real, and
/// their DECLARED columns are then mutated in a temporary in-memory copy of
/// the manifest. The comparison must name exactly the mutated row and no
/// other, which is what proves the harness fails when a row lies rather than
/// passing on everything.
#[test]
#[cfg(all(feature = "blake3-256", feature = "xxh3-128"))]
fn the_execution_harness_fails_when_a_rows_declared_columns_are_wrong() {
    let _guard = command_execution_guard();
    let rows = committed_rows();

    let zero = rows
        .iter()
        .find(|row| row.exit_class == "0")
        .expect("one committed row declares exit class 0");
    let nonzero = rows
        .iter()
        .find(|row| row.exit_class != "0")
        .expect("one committed row declares a nonzero exit class");
    let tokened = rows
        .iter()
        .find(|row| !row.diagnostic.is_empty())
        .expect("one committed row declares a stable token");

    for row in [zero, nonzero, tokened] {
        let observed = observe(row);
        assert!(
            faults(row, &observed).is_empty(),
            "the positive case must not be vacuous for {}",
            row.key()
        );

        // A wrong exit class must be detected, and must name this row.
        let wrong_exit = ManifestRow {
            requirement: row.requirement.clone(),
            scenario: row.scenario.clone(),
            polarity: row.polarity.clone(),
            command: row.command.clone(),
            exit_class: String::from("213"),
            diagnostic: String::new(),
            focused_test: row.focused_test.clone(),
        };
        let detected = faults(&wrong_exit, &observed);
        assert_eq!(
            detected.len(),
            1,
            "exactly one fault must be reported for {}: {detected:?}",
            row.key()
        );
        assert!(
            detected[0].starts_with(&format!("{}: declares exit class 213", row.key())),
            "the fault must name exactly {} and its false exit class: {}",
            row.key(),
            detected[0]
        );

        // An invented token must be detected, and must name this row.
        let wrong_token = ManifestRow {
            requirement: row.requirement.clone(),
            scenario: row.scenario.clone(),
            polarity: row.polarity.clone(),
            command: row.command.clone(),
            exit_class: row.exit_class.clone(),
            diagnostic: String::from("TPT-A-CODE-NO-PRODUCTION-PATH-EMITS"),
            focused_test: row.focused_test.clone(),
        };
        let detected = faults(&wrong_token, &observed);
        assert_eq!(
            detected.len(),
            1,
            "exactly one fault must be reported for {}: {detected:?}",
            row.key()
        );
        assert!(
            detected[0].starts_with(&format!(
                "{}: declares diagnostic `TPT-A-CODE-NO-PRODUCTION-PATH-EMITS`",
                row.key()
            )),
            "the fault must name exactly {} and its invented token: {}",
            row.key(),
            detected[0]
        );
    }
}

/// An unregistered scenario is a FAILURE, not a skip: this proves the
/// completeness control above really rejects a key the registry does not
/// know, so adding a scenario row without an executable invocation cannot
/// pass silently.
#[test]
#[cfg(all(feature = "blake3-256", feature = "xxh3-128"))]
fn the_manifest_rejects_command_family_drift_and_keeps_rtp_polarities_distinct() {
    let _guard = command_execution_guard();
    let rows = committed_rows();
    let positive = rows
        .iter()
        .find(|row| row.scenario == "positive-published-page-identifies-the-complete-chain")
        .expect("RTP positive row");
    let negative = rows
        .iter()
        .find(|row| row.scenario == "negative-renderer-bytes-change-without-a-new-schema")
        .expect("RTP renderer negative row");
    assert_ne!(positive.polarity, negative.polarity);
    assert_ne!(positive.command, negative.command);
    assert_eq!(positive.command, "trace");
    assert_eq!(negative.command, "check-trace-projection");

    let observed = observe(negative);
    assert!(
        faults(negative, &observed).is_empty(),
        "real negative control must pass: {}",
        observed.transcript()
    );
    let drifted = ManifestRow {
        requirement: negative.requirement.clone(),
        scenario: negative.scenario.clone(),
        polarity: negative.polarity.clone(),
        command: String::from("trace"),
        exit_class: negative.exit_class.clone(),
        diagnostic: negative.diagnostic.clone(),
        focused_test: negative.focused_test.clone(),
    };
    assert!(
        !faults(&drifted, &observed).is_empty(),
        "command-family drift must fail closed"
    );
}

/// The focused golden remains an independent oracle for the positive renderer.
#[test]
fn the_rtp_negative_row_does_not_replace_its_focused_golden_oracle() {
    let row = committed_rows()
        .into_iter()
        .find(|row| row.scenario == "negative-renderer-bytes-change-without-a-new-schema")
        .expect("RTP negative row");
    assert_eq!(
        row.focused_test,
        "publication_cli_goldens::committed_page_golden_matches_the_real_cli_output"
    );
    assert_eq!(row.polarity, "negative");
}

#[test]
fn the_completeness_control_rejects_an_unregistered_scenario() {
    assert!(
        !manifest_invocation::is_registered("TPT-001", "positive-a-scenario-no-registry-knows"),
        "an unregistered scenario key must not be reported as registered"
    );
    assert!(
        manifest_invocation::invocation_for("TPT-001", "positive-a-scenario-no-registry-knows")
            .is_none(),
        "an unregistered scenario key must not resolve to an invocation"
    );
    let row = committed_rows()
        .into_iter()
        .next()
        .expect("one committed row");
    assert!(
        manifest_invocation::is_registered(&row.requirement, &row.scenario),
        "the positive case must not be vacuous: {} must resolve",
        row.key()
    );
}
