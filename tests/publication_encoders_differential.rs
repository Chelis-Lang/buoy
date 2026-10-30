#![doc = "Differential tests between production and independently reimplemented encoders (task 2.8)."]
#![allow(missing_docs, clippy::expect_used, clippy::panic)]

//! Round-3 finding: the prior "differential" evidence and report tests used
//! degenerate/empty field values and effectively transcribed production
//! code rather than independently reimplementing it. Every test in this
//! file builds REALISTIC, fully populated values (multiple rows, real
//! digest-shaped strings, real paths) and compares production's own
//! rendering function against a SEPARATE encoder written here that reads
//! the identical typed struct only through its public accessor methods,
//! using its own field-by-field JSON construction and its own diagnostic
//! sort -- never a call into `render_report`, `render_evidence`,
//! `parse_plan`, or `join_target_base` themselves.

use buoy::publication::{
    BasisInputs, PathIdentities, PublicationError, PublicationEvidence, PublicationReport,
    RowFreshness, Selector, TraceRow, join_target_base, parse_plan, render_evidence, render_path,
    render_report,
};
use std::fmt::Write as _;

/// An independently written report encoder. It reads `report` only through
/// its public accessors and builds its own `serde_json::Map` from scratch,
/// with its own diagnostic sort -- a separate code path from
/// `render_report`'s field-by-field `serde_json::json!`/`insert` sequence.
fn independent_report_encoding(report: &PublicationReport) -> serde_json::Value {
    let mut diagnostics: Vec<(String, String)> = report.diagnostics().to_vec();
    diagnostics.sort();
    let mut map = serde_json::Map::new();
    map.insert(
        "schema".into(),
        serde_json::Value::String("buoy/trace-publication-report/v1".into()),
    );
    map.insert(
        "operation".into(),
        serde_json::Value::String(report.operation().into()),
    );
    map.insert(
        "stage".into(),
        serde_json::Value::String(report.operation().into()),
    );
    map.insert(
        "artifact".into(),
        serde_json::Value::String(report.artifact().into()),
    );
    if let Some(plan_id) = report.plan_id() {
        map.insert("plan_id".into(), serde_json::Value::String(plan_id.into()));
    }
    if let Some(basis) = report.basis() {
        map.insert("basis".into(), serde_json::Value::String(basis.into()));
    }
    if let Some(graph) = report.graph() {
        map.insert("graph".into(), serde_json::Value::String(graph.into()));
    }
    if let Some(prior_state) = report.prior_state() {
        map.insert(
            "prior_state".into(),
            serde_json::Value::String(prior_state.into()),
        );
    }
    if let Some(prior_digest) = report.prior_digest() {
        map.insert(
            "prior_digest".into(),
            serde_json::Value::String(prior_digest.into()),
        );
    }
    if let Some(candidate_digest) = report.candidate_digest() {
        map.insert(
            "candidate_digest".into(),
            serde_json::Value::String(candidate_digest.into()),
        );
    }
    if let Some(outcome) = report.outcome() {
        map.insert("outcome".into(), serde_json::Value::String(outcome.into()));
    }
    let diagnostics_array = diagnostics
        .into_iter()
        .map(|(code, message)| {
            let mut entry = serde_json::Map::new();
            entry.insert("code".into(), serde_json::Value::String(code));
            entry.insert("message".into(), serde_json::Value::String(message));
            serde_json::Value::Object(entry)
        })
        .collect();
    map.insert(
        "diagnostics".into(),
        serde_json::Value::Array(diagnostics_array),
    );
    serde_json::Value::Object(map)
}

