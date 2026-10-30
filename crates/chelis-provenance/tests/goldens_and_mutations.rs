//! Frozen report goldens and deliberate faulty-implementation mutations.
//!
//! Regeneration policy: delete one golden file, run this suite twice, and
//! review the recorded diff before commit. A missing golden fails the first
//! run, so continuous integration cannot bootstrap goldens silently.

#![allow(
    clippy::expect_used,
    clippy::format_push_string,
    clippy::panic,
    clippy::too_many_lines
)]

use buoy::core::SemanticDigest;
use chelis_provenance::advisory::advisory_report;
use chelis_provenance::command::{
    ChangeManifest, EXIT_EXECUTION, change_check, render_human_static, static_check,
};
use chelis_provenance::execution::execute;
use chelis_provenance::records::ChelisDialect;
use chelis_provenance::snapshot::{ChelisConfiguration, ChelisSnapshot};
use std::fs;
use std::path::Path;

const GOLDEN_ROOT: &str = "../../fixtures/chelis-provenance/v1/goldens";

fn declaration_item_digest(declaration: &str) -> String {
    buoy::core::semantic_digest(
        "chelis-raw-artifact",
        vec![(
            1,
            buoy::core::CanonicalValue::Bytes(declaration.as_bytes().to_vec()),
        )],
    )
    .expect("item digest")
    .canonical_text()
}

fn configuration() -> ChelisConfiguration {
    ChelisConfiguration::parse(
        b"schema = \"chelis-adapter-config/v1\"\n\n[[selector]]\nroot = \"chelis\"\ndialect = \"surf\"\n",
    )
    .expect("configuration")
}

fn golden_scenario(statement: &str, drop_row: bool, stale_carrier: bool) -> ChelisSnapshot {
    let atom = buoy::core::Atom::new(
        buoy::core::StableId::new("CHELIS-GOLDEN-ATOM").expect("atom id"),
        buoy::core::LifecycleState::Proposed,
        buoy::core::AtomKind::Behavioral,
        vec![buoy::core::ScopeId::new("chelis-core").expect("scope")],
        statement,
        None,
    )
    .expect("atom value");
    let bound = if stale_carrier {
        buoy::core::Atom::new(
            buoy::core::StableId::new("CHELIS-GOLDEN-ATOM").expect("atom id"),
            buoy::core::LifecycleState::Proposed,
            buoy::core::AtomKind::Behavioral,
            vec![buoy::core::ScopeId::new("chelis-core").expect("scope")],
            "The golden backend MUST stay deterministic under a prior revision.",
            None,
        )
        .expect("stale atom value")
        .revision()
    } else {
        atom.revision()
    };
    let revision = atom.revision().canonical_text();
    let bound_revision = bound.canonical_text();
    let rows = if drop_row {
        format!("eval:normative:CHELIS-GOLDEN-ATOM@{revision}")
    } else {
        format!("eval:normative:CHELIS-GOLDEN-ATOM@{revision};metal:not-applicable")
    };
    let digest = declaration_item_digest("def golden_impl(x: f32) -> f32 = x");
    let source = format!(
        "-- chelis:provenance/v1 authority\n-- id = CHELIS-GOLDEN-ATOM\n-- kind = behavioral\n-- scopes = chelis-core\n-- statement = {statement}\ndef golden(x: f32) -> f32 = x\n\
         -- chelis:provenance/v1 surface\n-- id = CHELIS-GOLDEN-SURFACE\n-- member-key-schema = chelis-backend-member/v1\n-- disposition-schema = chelis-backend-disposition/v1\n-- generation = 1\n-- members = eval;metal\n-- rows = {rows}\ndef golden_surface(x: f32) -> f32 = x\n\
         -- chelis:provenance/v1 implementation-link\n-- id = CHELIS-GOLDEN-LINK\n-- atoms = CHELIS-GOLDEN-ATOM@{bound_revision}\n-- item-digest = {digest}\n-- navigation = implements\ndef golden_impl(x: f32) -> f32 = x\n"
    );
    ChelisSnapshot::from_files(vec![(
        String::from("chelis/golden.ch"),
        ChelisDialect::Surf,
        source.into_bytes(),
    )])
    .expect("snapshot")
}

