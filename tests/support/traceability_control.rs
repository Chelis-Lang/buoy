//! Reviewed workflow control for one completed `traceability.md` artifact.
//!
//! The control reads repository-owned planning text only. It reads no provider
//! state and establishes no assurance class.

// This module is `#[path]`-included by several independent test targets, and
// each one uses a different subset of its API (an archived-artifact target
// never calls the active-artifact entry point, and vice versa). Unused-item
// warnings here therefore report only which target is compiling, not a real
// defect.
#![allow(dead_code)]

/// Closed fault classes one traceability artifact can carry.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum TraceabilityFault {
    /// One table row carries a placeholder identity instead of real intent.
    PlaceholderIdentity,
    /// One planned row carries a current digest that no bind operation produced.
    PrematureCompletion,
    /// One completion row copies the immutable record body.
    CopiedRecordBody,
    /// The artifact records provider state.
    ProviderStatus,
    /// One table row omits a required column or leaves one column empty.
    IncompleteRow,
    /// One affected requirement has no link-review row.
    MissingLinkReview,
    /// A manual completion row conflicts with generated evidence.
    GeneratedEvidenceDrift,
    /// The document claims to be the current state itself.
    CurrentAuthorityClaim,
    /// An ACTIVE workflow artifact omits the `## Completion Snapshot` heading.
    ///
    /// Every completion rule in this control is scoped to that section, so a
    /// deleted heading is strictly WEAKER than a deleted block: it skips the
    /// copied-record-body check, the row-shape check, the placeholder check,
    /// the drift check, the current-authority check, AND both block checks
    /// below, all silently. Deleting the section must therefore never be
    /// cheaper than deleting anything inside it.
    MissingCompletionSection,
    /// An ACTIVE workflow artifact omits the marker-delimited generated
    /// evidence block entirely.
    ///
    /// This is not a cosmetic omission. Every other generated-evidence check
    /// in this control -- drift against a manual row, a placeholder identity,
    /// a current-authority claim -- is reachable only once the marker is
    /// present. An active artifact with no marker therefore skips all of them
    /// silently AND falls back to the legacy four-column shape, so the
    /// generated-evidence contract stops being enforced at all. Archived
    /// documents legitimately predate the marker, which is why this fault is
    /// raised only for artifacts declared active.
    MissingGeneratedBlock,
    /// An ACTIVE workflow artifact opens the generated evidence block but
    /// never closes it.
    ///
    /// An unterminated block leaves the generated region unbounded, so
    /// arbitrary later prose and hand-authored rows would read as generated
    /// evidence. This is a distinct defect from an absent block: the marker
    /// exists, so an absence check alone would accept the document.
    UnterminatedGeneratedBlock,
    /// An ACTIVE workflow artifact omits the separate human completion block.
    ///
    /// The generated block cannot supply owner, consumer commit, or review
    /// decision; those stay human-authored. Deleting the human table would
    /// otherwise leave the section looking complete while every human
    /// completion fact silently disappeared.
    MissingHumanCompletionBlock,
}

impl TraceabilityFault {
    /// Stable machine diagnostic name.
    pub const fn name(self) -> &'static str {
        match self {
            Self::PlaceholderIdentity => "placeholder-identity",
            Self::PrematureCompletion => "premature-completion",
            Self::CopiedRecordBody => "copied-record-body",
            Self::ProviderStatus => "provider-status",
            Self::IncompleteRow => "incomplete-row",
            Self::MissingLinkReview => "missing-link-review",
            Self::GeneratedEvidenceDrift => "generated-evidence-drift",
            Self::CurrentAuthorityClaim => "current-authority-claim",
            Self::MissingCompletionSection => "missing-completion-section",
            Self::MissingGeneratedBlock => "missing-generated-block",
            Self::UnterminatedGeneratedBlock => "unterminated-generated-block",
            Self::MissingHumanCompletionBlock => "missing-human-completion-block",
        }
    }
}

/// Whether one traceability artifact is an ACTIVE workflow document, which
/// must carry the marker-delimited generated evidence block, or an ARCHIVED
/// document, which may legitimately predate the marker.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ArtifactLifecycle {
    /// A live workflow artifact. The generated block is mandatory.
    Active,
    /// A historical artifact retained for read-only compatibility. The
    /// original four-column completion table stays readable without a fault.
    Archived,
}

/// Placeholder text that never names one real object.
const PLACEHOLDERS: [&str; 5] = ["none", "n/a", "tbd", "pending", "todo"];

/// Record-body keys that belong to the immutable object, not to the snapshot.
const RECORD_BODY_KEYS: [&str; 5] = [
    "schema = \"buoy/implementation-binding/v1\"",
    "navigation =",
    "subject =",
    "requirement =",
    "[atom]",
];

