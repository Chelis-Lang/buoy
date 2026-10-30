//! Sanctioned rebind regeneration contracts.

#![allow(clippy::expect_used, clippy::panic)]

use chelis_provenance::command::{EXIT_OK, artifact_digest, static_check};
use chelis_provenance::rebind::{RebindError, rebind};
use chelis_provenance::snapshot::{ChelisConfiguration, ChelisSnapshot};
use std::fs;
use std::path::Path;

fn configuration() -> ChelisConfiguration {
    ChelisConfiguration::parse(
        b"schema = \"chelis-adapter-config/v1\"\n\n[[selector]]\nroot = \"chelis\"\ndialect = \"surf\"\n",
    )
    .expect("configuration")
}

const STALE: &str = "xxh3-128:000000000000000000000000000000aa";

/// A tree whose bound revisions, item digests, oracle digest, and runner
/// digests are all deliberately stale placeholders.
fn stale_scenario() -> String {
    format!(
        "-- chelis:provenance/v1 authority\n-- id = REBIND-ATOM\n-- kind = behavioral\n-- scopes = chelis-core\n-- statement = The rebound subject MUST stay bound end to end.\ndef rebind_subject(x: f32) -> f32 = x\n\
         -- chelis:provenance/v1 surface\n-- id = REBIND-SURFACE\n-- member-key-schema = chelis-backend-member/v1\n-- disposition-schema = chelis-backend-disposition/v1\n-- generation = 1\n-- members = eval;metal\n-- rows = eval:normative:REBIND-ATOM@{STALE};metal:not-applicable\ndef rebind_surface(x: f32) -> f32 = x\n\
         -- chelis:provenance/v1 implementation-link\n-- id = REBIND-LINK\n-- atoms = REBIND-ATOM@{STALE}\n-- item-digest = {STALE}\n-- navigation = implements\ndef rebind_impl(x: f32) -> f32 = x\n\
         -- chelis:provenance/v1 oracle\n-- id = REBIND-ORACLE\n-- oracle-kind = test\n-- runner-id = chelis-test\n-- runner-version = 1.0.0\n-- runner-digest = {STALE}\n-- runner-configuration-digest = {STALE}\n-- runner-config-paths = chelis/reef.toml;chelis/ci.toml\n-- selection-schema = chelis-oracle-selection/v1\n-- selection = selection\n-- normalization-schema = chelis-oracle-normalization/v1\n-- normalization = normalization\n-- configuration-digest = xxh3-128:00000000000000000000000000000003\n-- scope-schema = chelis-oracle-scope/v1\n-- scope = scope\ndef rebind_oracle(x: f32) -> f32 = x\n\
         -- chelis:provenance/v1 carrier\n-- id = REBIND-CARRIER\n-- role = positive\n-- atoms = REBIND-ATOM@{STALE}\n-- item-digest = {STALE}\n-- oracle-id = REBIND-ORACLE\n-- oracle-digest = {STALE}\n-- configuration-digest = xxh3-128:00000000000000000000000000000003\n-- scope-schema = chelis-carrier-scope/v1\n-- scope = rebind\ndef rebind_carrier(x: f32) -> f32 = x\n"
    )
}

fn write_tree(root: &Path) {
    fs::create_dir_all(root.join("chelis")).expect("selector root");
    fs::write(root.join("chelis/rebind.ch"), stale_scenario()).expect("write source");
    fs::write(root.join("chelis/reef.toml"), b"version = \"1.0.0\"\n").expect("write artifact");
    fs::write(root.join("chelis/ci.toml"), b"jobs = 1\n").expect("write configuration");
}

#[test]
fn dry_run_reports_the_plan_without_mutating() {
    let root = tempfile::tempdir().expect("root");
    write_tree(root.path());
    let before = fs::read(root.path().join("chelis/rebind.ch")).expect("before bytes");
    let outcome = rebind(root.path(), &configuration(), false).expect("dry run");
    assert!(
        outcome.entries >= 8,
        "plan covers every stale field: {}",
        outcome.report
    );
    assert!(!outcome.written);
    assert!(outcome.report.contains("\"written\":false"));
    assert!(outcome.report.contains(&format!("\"bound\":\"{STALE}\"")));
    let after = fs::read(root.path().join("chelis/rebind.ch")).expect("after bytes");
    assert_eq!(before, after, "a dry run cannot mutate saved bytes");
}

