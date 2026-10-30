#![doc = "RTP-007 publication projection conformance against real rendered Markdown."]
#![allow(missing_docs, clippy::expect_used, clippy::panic)]
// The reviewed trace snapshot below selects one collision-resistant member,
// so these contracts run only under a build that contains that adapter.
#![cfg(all(unix, feature = "blake3-256"))]

use buoy::publication::{ProjectionError, validate_markdown_projection};
use std::path::Path;
use std::process::Command;

/// The reviewed trace snapshot `readable-trace-projections` stages for its
/// own command tests. Rendering through the real `buoy trace` executable --
/// rather than a hand-written Markdown string -- is what makes these
/// negative controls oracles for the *production* renderer: if the renderer
/// stops emitting a required label or facet, the positive control inside
/// each test below fails first.
const FIXTURE_REPOSITORY: &str = "fixtures/readable-trace-projections/v1/repository";

const RELATION_ID: &str = "BGLC-RENDER-CATALOG";

/// Copies the complete reviewed snapshot into one writable temporary root.
///
/// The whole tree is walked rather than a hand-listed file set, so this
/// stages every committed fixture byte -- including the content-addressed
/// binding record whose name is a digest -- without restating any digest
/// literal in test source.
fn staged_repository() -> tempfile::TempDir {
    let target = tempfile::TempDir::new().expect("temporary trace root");
    let source = Path::new(env!("CARGO_MANIFEST_DIR")).join(FIXTURE_REPOSITORY);
    let mut directories = Vec::from([source.clone()]);
    let mut copied = 0_usize;
    while let Some(directory) = directories.pop() {
        for entry in std::fs::read_dir(&directory).expect("fixture directory") {
            let path = entry.expect("fixture entry").path();
            if path.is_dir() {
                directories.push(path);
                continue;
            }
            let relative = path.strip_prefix(&source).expect("fixture-relative path");
            let destination = target.path().join(relative);
            std::fs::create_dir_all(destination.parent().expect("fixture parent"))
                .expect("staged fixture directory");
            std::fs::copy(&path, &destination).expect("copied trace fixture");
            copied += 1;
        }
    }
    assert!(copied >= 5, "the reviewed snapshot must stage real files");
    target
}

/// Renders one real `buoy/readable-trace-markdown/v1` projection through the
/// committed executable.
fn rendered_projection() -> String {
    let root = staged_repository();
    let config = root.path().join("buoy.toml");
    let output = Command::new(env!("CARGO_BIN_EXE_buoy"))
        .args([
            "trace",
            "--root",
            root.path().to_str().expect("UTF-8 root"),
            "--config",
            config.to_str().expect("UTF-8 config"),
            "--relation-id",
            RELATION_ID,
            "--format",
            "markdown",
        ])
        .output()
        .expect("real trace command");
    assert_eq!(
        output.status.code(),
        Some(0),
        "the Markdown projection must render: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).expect("UTF-8 Markdown projection")
}

/// RTP-007 negative "graph domain is ambiguous": a projection that labels a
/// source-graph digest only as `Graph` must fail publication projection
/// conformance with the stable `RTP-AMBIGUOUS-GRAPH-LABEL` code.
///
/// The unmutated production render is asserted to PASS first, so this is a
/// real fail-closed control rather than a vacuous rejection of arbitrary
/// bytes.
#[test]
fn an_ambiguous_graph_label_fails_publication_projection_conformance() {
    let rendered = rendered_projection();
    validate_markdown_projection(&rendered)
        .expect("the real production projection must satisfy its own conformance");

    assert!(
        rendered.contains("- Source graph: `"),
        "the production renderer must label the source graph unambiguously: {rendered}"
    );
    let ambiguous = rendered.replace("- Source graph: `", "- Graph: `");
    assert_ne!(ambiguous, rendered, "the mutation must change real bytes");
    assert_eq!(
        validate_markdown_projection(&ambiguous),
        Err(ProjectionError::AmbiguousGraphLabel)
    );
    assert_eq!(
        ProjectionError::AmbiguousGraphLabel.stable_code(),
        "RTP-AMBIGUOUS-GRAPH-LABEL"
    );
}

/// RTP-007 negative "page omits the atom revision": a relation row that
/// drops its complete semantic atom revision must fail publication
/// projection conformance with the stable `RTP-MISSING-ATOM-REVISION` code,
/// while `buoy/trace-report/v1` itself remains unchanged.
#[test]
fn a_row_without_its_complete_atom_revision_fails_publication_projection_conformance() {
    let rendered = rendered_projection();
    validate_markdown_projection(&rendered)
        .expect("the real production projection must satisfy its own conformance");

    assert!(
        rendered.contains("Atom revision"),
        "the production renderer must expose the atom revision column: {rendered}"
    );
    let without_revision = rendered.replace("| Atom revision |", "|");
    assert_ne!(
        without_revision, rendered,
        "the mutation must change real bytes"
    );
    assert_eq!(
        validate_markdown_projection(&without_revision),
        Err(ProjectionError::MissingAtomRevision)
    );
    assert_eq!(
        ProjectionError::MissingAtomRevision.stable_code(),
        "RTP-MISSING-ATOM-REVISION"
    );

    // The machine trace report is a separate schema: dropping a rendered
    // Markdown column must never be reachable from, or change, it.
    assert!(
        rendered.contains("buoy/trace-report/v1"),
        "the projection must keep identifying its unchanged machine schema"
    );
}
