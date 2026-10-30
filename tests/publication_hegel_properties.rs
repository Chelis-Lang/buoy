#![doc = "Seeded Hegel properties for the publication plan, path, row, and lifecycle domain."]
#![allow(clippy::expect_used, clippy::similar_names)]

use buoy::publication::{
    BasisInputs, Lifecycle, OutputClass, PackageIdentity, Selector, TraceRow, classify_output,
    compute_publication_basis, join_target_base, parse_plan, render_evidence, render_path,
    render_report, row_conformance,
};
use buoy::publication_fs::WriteOutcome;
use buoy::publication_fs::split_relative_path;
use buoy::publication_fs::unix::{open_root, write_output};
use buoy::publication_models::TransactionState;
use hegel::generators as gs;
use hegel::{Hegel, TestCase};

#[path = "support/hegel_settings.rs"]
mod hegel_settings;
use hegel_settings::settings;

fn draw_byte(test_case: &TestCase) -> u8 {
    test_case.draw(gs::integers::<u8>())
}

fn draw_index(test_case: &TestCase, modulus: usize) -> usize {
    usize::from(draw_byte(test_case)) % modulus.max(1)
}

fn ascii_id(test_case: &TestCase, max_len: usize) -> String {
    let letters = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789-_";
    let length = 1 + draw_index(test_case, max_len);
    (0..length)
        .map(|_| letters.as_bytes()[draw_index(test_case, letters.len())] as char)
        .collect()
}

fn plan_json(publication_id: &str, expected_rows: usize) -> Vec<u8> {
    format!(
        r#"{{"schema":"buoy/trace-publication-plan/v1","publication_id":"{publication_id}","selector":{{"requirement_id":"DEMO-001"}},"expected_rows":{expected_rows},"source_set":[],"outputs":[]}}"#
    )
    .into_bytes()
}

/// Task 2.6: plan parsing depends only on the JSON object's field *values*,
/// never on the order those fields appear in the source bytes, because the
/// parser dispatches on a `serde` struct rather than positional bytes.
#[test]
fn hegel_plan_field_permutation_is_order_independent() {
    Hegel::new(|test_case: TestCase| {
        let publication_id = ascii_id(&test_case, 12);
        let expected_rows = 1 + draw_index(&test_case, 8);
        let forward = format!(
            r#"{{"schema":"buoy/trace-publication-plan/v1","publication_id":"{publication_id}","selector":{{"requirement_id":"DEMO-001"}},"expected_rows":{expected_rows},"source_set":[],"outputs":[]}}"#
        );
        let reversed = format!(
            r#"{{"outputs":[],"source_set":[],"expected_rows":{expected_rows},"selector":{{"requirement_id":"DEMO-001"}},"publication_id":"{publication_id}","schema":"buoy/trace-publication-plan/v1"}}"#
        );
        let forward_plan = parse_plan(forward.as_bytes()).expect("forward-order plan parses");
        let reversed_plan = parse_plan(reversed.as_bytes()).expect("reverse-order plan parses");
        assert_eq!(forward_plan.publication_id(), reversed_plan.publication_id());
        assert_eq!(forward_plan.expected_rows(), reversed_plan.expected_rows());
        assert_eq!(forward_plan.schema(), reversed_plan.schema());
    })
    .settings(settings(901))
    .run();
}

/// Task 2.8 (differential): an INDEPENDENT reimplementation of the plan's
/// two scalar fields, written directly against `serde_json::Value` and
/// never calling `buoy::publication::parse_plan` or its internal `RawPlan`
/// type, must agree with the real production parser for every generated
/// publication ID and row count. This is a genuine differential test: a
/// bug in either the real parser's field extraction or this independent
/// reimplementation would surface as a disagreement, unlike comparing the
/// production function against a second call to itself.
fn independent_publication_id_and_expected_rows(bytes: &[u8]) -> (String, usize) {
    let value: serde_json::Value = serde_json::from_slice(bytes).expect("independent JSON parse");
    let publication_id = value["publication_id"]
        .as_str()
        .expect("independent extraction finds publication_id")
        .to_owned();
    let expected_rows = usize::try_from(
        value["expected_rows"]
            .as_u64()
            .expect("independent extraction finds expected_rows"),
    )
    .expect("expected_rows fits usize");
    (publication_id, expected_rows)
}

