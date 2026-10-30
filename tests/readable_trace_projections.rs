#![doc = "Test-first contracts for readable trace projections."]
#![allow(clippy::expect_used, clippy::panic, clippy::too_many_lines)]

use std::path::{Path, PathBuf};

#[path = "support/traceability_control.rs"]
mod traceability_control;

/// Repository-relative files that make one complete trace fixture snapshot.
#[cfg_attr(
    not(all(unix, feature = "blake3-256")),
    expect(
        dead_code,
        reason = "the staged snapshot needs a collision-resistant build"
    )
)]
const FIXTURE_FILES: [&str; 5] = [
    "buoy.toml",
    "configuration.toml",
    "openspec/specs/demo/spec.md",
    "src/lib.rs",
    "buoy/objects/v1/blake3-256/0e857e33e387325fbf457e426bc42286d9d99926b19f3efa3c203795cfeeb6c4.toml",
];

fn repository_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf()
}

fn fixture_root() -> PathBuf {
    repository_root().join("fixtures/readable-trace-projections/v1")
}

/// Reads one reviewed case table without its header row.
fn case_rows(relative: &str) -> Vec<Vec<String>> {
    let path = fixture_root().join(relative);
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("case table {}: {error}", path.display()));
    text.lines()
        .skip(1)
        .filter(|line| !line.is_empty())
        .map(|line| line.split('\t').map(str::to_owned).collect())
        .collect()
}

#[test]
fn every_reviewed_case_table_is_present_and_complete() {
    for (relative, columns, minimum) in [
        ("selectors/positive.tsv", 6, 4),
        ("selectors/negative.tsv", 6, 11),
        ("relations/cases.tsv", 7, 4),
        ("collisions/cases.tsv", 6, 3),
        ("contracts/machine.tsv", 3, 20),
        ("contracts/human.tsv", 3, 10),
        ("contracts/markdown.tsv", 3, 9),
        ("workflow/cases.tsv", 5, 4),
    ] {
        let rows = case_rows(relative);
        assert!(
            rows.len() >= minimum,
            "{relative}: expected at least {minimum} cases, found {}",
            rows.len()
        );
        for row in &rows {
            assert_eq!(row.len(), columns, "{relative}: malformed row {row:?}");
            assert!(
                row.iter().all(|field| !field.is_empty()),
                "{relative}: empty field in {row:?}"
            );
        }
    }
}

#[test]
fn negative_selector_cases_declare_only_usage_and_input_exit_classes() {
    for row in case_rows("selectors/negative.tsv") {
        let expected_exit = row[4].as_str();
        match row[1].as_str() {
            "usage" => assert_eq!(expected_exit, "64", "usage case {}", row[0]),
            "input" => assert_eq!(expected_exit, "70", "input case {}", row[0]),
            other => panic!("unknown selector failure class: {other}"),
        }
    }
}

#[test]
fn relation_cases_bind_each_state_to_its_exact_exit_class() {
    for row in case_rows("relations/cases.tsv") {
        let case = row[0].as_str();
        let expected_exit = row[4].as_str();
        let diagnostic = row[5].as_str();
        match row[2].as_str() {
            "current" | "historical" => {
                assert_eq!(expected_exit, "0", "{case}: a resolved query succeeds");
                assert_eq!(diagnostic, "none", "{case}: no primary blocker applies");
            }
            "stale" => {
                assert_eq!(expected_exit, "2", "{case}: a stale attachment blocks");
                assert!(
                    diagnostic == "CAB-REQUIREMENT-STALE" || diagnostic == "CAB-SUBJECT-STALE",
                    "{case}: unexpected primary diagnostic {diagnostic}"
                );
            }
            other => panic!("unknown relation state: {other}"),
        }
    }
}

#[test]
fn machine_contract_forbids_every_noncanonical_field() {
    let rows = case_rows("contracts/machine.tsv");
    let forbidden = rows
        .iter()
        .filter(|row| row[0] == "forbidden")
        .map(|row| row[1].clone())
        .collect::<Vec<_>>();
    for required in [
        "root",
        "mount_root",
        "absolute_path",
        "timestamp",
        "locale",
        "terminal_width",
        "provider",
    ] {
        assert!(
            forbidden.iter().any(|field| field.as_str() == required),
            "the machine contract must forbid {required}"
        );
    }
    let order = rows
        .iter()
        .filter(|row| row[0] == "order")
        .map(|row| row[1].clone())
        .collect::<Vec<_>>();
    assert_eq!(
        order,
        vec![
            "current-before-historical",
            "relation-id",
            "requirement-id",
            "implementation-path",
            "implementation-node",
            "record-digest",
        ],
        "the canonical row order must match the reviewed design"
    );
}

