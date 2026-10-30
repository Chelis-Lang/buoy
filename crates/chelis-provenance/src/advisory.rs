//! Advisory Chelis provenance reporting.
//!
//! The advisory report names visible gaps without granting or blocking any
//! assurance class. It reports unatomized normative-looking text, uncovered
//! atoms, undecided surface members, possible untracked copies, active debt
//! and waivers, stale derivations, and carrier counts.

use crate::command::{CommandError, StaticOutcome};
use crate::records::extract;
use crate::snapshot::ChelisSnapshot;
use buoy::core::EvidenceRole;
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};

const RFC_2119_TERMS: [&str; 5] = ["MUST", "SHALL", "REQUIRED", "SHOULD", "RECOMMENDED"];

#[derive(Serialize)]
struct AdvisoryReport<'a> {
    schema: &'static str,
    unatomized_normative_lines: &'a [String],
    uncovered_atoms: &'a [String],
    undecided_surface_members: &'a [String],
    possible_untracked_copies: &'a [String],
    active_debt: &'a [String],
    active_waivers: &'a [String],
    stale_derivations: &'a [String],
    carrier_counts: &'a BTreeMap<&'static str, usize>,
}

const fn role_name(role: EvidenceRole) -> &'static str {
    match role {
        EvidenceRole::Positive => "positive",
        EvidenceRole::Negative => "negative",
        EvidenceRole::Property => "property",
        EvidenceRole::ExhaustiveModel => "exhaustive-model",
        EvidenceRole::FormalProperty => "formal-property",
        EvidenceRole::Structural => "structural",
    }
}

/// Renders the canonical advisory report for one static outcome.
///
/// # Errors
///
/// Returns an encoding failure outside the report protocol.
#[allow(clippy::too_many_lines)]
pub fn advisory_report(
    snapshot: &ChelisSnapshot,
    outcome: &StaticOutcome,
) -> Result<String, CommandError> {
    let mut unatomized = Vec::new();
    for (path, dialect, bytes) in snapshot.files() {
        let Ok(extraction) = extract(dialect, bytes) else {
            continue;
        };
        let record_spans = extraction
            .records
            .iter()
            .map(|record| (record.start, record.end))
            .collect::<Vec<_>>();
        let Ok(text) = core::str::from_utf8(bytes) else {
            continue;
        };
        let mut offset = 0_u64;
        for (index, line) in text.split_inclusive('\n').enumerate() {
            let start = offset;
            offset += line.len() as u64;
            let inside_record = record_spans
                .iter()
                .any(|(record_start, record_end)| start >= *record_start && start < *record_end);
            if inside_record {
                continue;
            }
            if RFC_2119_TERMS.iter().any(|term| line.contains(term)) {
                unatomized.push(format!("{}:{}", path.as_str(), index + 1));
            }
        }
    }
    unatomized.sort();
    let covered = outcome
        .graph
        .evidence_carriers
        .iter()
        .flat_map(|carrier| {
            carrier
                .atoms()
                .iter()
                .map(|atom| atom.id().as_str().to_owned())
        })
        .collect::<BTreeSet<_>>();
    let uncovered = outcome
        .graph
        .atoms
        .iter()
        .map(|atom| atom.meta().id().as_str().to_owned())
        .filter(|id| !covered.contains(id))
        .collect::<Vec<_>>();
    let mut undecided = Vec::new();
    for surface in &outcome.graph.decision_surfaces {
        let decided = surface
            .dispositions()
            .iter()
            .map(|row| row.key().bytes().to_vec())
            .collect::<BTreeSet<_>>();
        for member in surface.domain().member_keys() {
            if !decided.contains(member.bytes()) {
                undecided.push(format!(
                    "{}:{}",
                    surface.meta().id().as_str(),
                    String::from_utf8_lossy(member.bytes())
                ));
            }
        }
    }
    undecided.sort();
    let mut byte_groups = BTreeMap::<&[u8], Vec<String>>::new();
    for (path, _, bytes) in snapshot.files() {
        byte_groups
            .entry(bytes)
            .or_default()
            .push(path.as_str().to_owned());
    }
    let mut copies = byte_groups
        .into_values()
        .filter(|paths| paths.len() > 1)
        .map(|paths| paths.join("|"))
        .collect::<Vec<_>>();
    copies.sort();
    let mut debt = outcome
        .graph
        .debt_records
        .iter()
        .map(|record| record.meta().id().as_str().to_owned())
        .collect::<Vec<_>>();
    debt.sort();
    let mut waivers = outcome
        .graph
        .waivers
        .iter()
        .map(|record| record.meta().id().as_str().to_owned())
        .collect::<Vec<_>>();
    waivers.sort();
    let mut stale = outcome
        .diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.code.starts_with("CHELIS-PROV-DERIVATION"))
        .map(|diagnostic| format!("{}:{}", diagnostic.code, diagnostic.context))
        .collect::<Vec<_>>();
    stale.sort();
    stale.dedup();
    let mut carrier_counts = BTreeMap::new();
    for carrier in &outcome.graph.evidence_carriers {
        *carrier_counts.entry(role_name(carrier.role())).or_default() += 1;
    }
    serde_json::to_string(&AdvisoryReport {
        schema: "chelis-advisory-report/v1",
        unatomized_normative_lines: &unatomized,
        uncovered_atoms: &uncovered,
        undecided_surface_members: &undecided,
        possible_untracked_copies: &copies,
        active_debt: &debt,
        active_waivers: &waivers,
        stale_derivations: &stale,
        carrier_counts: &carrier_counts,
    })
    .map_err(|_| CommandError::Encoding)
}