fn assert_golden(name: &str, current: &str) {
    let path = Path::new(GOLDEN_ROOT).join(name);
    if !path.exists() {
        fs::create_dir_all(Path::new(GOLDEN_ROOT)).expect("golden root");
        fs::write(&path, current).expect("record golden");
        panic!("golden {name} was absent; recorded current bytes, review and rerun");
    }
    let frozen = fs::read_to_string(&path).expect("golden bytes");
    assert_eq!(
        frozen, current,
        "golden {name} drifted; review before regenerating"
    );
}

#[test]
fn machine_and_human_report_goldens_stay_frozen() {
    let snapshot = golden_scenario("The golden backend MUST stay deterministic.", false, false);
    let outcome = static_check(&snapshot, &configuration()).expect("static outcome");
    assert_golden("static-report.json", &outcome.report);
    assert_golden("static-report.human.txt", &render_human_static(&outcome));
    assert_golden(
        "advisory-report.json",
        &advisory_report(&snapshot, &outcome).expect("advisory report"),
    );

    let head_snapshot = golden_scenario(
        "The golden backend MUST stay deterministic under revision.",
        false,
        false,
    );
    let head = static_check(&head_snapshot, &configuration()).expect("head outcome");
    let manifest = ChangeManifest::parse(
        b"schema = \"chelis-change-manifest/v1\"\n\n[[disposition]]\nobject-kind = \"atom\"\nobject-id = \"CHELIS-GOLDEN-ATOM\"\nkind = \"revised\"\n\n[[disposition]]\nobject-kind = \"decision-surface\"\nobject-id = \"CHELIS-GOLDEN-SURFACE\"\nkind = \"revised\"\n\n[[disposition]]\nobject-kind = \"implementation-link\"\nobject-id = \"CHELIS-GOLDEN-LINK\"\nkind = \"revised\"\n",
    )
    .expect("manifest");
    let change = change_check(&outcome, &head, &manifest).expect("change outcome");
    assert_golden("impact-report.json", &change.report);

    let commands = [toml::from_str::<buoy::repository::OracleCommand>(
        "id = \"O-GOLDEN\"\nprogram = \"builtin:pass\"\ntimeout_ms = 1000\n",
    )
    .expect("oracle command")];
    let execution = execute(
        SemanticDigest::parse_canonical("xxh3-128:000000000000000000000000000000aa")
            .expect("identity"),
        &commands,
        Path::new("."),
    )
    .expect("execution outcome");
    assert_golden("execution-report.json", &execution.report);
}

#[test]
fn deliberate_faulty_implementations_flip_every_guarded_verdict() {
    let clean = static_check(
        &golden_scenario("The golden backend MUST stay deterministic.", false, false),
        &configuration(),
    )
    .expect("clean outcome");
    assert!(clean.diagnostics.is_empty(), "{:?}", clean.diagnostics);

    let dropped_row = static_check(
        &golden_scenario("The golden backend MUST stay deterministic.", true, false),
        &configuration(),
    )
    .expect("surface mutation outcome");
    assert!(
        dropped_row
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "SPG-SURFACE-INCOMPLETE"),
        "surface mutation must reject: {:?}",
        dropped_row.diagnostics
    );

    let stale_link = static_check(
        &golden_scenario("The golden backend MUST stay deterministic.", false, true),
        &configuration(),
    )
    .expect("freshness mutation outcome");
    assert!(
        !stale_link.diagnostics.is_empty(),
        "freshness mutation must stay visible"
    );

    let head = static_check(
        &golden_scenario(
            "The golden backend MUST stay deterministic under revision.",
            false,
            false,
        ),
        &configuration(),
    )
    .expect("head outcome");
    let empty =
        ChangeManifest::parse(b"schema = \"chelis-change-manifest/v1\"\n").expect("empty manifest");
    let change = change_check(&clean, &head, &empty).expect("impact mutation outcome");
    assert!(
        change
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "CHELIS-PROV-IMPACT-DISPOSITION-MISSING"),
        "impact mutation must reject: {:?}",
        change.diagnostics
    );

    let failing = [toml::from_str::<buoy::repository::OracleCommand>(
        "id = \"O-FAIL\"\nprogram = \"builtin:fail\"\ntimeout_ms = 1000\n",
    )
    .expect("oracle command")];
    let execution = execute(clean.identity, &failing, Path::new(".")).expect("execution");
    assert_eq!(
        execution.exit, EXIT_EXECUTION,
        "no waiver, reaffirmation, debt, or review synthesizes a passing execution"
    );
    assert_eq!(execution.static_identity, clean.identity);
}
