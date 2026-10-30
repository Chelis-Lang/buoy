//! Deterministic full-chain trace projection for one stable ID.
//!
//! The projection derives from the same graph the static check builds and
//! from the saved record bytes. Every hop reports its bound value beside the
//! value recomputed from the current saved bytes, so one command replaces
//! the manual chain audit. The projection grants no authority: it is a
//! rendering of the static outcome and the saved records.

use crate::command::{
    CommandError, EXIT_INPUT, EXIT_OK, EXIT_STATIC, StaticOutcome, artifact_digest, binding_digest,
    snapshot_digests, static_check,
};
use crate::records::extract;
use crate::snapshot::{ChelisConfiguration, ChelisSnapshot};
use buoy::core::{AtomRevisionRef, SemanticDigest};
use serde::Serialize;
use std::collections::BTreeMap;

/// One bound-versus-current hop comparison.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct TraceHop {
    /// Hop description, for example `atom-revision` or `item-digest`.
    pub hop: String,
    /// Recorded bound value.
    pub bound: String,
    /// Value recomputed from the current saved bytes, or `absent`.
    pub current: String,
    /// Whether the bound value matches the recomputed value.
    pub matches: bool,
}

/// One traced object with its hop comparisons.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct TraceObject {
    /// Stable object ID.
    pub id: String,
    /// Canonical object identity digest.
    pub digest: String,
    /// Hop comparisons in deterministic order.
    pub hops: Vec<TraceHop>,
}

#[derive(Serialize)]
struct BindingTraceObject {
    id: String,
    digest: String,
    verified: bool,
    hops: Vec<TraceHop>,
}

#[derive(Serialize)]
struct TraceReport<'a> {
    schema: &'static str,
    id: &'a str,
    atom: Option<TraceObject>,
    surfaces: Vec<TraceObject>,
    links: Vec<TraceObject>,
    bindings: Vec<BindingTraceObject>,
    carriers: Vec<TraceObject>,
    static_verdict: &'a str,
    verdict: &'static str,
}

/// Complete deterministic trace outcome.
#[derive(Clone, Debug)]
pub struct TraceOutcome {
    /// Canonical machine-report bytes.
    pub report: String,
    /// Core exit class.
    pub exit: u8,
}

fn atom_hops(
    atoms: &BTreeMap<String, SemanticDigest>,
    references: &[AtomRevisionRef],
    subject: &str,
    hops: &mut Vec<TraceHop>,
) {
    for reference in references {
        if reference.id().as_str() != subject {
            continue;
        }
        let bound = reference.revision().canonical_text();
        let current = atoms
            .get(reference.id().as_str())
            .map_or_else(|| String::from("absent"), SemanticDigest::canonical_text);
        hops.push(TraceHop {
            hop: format!("atom-revision:{}", reference.id().as_str()),
            matches: bound == current,
            bound,
            current,
        });
    }
}

fn item_hop(
    items: &BTreeMap<String, SemanticDigest>,
    object_id: &str,
    bound_digest: SemanticDigest,
    hops: &mut Vec<TraceHop>,
) {
    let bound = bound_digest.canonical_text();
    let current = items
        .get(object_id)
        .map_or_else(|| String::from("absent"), SemanticDigest::canonical_text);
    hops.push(TraceHop {
        hop: String::from("item-digest"),
        matches: bound == current,
        bound,
        current,
    });
}

/// Returns the materialized binding one marker address resolved to.
fn resolved_binding<'a>(
    graph: &'a buoy::core::SpecGraph,
    address: &str,
) -> Option<&'a buoy::core::ImplementationBinding> {
    graph
        .implementation_bindings
        .iter()
        .find(|binding| binding.record().canonical_text() == address)
}

/// Reads the record a refused store object still describes, when it has one.
///
/// The static check materializes nothing from an object it refused, so the
/// projection reads those bytes directly to name the address they now hash
/// to. The record grants no authority here: it exists only so a tampered
/// object reports one failed hop instead of disappearing from the trace.
fn unresolved_binding(
    snapshot: &ChelisSnapshot,
    configuration: &ChelisConfiguration,
    address: &str,
) -> Option<buoy::core::ImplementationBinding> {
    let active = crate::command::binding_algorithm().ok()?;
    let store = configuration.binding_store.as_ref()?;
    let digest = SemanticDigest::parse_canonical(address).ok()?;
    let object_path = crate::command::binding_object_repository_path(store.as_str(), digest);
    let bytes = snapshot.artifact(&object_path)?;
    buoy::adapters::parse_binding_record(active, bytes)
}

