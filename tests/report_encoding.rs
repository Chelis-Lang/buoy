#![doc = "Authoritative binary report identity, complete empty sets, and independent goldens."]

use buoy::core::{
    AuthorityFact, CanonicalExecutionReport, CanonicalImpactReport, CanonicalStaticReport,
    CanonicalValue, DecodeBudget, LifecycleState, ObjectKind, ObjectRef, ReportDecodingError,
    SemanticDigest, Validity,
};
use buoy::presentation::{render_authoritative_human, render_authoritative_json};
use buoy_reference::Value;
use std::fmt::Write as _;

const GOLDENS: &str = include_str!("../fixtures/hardening/v2/canonical-report-goldens.tsv");
const JSON_GOLDEN: &str = include_str!("../fixtures/hardening/v2/static-report.json");
const HUMAN_GOLDEN: &str = include_str!("../fixtures/hardening/v2/static-report.txt");

fn digest(byte: u8) -> SemanticDigest {
    SemanticDigest::xxh3_128([byte; 16])
}

#[allow(clippy::expect_used)]
fn test_object_ref(kind: ObjectKind, id: buoy::core::StableId, value: SemanticDigest) -> ObjectRef {
    ObjectRef::new(kind, id, value)
}

fn reports() -> Option<(
    CanonicalStaticReport,
    CanonicalExecutionReport,
    CanonicalImpactReport,
)> {
    let static_report = CanonicalStaticReport::new(
        digest(1),
        Vec::new(),
        Vec::new(),
        Vec::new(),
        Vec::new(),
        Vec::new(),
        Vec::new(),
        Vec::new(),
        Vec::new(),
        Vec::new(),
    )
    .ok()?;
    let execution_report =
        CanonicalExecutionReport::new(static_report.digest(), Vec::new(), Vec::new(), Vec::new())
            .ok()?;
    let impact_report = CanonicalImpactReport::new(
        digest(2),
        digest(3),
        digest(4),
        digest(5),
        Vec::new(),
        Vec::new(),
    )
    .ok()?;
    Some((static_report, execution_report, impact_report))
}

fn reference(value: &CanonicalValue) -> Value {
    match value {
        CanonicalValue::False => Value::False,
        CanonicalValue::True => Value::True,
        CanonicalValue::U64(value) => Value::Unsigned(*value),
        CanonicalValue::I64(value) => Value::Signed(*value),
        CanonicalValue::Bytes(value) => Value::Bytes(value.clone()),
        CanonicalValue::Text(value) => Value::Text(value.clone()),
        CanonicalValue::Record(fields) => Value::Record(
            fields
                .iter()
                .map(|(tag, value)| (*tag, reference(value)))
                .collect(),
        ),
        CanonicalValue::Sequence(values) => Value::Sequence(values.iter().map(reference).collect()),
        CanonicalValue::Set(values) => Value::Set(values.iter().map(reference).collect()),
        CanonicalValue::None => Value::None,
        CanonicalValue::Some(value) => Value::Some(Box::new(reference(value))),
        CanonicalValue::Enum(tag, value) => {
            Value::Enum(*tag, value.as_ref().map(|value| Box::new(reference(value))))
        }
        CanonicalValue::Digest(value) => {
            Value::Digest(value.algorithm().tag(), value.bytes().to_vec())
        }
    }
}

fn hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push(char::from(HEX[usize::from(byte >> 4)]));
        output.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    output
}

#[test]
fn complete_report_domains_retain_every_explicit_empty_fact_set() {
    let Some((static_report, execution_report, impact_report)) = reports() else {
        return;
    };
    let field_count = |value: &CanonicalValue| match value {
        CanonicalValue::Record(fields) => fields.len(),
        _ => 0,
    };
    assert_eq!(field_count(static_report.value()), 10);
    assert_eq!(field_count(execution_report.value()), 4);
    assert_eq!(field_count(impact_report.value()), 6);

    for (value, bytes) in [
        (static_report.value(), static_report.bytes()),
        (execution_report.value(), execution_report.bytes()),
        (impact_report.value(), impact_report.bytes()),
    ] {
        assert_eq!(reference(value).encode().ok().as_deref(), Some(bytes));
        let decoded = CanonicalValue::decode(bytes, DecodeBudget::v2_default());
        assert_eq!(decoded.as_ref().ok(), Some(value));
        assert_eq!(
            decoded
                .ok()
                .and_then(|value| value.encode().ok())
                .as_deref(),
            Some(bytes)
        );
    }
}