#[test]
fn human_and_markdown_contracts_keep_the_assurance_boundary() {
    for relative in ["contracts/human.tsv", "contracts/markdown.tsv"] {
        let rows = case_rows(relative);
        assert!(
            rows.iter()
                .any(|row| row[0] == "required" && row[1] == "navigation and impact only"),
            "{relative}: the view must state the navigation boundary"
        );
        for forbidden in ["coverage established", "proved"] {
            assert!(
                rows.iter()
                    .any(|row| row[0] == "forbidden" && row[1] == forbidden),
                "{relative}: the view must forbid {forbidden}"
            );
        }
    }
}

/// The fixture snapshot selects one collision-resistant member, so these
/// contracts run only under a build that contains that adapter.
#[cfg(all(unix, feature = "blake3-256"))]
mod command {
    use super::{FIXTURE_FILES, case_rows, fixture_root};
    use std::path::Path;
    use std::process::{Command, Output};

    /// Copies the reviewed snapshot into one writable temporary root.
    fn staged_repository() -> tempfile::TempDir {
        let target = tempfile::TempDir::new().expect("temporary trace root");
        let source = fixture_root().join("repository");
        for relative in FIXTURE_FILES {
            let destination = target.path().join(relative);
            std::fs::create_dir_all(destination.parent().expect("fixture parent"))
                .expect("fixture directory");
            std::fs::copy(source.join(relative), destination).expect("copied trace fixture");
        }
        target
    }

    fn run_trace(root: &Path, arguments: &[&str]) -> Output {
        let config = root.join("buoy.toml");
        let mut all = vec![
            "trace",
            "--root",
            root.to_str().expect("UTF-8 root"),
            "--config",
            config.to_str().expect("UTF-8 config"),
        ];
        all.extend_from_slice(arguments);
        Command::new(env!("CARGO_BIN_EXE_buoy"))
            .args(all)
            .output()
            .expect("trace command")
    }

    fn exit_code(output: &Output) -> i32 {
        output.status.code().expect("a normal exit status")
    }

    #[test]
    fn trace_command_accepts_each_typed_selector() {
        let root = staged_repository();
        for row in case_rows("selectors/positive.tsv") {
            let case = row[0].as_str();
            let output = run_trace(
                root.path(),
                &[row[1].as_str(), row[2].as_str(), "--format", "machine"],
            );
            assert_eq!(
                exit_code(&output),
                0,
                "{case}: an accepted selector must succeed. stderr={}",
                String::from_utf8_lossy(&output.stderr)
            );
            let report = serde_json::from_slice::<serde_json::Value>(&output.stdout)
                .expect("machine trace report");
            let rows = report["rows"].as_array().expect("trace rows");
            assert_eq!(
                rows.len(),
                row[3].parse::<usize>().expect("declared row count"),
                "{case}: selected row count"
            );
        }
    }

    #[test]
    fn trace_command_rejects_absent_conflicting_malformed_and_unresolved_selectors() {
        let root = staged_repository();
        for row in case_rows("selectors/negative.tsv") {
            let case = row[0].as_str();
            let expected = row[4].parse::<i32>().expect("declared exit class");
            let arguments = if row[2].as_str() == "none" {
                Vec::new()
            } else {
                row[2]
                    .split(' ')
                    .zip(row[3].split(' '))
                    .flat_map(|(flag, value)| [flag, value])
                    .collect::<Vec<_>>()
            };
            let output = run_trace(root.path(), &arguments);
            assert_eq!(
                exit_code(&output),
                expected,
                "{case}: unexpected exit class. stderr={}",
                String::from_utf8_lossy(&output.stderr)
            );
            assert!(
                output.stdout.is_empty(),
                "{case}: a rejected selector must emit no projection"
            );
            if case.starts_with("unresolved-") {
                assert!(
                    String::from_utf8_lossy(&output.stderr).contains("TRC-SELECTOR-NOT-FOUND"),
                    "{case}: an absent object must emit the stable diagnostic"
                );
            }
        }
    }

    #[test]
    fn relation_states_report_current_historical_and_stale_results() {
        for row in case_rows("relations/cases.tsv") {
            let case = row[0].as_str();
            let diagnostic = row[5].as_str();
            let root = staged_repository();
            apply_mutation(root.path(), row[1].as_str());
            let mut arguments = vec![
                "--relation-id",
                "BGLC-RENDER-CATALOG",
                "--format",
                "machine",
            ];
            if row[3].as_str() == "true" {
                arguments.push("--include-history");
            }
            let output = run_trace(root.path(), &arguments);
            assert_eq!(
                exit_code(&output),
                row[4].parse::<i32>().expect("declared exit class"),
                "{case}: unexpected exit class. stderr={}",
                String::from_utf8_lossy(&output.stderr)
            );
            assert!(
                !output.stdout.is_empty(),
                "{case}: a materialized query must emit one projection"
            );
            if diagnostic != "none" {
                let rendered = String::from_utf8_lossy(&output.stdout);
                assert!(
                    rendered.contains(diagnostic),
                    "{case}: the projection must retain {diagnostic}"
                );
            }
        }
    }

