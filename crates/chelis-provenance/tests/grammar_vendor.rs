//! Fail-closed drift check for the vendored Surf grammar derivation.
//!
//! The registered derivation in the frozen boundary names the exact Chelis
//! revision and the per-file digests of the vendored grammar artifacts. Any
//! byte drift in `crates/chelis-surf-sitter/vendor/` fails here until the
//! registration is reviewed and updated together with the vendor refresh.

#![allow(clippy::expect_used, clippy::panic)]

use buoy::core::{CanonicalValue, semantic_digest};
use chelis_provenance::records::ChelisDialect;
use serde::Deserialize;
use std::path::PathBuf;

#[derive(Deserialize)]
struct Boundary {
    adapters: Vec<Adapter>,
}

#[derive(Deserialize)]
struct Adapter {
    #[serde(rename = "adapter_id")]
    id: String,
    parser_version: String,
    syntax_schema_id: String,
    grammar: Option<Grammar>,
}

#[derive(Deserialize)]
struct Grammar {
    revision: String,
    tag: String,
    vendor_prefix: String,
    runtime_version: String,
    artifacts: Vec<Artifact>,
}

#[derive(Deserialize)]
struct Artifact {
    path: String,
    digest: String,
}

fn repository_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("repository root")
}

fn boundary() -> Boundary {
    let path = repository_root().join("fixtures/chelis-provenance/v1/boundary.toml");
    let text = std::fs::read_to_string(path).expect("boundary registration");
    toml::from_str::<Boundary>(&text).expect("boundary adapters")
}

#[test]
fn vendored_grammar_bytes_match_the_registered_derivation() {
    let boundary = boundary();
    let surf = boundary
        .adapters
        .iter()
        .find(|adapter| adapter.id == "chelis-surf/v1")
        .expect("surf adapter registration");
    let grammar = surf.grammar.as_ref().expect("registered grammar");
    assert_eq!(
        grammar.revision,
        chelis_surf_sitter::GRAMMAR_SOURCE_REVISION
    );
    assert_eq!(grammar.tag, chelis_surf_sitter::GRAMMAR_SOURCE_TAG);
    assert_eq!(grammar.runtime_version, "0.26.8");
    assert!(
        !grammar.artifacts.is_empty(),
        "registration lists the vendored artifacts"
    );
    let vendor = repository_root().join(&grammar.vendor_prefix);
    for artifact in &grammar.artifacts {
        let bytes = std::fs::read(vendor.join(&artifact.path))
            .unwrap_or_else(|_| panic!("vendored artifact {} exists", artifact.path));
        let rendered = semantic_digest(
            "chelis-raw-artifact",
            vec![(1, CanonicalValue::Bytes(bytes))],
        )
        .expect("artifact digest")
        .canonical_text();
        assert_eq!(
            rendered, artifact.digest,
            "vendored {} drifted from the registered derivation",
            artifact.path
        );
    }
    let mut vendored_files = Vec::new();
    collect_files(&vendor, &vendor, &mut vendored_files);
    vendored_files.sort();
    let mut registered = grammar
        .artifacts
        .iter()
        .map(|artifact| artifact.path.clone())
        .collect::<Vec<_>>();
    registered.sort();
    assert_eq!(
        vendored_files, registered,
        "every vendored file is registered and every registered file is vendored"
    );
}

fn collect_files(root: &std::path::Path, directory: &std::path::Path, out: &mut Vec<String>) {
    let entries = std::fs::read_dir(directory).expect("vendor directory");
    for entry in entries {
        let entry = entry.expect("vendor entry");
        let path = entry.path();
        if path.is_dir() {
            collect_files(root, &path, out);
        } else if let Ok(relative) = path.strip_prefix(root) {
            out.push(relative.to_string_lossy().replace('\\', "/"));
        }
    }
}

#[test]
fn frozen_adapter_identities_match_the_registration() {
    let boundary = boundary();
    for (dialect, adapter_id) in [
        (ChelisDialect::Surf, "chelis-surf/v1"),
        (ChelisDialect::Deep, "chelis-deep/v1"),
    ] {
        let registered = boundary
            .adapters
            .iter()
            .find(|adapter| adapter.id == adapter_id)
            .expect("adapter registration");
        assert_eq!(registered.parser_version, dialect.parser_version());
        assert_eq!(registered.syntax_schema_id, dialect.language_schema_id());
    }
}