#[test]
fn report_decoders_reject_unknown_omitted_and_wrong_shape_fields() {
    let Some((static_report, execution_report, impact_report)) = reports() else {
        return;
    };
    let budget = DecodeBudget::v2_default();
    assert_eq!(
        CanonicalStaticReport::decode(static_report.bytes(), budget)
            .ok()
            .map(|report| report.digest()),
        Some(static_report.digest())
    );
    assert_eq!(
        CanonicalExecutionReport::decode(execution_report.bytes(), budget)
            .ok()
            .map(|report| report.digest()),
        Some(execution_report.digest())
    );
    assert_eq!(
        CanonicalImpactReport::decode(impact_report.bytes(), budget)
            .ok()
            .map(|report| report.digest()),
        Some(impact_report.digest())
    );

    let CanonicalValue::Record(fields) = static_report.value() else {
        return;
    };
    let mut omitted = fields.clone();
    omitted.pop();
    let omitted = CanonicalValue::Record(omitted).encode().ok();
    assert_eq!(
        omitted
            .as_deref()
            .map(|bytes| CanonicalStaticReport::decode(bytes, budget)),
        Some(Err(ReportDecodingError::Schema))
    );
    let mut unknown = fields.clone();
    unknown.push((11, CanonicalValue::Set(Vec::new())));
    let unknown = CanonicalValue::Record(unknown).encode().ok();
    assert_eq!(
        unknown
            .as_deref()
            .map(|bytes| CanonicalStaticReport::decode(bytes, budget)),
        Some(Err(ReportDecodingError::Schema))
    );
    let mut wrong_shape = fields.clone();
    wrong_shape[1].1 = CanonicalValue::Sequence(Vec::new());
    let wrong_shape = CanonicalValue::Record(wrong_shape).encode().ok();
    assert_eq!(
        wrong_shape
            .as_deref()
            .map(|bytes| CanonicalStaticReport::decode(bytes, budget)),
        Some(Err(ReportDecodingError::Schema))
    );
}

#[test]
fn derived_presentations_bind_binary_identity_and_ignore_fact_insertion_order() {
    let fact = |id: &str, byte: u8| {
        let id = buoy::core::StableId::new(id).ok()?;
        AuthorityFact::new(
            test_object_ref(ObjectKind::Atom, id, digest(byte)),
            Validity::Valid,
            LifecycleState::Ratified,
            1,
            Validity::Valid,
            Vec::new(),
        )
        .ok()
    };
    let (Some(first), Some(second)) = (fact("REPORT-A", 21), fact("REPORT-B", 22)) else {
        return;
    };
    let report = |facts| {
        CanonicalStaticReport::new(
            digest(20),
            facts,
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
        )
        .ok()
    };
    let (Some(left), Some(right)) = (
        report(vec![first.clone(), second.clone()]),
        report(vec![second, first]),
    ) else {
        return;
    };
    assert_eq!(left.bytes(), right.bytes());
    assert_eq!(left.digest(), right.digest());
    let left_json = render_authoritative_json(&left).ok();
    let right_json = render_authoritative_json(&right).ok();
    assert_eq!(left_json, right_json);
    let left_human = render_authoritative_human(&left).ok();
    let right_human = render_authoritative_human(&right).ok();
    assert_eq!(left_human, right_human);
    let Some(json) = left_json else { return };
    assert!(json.contains("spec-provenance-report-json/v2"));
    assert!(json.contains("\"algorithm\":\"xxh3-128\""));
    assert!(json.contains(&left.digest().canonical_text()));
    assert!(json.contains(&hex(left.bytes())));
    assert!(left_human.is_some_and(|value| {
        value.contains("spec-provenance-report-human/v2")
            && value.contains(&left.digest().canonical_text())
    }));
}

#[test]
fn reviewed_report_golden_bytes_and_identities_are_current() {
    let Some((static_report, execution_report, impact_report)) = reports() else {
        return;
    };
    let mut rendered = String::from("schema\treport\tdigest\tcanonical_hex\n");
    for (name, report_digest, bytes) in [
        ("static", static_report.digest(), static_report.bytes()),
        (
            "execution",
            execution_report.digest(),
            execution_report.bytes(),
        ),
        ("impact", impact_report.digest(), impact_report.bytes()),
    ] {
        assert!(
            writeln!(
                &mut rendered,
                "spec-provenance-report/v2\t{name}\t{}\t{}",
                report_digest.canonical_text(),
                hex(bytes)
            )
            .is_ok()
        );
    }
    assert_eq!(rendered, GOLDENS);
    assert_eq!(
        render_authoritative_json(&static_report)
            .ok()
            .map(|value| value + "\n")
            .as_deref(),
        Some(JSON_GOLDEN)
    );
    assert_eq!(
        render_authoritative_human(&static_report).ok().as_deref(),
        Some(HUMAN_GOLDEN)
    );
}