    #[test]
    fn implementation_path_query_orders_current_before_historical_rows() {
        let root = staged_repository();
        apply_mutation(root.path(), "rebind-relation");
        let output = run_trace(
            root.path(),
            &[
                "--implementation-path",
                "src/lib.rs",
                "--include-history",
                "--format",
                "machine",
            ],
        );
        assert_eq!(
            exit_code(&output),
            0,
            "the mixed current and historical query must succeed"
        );
        let report = serde_json::from_slice::<serde_json::Value>(&output.stdout)
            .expect("mixed trace report");
        let rows = report["rows"].as_array().expect("mixed trace rows");
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0]["state"], "current");
        assert_eq!(rows[1]["state"], "historical");
        assert_eq!(rows[0]["relation"], "BGLC-RENDER-CATALOG-V2");
        assert_eq!(rows[1]["relation"], "BGLC-RENDER-CATALOG");
    }

    /// Edits one staged snapshot to create the declared relation state.
    fn apply_mutation(root: &Path, mutation: &str) {
        match mutation {
            "none" => {}
            "edit-requirement-bytes" => {
                let path = root.join("openspec/specs/demo/spec.md");
                let text = std::fs::read_to_string(&path).expect("staged specification");
                let edited = text.replace("in stable order", "in one stable order");
                assert_ne!(text, edited, "the requirement edit must change bytes");
                std::fs::write(&path, edited).expect("edited specification");
            }
            "edit-implementation-bytes" => {
                let path = root.join("src/lib.rs");
                let text = std::fs::read_to_string(&path).expect("staged implementation");
                let edited = text.replace("libraries.join(\"\\n\")", "libraries.concat()");
                assert_ne!(text, edited, "the implementation edit must change bytes");
                std::fs::write(&path, edited).expect("edited implementation");
            }
            // One replacement relation moves the compact attribute to a new
            // record. The staged record keeps its bytes and loses its current
            // attachment, so it becomes retained history.
            "rebind-relation" => {
                let output = Command::new(env!("CARGO_BIN_EXE_buoy"))
                    .args([
                        "bind",
                        "--root",
                        root.to_str().expect("UTF-8 root"),
                        "--config",
                        root.join("buoy.toml").to_str().expect("UTF-8 config"),
                        "--requirement",
                        "blake3-256:a07ec0fdd5f3250af9d18ba6025f9788ccb2f98df6c4538d16e0c695ccff34b7",
                        "--implementation",
                        "blake3-256:686f70dca776da0eb0c9159ffb1eb1eb04adeb83ecb8e81f3435d1dedbd71915",
                        "--relation",
                        "BGLC-RENDER-CATALOG-V2",
                        "--format",
                        "machine",
                    ])
                    .output()
                    .expect("rebind command");
                assert!(
                    output.status.success(),
                    "the history case requires one replacement record; stderr={}",
                    String::from_utf8_lossy(&output.stderr)
                );
            }
            other => panic!("unknown relation mutation: {other}"),
        }
    }

    #[test]
    fn machine_view_satisfies_its_reviewed_contract() {
        let root = staged_repository();
        let output = run_trace(
            root.path(),
            &[
                "--relation-id",
                "BGLC-RENDER-CATALOG",
                "--format",
                "machine",
            ],
        );
        assert_eq!(exit_code(&output), 0, "the machine view must render");
        let rendered = String::from_utf8(output.stdout).expect("UTF-8 machine output");
        let report = serde_json::from_str::<serde_json::Value>(&rendered)
            .expect("the machine view must be deterministic JSON");
        assert_eq!(
            report.get("schema").and_then(serde_json::Value::as_str),
            Some("buoy/trace-report/v1")
        );
        for row in case_rows("contracts/machine.tsv") {
            match row[0].as_str() {
                "required" => assert!(
                    report.get(&row[1]).is_some(),
                    "the machine view must contain {}",
                    row[1]
                ),
                "required-row" => {
                    let rows = report
                        .get("rows")
                        .and_then(serde_json::Value::as_array)
                        .expect("the machine view must contain rows");
                    assert!(
                        rows.iter().all(|entry| entry.get(&row[1]).is_some()),
                        "each machine row must contain {}",
                        row[1]
                    );
                }
                "forbidden" => assert!(
                    !rendered.contains(&row[1]),
                    "the machine view must exclude {}",
                    row[1]
                ),
                _ => {}
            }
        }
    }

    #[test]
    fn human_view_satisfies_its_reviewed_contract() {
        let root = staged_repository();
        let output = run_trace(
            root.path(),
            &["--relation-id", "BGLC-RENDER-CATALOG", "--format", "human"],
        );
        assert_eq!(exit_code(&output), 0, "the human view must render");
        let rendered = String::from_utf8(output.stdout).expect("UTF-8 human output");
        for row in case_rows("contracts/human.tsv") {
            match row[0].as_str() {
                "required" => assert!(
                    rendered.contains(&row[1]),
                    "the human view must contain {}",
                    row[1]
                ),
                "forbidden" => assert!(
                    !rendered.contains(&row[1]),
                    "the human view must exclude {}",
                    row[1]
                ),
                _ => {}
            }
        }
    }

    #[test]
    fn markdown_view_satisfies_its_reviewed_contract() {
        let root = staged_repository();
        let specification = root.path().join("openspec/specs/demo/spec.md");
        let before = std::fs::read(&specification).expect("staged authority bytes");
        let output = run_trace(
            root.path(),
            &[
                "--relation-id",
                "BGLC-RENDER-CATALOG",
                "--format",
                "markdown",
            ],
        );
        assert_eq!(exit_code(&output), 0, "the Markdown view must render");
        let rendered = String::from_utf8(output.stdout).expect("UTF-8 Markdown output");
        for row in case_rows("contracts/markdown.tsv") {
            match row[0].as_str() {
                "required" => assert!(
                    rendered.contains(&row[1]),
                    "the Markdown view must contain {}",
                    row[1]
                ),
                "forbidden" => assert!(
                    !rendered.contains(&row[1]),
                    "the Markdown view must exclude {}",
                    row[1]
                ),
                _ => {}
            }
        }
        let after = std::fs::read(&specification).expect("staged authority bytes");
        assert_eq!(before, after, "a projection must not write an authority");
    }

    /// Format parsing is command-specific, so Markdown reaches `trace` only.
    #[test]
    fn only_the_trace_command_accepts_the_markdown_format() {
        let root = staged_repository();
        let config = root.path().join("buoy.toml");
        for command in ["static", "execute"] {
            let output = Command::new(env!("CARGO_BIN_EXE_buoy"))
                .args([
                    command,
                    "--root",
                    root.path().to_str().expect("UTF-8 root"),
                    "--config",
                    config.to_str().expect("UTF-8 config"),
                    "--format",
                    "markdown",
                ])
                .output()
                .expect("existing command");
            assert_eq!(
                exit_code(&output),
                70,
                "{command}: an unknown format stays an input failure"
            );
        }
        let unknown = run_trace(
            root.path(),
            &["--relation-id", "BGLC-RENDER-CATALOG", "--format", "json"],
        );
        assert_eq!(
            exit_code(&unknown),
            64,
            "an unknown trace format is a usage failure"
        );
        assert!(unknown.stdout.is_empty());
    }

    /// A boolean flag stays explicit and rejects a repeated declaration.
    #[test]
    fn a_repeated_history_flag_is_a_usage_failure() {
        let root = staged_repository();
        let output = run_trace(
            root.path(),
            &[
                "--relation-id",
                "BGLC-RENDER-CATALOG",
                "--include-history",
                "--include-history",
            ],
        );
        assert_eq!(exit_code(&output), 64);
        assert!(output.stdout.is_empty());
    }

    /// A single-root configuration rejects a logical mount mapping.
    #[test]
    fn a_mount_mapping_without_a_mounted_configuration_fails_closed() {
        let root = staged_repository();
        let config = root.path().join("buoy.toml");
        let output = Command::new(env!("CARGO_BIN_EXE_buoy"))
            .args([
                "trace",
                "--mount",
                &format!("project={}", root.path().to_str().expect("UTF-8 root")),
                "--config",
                config.to_str().expect("UTF-8 config"),
                "--relation-id",
                "BGLC-RENDER-CATALOG",
            ])
            .output()
            .expect("trace command");
        assert_eq!(exit_code(&output), 70);
        assert!(output.stdout.is_empty());
    }

    /// A historical-only query stays successful and emits its projection.
    #[test]
    fn a_historical_only_record_query_returns_success() {
        let root = staged_repository();
        apply_mutation(root.path(), "rebind-relation");
        let output = run_trace(
            root.path(),
            &[
                "--record",
                "blake3-256:0e857e33e387325fbf457e426bc42286d9d99926b19f3efa3c203795cfeeb6c4",
                "--format",
                "machine",
            ],
        );
        assert_eq!(
            exit_code(&output),
            0,
            "history never changes a successful exit class; stderr={}",
            String::from_utf8_lossy(&output.stderr)
        );
        let rendered = String::from_utf8(output.stdout).expect("UTF-8 machine output");
        assert!(rendered.contains("\"state\":\"historical\""));
    }

    #[test]
    fn trace_query_does_not_run_oracles_or_modify_repository_bytes() {
        let root = staged_repository();
        let config = root.path().join("buoy.toml");
        let mut configuration = std::fs::read_to_string(&config).expect("trace configuration");
        configuration.push_str(
            "\n[[oracles]]\nid = \"TRACE-MUST-NOT-RUN\"\nprogram = \"builtin:fail\"\ntimeout_ms = 10\n",
        );
        std::fs::write(&config, configuration).expect("trace oracle control");
        let before = FIXTURE_FILES
            .iter()
            .map(|relative| {
                (
                    *relative,
                    std::fs::read(root.path().join(relative)).expect("fixture bytes"),
                )
            })
            .collect::<Vec<_>>();
        let output = run_trace(
            root.path(),
            &[
                "--relation-id",
                "BGLC-RENDER-CATALOG",
                "--format",
                "machine",
            ],
        );
        assert_eq!(
            exit_code(&output),
            0,
            "a failing configured oracle must remain unexecuted"
        );
        let rendered = String::from_utf8(output.stdout).expect("machine trace report");
        assert!(!rendered.contains("provider"));
        for (relative, expected) in before {
            assert_eq!(
                std::fs::read(root.path().join(relative)).expect("fixture bytes after query"),
                expected,
                "trace modified {relative}"
            );
        }
    }

    #[test]
    fn equal_logical_snapshots_under_separate_roots_produce_equal_reports() {
        let first = staged_repository();
        let second = staged_repository();
        let arguments = [
            "--relation-id",
            "BGLC-RENDER-CATALOG",
            "--format",
            "machine",
        ];
        let left = run_trace(first.path(), &arguments);
        let right = run_trace(second.path(), &arguments);
        assert_eq!(exit_code(&left), 0, "the first snapshot must render");
        assert_eq!(exit_code(&right), 0, "the second snapshot must render");
        assert_eq!(
            left.stdout, right.stdout,
            "two physical roots must produce equal report bytes"
        );
    }

    #[test]
    fn human_digest_abbreviation_expands_on_a_shared_prefix() {
        let cases = case_rows("collisions/cases.tsv");
        assert!(
            cases
                .iter()
                .any(|row| row[4] == "complete" && row[3] != "0"),
            "one reviewed case must require complete digests"
        );
        let root = staged_repository();
        let output = run_trace(
            root.path(),
            &["--relation-id", "BGLC-RENDER-CATALOG", "--format", "human"],
        );
        assert_eq!(exit_code(&output), 0, "the human view must render");
        let rendered = String::from_utf8(output.stdout).expect("UTF-8 human output");
        assert!(
            rendered.contains("0e857e33"),
            "the human view must display the record digest prefix"
        );
        assert!(
            !rendered.contains(
                "blake3-256:0e857e33e387325fbf457e426bc42286d9d99926b19f3efa3c203795cfeeb6c4"
            ),
            "one distinct prefix must stay abbreviated"
        );
    }
}