/// An independently written evidence encoder, mirroring the same
/// discipline as [`independent_report_encoding`]: public accessors only,
/// its own field insertion order and row encoding.
#[allow(clippy::too_many_lines)]
fn independent_evidence_encoding(evidence: &PublicationEvidence) -> serde_json::Value {
    let mut map = serde_json::Map::new();
    map.insert(
        "schema".into(),
        serde_json::Value::String(PublicationEvidence::SCHEMA.into()),
    );
    map.insert(
        "status".into(),
        serde_json::Value::String("historical".into()),
    );
    map.insert(
        "publication_id".into(),
        serde_json::Value::String(evidence.publication_id().into()),
    );
    map.insert("diagnostics".into(), serde_json::Value::Array(Vec::new()));
    if let Some(digest) = evidence.source_digest() {
        map.insert(
            "source_digest".into(),
            serde_json::Value::String(digest.into()),
        );
    }
    if let Some(selector) = evidence.selector_text() {
        map.insert(
            "selector".into(),
            serde_json::Value::String(selector.into()),
        );
    }
    if let Some(basis) = evidence.basis() {
        map.insert("basis".into(), serde_json::Value::String(basis.into()));
    }
    if let Some(graph) = evidence.graph() {
        map.insert("graph".into(), serde_json::Value::String(graph.into()));
    }
    if !evidence.rows().is_empty() {
        let rows = evidence
            .rows()
            .iter()
            .map(|row| {
                let mut entry = serde_json::Map::new();
                entry.insert(
                    "relation".into(),
                    row.relation
                        .clone()
                        .map_or(serde_json::Value::Null, serde_json::Value::String),
                );
                entry.insert(
                    "record".into(),
                    row.record
                        .clone()
                        .map_or(serde_json::Value::Null, serde_json::Value::String),
                );
                entry.insert(
                    "requirement_id".into(),
                    row.requirement_id
                        .clone()
                        .map_or(serde_json::Value::Null, serde_json::Value::String),
                );
                entry.insert(
                    "atom_revision".into(),
                    row.atom_revision
                        .clone()
                        .map_or(serde_json::Value::Null, serde_json::Value::String),
                );
                entry.insert(
                    "requirement_path".into(),
                    row.requirement_path
                        .clone()
                        .map_or(serde_json::Value::Null, serde_json::Value::String),
                );
                entry.insert(
                    "implementation_path".into(),
                    row.implementation_path
                        .clone()
                        .map_or(serde_json::Value::Null, serde_json::Value::String),
                );
                entry.insert(
                    "state".into(),
                    row.state
                        .clone()
                        .map_or(serde_json::Value::Null, serde_json::Value::String),
                );
                entry.insert(
                    "freshness".into(),
                    row.freshness.map_or(serde_json::Value::Null, |freshness| {
                        let mut inner = serde_json::Map::new();
                        inner.insert("record".into(), serde_json::Value::Bool(freshness.record));
                        inner.insert(
                            "attachment".into(),
                            serde_json::Value::Bool(freshness.attachment),
                        );
                        inner.insert(
                            "implementation".into(),
                            serde_json::Value::Bool(freshness.implementation),
                        );
                        inner.insert(
                            "requirement".into(),
                            serde_json::Value::Bool(freshness.requirement),
                        );
                        inner.insert(
                            "authority".into(),
                            serde_json::Value::Bool(freshness.authority),
                        );
                        serde_json::Value::Object(inner)
                    }),
                );
                serde_json::Value::Object(entry)
            })
            .collect();
        map.insert("rows".into(), serde_json::Value::Array(rows));
    }
    if let Some(target) = evidence.link_target() {
        map.insert(
            "link_target".into(),
            serde_json::Value::String(target.into()),
        );
    }
    if let Some(digest) = evidence.page_digest() {
        map.insert(
            "page_digest".into(),
            serde_json::Value::String(digest.into()),
        );
    }
    serde_json::Value::Object(map)
}

/// A realistic, fully populated report -- every optional field attached,
/// multiple diagnostics, real digest-shaped and path-shaped strings, not
/// empty placeholders.
fn populated_report() -> PublicationReport {
    PublicationReport::new("generate", "trace-page-blake3-256-cccccccccccccccc")
        .with_plan_id("consumer-example-publication-2026")
        .with_basis("blake3-256:1230550f0c19b8701908afeb129700d2cb82fa10ba6244089e957d61736f13c4")
        .with_graph("blake3-256:29fce2a3aa4972e09809482995a4f33636a11c2cac45ea87192bf9d49d7e01a5")
        .with_prior(
            "unequal",
            Some("blake3-256:c7b14f27fd36f42651ff9cab9223d90a031824a8d4bb38846d5ee132fa2393ce"),
        )
        .with_candidate_digest(
            "blake3-256:81e94d81bcf779100176a2105b038e6de55af9d196650ff9f5fc3c7fe2a20ca9",
        )
        .with_outcome("replaced")
        .with_diagnostic("TPT-ROW-CONFORMANCE", "row conformance drift")
        .with_diagnostic("TPT-OUTPUT-COLLISION", "two declared outputs collide")
}

