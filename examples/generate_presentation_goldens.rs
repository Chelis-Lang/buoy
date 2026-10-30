//! Maintainer-only generator for reviewed deterministic report presentations.

#![allow(clippy::expect_used)]

use buoy::core::{CanonicalStaticReport, SemanticDigest};
use buoy::presentation::{render_authoritative_human, render_authoritative_json};
use std::env;

fn digest(byte: u8) -> SemanticDigest {
    SemanticDigest::xxh3_128([byte; 16])
}

fn main() {
    let report = CanonicalStaticReport::new(
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
    let mode = env::args().nth(1);
    let rendered = match mode.as_deref() {
        Some("json") => render_authoritative_json(&report).map(|value| value + "\n"),
        Some("human") => render_authoritative_human(&report),
        _ => {
            eprintln!("usage: generate_presentation_goldens <json|human>");
            std::process::exit(2);
        }
    };
    print!(
        "{}",
        rendered.expect("closed report presentation is infallible")
    );
}
