//! Maintainer-only generator for reviewed canonical empty-report golden vectors.

#![allow(clippy::expect_used)]

use buoy::core::{
    CanonicalExecutionReport, CanonicalImpactReport, CanonicalStaticReport, SemanticDigest,
};

fn digest(byte: u8) -> SemanticDigest {
    SemanticDigest::xxh3_128([byte; 16])
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

fn main() {
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
    .expect("closed empty static report canonically encodes");
    let execution_report =
        CanonicalExecutionReport::new(static_report.digest(), Vec::new(), Vec::new(), Vec::new())
            .expect("closed empty execution report canonically encodes");
    let impact_report = CanonicalImpactReport::new(
        digest(2),
        digest(3),
        digest(4),
        digest(5),
        Vec::new(),
        Vec::new(),
    )
    .expect("closed empty impact report canonically encodes");
    println!("schema\treport\tdigest\tcanonical_hex");
    for (name, report_digest, bytes) in [
        ("static", static_report.digest(), static_report.bytes()),
        (
            "execution",
            execution_report.digest(),
            execution_report.bytes(),
        ),
        ("impact", impact_report.digest(), impact_report.bytes()),
    ] {
        println!(
            "spec-provenance-report/v2\t{name}\t{}\t{}",
            report_digest.canonical_text(),
            hex(bytes)
        );
    }
}