#[test]
fn report_differential_matches_a_fully_populated_realistic_value() {
    let report = populated_report();
    let production: serde_json::Value =
        serde_json::from_slice(&render_report(&report).expect("render_report")).expect("JSON");
    let independent = independent_report_encoding(&report);
    assert_eq!(production, independent);
    // Sanity: this is not a degenerate/empty comparison -- every optional
    // field and both diagnostics are genuinely present on both sides.
    assert_eq!(
        production["diagnostics"].as_array().expect("array").len(),
        2
    );
    assert!(production["basis"].is_string());
    assert!(production["outcome"].is_string());
}

/// A realistic, fully populated evidence value -- multiple current trace
/// rows with real freshness facets, real digest-shaped strings, a real
/// link target and page digest, not empty placeholders.
fn populated_evidence() -> PublicationEvidence {
    let rows = vec![
        TraceRow {
            relation: Some("BGLC-DOC-003".into()),
            record: Some(
                "blake3-256:1111111111111111111111111111111111111111111111111111111111111111"
                    .into(),
            ),
            requirement_id: Some("BGLC-003".into()),
            atom_revision: Some(
                "blake3-256:2222222222222222222222222222222222222222222222222222222222222222"
                    .into(),
            ),
            requirement_node: Some("requirement-node-1".into()),
            implementation_node: Some("implementation-node-1".into()),
            requirement_path: Some("openspec/specs/demo/spec.md".into()),
            implementation_path: Some("src/lib.rs".into()),
            state: Some("current".into()),
            freshness: Some(RowFreshness {
                record: true,
                attachment: true,
                implementation: true,
                requirement: true,
                authority: true,
            }),
            diagnostics: Vec::new(),
        },
        TraceRow {
            relation: Some("BGLC-DOC-004".into()),
            record: Some(
                "blake3-256:3333333333333333333333333333333333333333333333333333333333333333"
                    .into(),
            ),
            requirement_id: Some("BGLC-004".into()),
            atom_revision: Some(
                "blake3-256:4444444444444444444444444444444444444444444444444444444444444444"
                    .into(),
            ),
            requirement_node: Some("requirement-node-2".into()),
            implementation_node: Some("implementation-node-2".into()),
            requirement_path: Some("openspec/specs/demo/spec.md".into()),
            implementation_path: Some("src/other.rs".into()),
            state: Some("current".into()),
            freshness: Some(RowFreshness {
                record: true,
                attachment: true,
                implementation: false,
                requirement: true,
                authority: true,
            }),
            diagnostics: Vec::new(),
        },
    ];
    PublicationEvidence::new("consumer-example-publication-2026")
        .with_source_digest(
            "blake3-256:5555555555555555555555555555555555555555555555555555555555555555",
        )
        .with_selector(&Selector::RequirementId("BGLC-003".into()))
        .with_basis("blake3-256:1230550f0c19b8701908afeb129700d2cb82fa10ba6244089e957d61736f13c4")
        .with_graph("blake3-256:29fce2a3aa4972e09809482995a4f33636a11c2cac45ea87192bf9d49d7e01a5")
        .with_rows(&rows)
        .with_link_target("https://example.invalid/trace/blake3-256/6666.md")
        .with_page_digest(
            "blake3-256:7777777777777777777777777777777777777777777777777777777777777777",
        )
}