/// The reviewed record that this change selects no formal proof lane.
mod formal_lane {
    use super::repository_root;

    #[test]
    fn the_change_records_no_verus_or_aeneas_lane() {
        let record = std::fs::read_to_string(
            repository_root().join("docs/assurance/readable-trace-projections-formal-lane.md"),
        )
        .expect("the formal-lane record must exist");
        for required in [
            "This change selects no Verus proof lane",
            "It also selects no AeneasVerif proof lane",
            "No formal property, proof record, derivation record, or correspondence approval changes",
            "do not establish proof",
        ] {
            assert!(
                record.contains(required),
                "the formal-lane record must state: {required}"
            );
        }

        // The record must not promote an existing theorem or obligation.
        for forbidden in ["formally-proved", "satisfied"] {
            assert!(
                !record.contains(forbidden),
                "the formal-lane record must not claim {forbidden}"
            );
        }
    }

    #[test]
    fn no_proof_binding_changes_for_this_change() {
        let proofs = repository_root().join("proofs");
        for relative in [
            "core/proof-record.toml",
            "core/derivation-record.toml",
            "core/correspondence-approval.toml",
        ] {
            let text = std::fs::read_to_string(proofs.join(relative))
                .unwrap_or_else(|error| panic!("{relative}: {error}"));
            for absent in ["trace-report", "readable-trace", "standing-spec/v2"] {
                assert!(
                    !text.contains(absent),
                    "{relative}: this change binds no proof artifact for {absent}"
                );
            }
        }
    }
}