/// The completion section heading every workflow artifact declares.
const COMPLETION_HEADING: &str = "## Completion Snapshot";

/// Opening marker of the generated evidence block.
const EVIDENCE_BEGIN: &str = "<!-- BEGIN buoy-trace-publication-evidence-v1 -->";

/// Closing marker of the generated evidence block.
const EVIDENCE_END: &str = "<!-- END buoy-trace-publication-evidence-v1 -->";

/// Column labels the separate human completion block must declare. The
/// generated block cannot supply any of them.
const HUMAN_COMPLETION_COLUMNS: [&str; 3] = ["Owner", "Consumer commit", "Review decision"];

/// Provider-state keys that no repository-owned artifact may record.
const PROVIDER_KEYS: [&str; 6] = [
    "provider_status",
    "pull_request_state",
    "issue_state",
    "review_state",
    "merged_at",
    "fetched_title",
];

/// Returns the text of one level-two section, without its heading.
fn section<'a>(text: &'a str, heading: &str) -> Option<&'a str> {
    let start = text.find(heading)? + heading.len();
    let rest = &text[start..];
    let end = rest.find("\n## ").unwrap_or(rest.len());
    Some(&rest[..end])
}

/// Returns the data rows of one Markdown table, without header or separator.
fn data_rows(section: &str) -> Vec<Vec<String>> {
    section
        .lines()
        .map(str::trim)
        .filter(|line| line.starts_with('|'))
        .filter(|line| !line.contains("---"))
        .skip(1)
        .map(|line| {
            line.trim_matches('|')
                .split('|')
                .map(|cell| cell.trim().trim_matches('`').to_owned())
                .collect()
        })
        .collect()
}

fn is_placeholder(cell: &str) -> bool {
    let lowered = cell.to_ascii_lowercase();
    PLACEHOLDERS.contains(&lowered.as_str()) || (cell.starts_with('<') && cell.ends_with('>'))
}

/// One algorithm-qualified digest, which no planned row may carry.
fn carries_digest(cell: &str) -> bool {
    let canonical = ["xxh3-128:", "blake3-256:", "sha-256:"]
        .iter()
        .any(|prefix| cell.contains(prefix));
    let object_locator = cell.contains("buoy/objects/v1/")
        && std::path::Path::new(cell)
            .extension()
            .is_some_and(|extension| extension.eq_ignore_ascii_case("toml"));
    let bare = cell
        .split(|value: char| !value.is_ascii_hexdigit())
        .any(|token| {
            matches!(token.len(), 32 | 40 | 64)
                && token.chars().all(|value| value.is_ascii_hexdigit())
        });
    canonical || object_locator || bare
}

/// Whether one section declares the separate human completion block, that is
/// one Markdown table header naming every human-owned completion column.
fn declares_human_completion_block(completion: &str) -> bool {
    completion
        .lines()
        .map(str::trim)
        .filter(|line| line.starts_with('|'))
        .any(|line| {
            HUMAN_COMPLETION_COLUMNS
                .iter()
                .all(|column| line.contains(column))
        })
}

/// Structural faults an ACTIVE artifact's completion section can carry.
///
/// This runs on the WHOLE document rather than on an already-extracted
/// section. Scoping it to `section(text, COMPLETION_HEADING)` was the real
/// defect: deleting the heading made the section lookup return `None`, so
/// every rule below -- including the generated-block requirement itself --
/// was skipped and the mutilated document reported no fault at all.
fn active_completion_faults(text: &str) -> Vec<TraceabilityFault> {
    let Some(completion) = section(text, COMPLETION_HEADING) else {
        // The section is gone, so the two blocks it must contain are gone
        // with it. Each absence keeps its own identity: a reader must be
        // able to tell a deleted section from a deleted block.
        return vec![
            TraceabilityFault::MissingCompletionSection,
            TraceabilityFault::MissingGeneratedBlock,
            TraceabilityFault::MissingHumanCompletionBlock,
        ];
    };
    let mut faults = Vec::new();
    match completion.find(EVIDENCE_BEGIN) {
        None => faults.push(TraceabilityFault::MissingGeneratedBlock),
        // The closing marker must follow the opening one. A closing marker
        // that precedes it bounds nothing.
        Some(begin) if !completion[begin..].contains(EVIDENCE_END) => {
            faults.push(TraceabilityFault::UnterminatedGeneratedBlock);
        }
        Some(_) => {}
    }
    if !declares_human_completion_block(completion) {
        faults.push(TraceabilityFault::MissingHumanCompletionBlock);
    }
    faults
}