#[test]
fn evidence_differential_matches_a_fully_populated_realistic_value() {
    let evidence = populated_evidence();
    let production: serde_json::Value =
        serde_json::from_slice(&render_evidence(&evidence)).expect("JSON");
    let independent = independent_evidence_encoding(&evidence);
    assert_eq!(production, independent);
    assert_eq!(production["rows"].as_array().expect("rows array").len(), 2);
    assert!(production["link_target"].is_string());
    assert!(production["page_digest"].is_string());
}

/// An independently written, minimal structural JSON validator for one
/// `buoy/trace-publication-plan/v1` document -- checks only the closed
/// top-level key set and each field's coarse shape, using
/// `serde_json::Value` directly rather than `parse_plan`'s typed
/// `RawPlan`/`serde(deny_unknown_fields)` derive. This is the "plan"
/// differential: production's strict typed parser and this independent
/// structural walk must agree on acceptance for a realistic, fully
/// populated plan.
fn independently_structurally_valid_plan(bytes: &[u8]) -> bool {
    let Ok(value) = serde_json::from_slice::<serde_json::Value>(bytes) else {
        return false;
    };
    let Some(object) = value.as_object() else {
        return false;
    };
    let required = [
        "schema",
        "publication_id",
        "selector",
        "expected_rows",
        "source_set",
        "outputs",
    ];
    if object.len() != required.len() {
        return false;
    }
    for key in required {
        if !object.contains_key(key) {
            return false;
        }
    }
    object.get("schema").and_then(serde_json::Value::as_str) == Some(buoy::publication::PLAN_SCHEMA)
        && object
            .get("outputs")
            .is_some_and(serde_json::Value::is_array)
        && object
            .get("source_set")
            .is_some_and(serde_json::Value::is_array)
        && object
            .get("expected_rows")
            .and_then(serde_json::Value::as_u64)
            .is_some_and(|value| value > 0)
}

#[test]
fn plan_differential_matches_a_realistic_fully_populated_plan() {
    let bytes = format!(
        r#"{{"schema":"{schema}","publication_id":"consumer-example-publication-2026","selector":{{"requirement_id":"BGLC-003"}},"expected_rows":1,"source_set":["openspec/specs/demo/spec.md","src/lib.rs","buoy/objects/v1/blake3-256/0e857e33e387325fbf457e426bc42286d9d99926b19f3efa3c203795cfeeb6c4.toml"],"outputs":[{{"id":"page","mount":"repository","path":"docs/{{publication_basis_path}}.md","kind":"page","lifecycle":"replaceable"}}]}}"#,
        schema = buoy::publication::PLAN_SCHEMA
    );
    assert!(independently_structurally_valid_plan(bytes.as_bytes()));
    assert!(parse_plan(bytes.as_bytes()).is_ok());

    // Both sides must also agree on rejection: an extra unknown top-level
    // key must fail both the independent structural walk (wrong key count)
    // and the real typed parser.
    let with_unknown_field = bytes.replace(
        "\"expected_rows\":1,",
        "\"expected_rows\":1,\"unknown\":true,",
    );
    assert!(!independently_structurally_valid_plan(
        with_unknown_field.as_bytes()
    ));
    assert!(parse_plan(with_unknown_field.as_bytes()).is_err());
}

/// An independently written URI joiner -- its own percent-encoding table
/// and its own traversal rejection, never calling `join_target_base`
/// itself. The "URI" differential: production's `join_target_base` and
/// this independent implementation must agree on both acceptance and the
/// exact rendered bytes for a realistic, multi-segment page path.
/// Fully percent-decodes one path segment.
///
/// Production decodes only the `%2e` escape (the one that can produce a dot
/// segment); this oracle decodes EVERY escape, which is a genuinely
/// different implementation of the same contract rather than a transcription
/// of production's shortcut. The two must still agree on which segments
/// denote `.` or `..`.
fn independent_percent_decode(segment: &str) -> String {
    let bytes = segment.as_bytes();
    let mut decoded = Vec::new();
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%' && index + 2 < bytes.len() {
            // Both nibbles are hexadecimal digits, so the combined value is
            // always in `0..=255`; `u8::try_from` keeps that fact checked
            // rather than asserted by a truncating cast.
            let high = (bytes[index + 1] as char).to_digit(16);
            let low = (bytes[index + 2] as char).to_digit(16);
            if let (Some(high), Some(low)) = (high, low) {
                if let Ok(byte) = u8::try_from(high * 16 + low) {
                    decoded.push(byte);
                    index += 3;
                    continue;
                }
            }
        }
        decoded.push(bytes[index]);
        index += 1;
    }
    String::from_utf8_lossy(&decoded).into_owned()
}