#[test]
fn write_converges_and_the_static_check_accepts() {
    let root = tempfile::tempdir().expect("root");
    write_tree(root.path());
    let outcome = rebind(root.path(), &configuration(), true).expect("write run");
    assert!(outcome.written);
    let empty = rebind(root.path(), &configuration(), true).expect("second run");
    assert_eq!(empty.entries, 0, "a second run reports an empty plan");
    assert!(!empty.written);
    let configuration = configuration();
    let snapshot = ChelisSnapshot::from_directory(root.path(), &configuration).expect("snapshot");
    let checked = static_check(&snapshot, &configuration).expect("static outcome");
    assert_eq!(checked.exit, EXIT_OK, "{:?}", checked.diagnostics);
}

#[test]
fn write_touches_only_record_fields_and_never_declarations() {
    let root = tempfile::tempdir().expect("root");
    write_tree(root.path());
    let before = fs::read_to_string(root.path().join("chelis/rebind.ch")).expect("before");
    rebind(root.path(), &configuration(), true).expect("write run");
    let after = fs::read_to_string(root.path().join("chelis/rebind.ch")).expect("after");
    let declarations = |text: &str| {
        text.lines()
            .filter(|line| !line.trim_start().starts_with("--"))
            .map(str::to_owned)
            .collect::<Vec<_>>()
    };
    assert_eq!(
        declarations(&before),
        declarations(&after),
        "declaration bytes stay identical"
    );
    let changed = before
        .lines()
        .zip(after.lines())
        .filter(|(left, right)| left != right)
        .collect::<Vec<_>>();
    assert!(!changed.is_empty());
    assert!(
        changed
            .iter()
            .all(|(left, _)| left.trim_start().starts_with("--")),
        "only record comment fields change: {changed:?}"
    );
    assert!(!after.contains(STALE), "no stale placeholder survives");
}

#[test]
fn rebound_runner_digest_matches_the_saved_artifact() {
    let root = tempfile::tempdir().expect("root");
    write_tree(root.path());
    rebind(root.path(), &configuration(), true).expect("write run");
    let after = fs::read_to_string(root.path().join("chelis/rebind.ch")).expect("after");
    let reef_digest = artifact_digest(b"version = \"1.0.0\"\n")
        .expect("reef digest")
        .canonical_text();
    assert!(
        after.contains(&format!("-- runner-digest = {reef_digest}")),
        "runner digest rebinds to the saved artifact bytes"
    );
}

#[test]
fn parse_rejection_refuses_before_any_write() {
    let root = tempfile::tempdir().expect("root");
    write_tree(root.path());
    fs::write(root.path().join("chelis/broken.ch"), b"def broken( = {\n").expect("write broken");
    let before = fs::read(root.path().join("chelis/rebind.ch")).expect("before bytes");
    let error = rebind(root.path(), &configuration(), true).expect_err("must refuse");
    assert_eq!(
        error,
        RebindError::ParseRejected(String::from("chelis/broken.ch"))
    );
    let after = fs::read(root.path().join("chelis/rebind.ch")).expect("after bytes");
    assert_eq!(
        before, after,
        "nothing is rewritten under a parse rejection"
    );
}