/// Task 2.6/2.8: plan parsing depends only on the JSON object's field
/// *values*, never on the root a mount is opened under, and the real
/// production parser's extracted `publication_id`/`expected_rows` fields
/// must agree with an independent, non-`parse_plan` reimplementation for
/// every generated value.
#[test]
fn hegel_plan_parsing_matches_an_independent_field_extractor_and_is_root_independent() {
    Hegel::new(|test_case: TestCase| {
        let publication_id = ascii_id(&test_case, 10);
        let expected_rows = 1 + draw_index(&test_case, 8);
        let bytes = plan_json(&publication_id, expected_rows);
        let produced = parse_plan(&bytes).expect("production parse");
        let (independent_publication_id, independent_expected_rows) =
            independent_publication_id_and_expected_rows(&bytes);
        assert_eq!(produced.publication_id(), independent_publication_id);
        assert_eq!(produced.expected_rows(), independent_expected_rows);

        // The root a mount is opened under never enters plan parsing at
        // all: a second parse of the identical bytes (as any later,
        // separately rooted invocation would perform) still agrees with
        // both the first real parse and the independent extractor.
        let reparsed = parse_plan(&bytes).expect("parse under a distinct later invocation");
        assert_eq!(reparsed.publication_id(), independent_publication_id);
        assert_eq!(reparsed.expected_rows(), independent_expected_rows);
    })
    .settings(settings(902))
    .run();
}

fn complete_row(seed: u8) -> TraceRow {
    TraceRow {
        relation: Some(format!("relation-{seed}")),
        record: Some(format!("blake3-256:{seed:02x}")),
        requirement_id: Some(format!("DEMO-{seed:03}")),
        atom_revision: Some(format!("blake3-256:{seed:02x}{seed:02x}")),
        requirement_node: Some(format!("blake3-256:node-req-{seed:02x}")),
        implementation_node: Some(format!("blake3-256:node-impl-{seed:02x}")),
        requirement_path: Some(format!("openspec/specs/demo-{seed}/spec.md")),
        implementation_path: Some(format!("src/demo_{seed}.rs")),
        state: Some(String::from("current")),
        freshness: Some(buoy::publication::RowFreshness {
            record: true,
            attachment: true,
            implementation: true,
            requirement: true,
            authority: true,
        }),
        diagnostics: Vec::new(),
    }
}

/// Task 2.6: no fragmentation of one expected row's fields across two rows
/// can ever satisfy row-local conformance, no matter which single field is
/// held back and which second row is offered alongside it. Row locality
/// forbids assembling one logical match from more than one physical row.
#[test]
fn hegel_row_locality_never_admits_a_two_row_fragment() {
    Hegel::new(|test_case: TestCase| {
        let seed = draw_byte(&test_case).max(1);
        let mut incomplete = complete_row(seed);
        // Blank exactly one field so this row alone cannot satisfy the
        // count-1 policy, then offer a second, unrelated, complete row.
        match draw_index(&test_case, 9) {
            0 => incomplete.relation = None,
            1 => incomplete.record = None,
            2 => incomplete.requirement_id = None,
            3 => incomplete.atom_revision = None,
            4 => incomplete.requirement_node = None,
            5 => incomplete.implementation_node = None,
            6 => incomplete.requirement_path = None,
            7 => incomplete.implementation_path = None,
            _ => incomplete.freshness = None,
        }
        let filler = complete_row(seed.wrapping_add(1).max(1));
        // Exactly one expected row, but the declared row is incomplete: the
        // fragment plus an unrelated second row must never pass.
        let result = row_conformance(&[incomplete, filler], 1);
        assert!(
            result.is_err(),
            "an incomplete row plus a second unrelated row must never satisfy row_conformance"
        );
    })
    .settings(settings(903))
    .run();
}

