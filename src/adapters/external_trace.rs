//! Shared strict parser for repository-owned external trace documents.
//!
//! One parser owns the fence value grammar. Each host adapter keeps its own
//! parser-backed item identity and exact source mapping, so equal fence bytes
//! produce equal link values under every accepted host.
//!
//! An external trace link supplies navigation and impact data only. It
//! establishes no assurance class and reads no provider state.

use crate::traceability::{ExternalTraceLink, ExternalTraceRelation, ExternalTraceTarget};
use buoy_core::{AtomRevisionRef, SemanticDigest, StableId};
use serde::Deserialize;

/// Exact info string that marks one repository-owned trace fence.
pub const EXTERNAL_TRACE_FENCE: &str = "buoy-external-trace-v1";

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ExternalTraceDocument {
    id: String,
    atoms: Vec<String>,
    relation: String,
    target: String,
}

/// Parses one strict external trace document into its complete link value.
///
/// Unknown, duplicate, and malformed fields fail closed with one stable code.
///
/// # Errors
///
/// Returns the exact primary trace-document diagnostic for the first fault.
#[allow(clippy::redundant_closure_for_method_calls)]
pub fn parse_external_trace_document(body: &str) -> Result<ExternalTraceLink, &'static str> {
    let document =
        toml::from_str::<ExternalTraceDocument>(body).map_err(|_| "SPG-EXTERNAL-TRACE-DOCUMENT")?;
    let id = StableId::new(&document.id).map_err(|error| error.code())?;
    let atoms = document
        .atoms
        .iter()
        .map(|binding| {
            let (id, digest) = binding
                .split_once('@')
                .ok_or("SPG-EXTERNAL-TRACE-ATOM-BINDING")?;
            Ok(AtomRevisionRef::new(
                StableId::new(id).map_err(|error| error.code())?,
                SemanticDigest::parse_canonical(digest).map_err(|error| error.code())?,
            ))
        })
        .collect::<Result<Vec<_>, &'static str>>()?;
    let relation =
        ExternalTraceRelation::parse(&document.relation).map_err(|error| error.code())?;
    let target = ExternalTraceTarget::new(&document.target).map_err(|error| error.code())?;
    ExternalTraceLink::new(id, atoms, relation, target).map_err(|error| error.code())
}