#[test]
fn rebind_repairs_resolvable_fields_without_inventing_missing_references() {
    let baseline = tempfile::tempdir().expect("baseline root");
    write_tree(baseline.path());
    rebind(baseline.path(), &configuration(), true).expect("baseline repair");
    let current =
        fs::read_to_string(baseline.path().join("chelis/rebind.ch")).expect("current source");
    let extraction = chelis_provenance::records::extract(
        chelis_provenance::records::ChelisDialect::Surf,
        current.as_bytes(),
    )
    .expect("current records");
    let carrier = extraction
        .records
        .iter()
        .find(|record| record.kind == "carrier")
        .expect("carrier");
    let item = carrier.fields.get("item-digest").expect("current item");
    let oracle = extraction
        .records
        .iter()
        .find(|record| record.kind == "oracle")
        .expect("oracle");
    let runner = oracle.fields.get("runner-digest").expect("current runner");
    let atoms = carrier.fields.get("atoms").expect("current atoms");
    let surface = extraction
        .records
        .iter()
        .find(|record| record.kind == "surface")
        .expect("surface");
    let rows = surface.fields.get("rows").expect("current rows");
    for (before, after) in [
        (
            format!("-- atoms = {atoms}"),
            format!("-- atoms = UNKNOWN-ATOM@{STALE}"),
        ),
        (
            format!("-- atoms = {atoms}"),
            String::from("-- atoms = REBIND-ATOM"),
        ),
        (
            format!("-- rows = {rows}"),
            format!("-- rows = eval:normative:UNKNOWN-ATOM@{STALE};metal:not-applicable"),
        ),
        (
            format!("-- rows = {rows}"),
            String::from("-- rows = eval:normative:REBIND-ATOM;metal:not-applicable"),
        ),
        (String::from("-- id = REBIND-CARRIER\n"), String::new()),
        (format!("-- atoms = {atoms}\n"), String::new()),
        (format!("-- rows = {rows}\n"), String::new()),
        (
            String::from("-- oracle-id = REBIND-ORACLE"),
            String::from("-- oracle-id = MISSING-ORACLE"),
        ),
        (
            format!("-- item-digest = {item}"),
            String::from("-- item-digest ="),
        ),
        (format!("-- item-digest = {item}\n"), String::new()),
        (format!("-- runner-digest = {runner}\n"), String::new()),
    ] {
        let root = tempfile::tempdir().expect("case root");
        write_tree(root.path());
        let malformed = current.replace(&before, &after);
        assert_ne!(malformed, current, "{before}");
        let source = format!(
            "{malformed}\n-- chelis:provenance/v1 implementation-link\n-- id = REPAIR-LINK\n-- atoms = REBIND-ATOM@{STALE}\n-- item-digest = {STALE}\n-- navigation = implements\ndef repair(x: f32) -> f32 = x\n"
        );
        let path = root.path().join("chelis/rebind.ch");
        fs::write(&path, &source).expect("case source");
        let plan = rebind(root.path(), &configuration(), false).expect("repair plan");
        let report: serde_json::Value = serde_json::from_str(&plan.report).expect("plan JSON");
        let entries = report["entries"].as_array().expect("entries");
        assert_eq!(entries.len(), 2, "{before}: {}", plan.report);
        assert!(entries.iter().all(|entry| entry["record"] == "REPAIR-LINK"));
        let repaired = source
            .replace(
                &format!("-- atoms = REBIND-ATOM@{STALE}"),
                &format!("-- atoms = {atoms}"),
            )
            .replace(
                &format!("-- item-digest = {STALE}"),
                &format!(
                    "-- item-digest = {}",
                    artifact_digest(b"def repair(x: f32) -> f32 = x")
                        .expect("repair digest")
                        .canonical_text()
                ),
            );
        rebind(root.path(), &configuration(), true).expect("selective repair");
        assert_eq!(
            fs::read_to_string(&path).expect("saved source"),
            repaired,
            "{before}"
        );
        assert!(!root.path().join("chelis/objects").exists());
        assert_eq!(
            fs::read(root.path().join("chelis/reef.toml")).expect("artifact bytes"),
            b"version = \"1.0.0\"\n"
        );
    }
}