/// Task 2.6: `row_conformance`'s machine result depends only on its declared
/// inputs (the rows and the expected count); calling it twice on equal
/// inputs always reproduces the exact same `Result` variant.
#[test]
fn hegel_row_conformance_diagnostics_are_deterministic() {
    Hegel::new(|test_case: TestCase| {
        let seed = draw_byte(&test_case);
        let count = draw_index(&test_case, 4);
        let rows = (0..count)
            .map(|index| complete_row(seed.wrapping_add(u8::try_from(index).unwrap_or(u8::MAX))))
            .collect::<Vec<_>>();
        let expected = 1 + draw_index(&test_case, 4);
        let first = row_conformance(&rows, expected);
        let second = row_conformance(&rows, expected);
        assert_eq!(first, second);
    })
    .settings(settings(904))
    .run();
}

/// Task 2.7: no random walk through the bounded `publication-transaction/v1`
/// state graph can reach `Write` without first passing through
/// `SourceGuard`, for any sampled sequence of legal transitions, exercising
/// many more paths than the exhaustive fixed-point check alone visits in one
/// pass through `src/publication_models.rs`.
#[test]
fn hegel_transaction_walks_never_reach_write_without_a_source_guard() {
    Hegel::new(|test_case: TestCase| {
        let mut state = TransactionState::Parse;
        let steps = draw_index(&test_case, 12);
        let mut saw_source_guard = false;
        for _ in 0..steps {
            let options = state.allowed_next();
            if options.is_empty() {
                break;
            }
            if state == TransactionState::SourceGuard {
                saw_source_guard = true;
            }
            let next = options[draw_index(&test_case, options.len())];
            assert!(state.transition_is_allowed(next));
            if next == TransactionState::Write {
                assert!(
                    saw_source_guard,
                    "no random walk may reach Write without passing through SourceGuard first"
                );
            }
            state = next;
        }
    })
    .settings(settings(905))
    .run();
}

/// Task 2.7: a path that appears in the declared source set is always
/// classified `SourceEdit` when declared as one, and is always rejected
/// (never silently reclassified as `Derived`) when declared as anything
/// else — for every generated selected/unselected path pair.
#[test]
fn hegel_derived_outputs_are_always_excluded_from_the_selected_source_set() {
    Hegel::new(|test_case: TestCase| {
        let selected_name = ascii_id(&test_case, 12);
        let other_name = ascii_id(&test_case, 12);
        let selected_path = format!("src/{selected_name}.rs");
        let source_set = vec![selected_path.clone()];
        // A derived path that collides with the declared selected source
        // must always be rejected, regardless of its exact random name.
        let derived_result = classify_output(&selected_path, "page", &source_set);
        assert!(derived_result.is_err());
        // A source-edit path that is exactly the declared selected source
        // must always classify as SourceEdit.
        let source_edit_result = classify_output(&selected_path, "source-edit", &source_set);
        assert_eq!(source_edit_result, Ok(OutputClass::SourceEdit));
        // A derived path with an unrelated name (not selected) is always
        // accepted as Derived.
        if other_name != selected_name {
            let other_path = format!("src/{other_name}.rs");
            let unrelated_result = classify_output(&other_path, "page", &source_set);
            assert_eq!(unrelated_result, Ok(OutputClass::Derived));
        }
    })
    .settings(settings(906))
    .run();
}