/// Cross-adapter contracts for equal external trace fence bytes.
mod cross_adapter {
    use super::{fixture_root, repository_root};
    use buoy::adapters::{
        AuthorityExpectation, OpenSpecConfiguration, extract_markdown, extract_openspec_configured,
    };
    use buoy::core::RepositoryPath;

    fn openspec_version_two() -> OpenSpecConfiguration {
        let bytes = std::fs::read(
            repository_root().join("fixtures/openspec/v2/positive/configuration.toml"),
        )
        .expect("version 2 configuration bytes");
        OpenSpecConfiguration::parse(&bytes)
            .expect("the adapter must accept openspec/standing-spec/v2")
    }

    #[test]
    fn equal_fence_bytes_produce_equal_links_in_both_adapters() {
        let configuration = openspec_version_two();
        let markdown_path = RepositoryPath::new("docs/markdown-valid.md").expect("Markdown path");
        let markdown_bytes = std::fs::read(fixture_root().join("cross-adapter/markdown-valid.md"))
            .expect("Markdown carrier");
        let markdown = extract_markdown(
            &markdown_path,
            &markdown_bytes,
            AuthorityExpectation::Optional,
        );
        assert!(markdown.diagnostics.is_empty());
        assert_eq!(markdown.external_links.len(), 1);

        let spec_path = RepositoryPath::new("openspec/specs/demo/spec.md").expect("spec path");
        let spec_bytes =
            std::fs::read(repository_root().join("fixtures/openspec/v2/positive/standing-spec.md"))
                .expect("version 2 standing spec");
        let openspec = extract_openspec_configured(&spec_path, &spec_bytes, &configuration);
        assert!(
            openspec.diagnostics.is_empty(),
            "the version 2 adapter must accept a later trace section: {:?}",
            openspec.diagnostics
        );
        assert_eq!(openspec.external_links.len(), 1);

        // The complete link values must agree, field for field and by digest.
        let markdown_link = &markdown.external_links[0];
        let openspec_link = &openspec.external_links[0];
        assert_eq!(
            markdown_link.value, openspec_link.value,
            "equal fence bytes must produce equal link values"
        );
        assert_eq!(
            markdown_link.value.meta().digest(),
            openspec_link.value.meta().digest()
        );
        assert_eq!(
            markdown_link.value.meta().id(),
            openspec_link.value.meta().id()
        );

        // Each host keeps its own parser-backed item and exact source mapping.
        assert_ne!(
            markdown_link.item.digest(),
            openspec_link.item.digest(),
            "each host adapter owns its own item identity"
        );
        assert_eq!(openspec_link.item.path(), &spec_path);
        assert_eq!(openspec_link.span.path, spec_path);
        assert_eq!(markdown_link.item.path(), &markdown_path);
        assert_eq!(
            openspec_link.item.kind().as_str(),
            "openspec/external-trace/v1"
        );
        assert_eq!(
            markdown_link.item.kind().as_str(),
            "markdown/external-trace/v1"
        );

        // The exact span must select the fence bytes inside the source file.
        let start = usize::try_from(openspec_link.span.start).expect("bounded span start");
        let end = usize::try_from(openspec_link.span.end).expect("bounded span end");
        let selected = std::str::from_utf8(&spec_bytes[start..end]).expect("UTF-8 fence");
        assert!(selected.starts_with("```buoy-external-trace-v1"));
        assert!(selected.contains("TRACE-DEMO-PAGE"));
    }