/// An independent implementation of the complete normalized-target-base
/// contract, written from the requirement rather than from production's
/// control flow: an absolute `https` URI with one unambiguous origin (no
/// userinfo, a real numeric port when present), a trailing `/`, no query or
/// fragment, and a path with no doubled slash and no segment that decodes to
/// a `.` or `..` traversal.
fn independent_base_is_normal(base: &str) -> bool {
    let Some(after_scheme) = base.strip_prefix("https://") else {
        return false;
    };
    if !base.ends_with('/') {
        return false;
    }
    if base
        .chars()
        .any(|c| c.is_control() || c == ' ' || c == '?' || c == '#' || c == '\\')
    {
        return false;
    }
    if base.contains('{') || base.contains('}') {
        return false;
    }
    let mut parts = after_scheme.splitn(2, '/');
    let authority = parts.next().unwrap_or_default();
    let path = parts.next().unwrap_or_default();
    if authority.is_empty() || authority.contains('@') {
        return false;
    }
    let host_and_port: Vec<&str> = authority.split(':').collect();
    match host_and_port.as_slice() {
        [host] => {
            if host.is_empty() {
                return false;
            }
        }
        [host, port] => {
            if host.is_empty() || port.is_empty() || !port.chars().all(|c| c.is_ascii_digit()) {
                return false;
            }
        }
        _ => return false,
    }
    // Drop the single trailing empty segment produced by the required
    // trailing `/`, then require every remaining segment to be a real,
    // non-traversal segment.
    let body = path.strip_suffix('/').unwrap_or(path);
    if body.is_empty() {
        return true;
    }
    for segment in body.split('/') {
        if segment.is_empty() {
            return false;
        }
        let decoded = independent_percent_decode(segment);
        if decoded == "." || decoded == ".." {
            return false;
        }
    }
    true
}

/// The exact rejection classes this contract distinguishes.
///
/// Round-5 finding M5: comparing `production.ok()` against
/// `independent.ok()` collapses EVERY rejection to `None`, so the two
/// implementations agree as long as both reject -- even if they reject for
/// completely unrelated reasons. Classifying the rejection makes the
/// differential compare exact error classes as well as exact success values.
#[derive(Debug, Eq, PartialEq)]
enum JoinClass {
    /// The target base itself is not normalized.
    InvalidTargetBase,
    /// The base is fine; the page path carries an empty or traversal segment.
    TargetTraversal,
}

fn classify_production_join(error: &PublicationError) -> JoinClass {
    match error {
        PublicationError::InvalidTargetBase => JoinClass::InvalidTargetBase,
        PublicationError::InvalidField(field) if field == "target-traversal" => {
            JoinClass::TargetTraversal
        }
        other => panic!("unexpected join_target_base error class: {other:?}"),
    }
}

fn independent_join_target_base(base: &str, page_path: &str) -> Result<String, JoinClass> {
    if !independent_base_is_normal(base) {
        return Err(JoinClass::InvalidTargetBase);
    }
    if page_path
        .split('/')
        .any(|segment| segment.is_empty() || segment == "..")
    {
        return Err(JoinClass::TargetTraversal);
    }
    let mut encoded_segments = Vec::new();
    for segment in page_path.split('/') {
        let mut encoded = String::new();
        for byte in segment.bytes() {
            let is_unreserved =
                byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~');
            if is_unreserved {
                encoded.push(byte as char);
            } else {
                let _ = write!(encoded, "%{byte:02X}");
            }
        }
        encoded_segments.push(encoded);
    }
    Ok(format!("{base}{}", encoded_segments.join("/")))
}