#[test]
fn unavailable_runner_and_malformed_oracle_keep_unresolved_commitments() {
    for (needle, replacement, preserved) in [
        (
            "chelis/reef.toml;chelis/ci.toml",
            "chelis/missing.toml;chelis/missing-config.toml",
            format!("-- runner-digest = {STALE}"),
        ),
        (
            "-- oracle-kind = test\n",
            "",
            format!("-- oracle-digest = {STALE}"),
        ),
    ] {
        let root = tempfile::tempdir().expect("root");
        write_tree(root.path());
        let path = root.path().join("chelis/rebind.ch");
        let source = stale_scenario().replace(needle, replacement);
        fs::write(&path, &source).expect("unresolved source");
        rebind(root.path(), &configuration(), true).expect("partial repair");
        let saved = fs::read_to_string(&path).expect("saved source");
        assert!(saved.contains(&preserved), "{saved}");
        assert!(saved.contains(replacement), "{saved}");
        let snapshot =
            ChelisSnapshot::from_directory(root.path(), &configuration()).expect("snapshot");
        let checked = static_check(&snapshot, &configuration()).expect("static result");
        assert_eq!(checked.exit, chelis_provenance::command::EXIT_STATIC);
        assert_eq!(
            rebind(root.path(), &configuration(), false)
                .expect("next plan")
                .entries,
            0
        );
        assert!(!root.path().join("chelis/objects").exists());
    }
}

#[test]
fn mixed_dialects_and_invalid_utf8_keep_their_bytes_during_surf_repair() {
    let root = tempfile::tempdir().expect("root");
    write_tree(root.path());
    fs::create_dir(root.path().join("deep")).expect("Deep root");
    let deep = b"(module untouched)\n";
    fs::write(root.path().join("deep/source.dp"), deep).expect("Deep source");
    fs::write(root.path().join("chelis/invalid.ch"), [0xff]).expect("invalid source");
    let config = ChelisConfiguration::parse(
        b"schema = \"chelis-adapter-config/v1\"\nbinding-store = \"chelis/objects\"\n[[selector]]\nroot = \"chelis\"\ndialect = \"surf\"\n[[selector]]\nroot = \"deep\"\ndialect = \"deep\"\n",
    ).expect("mixed configuration");
    rebind(root.path(), &config, true).expect("Surf repair");
    assert_eq!(
        fs::read(root.path().join("deep/source.dp")).expect("Deep bytes"),
        deep
    );
    assert_eq!(
        fs::read(root.path().join("chelis/invalid.ch")).expect("invalid bytes"),
        [0xff]
    );
    let saved = fs::read_to_string(root.path().join("chelis/rebind.ch")).expect("repaired source");
    assert!(!saved.contains(STALE), "{saved}");
    assert_eq!(
        rebind(root.path(), &config, false)
            .expect("next plan")
            .entries,
        0
    );
}

#[test]
fn a_self_referential_runner_reports_divergence_after_the_saved_write() {
    let root = tempfile::tempdir().expect("root");
    write_tree(root.path());
    let path = root.path().join("chelis/rebind.ch");
    let source = stale_scenario().replace(
        "chelis/reef.toml;chelis/ci.toml",
        "chelis/rebind.ch;chelis/ci.toml",
    );
    fs::write(&path, &source).expect("self-referential runner");
    let before = artifact_digest(source.as_bytes())
        .expect("before digest")
        .canonical_text();
    assert_eq!(
        rebind(root.path(), &configuration(), true).expect_err("divergence"),
        RebindError::Diverged
    );
    let saved = fs::read_to_string(&path).expect("saved source");
    assert!(saved.contains(&format!("-- runner-digest = {before}")));
    let after = artifact_digest(saved.as_bytes())
        .expect("after digest")
        .canonical_text();
    assert_ne!(before, after);
    let next = rebind(root.path(), &configuration(), false).expect("remaining plan");
    let report: serde_json::Value = serde_json::from_str(&next.report).expect("plan JSON");
    let entry = report["entries"]
        .as_array()
        .expect("entries")
        .iter()
        .find(|entry| entry["field"] == "runner-digest")
        .expect("runner update");
    assert_eq!(entry["bound"], before);
    assert_eq!(entry["current"], after);
    let declarations = |text: &str| {
        text.lines()
            .filter(|line| line.starts_with("def "))
            .map(str::to_owned)
            .collect::<Vec<_>>()
    };
    assert_eq!(declarations(&saved), declarations(&source));
    assert!(!root.path().join("chelis/objects").exists());
}