    #[test]
    fn malformed_fence_bytes_fail_with_one_shared_primary_diagnostic() {
        let configuration = openspec_version_two();
        let markdown_path =
            RepositoryPath::new("docs/markdown-malformed.md").expect("Markdown path");
        let markdown_bytes =
            std::fs::read(fixture_root().join("cross-adapter/markdown-malformed.md"))
                .expect("Markdown carrier");
        let markdown = extract_markdown(
            &markdown_path,
            &markdown_bytes,
            AuthorityExpectation::Optional,
        );
        assert_eq!(markdown.diagnostics.len(), 1);
        let markdown_code = markdown.diagnostics[0].code;

        let spec_path = RepositoryPath::new("openspec/specs/demo/spec.md").expect("spec path");
        let spec_bytes = std::fs::read(
            repository_root().join("fixtures/openspec/v2/negative/malformed-trace.md"),
        )
        .expect("version 2 malformed fixture");
        let openspec = extract_openspec_configured(&spec_path, &spec_bytes, &configuration);
        assert!(
            openspec
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == markdown_code),
            "both adapters must report {markdown_code}, found {:?}",
            openspec.diagnostics
        );
        // The OpenSpec adapter fails closed, so no partial product survives.
        assert!(openspec.external_links.is_empty());
        assert!(openspec.source_nodes.is_empty());
        assert!(openspec.semantic.is_empty());
        assert!(openspec.document_root.is_none());
    }
}

