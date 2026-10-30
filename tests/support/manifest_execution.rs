//! Shared row model for `fixtures/trace-publication/v1/manifest.tsv`.
//!
//! Both the manifest's shape checker and its command-execution harness parse
//! the same eight-field row model from this one place, so the two fail-closed
//! contracts can never silently drift apart.
//!
//! Schema `v3` is keyed per SCENARIO, not per requirement: the change's delta
//! specs declare many scenarios under one requirement ID, and round-4 finding
//! H5 showed that a per-requirement manifest silently covers only two of them.
//!
//! Round-5 note: this module deliberately carries NO source-scanning helper.
//! The previous execution harness "verified" a row's `exit_class` and
//! `diagnostic` by extracting the named focused test's function body and
//! searching it for the literal `Some(<class>)` and for the diagnostic
//! string. That made both columns decorative -- a row could claim any exit
//! class or code that merely appeared somewhere in a test body, and no
//! command ever ran. Those helpers are removed; `tests/support/
//! manifest_invocation.rs` now builds one isolated fixture per scenario and
//! the harness observes the real process exit status and real output bytes.

#![allow(dead_code)]

pub const MANIFEST_PATH: &str = "fixtures/trace-publication/v1/manifest.tsv";
pub const MANIFEST_SCHEMA: &str = "buoy-trace-publication-fixture-manifest/v3";
pub const MANIFEST_HEADER: &str =
    "schema\trequirement\tscenario\tpolarity\tcommand\texit_class\tdiagnostic\tfocused_test";

/// One parsed manifest row.
pub struct ManifestRow {
    pub requirement: String,
    pub scenario: String,
    pub polarity: String,
    pub command: String,
    pub exit_class: String,
    pub diagnostic: String,
    pub focused_test: String,
}

impl ManifestRow {
    /// The `requirement/scenario` key this row is uniquely identified by.
    #[must_use]
    pub fn key(&self) -> String {
        format!("{}/{}", self.requirement, self.scenario)
    }
}

/// Parses every well-formed data row's fields, in file order.
///
/// This performs only the minimal shape check this shared consumer needs
/// (eight tab-separated fields, exact schema); the complete fail-closed
/// completeness contract lives in `trace_publication_fixture_manifest.rs`.
///
/// # Panics
///
/// Panics if the manifest header or any row does not match the expected
/// eight-field shape.
#[must_use]
pub fn parse_rows(text: &str) -> Vec<ManifestRow> {
    let mut lines = text.lines();
    let header = lines.next().expect("manifest header");
    assert_eq!(header, MANIFEST_HEADER);
    lines
        .filter(|line| !line.is_empty())
        .map(|line| {
            let fields = line.split('\t').collect::<Vec<_>>();
            assert_eq!(
                fields.len(),
                8,
                "manifest row must have eight fields: {line}"
            );
            assert_eq!(fields[0], MANIFEST_SCHEMA);
            ManifestRow {
                requirement: fields[1].to_owned(),
                scenario: fields[2].to_owned(),
                polarity: fields[3].to_owned(),
                command: fields[4].to_owned(),
                exit_class: fields[5].to_owned(),
                diagnostic: fields[6].to_owned(),
                focused_test: fields[7].to_owned(),
            }
        })
        .collect()
}
