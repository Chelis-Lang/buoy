//! Fixed opaque saved-byte source adapter.

use super::SourceSpan;
use buoy_core::{
    ActiveAlgorithm, CanonicalValue, ParsedItemIdentity, RepositoryPath, SchemaId, SourceItemKind,
    SourceNode,
};

/// Built-in opaque artifact adapter identity.
pub const ARTIFACT_ADAPTER_ID: &str = "artifact/v1";
/// Fixed opaque saved-byte capability.
pub const ARTIFACT_CAPABILITY: &str = "artifact.opaque-bytes/v1";
/// Exact full-file source item kind.
pub const ARTIFACT_FILE_KIND: &str = "artifact/file/v1";
/// Fixed media type bound into every item identity.
pub const ARTIFACT_MEDIA_TYPE: &str = "application/octet-stream";
/// Fixed saved-byte representation bound into every item identity.
pub const ARTIFACT_REPRESENTATION: &str = "saved-bytes/v1";

/// One complete opaque full-file source product.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ArtifactOutput {
    /// The exact full-file source node.
    pub node: SourceNode,
    /// The complete byte-offset source span.
    pub span: SourceSpan,
}

/// Materializes one saved file without text decoding or language interpretation.
///
/// # Panics
///
/// Panics only if the compiled fixed schema IDs or canonical metadata become invalid.
#[must_use]
#[allow(clippy::expect_used)]
pub fn extract_artifact(
    active: ActiveAlgorithm,
    path: &RepositoryPath,
    bytes: &[u8],
) -> ArtifactOutput {
    let semantic_bytes = CanonicalValue::Record(vec![
        (1, CanonicalValue::Text(ARTIFACT_MEDIA_TYPE.to_owned())),
        (2, CanonicalValue::Text(ARTIFACT_REPRESENTATION.to_owned())),
        (3, CanonicalValue::Bytes(bytes.to_vec())),
    ])
    .encode()
    .expect("fixed opaque metadata and arbitrary bytes canonically encode");
    let item = ParsedItemIdentity::new_with(
        active,
        SchemaId::new(ARTIFACT_CAPABILITY).expect("the artifact schema ID is valid"),
        path.clone(),
        SourceItemKind::new(ARTIFACT_FILE_KIND).expect("the artifact item kind is valid"),
        semantic_bytes,
        None,
    )
    .expect("one parsed artifact path and fixed metadata construct");
    let node = SourceNode::new_with(active, item, Vec::new())
        .expect("one childless artifact source node constructs");
    ArtifactOutput {
        node,
        span: SourceSpan {
            path: path.clone(),
            start: 0,
            end: bytes.len() as u64,
        },
    }
}