/// Workflow template contracts for planned rows and completion snapshots.
mod workflow {
    use super::traceability_control::{TraceabilityFault, validate_traceability};
    use super::{case_rows, fixture_root, repository_root};

    fn template() -> String {
        std::fs::read_to_string(
            repository_root().join("openspec/schemas/buoy-bridge/templates/traceability.md"),
        )
        .expect("traceability template")
    }

    #[test]
    fn template_declares_planned_relation_and_completion_fields() {
        let template = template();
        for required in [
            "Relation ID",
            "Consumer",
            "Logical target selector",
            "Focused Buoy command",
            "Completion Snapshot",
            "Consumer commit",
            "Review decision",
            "<!-- BEGIN buoy-trace-publication-evidence-v1 -->",
            "Buoy remains the current-state authority",
        ] {
            assert!(
                template.contains(required),
                "the traceability template must declare {required}"
            );
        }
    }

    #[test]
    fn reviewed_workflow_artifacts_match_their_declared_disposition() {
        for row in case_rows("workflow/cases.tsv") {
            let (case, artifact, accepted, expected) =
                (&row[0], &row[1], row[2].as_str(), row[3].as_str());
            let text = std::fs::read_to_string(fixture_root().join(artifact))
                .unwrap_or_else(|error| panic!("{case}: {artifact}: {error}"));
            let faults = validate_traceability(&text)
                .into_iter()
                .map(TraceabilityFault::name)
                .collect::<Vec<_>>();
            match accepted {
                "true" => assert!(
                    faults.is_empty(),
                    "{case}: an accepted artifact carries no fault, found {faults:?}"
                ),
                "false" => assert!(
                    faults.contains(&expected),
                    "{case}: the control must report {expected}, found {faults:?}"
                ),
                other => panic!("{case}: unknown disposition {other}"),
            }
        }
    }

    /// The archived change artifact must satisfy the reviewed workflow control.
    #[test]
    fn the_archived_change_artifact_satisfies_the_workflow_control() {
        let text = std::fs::read_to_string(repository_root().join(
            "openspec/changes/archive/2026-08-21-add-readable-buoy-trace-projections/traceability.md",
        ))
        .expect("the archived traceability artifact");
        assert_eq!(
            validate_traceability(&text)
                .into_iter()
                .map(TraceabilityFault::name)
                .collect::<Vec<_>>(),
            Vec::<&str>::new()
        );
    }
}

// ---------------------------------------------------------------------------
// RTP-007 round-4 controls: publication projection conformance.
//
// The audit found RTP-007's ambiguous-graph-label scenario had no diagnostic
// and no conformance entry point at all. `validate_markdown_projection` is
// that entry point; it runs over RENDERED bytes rather than the typed report,
// so it catches a renderer that silently drops a required label or facet even
// though `buoy/trace-report/v1` still carries it -- exactly the drift an
// immutable publication basis must never key.
// ---------------------------------------------------------------------------

/// One complete, conforming projection. Every negative control below mutates
/// exactly one aspect of this same document, so each proves its own specific
/// rejection rather than passing because the document was malformed anyway.
fn conforming_projection() -> String {
    String::from(
        "# Implementation trace\n\n\
Derived data. Buoy remains the current-state authority. Regenerate this page from a current query.\n\n\
- Renderer schema: `buoy/readable-trace-markdown/v1`\n\
- Trace-report schema: `buoy/trace-report/v1`\n\
- Selector: `requirement-id BGLC-003`\n\
- Source graph: `blake3-256:1111111111111111111111111111111111111111111111111111111111111111`\n\
- History included: false\n\n\
## Requirement\n\n- `BGLC-003` at openspec/specs/demo/spec.md:1-10\n\n\
## Implementation\n\n- src/lib.rs:1-5 (`rust/function/v1`)\n\n\
## Relations\n\n\
| State | Relation | Navigation | Record | Requirement | Atom revision | Requirement path | Implementation path | Freshness |\n\
|---|---|---|---|---|---|---|---|---|\n\
| current | `REL-1` | navigation-only | `blake3-256:2222` | `BGLC-003` | `blake3-256:3333` | openspec/specs/demo/spec.md | src/lib.rs | record=true attachment=true implementation=true requirement=true authority=true |\n\n\
## Boundary\n\nThis page supplies navigation and impact only. It establishes no coverage and proves nothing.\n",
    )
}

