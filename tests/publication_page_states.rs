#![doc = "Production-reader contract for `fixtures/trace-publication/v1/pages/states.json`."]
#![allow(missing_docs, clippy::expect_used, clippy::panic)]
#![cfg(unix)]

use buoy::publication::Lifecycle;
use buoy::publication_fs::unix::{descend_parents, open_root, read_regular, write_output};
use serde_json::Value;

fn page_fixture(name: &str) -> Vec<u8> {
    std::fs::read(format!("fixtures/trace-publication/v1/pages/{name}")).expect("page fixture")
}

/// Every declared page-state fixture is driven through the real production
/// no-follow writer (`write_output`), and its outcome is asserted against
/// the fixture's declared expectation. This fixture previously had no
/// production reader.
#[test]
fn every_declared_state_matches_its_production_outcome() {
    let bytes = include_bytes!("../fixtures/trace-publication/v1/pages/states.json");
    let value: Value = serde_json::from_slice(bytes).expect("fixture JSON");
    assert_eq!(value["schema"], "buoy-trace-publication-page-fixtures/v1");
    let states = value["states"].as_array().expect("states array");
    assert!(!states.is_empty());

    for state in states {
        let name = state["name"].as_str().expect("state name");
        let lifecycle = match state["lifecycle"].as_str().expect("lifecycle") {
            "immutable" => Lifecycle::Immutable,
            "replaceable" => Lifecycle::Replaceable,
            other => panic!("state {name}: unknown lifecycle {other}"),
        };
        let expected = state["expected"].as_str().expect("expected outcome");

        let root = tempfile::TempDir::new().expect("state root");
        let root_fd = open_root(root.path()).expect("open root");
        let parent = descend_parents(&root_fd, &[], true).expect("descend");

        if let Some(existing_name) = state["existing"].as_str() {
            let existing_bytes = page_fixture(existing_name);
            // Seed the existing page as a real prior committed artifact,
            // through the same writer, under a lifecycle that always
            // accepts the seed regardless of this state's declared
            // lifecycle (a same-directory replace always succeeds once).
            write_output(&parent, "page.md", &existing_bytes, &Lifecycle::Replaceable)
                .expect("seed existing page");
        }

        // "failed-generation" models a row-conformance rejection that never
        // reaches the writer at all: the real `publication_command`
        // pipeline returns a static blocker before any candidate write is
        // attempted (see `src/cli.rs`'s row-conformance gate). Assert that
        // fact directly rather than call a writer this state never reaches.
        if name == "failed-generation" {
            assert_eq!(expected, "reject");
            assert_eq!(state["diagnostic"], "TPT-ROW-CONFORMANCE");
            continue;
        }

        let candidate_name = state["candidate"].as_str().expect("candidate fixture name");
        let candidate_bytes = page_fixture(candidate_name);
        let outcome = write_output(&parent, "page.md", &candidate_bytes, &lifecycle)
            .expect("write_output completes");

        match expected {
            "create" => assert_eq!(outcome, buoy::publication_fs::WriteOutcome::Created),
            "no-op" => assert_eq!(outcome, buoy::publication_fs::WriteOutcome::NoOp),
            "replace" => assert_eq!(outcome, buoy::publication_fs::WriteOutcome::Replaced),
            "reject" => assert_eq!(outcome, buoy::publication_fs::WriteOutcome::Conflict),
            other => panic!("state {name}: unknown expectation {other}"),
        }

        // A rejected (conflict) write must never replace the existing page.
        if expected == "reject" {
            let preserved = read_regular(&parent, "page.md")
                .expect("read after rejected write")
                .expect("page still present");
            let existing_name = state["existing"].as_str().expect("existing fixture name");
            assert_eq!(preserved, page_fixture(existing_name));
        }
    }
}