/// Projects the complete digest chain for one stable ID.
///
/// # Errors
///
/// Returns an adapter or encoding failure outside the report protocol.
#[allow(clippy::too_many_lines)]
pub fn trace_projection(
    snapshot: &ChelisSnapshot,
    configuration: &ChelisConfiguration,
    subject: &str,
) -> Result<TraceOutcome, CommandError> {
    let outcome: StaticOutcome = static_check(snapshot, configuration)?;
    let graph = &outcome.graph;
    let atoms = graph
        .atoms
        .iter()
        .map(|atom| (atom.meta().id().as_str().to_owned(), atom.revision()))
        .collect::<BTreeMap<_, _>>();
    let oracles = graph
        .oracles
        .iter()
        .map(|oracle| {
            (
                oracle.meta().id().as_str().to_owned(),
                oracle.meta().digest(),
            )
        })
        .collect::<BTreeMap<_, _>>();
    let mut items = BTreeMap::new();
    let file_digests = snapshot_digests(snapshot)?;
    let mut runner_bindings = Vec::new();
    // A compact marker carries no stable ID, so its declaration is keyed by
    // the content address the object resolved from.
    let mut binding_declarations = BTreeMap::new();
    for (_, dialect, bytes) in snapshot.files() {
        let Ok(extraction) = extract(dialect, bytes) else {
            continue;
        };
        for record in &extraction.records {
            if dialect == crate::records::ChelisDialect::Surf
                && record.kind == crate::records::BINDING_RECORD_KIND
            {
                if let Some(address) = record.fields.get(crate::records::BINDING_ADDRESS_FIELD) {
                    binding_declarations.insert(address.clone(), record.declaration_text.clone());
                }
                continue;
            }
            let Some(id) = record.fields.get("id") else {
                continue;
            };
            items.insert(
                id.clone(),
                artifact_digest(record.declaration_text.as_bytes())?,
            );
            if record.kind == "oracle" {
                if let Some(paths) = record.fields.get("runner-config-paths") {
                    runner_bindings.push((id.clone(), paths.clone(), record.fields.clone()));
                }
            }
        }
    }
    let atom = graph
        .atoms
        .iter()
        .find(|value| value.meta().id().as_str() == subject)
        .map(|value| TraceObject {
            id: subject.to_owned(),
            digest: value.meta().digest().canonical_text(),
            hops: vec![TraceHop {
                hop: String::from("atom-revision"),
                bound: value.revision().canonical_text(),
                current: value.revision().canonical_text(),
                matches: true,
            }],
        });
    let mut surfaces = Vec::new();
    for surface in &graph.decision_surfaces {
        let mut hops = Vec::new();
        for row in surface.dispositions() {
            if let Some(reference) = row.authority() {
                atom_hops(&atoms, core::slice::from_ref(reference), subject, &mut hops);
            }
        }
        if !hops.is_empty() {
            surfaces.push(TraceObject {
                id: surface.meta().id().as_str().to_owned(),
                digest: surface.meta().digest().canonical_text(),
                hops,
            });
        }
    }
    let mut links = Vec::new();
    for link in &graph.implementation_links {
        let mut hops = Vec::new();
        atom_hops(&atoms, link.atoms(), subject, &mut hops);
        if hops.is_empty() {
            continue;
        }
        item_hop(
            &items,
            link.meta().id().as_str(),
            link.parsed_item_digest(),
            &mut hops,
        );
        links.push(TraceObject {
            id: link.meta().id().as_str().to_owned(),
            digest: link.meta().digest().canonical_text(),
            hops,
        });
    }
    // A compact binding hides its fields behind one content address, so the
    // projection expands the store hop explicitly. Markers drive the walk, not
    // the graph, so an object the static check refused still projects its own
    // failed hop instead of vanishing into an empty array.
    let mut bindings = Vec::new();
    for (address, declaration) in &binding_declarations {
        let (record, verified) = if let Some(record) = resolved_binding(graph, address).cloned() {
            (record, true)
        } else {
            let Some(record) = unresolved_binding(snapshot, configuration, address) else {
                continue;
            };
            (record, false)
        };
        let mut hops = Vec::new();
        atom_hops(
            &atoms,
            core::slice::from_ref(record.atom()),
            subject,
            &mut hops,
        );
        if hops.is_empty() {
            continue;
        }
        let recomputed = record.record().canonical_text();
        hops.push(TraceHop {
            hop: String::from("binding-object"),
            matches: *address == recomputed,
            bound: address.clone(),
            current: recomputed,
        });
        let active = crate::command::binding_algorithm().ok();
        let recompute = |bytes: &[u8]| match active {
            Some(active) => binding_digest(active, bytes).map(|digest| digest.canonical_text()),
            None => Ok(String::from("unavailable")),
        };
        let bound_subject = record.subject().digest().canonical_text();
        let current_subject = recompute(declaration.as_bytes())?;
        hops.push(TraceHop {
            hop: String::from("binding-subject"),
            matches: current_subject == bound_subject,
            bound: bound_subject,
            current: current_subject,
        });
        // The requirement digest covers the exact authority record block, so
        // the current value comes from those bytes and not from the record
        // that already claims them.
        let bound_requirement = record.requirement().digest().canonical_text();
        let current_requirement =
            match crate::bind::authority_block(snapshot, record.atom().id().as_str()) {
                Some(authority) => recompute(authority.bytes.as_bytes())?,
                None => String::from("absent"),
            };
        hops.push(TraceHop {
            hop: String::from("binding-requirement"),
            matches: current_requirement == bound_requirement,
            bound: bound_requirement,
            current: current_requirement,
        });
        bindings.push(BindingTraceObject {
            id: record.meta().id().as_str().to_owned(),
            digest: record.meta().digest().canonical_text(),
            verified,
            hops,
        });
    }
    let mut carriers = Vec::new();
    for carrier in &graph.evidence_carriers {
        let mut hops = Vec::new();
        atom_hops(&atoms, carrier.atoms(), subject, &mut hops);
        if hops.is_empty() {
            continue;
        }
        item_hop(
            &items,
            carrier.meta().id().as_str(),
            carrier.parsed_item_digest(),
            &mut hops,
        );
        let bound = carrier.oracle().digest().canonical_text();
        let current = oracles
            .get(carrier.oracle().id().as_str())
            .map_or_else(|| String::from("absent"), SemanticDigest::canonical_text);
        hops.push(TraceHop {
            hop: format!("oracle-digest:{}", carrier.oracle().id().as_str()),
            matches: bound == current,
            bound,
            current,
        });
        for (oracle_id, paths, fields) in &runner_bindings {
            if oracle_id != carrier.oracle().id().as_str() {
                continue;
            }
            for (index, path) in paths
                .split(';')
                .map(str::trim)
                .filter(|entry| !entry.is_empty())
                .enumerate()
            {
                let field = if index == 0 {
                    "runner-digest"
                } else {
                    "runner-configuration-digest"
                };
                let bound = fields.get(field).cloned().unwrap_or_default();
                let current = file_digests
                    .get(path)
                    .map_or_else(|| String::from("absent"), SemanticDigest::canonical_text);
                hops.push(TraceHop {
                    hop: format!("{field}:{path}"),
                    matches: bound == current,
                    bound,
                    current,
                });
            }
        }
        carriers.push(TraceObject {
            id: carrier.meta().id().as_str().to_owned(),
            digest: carrier.meta().digest().canonical_text(),
            hops,
        });
    }
    if atom.is_none()
        && surfaces.is_empty()
        && links.is_empty()
        && bindings.is_empty()
        && carriers.is_empty()
    {
        return Ok(TraceOutcome {
            report: format!(
                "{{\"schema\":\"chelis-trace-projection/v2\",\"id\":\"{subject}\",\"verdict\":\"unknown-id\"}}"
            ),
            exit: EXIT_INPUT,
        });
    }
    let object_hops_match = surfaces
        .iter()
        .chain(&links)
        .chain(&carriers)
        .flat_map(|object| &object.hops)
        .all(|hop| hop.matches);
    let binding_hops_match = bindings
        .iter()
        .flat_map(|object| &object.hops)
        .all(|hop| hop.matches);
    let all_match = object_hops_match && binding_hops_match;
    let accepted = outcome.exit == EXIT_OK && all_match;
    let report = serde_json::to_string(&TraceReport {
        schema: "chelis-trace-projection/v2",
        id: subject,
        atom,
        surfaces,
        links,
        bindings,
        carriers,
        static_verdict: if outcome.diagnostics.is_empty() {
            "accept"
        } else {
            "reject"
        },
        verdict: if accepted { "accept" } else { "reject" },
    })
    .map_err(|_| CommandError::Encoding)?;
    Ok(TraceOutcome {
        report,
        exit: if accepted { EXIT_OK } else { EXIT_STATIC },
    })
}