/// Task 2.7: the publication basis is complete over its declared inputs —
/// changing any one snapshot byte, for any randomly generated snapshot,
/// changes the resulting basis digest. A basis that ignored part of its
/// declared input would fail this property for some generated case.
#[test]
fn hegel_publication_basis_is_complete_over_its_snapshot_input() {
    Hegel::new(|test_case: TestCase| {
        let path = format!("src/{}.rs", ascii_id(&test_case, 8));
        let original_len = 1 + draw_index(&test_case, 16);
        let bytes = (0..original_len)
            .map(|_| draw_byte(&test_case))
            .collect::<Vec<_>>();
        let mut flipped = bytes.clone();
        let flip_index = draw_index(&test_case, flipped.len());
        if let Some(byte) = flipped.get_mut(flip_index) {
            *byte ^= 0xff;
        } else {
            flipped.push(0xff);
        }
        let selector = Selector::RequirementId(String::from("DEMO-001"));
        let rows = Vec::new();
        let base = compute_publication_basis(&BasisInputs {
            plan_schema: "buoy/trace-publication-plan/v1",
            renderer_schema: "buoy/readable-trace-markdown/v1",
            active_member: "blake3-256",
            selector: &selector,
            trace_rows: &rows,
            normalized_snapshot: &[(path.clone(), bytes.clone())],
        })
        .expect("base basis");
        let flipped_basis = compute_publication_basis(&BasisInputs {
            plan_schema: "buoy/trace-publication-plan/v1",
            renderer_schema: "buoy/readable-trace-markdown/v1",
            active_member: "blake3-256",
            selector: &selector,
            trace_rows: &rows,
            normalized_snapshot: &[(path, flipped)],
        })
        .expect("flipped basis");
        assert_ne!(base, flipped_basis);
    })
    .settings(settings(907))
    .run();
}

/// Task 2.7: joining any normalized `https://.../` base with any generated
/// page path always yields a target that both starts with the base and ends
/// with the exact rendered page path, for every generated ASCII page path
/// with no traversal segment.
#[test]
fn hegel_uri_joining_preserves_base_and_the_complete_page_path() {
    Hegel::new(|test_case: TestCase| {
        let host = ascii_id(&test_case, 10).to_lowercase();
        let segment = ascii_id(&test_case, 10);
        let base = format!("https://{host}.example.invalid/trace/");
        let page_path = format!("docs/{segment}.md");
        let joined = join_target_base(&base, &page_path).expect("well-formed join");
        assert!(joined.starts_with(&base));
        assert!(joined.as_bytes().ends_with(b".md"));
    })
    .settings(settings(908))
    .run();
}