/// Validates one ARCHIVED traceability artifact against the reviewed rules.
///
/// An empty planned or completion table is valid. A placeholder row is not.
/// An archived document may omit the generated evidence block; use
/// [`validate_active_traceability`] for a live workflow artifact, which must
/// carry it.
pub fn validate_traceability(text: &str) -> Vec<TraceabilityFault> {
    validate_traceability_for(text, ArtifactLifecycle::Archived)
}

/// Validates one ACTIVE workflow traceability artifact.
///
/// Identical to [`validate_traceability`] except that the whole completion
/// structure is REQUIRED: the `## Completion Snapshot` heading, the
/// marker-delimited generated evidence block with both markers, and the
/// separate human completion block. Each absence is a distinct fault, because
/// every remaining completion rule is reachable only once the enclosing
/// structure exists.
pub fn validate_active_traceability(text: &str) -> Vec<TraceabilityFault> {
    validate_traceability_for(text, ArtifactLifecycle::Active)
}

/// Validates one traceability artifact under its declared lifecycle.
pub fn validate_traceability_for(
    text: &str,
    lifecycle: ArtifactLifecycle,
) -> Vec<TraceabilityFault> {
    let mut faults = Vec::new();

    if PROVIDER_KEYS.iter().any(|key| text.contains(key)) {
        faults.push(TraceabilityFault::ProviderStatus);
    }

    // Run the ACTIVE structural rules on the whole document, BEFORE the
    // section-scoped rules below. An active artifact that deleted the
    // completion heading outright must not escape them.
    if lifecycle == ArtifactLifecycle::Active {
        faults.extend(active_completion_faults(text));
    }

    if let Some(actions) = section(text, "## Requirement Actions") {
        let action_ids = data_rows(actions)
            .into_iter()
            .filter_map(|row| row.first().cloned())
            .collect::<std::collections::BTreeSet<_>>();
        let reviewed_ids = section(text, "## Link Review")
            .map(data_rows)
            .unwrap_or_default()
            .into_iter()
            .filter_map(|row| row.first().cloned())
            .collect::<std::collections::BTreeSet<_>>();
        if action_ids.iter().any(|id| !reviewed_ids.contains(id)) {
            faults.push(TraceabilityFault::MissingLinkReview);
        }
    }

    if let Some(planned) = section(text, "## Planned Implementation Relations") {
        for row in data_rows(planned) {
            if row.len() != 7 || row.iter().any(String::is_empty) {
                faults.push(TraceabilityFault::IncompleteRow);
                continue;
            }
            if row.iter().any(|cell| is_placeholder(cell)) {
                faults.push(TraceabilityFault::PlaceholderIdentity);
            }
            if row.iter().any(|cell| carries_digest(cell)) {
                faults.push(TraceabilityFault::PrematureCompletion);
            }
        }
    }

    if let Some(completion) = section(text, "## Completion Snapshot") {
        if RECORD_BODY_KEYS.iter().any(|key| completion.contains(key)) {
            faults.push(TraceabilityFault::CopiedRecordBody);
        }
        // A generated-evidence document declares one marker-delimited block.
        // Its separate human block can use any nonempty column shape; a
        // legacy document with no marker keeps the original four-column
        // shape so archived tables remain readable without a fault.
        // Whether the ACTIVE structural rules above accepted this block is
        // irrelevant here: the row shape follows the marker that is actually
        // present, so an archived legacy document keeps its four-column
        // reading and a marked document keeps its free human shape.
        let has_generated_block = completion.contains(EVIDENCE_BEGIN);
        for row in data_rows(completion) {
            let malformed = if has_generated_block {
                row.is_empty() || row.iter().any(String::is_empty)
            } else {
                row.len() != 4 || row.iter().any(String::is_empty)
            };
            if malformed {
                faults.push(TraceabilityFault::IncompleteRow);
                continue;
            }
            if row.iter().any(|cell| is_placeholder(cell)) {
                faults.push(TraceabilityFault::PlaceholderIdentity);
            }
        }
        if has_generated_block {
            match buoy::publication::parse_completion_evidence(completion) {
                Ok(_) => {}
                Err(buoy::publication::PublicationError::InvalidField(reason))
                    if reason == "completion-evidence-drift" =>
                {
                    faults.push(TraceabilityFault::GeneratedEvidenceDrift);
                }
                Err(buoy::publication::PublicationError::InvalidField(reason))
                    if reason == "claim-boundary" =>
                {
                    faults.push(TraceabilityFault::CurrentAuthorityClaim);
                }
                Err(_) => faults.push(TraceabilityFault::IncompleteRow),
            }
        }
    }

    faults.sort_unstable();
    faults.dedup();
    faults
}