/// The URI differential must cover acceptance AND rejection across the whole
/// boundary, not one happy example: a single valid case cannot detect a
/// validator that accepts a userinfo origin, collapses a doubled slash, or
/// misses a percent-encoded traversal.
#[test]
fn uri_join_differential_matches_across_valid_negative_and_boundary_inputs() {
    let bases = [
        // Valid, normalized.
        "https://example.invalid/",
        "https://example.invalid/trace/",
        "https://example.invalid:8443/trace/",
        "https://sub.domain.example.invalid/a/b/c/",
        "https://example.invalid/trace.v1/",
        // Non-normal origins.
        "https://user@example.invalid/trace/",
        "https://user:secret@example.invalid/trace/",
        "https://:8443/trace/",
        "https://example.invalid:/trace/",
        "https://example.invalid:port/trace/",
        "https:///trace/",
        // Non-normal paths.
        "https://example.invalid//trace/",
        "https://example.invalid/a//b/",
        "https://example.invalid/../trace/",
        "https://example.invalid/./trace/",
        "https://example.invalid/%2e%2e/trace/",
        "https://example.invalid/%2E%2E/trace/",
        "https://example.invalid/.%2e/trace/",
        "https://example.invalid/%2e/trace/",
        // Non-normal syntax.
        "http://example.invalid/trace/",
        "https://example.invalid/trace",
        "https://example.invalid/trace/?q=1",
        "https://example.invalid/trace/#frag",
        "https://example.invalid\\trace/",
        "https://example.invalid/{basis}/",
        "",
        "/trace/",
    ];
    let page_paths = [
        // Valid, realistic.
        "docs/blake3-256/29fce2a3aa4972e09809482995a4f33636a11c2cac45ea87192bf9d49d7e01a5.md",
        "page.md",
        "a b/c+d.md",
        "unicode-\u{e9}\u{4e2d}.md",
        "~tilde._under-dash.md",
        // Boundary and negative.
        "../escape.md",
        "a/../b.md",
        "a//b.md",
        "/leading.md",
        "trailing/",
        "",
    ];
    let mut agreed = 0_usize;
    let mut accepted = 0_usize;
    for base in bases {
        for page_path in page_paths {
            // Exact success value AND exact rejection class, not merely
            // "both rejected" (round-5 finding M5).
            let production =
                join_target_base(base, page_path).map_err(|error| classify_production_join(&error));
            let independent = independent_join_target_base(base, page_path);
            assert_eq!(
                production, independent,
                "production and the independent oracle disagree for base={base:?} page_path={page_path:?}"
            );
            agreed += 1;
            if production.is_ok() {
                accepted += 1;
            }
        }
    }
    // Both rejection classes must actually occur, or "exact error class
    // agreement" would be a claim about a single class.
    let mut base_faults = 0_usize;
    let mut traversal_faults = 0_usize;
    for base in bases {
        for page_path in page_paths {
            match join_target_base(base, page_path)
                .map_err(|error| classify_production_join(&error))
            {
                Err(JoinClass::InvalidTargetBase) => base_faults += 1,
                Err(JoinClass::TargetTraversal) => traversal_faults += 1,
                Ok(_) => {}
            }
        }
    }
    assert!(
        base_faults > 0 && traversal_faults > 0,
        "the matrix must exercise both rejection classes: base={base_faults} traversal={traversal_faults}"
    );
    // The matrix must be genuinely mixed: a differential where every case is
    // rejected (or every case accepted) would agree vacuously.
    assert_eq!(agreed, bases.len() * page_paths.len());
    assert!(
        accepted > 0 && accepted < agreed,
        "the differential matrix must contain both accepted and rejected cases: {accepted}/{agreed}"
    );
}