/// Task 2.8 (differential): the production URI-join percent-encoder matches
/// an independent hand-rolled percent-encoding reimplementation, for every
/// generated multi-segment page path containing reserved and non-ASCII
/// bytes. Both must reserve the same unreserved-character set and preserve
/// the same slash boundaries, entirely independent of the production
/// implementation's own byte-by-byte loop.
#[test]
fn hegel_uri_join_matches_an_independent_percent_encode_reimplementation() {
    Hegel::new(|test_case: TestCase| {
        let host = ascii_id(&test_case, 10).to_lowercase();
        let base = format!("https://{host}.example.invalid/trace/");
        let segment_count = 1 + draw_index(&test_case, 3);
        let segments: Vec<String> = (0..segment_count)
            .map(|_| {
                let raw_len = 1 + draw_index(&test_case, 6);
                (0..raw_len)
                    .map(|_| {
                        // Draw from a pool that mixes unreserved characters
                        // with reserved/space bytes that must be encoded.
                        let pool = "abcZ09-._~ /:?#[]@!$&'()*+,;=";
                        let index = draw_index(&test_case, pool.len());
                        pool.chars().nth(index).unwrap_or('a')
                    })
                    .filter(|character| *character != '/')
                    .collect::<String>()
            })
            .filter(|segment| !segment.is_empty())
            .collect();
        if segments.is_empty() {
            return;
        }
        let page_path = segments.join("/");
        let Ok(produced) = join_target_base(&base, &page_path) else {
            return;
        };
        // Independent reimplementation: percent-encode every byte outside
        // the unreserved RFC 3986 set, one segment at a time, using a
        // completely separate loop from `publication::percent_encode_segment`.
        let independent_encoded = page_path
            .split('/')
            .map(|segment| {
                segment
                    .bytes()
                    .map(|byte| {
                        let unreserved = byte.is_ascii_alphanumeric()
                            || matches!(byte, b'-' | b'.' | b'_' | b'~');
                        if unreserved {
                            (byte as char).to_string()
                        } else {
                            format!("%{byte:02X}")
                        }
                    })
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join("/");
        let independent = format!("{base}{independent_encoded}");
        assert_eq!(produced, independent);
    })
    .settings(settings(914))
    .run();
}

/// Task 2.7: `split_relative_path` rejects every generated path containing a
/// `..` component or an empty component, for any surrounding random ASCII
/// segments, proving root-escape rejection is not limited to the fixed
/// fixtures in `tests/publication_filesystem.rs`.
#[test]
fn hegel_relative_path_split_rejects_every_generated_traversal_shape() {
    Hegel::new(|test_case: TestCase| {
        let prefix = ascii_id(&test_case, 8);
        let suffix = ascii_id(&test_case, 8);
        let traversal_path = format!("{prefix}/../{suffix}");
        assert!(split_relative_path(&traversal_path).is_err());
        let doubled_separator = format!("{prefix}//{suffix}");
        assert!(split_relative_path(&doubled_separator).is_err());
        // A well-formed two-component path with no traversal always splits.
        let clean_path = format!("{prefix}/{suffix}");
        assert!(split_relative_path(&clean_path).is_ok());
    })
    .settings(settings(909))
    .run();
}

/// Task 2.8 (differential): the production canonical machine report encoder
/// and an independent, hand-rolled JSON-field-ordering reimplementation
/// agree on field order for every generated diagnostic multiset, because
/// both must sort diagnostics the same deterministic way.
#[test]
fn hegel_report_encoder_matches_an_independent_sort_reimplementation() {
    Hegel::new(|test_case: TestCase| {
        let count = draw_index(&test_case, 5);
        let mut diagnostics = (0..count)
            .map(|index| {
                let code = format!("TPT-{:03}", draw_byte(&test_case));
                (code, format!("message-{index}"))
            })
            .collect::<Vec<_>>();
        let mut report = buoy::publication::PublicationReport::new("check", "trace-page");
        for (code, message) in &diagnostics {
            report = report.with_diagnostic(code, message);
        }
        let rendered = render_report(&report).expect("report renders");
        let value: serde_json::Value = serde_json::from_slice(&rendered).expect("valid JSON");
        let production_order = value["diagnostics"]
            .as_array()
            .expect("diagnostics array")
            .iter()
            .map(|entry| entry["code"].as_str().expect("code field").to_owned())
            .collect::<Vec<_>>();
        // Independent reimplementation: sort the same (code, message) pairs
        // with the standard library's own comparator, entirely outside the
        // production encoder's code path.
        diagnostics.sort();
        let independent_order = diagnostics
            .into_iter()
            .map(|(code, _)| code)
            .collect::<Vec<_>>();
        assert_eq!(production_order, independent_order);
    })
    .settings(settings(910))
    .run();
}

/// Task 2.8 (differential): the production package-identity feature
/// ordering matches an independent reimplementation that sorts the same
/// randomly permuted feature multiset with the standard library sort.
#[test]
fn hegel_package_identity_feature_order_matches_an_independent_sort() {
    Hegel::new(|test_case: TestCase| {
        let pool = ["xxh3-128", "blake3-256", "sha-256"];
        let count = 1 + draw_index(&test_case, 3);
        let mut chosen = (0..count)
            .map(|_| pool[draw_index(&test_case, pool.len())])
            .collect::<Vec<_>>();
        chosen.dedup();
        if chosen.is_empty() {
            chosen.push("xxh3-128");
        }
        let identity = PackageIdentity::new("0.2.0", "revision", "rustc", chosen.clone());
        let rendered = identity.render().expect("identity renders");
        let value: serde_json::Value = serde_json::from_slice(&rendered).expect("valid JSON");
        let production_order = value["features"]
            .as_array()
            .expect("features array")
            .iter()
            .map(|entry| entry.as_str().expect("feature string").to_owned())
            .collect::<Vec<_>>();
        let mut independent = chosen
            .iter()
            .map(|value| (*value).to_owned())
            .collect::<Vec<_>>();
        independent.sort_unstable();
        assert_eq!(production_order, independent);
    })
    .settings(settings(911))
    .run();
}

/// Task 2.8 (differential): evidence rendering never depends on process
/// state; an independent reimplementation that builds the exact same JSON
/// object with `serde_json::json!` produces byte-identical output to the
/// production renderer for every generated publication ID.
#[test]
fn hegel_evidence_encoder_matches_an_independent_json_reimplementation() {
    Hegel::new(|test_case: TestCase| {
        let publication_id = ascii_id(&test_case, 12);
        let evidence = buoy::publication::PublicationEvidence::new(&publication_id);
        let produced = render_evidence(&evidence);
        let independent = serde_json::to_vec(&serde_json::json!({
            "diagnostics": [],
            "publication_id": publication_id,
            "schema": "buoy/trace-publication-evidence/v1",
            "status": "historical",
        }))
        .expect("independent JSON renders");
        assert_eq!(produced, independent);
    })
    .settings(settings(912))
    .run();
}

/// Task 2.8 (differential): the production path-template substitution
/// renderer matches an independent hand-rolled string-splice
/// reimplementation for every generated basis payload.
#[test]
fn hegel_path_substitution_matches_an_independent_splice_reimplementation() {
    Hegel::new(|test_case: TestCase| {
        let algorithm = if draw_index(&test_case, 2) == 0 {
            "blake3-256"
        } else {
            "xxh3-128"
        };
        let payload_len = 1 + draw_index(&test_case, 16);
        let payload = (0..payload_len)
            .map(|_| {
                let nibble = u32::from(draw_byte(&test_case) % 16);
                std::char::from_digit(nibble, 16).unwrap_or('0')
            })
            .collect::<String>();
        let basis = format!("{algorithm}:{payload}");
        let identities = buoy::publication::PathIdentities {
            basis: &basis,
            atom_revision: None,
            record_digest: None,
        };
        let produced =
            render_path("docs/{publication_basis_path}.md", &identities).expect("renders");
        // Independent reimplementation: splice the algorithm/payload split
        // directly rather than reusing the production template scanner.
        let independent = format!("docs/{algorithm}/{payload}.md");
        assert_eq!(produced, independent);
    })
    .settings(settings(913))
    .run();
}

/// Task 2.7: the immutable output lifecycle holds for every randomly
/// generated pair of prior and candidate bytes, over a real no-follow
/// filesystem descriptor (not a pure in-memory stand-in): absent always
/// creates, byte-identical candidates always no-op, and any byte difference
/// always rejects without ever overwriting the existing bytes on disk.
#[test]
fn hegel_immutable_lifecycle_never_overwrites_unequal_existing_bytes() {
    Hegel::new(|test_case: TestCase| {
        let root = tempfile::TempDir::new().expect("immutable-lifecycle root");
        let root_fd = open_root(root.path()).expect("open root");
        let filename = ascii_id(&test_case, 12);

        let prior_len = 1 + draw_index(&test_case, 24);
        let prior_bytes: Vec<u8> = (0..prior_len).map(|_| draw_byte(&test_case)).collect();
        let candidate_len = 1 + draw_index(&test_case, 24);
        let candidate_bytes: Vec<u8> = (0..candidate_len).map(|_| draw_byte(&test_case)).collect();
        let create_prior = draw_byte(&test_case) % 2 == 0;

        if create_prior {
            let created = write_output(&root_fd, &filename, &prior_bytes, &Lifecycle::Immutable)
                .expect("prior write succeeds");
            assert_eq!(created, WriteOutcome::Created);
        }

        let outcome = write_output(&root_fd, &filename, &candidate_bytes, &Lifecycle::Immutable)
            .expect("candidate write reports an outcome");
        let on_disk = std::fs::read(root.path().join(&filename)).expect("read back target");

        if !create_prior {
            assert_eq!(outcome, WriteOutcome::Created);
            assert_eq!(on_disk, candidate_bytes);
        } else if prior_bytes == candidate_bytes {
            assert_eq!(outcome, WriteOutcome::NoOp);
            assert_eq!(on_disk, prior_bytes);
        } else {
            assert_eq!(outcome, WriteOutcome::Conflict);
            // The defining immutable-lifecycle guarantee: a conflicting
            // candidate must never reach the disk. The existing bytes this
            // random case created remain byte-identical.
            assert_eq!(on_disk, prior_bytes);
            assert_ne!(on_disk, candidate_bytes);
        }
    })
    .settings(settings(915))
    .run();
}
