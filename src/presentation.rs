//! Deterministic derived JSON and human presentations of authoritative binary reports.

use core::fmt::Write as _;

use buoy_core::{
    CanonicalExecutionReport, CanonicalImpactReport, CanonicalStaticReport, CanonicalValue,
    SemanticDigest,
};
use serde_json::{Map, Value, json};

/// Common read-only view of one authoritative binary report.
pub trait AuthoritativeReport {
    /// Stable report kind under `spec-provenance-report-json/v2`.
    fn report_kind(&self) -> &'static str;
    /// Complete canonical semantic value.
    fn canonical_value(&self) -> &CanonicalValue;
    /// Exact authoritative binary bytes.
    fn canonical_bytes(&self) -> &[u8];
    /// Domain-separated authoritative binary identity.
    fn canonical_digest(&self) -> SemanticDigest;
}

impl AuthoritativeReport for CanonicalStaticReport {
    fn report_kind(&self) -> &'static str {
        "static"
    }
    fn canonical_value(&self) -> &CanonicalValue {
        self.value()
    }
    fn canonical_bytes(&self) -> &[u8] {
        self.bytes()
    }
    fn canonical_digest(&self) -> SemanticDigest {
        self.digest()
    }
}

impl AuthoritativeReport for CanonicalExecutionReport {
    fn report_kind(&self) -> &'static str {
        "execution"
    }
    fn canonical_value(&self) -> &CanonicalValue {
        self.value()
    }
    fn canonical_bytes(&self) -> &[u8] {
        self.bytes()
    }
    fn canonical_digest(&self) -> SemanticDigest {
        self.digest()
    }
}

impl AuthoritativeReport for CanonicalImpactReport {
    fn report_kind(&self) -> &'static str {
        "impact"
    }
    fn canonical_value(&self) -> &CanonicalValue {
        self.value()
    }
    fn canonical_bytes(&self) -> &[u8] {
        self.bytes()
    }
    fn canonical_digest(&self) -> SemanticDigest {
        self.digest()
    }
}

/// Renders deterministic JSON derived from canonical binary identity.
///
/// # Errors
///
/// Returns a JSON error only when serde cannot serialize the closed value.
pub fn render_authoritative_json(
    report: &impl AuthoritativeReport,
) -> Result<String, serde_json::Error> {
    serde_json::to_string(&presentation_value(report))
}

/// Renders deterministic locale/terminal-independent human text.
///
/// # Errors
///
/// Returns a JSON error only when serde cannot pretty-print the closed value.
pub fn render_authoritative_human(
    report: &impl AuthoritativeReport,
) -> Result<String, serde_json::Error> {
    let mut output = format!(
        "report-schema: spec-provenance-report-human/v2\nreport-kind: {}\nauthoritative-binary-digest: {}\nauthoritative-binary-bytes: {}\ncanonical-value:\n",
        report.report_kind(),
        report.canonical_digest().canonical_text(),
        hex(report.canonical_bytes()),
    );
    let _ = writeln!(output, "{:#}", canonical_value(report.canonical_value()));
    Ok(output)
}

fn presentation_value(report: &impl AuthoritativeReport) -> Value {
    let mut object = Map::new();
    object.insert(
        String::from("schema"),
        Value::String(String::from("spec-provenance-report-json/v2")),
    );
    object.insert(
        String::from("report_kind"),
        Value::String(report.report_kind().to_owned()),
    );
    object.insert(
        String::from("authoritative_binary_digest"),
        Value::String(report.canonical_digest().canonical_text()),
    );
    object.insert(
        String::from("authoritative_binary_hex"),
        Value::String(hex(report.canonical_bytes())),
    );
    object.insert(
        String::from("canonical_value"),
        canonical_value(report.canonical_value()),
    );
    Value::Object(object)
}