#[test]
fn a_conforming_projection_passes_publication_projection_conformance() {
    buoy::publication::validate_markdown_projection(&conforming_projection())
        .expect("the conforming projection must pass");
}

/// RTP-007 negative: "one Markdown projection that labels a source-graph
/// digest only as `Graph`" must fail with a deterministic ambiguous-graph
/// -label diagnostic. A bare `Graph:` names no domain, so a reader cannot
/// tell a source graph from a binding graph or a page graph.
#[test]
fn an_ambiguous_graph_label_fails_conformance_with_its_stable_code() {
    let ambiguous = conforming_projection().replace("- Source graph: `", "- Graph: `");
    let error = buoy::publication::validate_markdown_projection(&ambiguous)
        .expect_err("an ambiguous graph label must fail conformance");
    assert_eq!(error.stable_code(), "RTP-AMBIGUOUS-GRAPH-LABEL");
}

/// A projection that carries BOTH the unambiguous and the ambiguous label is
/// still ambiguous: detecting only the ABSENCE of `Source graph` would miss
/// this entirely.
#[test]
fn carrying_both_graph_labels_is_still_ambiguous() {
    let both = conforming_projection().replace(
        "- History included: false",
        "- Graph: `blake3-256:9999`\n- History included: false",
    );
    let error = buoy::publication::validate_markdown_projection(&both)
        .expect_err("a projection carrying both labels must still fail");
    assert_eq!(error.stable_code(), "RTP-AMBIGUOUS-GRAPH-LABEL");
}

/// RTP-007 negative: "one current relation whose Markdown row omits its
/// complete semantic atom revision" must fail conformance while
/// `buoy/trace-report/v1` itself remains unchanged.
#[test]
fn a_row_missing_its_atom_revision_fails_conformance() {
    let missing = conforming_projection()
        .replace(" Atom revision |", " |")
        .replace(
            "|---|---|---|---|---|---|---|---|---|",
            "|---|---|---|---|---|---|---|---|",
        );
    let error = buoy::publication::validate_markdown_projection(&missing)
        .expect_err("a row omitting the atom revision must fail conformance");
    assert_eq!(error.stable_code(), "RTP-MISSING-ATOM-REVISION");
    // The machine schema is untouched by a renderer defect.
    assert!(missing.contains("buoy/trace-report/v1"));
}

/// Every freshness facet must survive into the rendered row. `attachment` is
/// a real, independent facet: a binding whose compact locator no longer
/// resolves is attachment-stale even when its record, implementation,
/// requirement, and authority are all current. Dropping it silently loses a
/// fact the machine report already carries.
#[test]
fn dropping_each_freshness_facet_is_detected() {
    for facet in [
        "record",
        "attachment",
        "implementation",
        "requirement",
        "authority",
    ] {
        let dropped = conforming_projection()
            .replace(&format!("{facet}=true "), "")
            .replace(&format!(" {facet}=true"), "");
        let error = buoy::publication::validate_markdown_projection(&dropped)
            .expect_err("dropping a freshness facet must fail conformance");
        assert_eq!(
            error.stable_code(),
            "RTP-MISSING-FRESHNESS-FACET",
            "dropping `{facet}` must be detected"
        );
    }
}

#[test]
fn a_projection_missing_a_logical_location_fails_conformance() {
    let missing = conforming_projection().replace(" Implementation path |", " |");
    let error = buoy::publication::validate_markdown_projection(&missing)
        .expect_err("a row omitting a logical location must fail conformance");
    assert_eq!(error.stable_code(), "RTP-MISSING-LOGICAL-LOCATION");
}

#[test]
fn a_projection_missing_a_schema_identity_fails_conformance() {
    for schema in ["buoy/readable-trace-markdown/v1", "buoy/trace-report/v1"] {
        let missing = conforming_projection().replace(schema, "buoy/some-other-schema/v9");
        let error = buoy::publication::validate_markdown_projection(&missing)
            .expect_err("a projection omitting a schema identity must fail conformance");
        assert_eq!(error.stable_code(), "RTP-MISSING-SCHEMA-IDENTITY");
    }
}

#[test]
fn a_projection_dropping_the_assurance_boundary_fails_conformance() {
    for boundary in [
        "Derived data",
        "navigation and impact only",
        "Buoy remains the current-state authority",
    ] {
        let dropped = conforming_projection().replace(boundary, "");
        let error = buoy::publication::validate_markdown_projection(&dropped)
            .expect_err("a projection dropping the assurance boundary must fail conformance");
        assert_eq!(error.stable_code(), "RTP-MISSING-BOUNDARY");
    }
}
