//! Registry-mediated content bindings for generated evidence.

#![allow(dead_code)]

use buoy_core::{CanonicalValue, SemanticDigest, semantic_digest};
use std::fs;
use std::path::Path;

/// Compute one canonical version 3 binding from authoritative bytes.
#[allow(clippy::expect_used)]
pub fn bytes_binding(bytes: &[u8]) -> String {
    semantic_digest(
        "repository-content-binding",
        vec![(1, CanonicalValue::Bytes(bytes.to_vec()))],
    )
    .expect("authoritative byte bindings must canonically encode")
    .canonical_text()
}

/// Compute one canonical version 3 binding for a file.
pub fn file_binding(path: &Path) -> Result<String, String> {
    fs::read(path)
        .map(|bytes| bytes_binding(&bytes))
        .map_err(|error| format!("cannot digest {}: {error}", path.display()))
}

fn validate_binding_pair(first: &str, second: &str) -> Result<(), String> {
    if first != second {
        return Err(String::from("digest binding is not deterministic"));
    }
    SemanticDigest::parse_canonical(first)
        .map_err(|error| format!("digest binding is not canonical: {}", error.code()))?;
    Ok(())
}

/// Verify deterministic registry dispatch and canonical parsing.
#[allow(clippy::expect_used, clippy::unnecessary_wraps)]
pub fn self_test() -> Result<(), String> {
    for bytes in [b"".as_slice(), b"abc".as_slice()] {
        let first = bytes_binding(bytes);
        let second = bytes_binding(bytes);
        validate_binding_pair(&first, &second).expect("repeated digest bindings are deterministic");
    }
    let _ = validate_binding_pair("first", "second").expect_err("different bindings must fail");
    let _ = validate_binding_pair("invalid", "invalid").expect_err("malformed bindings must fail");
    let missing = Path::new("fixtures/digest/v3/missing-binding-control");
    let _ = file_binding(missing).expect_err("missing binding input must fail");
    Ok(())
}