fn canonical_value(value: &CanonicalValue) -> Value {
    match value {
        CanonicalValue::False => json!({ "type": "false" }),
        CanonicalValue::True => json!({ "type": "true" }),
        CanonicalValue::U64(value) => json!({ "type": "u64", "value": value.to_string() }),
        CanonicalValue::I64(value) => json!({ "type": "i64", "value": value.to_string() }),
        CanonicalValue::Bytes(value) => json!({ "type": "bytes", "hex": hex(value) }),
        CanonicalValue::Text(value) => json!({ "type": "text", "value": value }),
        CanonicalValue::Record(fields) => json!({
            "type": "record",
            "fields": fields.iter().map(|(tag, value)| json!({
                "tag": tag,
                "value": canonical_value(value),
            })).collect::<Vec<_>>(),
        }),
        CanonicalValue::Sequence(values) => json!({
            "type": "sequence",
            "values": values.iter().map(canonical_value).collect::<Vec<_>>(),
        }),
        CanonicalValue::Set(values) => {
            let mut ordered = values
                .iter()
                .filter_map(|value| value.encode().ok().map(|bytes| (bytes, value)))
                .collect::<Vec<_>>();
            ordered.sort_unstable_by(|left, right| left.0.cmp(&right.0));
            json!({
                "type": "set",
                "values": ordered.into_iter().map(|(_, value)| canonical_value(value)).collect::<Vec<_>>(),
            })
        }
        CanonicalValue::None => json!({ "type": "option", "value": Value::Null }),
        CanonicalValue::Some(value) => {
            json!({ "type": "option", "value": canonical_value(value) })
        }
        CanonicalValue::Enum(tag, value) => json!({
            "type": "enum",
            "tag": tag,
            "value": value.as_ref().map(|value| canonical_value(value)),
        }),
        CanonicalValue::Digest(value) => json!({
            "type": "digest",
            "algorithm": value.algorithm().canonical_name(),
            "assurance_class": value.algorithm().assurance_class().canonical_name(),
            "hex": hex(value.bytes()),
        }),
    }
}

/// Renders one closed canonical value for deterministic coverage controls.
#[cfg(feature = "coverage-controls")]
#[doc(hidden)]
#[must_use]
pub fn render_canonical_value_control(value: &CanonicalValue) -> Value {
    canonical_value(value)
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

#[cfg(all(test, feature = "coverage-controls"))]
#[allow(clippy::expect_used)]
mod coverage_tests {
    use super::*;

    fn digest(byte: u8) -> SemanticDigest {
        SemanticDigest::xxh3_128([byte; 16])
    }

    #[test]
    fn every_report_view_and_canonical_value_variant_renders() {
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
        .expect("static coverage report");
        let execution_report = CanonicalExecutionReport::new(
            static_report.digest(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
        )
        .expect("execution coverage report");
        let impact_report = CanonicalImpactReport::new(
            digest(2),
            digest(3),
            digest(4),
            digest(5),
            Vec::new(),
            Vec::new(),
        )
        .expect("impact coverage report");

        assert!(
            render_authoritative_json(&execution_report)
                .expect("execution JSON")
                .contains("\"report_kind\":\"execution\"")
        );
        assert!(
            render_authoritative_human(&execution_report)
                .expect("execution human")
                .contains("report-kind: execution")
        );
        assert!(
            render_authoritative_json(&impact_report)
                .expect("impact JSON")
                .contains("\"report_kind\":\"impact\"")
        );
        assert!(
            render_authoritative_human(&impact_report)
                .expect("impact human")
                .contains("report-kind: impact")
        );

        let variants = vec![
            CanonicalValue::False,
            CanonicalValue::True,
            CanonicalValue::U64(1),
            CanonicalValue::I64(-1),
            CanonicalValue::Bytes(vec![0xab]),
            CanonicalValue::Text(String::from("text")),
            CanonicalValue::Record(vec![(1, CanonicalValue::True)]),
            CanonicalValue::Sequence(vec![CanonicalValue::False]),
            CanonicalValue::Set(vec![CanonicalValue::U64(2)]),
            CanonicalValue::None,
            CanonicalValue::Some(Box::new(CanonicalValue::I64(3))),
            CanonicalValue::Enum(4, Some(Box::new(CanonicalValue::True))),
            CanonicalValue::Digest(digest(6)),
        ];
        let rendered = variants
            .iter()
            .map(render_canonical_value_control)
            .collect::<Vec<_>>();
        assert_eq!(rendered.len(), variants.len());
        assert!(rendered.iter().all(Value::is_object));
    }
}