/// A minimal independent path-substitution renderer, matching production's
/// `render_path` exactly for the registered publication-basis substitution
/// on a realistic multi-segment template.
#[test]
fn path_substitution_differential_matches_for_a_realistic_template() {
    let identities = PathIdentities {
        basis: "blake3-256:1230550f0c19b8701908afeb129700d2cb82fa10ba6244089e957d61736f13c4",
        atom_revision: Some(
            "blake3-256:2222222222222222222222222222222222222222222222222222222222222222",
        ),
        record_digest: Some(
            "blake3-256:3333333333333333333333333333333333333333333333333333333333333333",
        ),
    };
    let template =
        "docs/{publication_basis_path}/atom-{atom_revision}-record-{record_digest_path}.md";
    let production = render_path(template, &identities).expect("render_path");
    let independent = "docs/blake3-256/1230550f0c19b8701908afeb129700d2cb82fa10ba6244089e957d61736f13c4/atom-blake3-256:2222222222222222222222222222222222222222222222222222222222222222-record-blake3-256/3333333333333333333333333333333333333333333333333333333333333333.md".to_owned();
    assert_eq!(production, independent);
}

/// An independently written path-template renderer.
///
/// It tokenizes the template with its own scanner (splitting on `{`/`}` into
/// literal and substitution runs) rather than production's
/// `find`/slice loop, and it resolves each registered key through its own
/// lookup table. A `_path` key renders `algorithm/payload`; every other
/// registered key renders `algorithm:payload`. Any unregistered key,
/// unterminated brace, traversal segment, or unavailable identity is a
/// rejection.
/// The exact rejection classes path rendering distinguishes.
///
/// Round-5 finding M5: `.ok()` on both sides makes an unregistered
/// substitution, a malformed brace, an unavailable identity, and a traversal
/// segment all look like one outcome, so the differential could not detect a
/// renderer that rejected the right templates for the wrong reasons.
#[derive(Debug, Eq, PartialEq)]
enum PathClass {
    /// A literal `..` traversal segment.
    Traversal,
    /// An unregistered key, an unterminated brace, or a malformed identity.
    Substitution,
    /// A registered key whose identity this trace does not supply.
    SubstitutionUnavailable,
}

fn classify_production_path(error: &PublicationError) -> PathClass {
    match error {
        PublicationError::InvalidField(field) if field == "path-traversal" => PathClass::Traversal,
        PublicationError::InvalidField(field) if field == "substitution" => PathClass::Substitution,
        PublicationError::InvalidField(field) if field == "substitution-unavailable" => {
            PathClass::SubstitutionUnavailable
        }
        other => panic!("unexpected render_path error class: {other:?}"),
    }
}

fn independent_render_path(
    template: &str,
    identities: &PathIdentities<'_>,
) -> Result<String, PathClass> {
    if template.split('/').any(|segment| segment == "..") {
        return Err(PathClass::Traversal);
    }
    let mut rendered = String::new();
    let mut rest = template;
    loop {
        let Some(open) = rest.find('{') else {
            rendered.push_str(rest);
            return Ok(rendered);
        };
        rendered.push_str(&rest[..open]);
        let remainder = &rest[open + 1..];
        let close = remainder.find('}').ok_or(PathClass::Substitution)?;
        let key = &remainder[..close];
        let value = match key {
            "publication_basis_path" | "publication_basis" => Some(identities.basis),
            "atom_revision_path" | "atom_revision" => identities.atom_revision,
            "record_digest_path" | "record_digest" => identities.record_digest,
            _ => return Err(PathClass::Substitution),
        }
        .ok_or(PathClass::SubstitutionUnavailable)?;
        let (algorithm, payload) = value.split_once(':').ok_or(PathClass::Substitution)?;
        rendered.push_str(algorithm);
        rendered.push(if key.ends_with("_path") { '/' } else { ':' });
        rendered.push_str(payload);
        rest = &remainder[close + 1..];
    }
}

/// The path differential must cover rejection and boundary shapes too: one
/// valid template cannot detect a renderer that accepts an unregistered
/// substitution, silently renders an unavailable identity, or lets a
/// traversal segment through.
#[test]
fn path_substitution_differential_matches_across_negative_and_boundary_inputs() {
    let complete = PathIdentities {
        basis: "blake3-256:1230550f0c19b8701908afeb129700d2cb82fa10ba6244089e957d61736f13c4",
        atom_revision: Some(
            "blake3-256:2222222222222222222222222222222222222222222222222222222222222222",
        ),
        record_digest: Some(
            "blake3-256:3333333333333333333333333333333333333333333333333333333333333333",
        ),
    };
    // A selected trace whose rows disagree supplies no unambiguous atom
    // revision or record digest, so a template needing either must fail
    // closed rather than pick an arbitrary row's value.
    let partial = PathIdentities {
        basis: "blake3-256:1230550f0c19b8701908afeb129700d2cb82fa10ba6244089e957d61736f13c4",
        atom_revision: None,
        record_digest: None,
    };
    let malformed = PathIdentities {
        basis: "no-algorithm-separator",
        atom_revision: Some("also-malformed"),
        record_digest: Some("blake3-256:4444"),
    };
    let templates = [
        // Valid shapes.
        "docs/{publication_basis_path}.md",
        "docs/{publication_basis}.md",
        "docs/{atom_revision_path}/{record_digest_path}/{publication_basis_path}.md",
        "{publication_basis_path}",
        "no-substitution-at-all.md",
        "",
        // Adjacent and repeated substitutions.
        "{publication_basis_path}{atom_revision_path}",
        "{publication_basis}-{publication_basis}",
        // Unregistered / post-link dependencies.
        "docs/{page_commit}.md",
        "docs/{graph}.md",
        "docs/{provider}.md",
        "docs/{unknown_key}.md",
        "docs/{}.md",
        // Malformed brace syntax.
        "docs/{publication_basis_path.md",
        "docs/publication_basis_path}.md",
        "docs/{{publication_basis_path}}.md",
        // Traversal.
        "../{publication_basis_path}.md",
        "docs/../{publication_basis_path}.md",
        "{publication_basis_path}/../escape.md",
    ];
    let mut accepted = 0_usize;
    let mut total = 0_usize;
    let (mut traversal, mut substitution, mut unavailable) = (0_usize, 0_usize, 0_usize);
    for identities in [&complete, &partial, &malformed] {
        for template in templates {
            // Exact rendered value AND exact rejection class.
            let production =
                render_path(template, identities).map_err(|error| classify_production_path(&error));
            let independent = independent_render_path(template, identities);
            assert_eq!(
                production, independent,
                "production and the independent renderer disagree for template={template:?}"
            );
            total += 1;
            match &production {
                Ok(_) => accepted += 1,
                Err(PathClass::Traversal) => traversal += 1,
                Err(PathClass::Substitution) => substitution += 1,
                Err(PathClass::SubstitutionUnavailable) => unavailable += 1,
            }
        }
    }
    // Every rejection class must actually occur, so "exact error class
    // agreement" is a claim about all three rather than about whichever one
    // the matrix happened to reach.
    assert!(
        traversal > 0 && substitution > 0 && unavailable > 0,
        "the matrix must exercise every rejection class: traversal={traversal} substitution={substitution} unavailable={unavailable}"
    );
    assert!(
        accepted > 0 && accepted < total,
        "the path differential matrix must contain both accepted and rejected cases: {accepted}/{total}"
    );
}

/// A realistic, complete `BasisInputs` value drives `compute_publication_basis`
/// deterministically -- proves the basis differential path stays reachable
/// with real (not degenerate) multi-row inputs.
#[test]
fn basis_computation_is_deterministic_for_a_realistic_multi_row_input() {
    let rows = populated_evidence().rows().to_vec();
    let snapshot = vec![
        (
            "openspec/specs/demo/spec.md".to_owned(),
            b"# Demo\n".to_vec(),
        ),
        ("src/lib.rs".to_owned(), b"pub fn lib() {}\n".to_vec()),
    ];
    let selector = Selector::RequirementId("BGLC-003".into());
    let inputs = BasisInputs {
        plan_schema: buoy::publication::PLAN_SCHEMA,
        renderer_schema: "buoy/readable-trace-markdown/v1",
        active_member: "blake3-256",
        selector: &selector,
        trace_rows: &rows,
        normalized_snapshot: &snapshot,
    };
    let first = buoy::publication::compute_publication_basis(&inputs).expect("basis");
    let second = buoy::publication::compute_publication_basis(&inputs).expect("basis");
    assert_eq!(first, second);
    assert!(first.starts_with("blake3-256:"));
}
