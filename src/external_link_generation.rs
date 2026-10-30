//! Cycle-safe `OpenSpec` publication-slot generation.
//!
//! This module edits only one standalone level-two nonauthority section per
//! standing specification: the declared publication slot for one stable
//! link ID. It never touches normative requirement or scenario bytes.

#![allow(missing_docs)]

const FENCE_INFO: &str = "buoy-external-trace-v1";
const REQUIREMENTS_HEADING: &str = "## Requirements";
/// Fixed marker body used only for publication-basis normalization. It never
/// enters a materialized document; `apply_slot` reuses it only to prove that
/// the region a real slot occupies is excluded from the basis in a way that
/// is independent of that slot's exact current state.
const BASIS_MARKER: &str = "<!-- buoy/trace-publication-basis-marker/v1 -->\n";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SlotError {
    MalformedFence,
    DuplicateSlot,
    MixedLegacySection,
    NoNormativeSection,
    AuthorityMutation,
    UnknownRequirement,
    AmbiguousAtom,
}

/// Resolves the one current semantic atom bound to a declared requirement ID
/// from the retained saved snapshot, before any external-link closure check
/// runs.
///
/// This is the authoritative pre-link atom lookup: slot rendering must never
/// copy an atom object digest or any other value, only this resolved atom ID
/// and its exact current [`buoy_core::SemanticDigest`] revision.
///
/// # Errors
///
/// Returns [`SlotError::UnknownRequirement`] when no current authority binds
/// the declared requirement ID, and [`SlotError::AmbiguousAtom`] when more
/// than one authority does (a shape the adapter's own closure checks already
/// reject upstream, but this function never assumes that and re-checks it).
pub fn resolve_current_atom(
    active: buoy_core::ActiveAlgorithm,
    path: &buoy_core::RepositoryPath,
    source: &[u8],
    configuration: &crate::adapters::OpenSpecConfiguration,
    requirement_id: &str,
) -> Result<(String, String), SlotError> {
    let output =
        crate::adapters::extract_openspec_configured_with(active, path, source, configuration);
    if !output.diagnostics.is_empty() {
        return Err(SlotError::UnknownRequirement);
    }
    select_current_atom(&output.authorities, requirement_id)
}

fn select_current_atom(
    authorities: &[buoy_core::SourceAuthority],
    requirement_id: &str,
) -> Result<(String, String), SlotError> {
    let matches = authorities
        .iter()
        .filter(|authority| authority.atom().id().as_str() == requirement_id)
        .collect::<Vec<_>>();
    match matches.as_slice() {
        [] => Err(SlotError::UnknownRequirement),
        [single] => Ok((
            single.atom().id().as_str().to_owned(),
            single.atom().revision().canonical_text(),
        )),
        _ => Err(SlotError::AmbiguousAtom),
    }
}

/// Finds the fixed insertion boundary immediately after the final normative
/// requirement and before any other nonauthority section.
///
/// The boundary is the byte offset of the first level-two (`## `) heading
/// that follows the `## Requirements` heading, or the end of the source when
/// no later level-two heading exists.
///
/// # Errors
///
/// Returns [`SlotError::NoNormativeSection`] when the source declares no
/// `## Requirements` heading.
pub fn find_insertion_boundary(source: &str) -> Result<usize, SlotError> {
    // A substring match alone can land inside prose or a fenced code block
    // that merely quotes the heading text; only a match that starts a line
    // is the real normative `## Requirements` heading.
    let mut search_from = 0_usize;
    let requirements_start = loop {
        let relative = source[search_from..]
            .find(REQUIREMENTS_HEADING)
            .ok_or(SlotError::NoNormativeSection)?;
        let candidate = search_from + relative;
        if candidate == 0 || source.as_bytes()[candidate - 1] == b'\n' {
            break candidate;
        }
        search_from = candidate + REQUIREMENTS_HEADING.len();
    };
    let after_heading = requirements_start + REQUIREMENTS_HEADING.len();
    let mut offset = after_heading;
    for line in source[after_heading..].split_inclusive('\n') {
        if line.starts_with("## ") {
            return Ok(offset);
        }
        offset += line.len();
    }
    Ok(source.len())
}

/// Finds the complete span of one declared level-two section whose heading
/// matches `## Publication: <link_id>`, if present.
///
/// # Errors
///
/// Returns [`SlotError::DuplicateSlot`] when more than one section declares
/// that heading, and [`SlotError::MalformedFence`] when the declared section
/// contains no valid `buoy-external-trace-v1` fence or its fence body omits
/// a required field.
pub fn find_slot(source: &str, link_id: &str) -> Result<Option<(usize, usize)>, SlotError> {
    let heading = format!("## Publication: {link_id}");
    let mut matches = Vec::new();
    let mut search_from = 0_usize;
    while let Some(relative) = source[search_from..].find(&heading) {
        let start = search_from + relative;
        let end = start + heading.len();
        // Require the heading to occupy a complete line: its start must
        // follow a newline (or begin the document) and its end must reach a
        // newline (or the document end). Without this, a link ID that is a
        // literal prefix of another declared ID (`LINK-1` inside
        // `LINK-10`) would match the wrong slot.
        let starts_line = start == 0 || source.as_bytes()[start - 1] == b'\n';
        let ends_line = end == source.len() || source.as_bytes()[end] == b'\n';
        if starts_line && ends_line {
            matches.push(start);
        }
        search_from = end;
    }
    if matches.len() > 1 {
        return Err(SlotError::DuplicateSlot);
    }
    let Some(start) = matches.into_iter().next() else {
        return Ok(None);
    };
    let end = match source[start..].find("\n## ") {
        Some(relative) => start + relative + 1,
        None => source.len(),
    };
    let section = &source[start..end];
    if !section.contains(&format!("```{FENCE_INFO}")) {
        return Err(SlotError::MalformedFence);
    }
    let fence_body = fence_body(section).ok_or(SlotError::MalformedFence)?;
    for required in ["id", "atoms", "relation", "target"] {
        if !fence_body.contains(required) {
            return Err(SlotError::MalformedFence);
        }
    }
    Ok(Some((start, end)))
}

fn fence_body(section: &str) -> Option<&str> {
    let fence_start = section.find(&format!("```{FENCE_INFO}\n"))?;
    let body_start = fence_start + FENCE_INFO.len() + 4;
    let body_end = section[body_start..].find("```")?;
    Some(&section[body_start..body_start + body_end])
}

/// Renders one canonical standalone publication slot.
#[must_use]
pub fn render_slot(
    link_id: &str,
    atom_id: &str,
    atom_revision: &str,
    relation: &str,
    target: &str,
) -> String {
    format!(
        "## Publication: {link_id}\n\n```{FENCE_INFO}\nid = \"{link_id}\"\natoms = [\"{atom_id}@{atom_revision}\"]\nrelation = \"{relation}\"\ntarget = \"{target}\"\n```\n"
    )
}

/// Replaces a present declared slot, or inserts the rendered slot at the
/// fixed boundary, preserving every other byte in the source.
///
/// A present legacy section may migrate only when it contains no
/// nonwhitespace bytes beyond its heading and its `buoy-external-trace-v1`
/// fence.
///
/// # Errors
///
/// Returns [`SlotError::MixedLegacySection`] when a present slot section
/// contains unrelated nonwhitespace content, and propagates every
/// [`find_slot`] and [`find_insertion_boundary`] error.
pub fn apply_slot(source: &str, link_id: &str, rendered_slot: &str) -> Result<String, SlotError> {
    if let Some((start, end)) = find_slot(source, link_id)? {
        let section = &source[start..end];
        if section_has_unrelated_content(section, link_id) {
            return Err(SlotError::MixedLegacySection);
        }
        let mut candidate = String::with_capacity(source.len() + rendered_slot.len());
        candidate.push_str(&source[..start]);
        candidate.push_str(rendered_slot);
        candidate.push_str(&source[end..]);
        return Ok(candidate);
    }
    let boundary = find_insertion_boundary(source)?;
    let mut candidate = String::with_capacity(source.len() + rendered_slot.len() + 1);
    candidate.push_str(&source[..boundary]);
    if !source[..boundary].ends_with("\n\n") {
        candidate.push('\n');
    }
    candidate.push_str(rendered_slot);
    candidate.push_str(&source[boundary..]);
    Ok(candidate)
}

/// Renders the fixed, target-independent basis-marker slot for one link ID.
///
/// This never depends on a rendered target, atom revision, or record digest:
/// those identities are what the publication basis exists to compute, so the
/// normalization step that removes the operational-basis effect of a
/// present slot cannot itself depend on any of them.
#[must_use]
pub fn render_basis_marker(link_id: &str) -> String {
    format!("## Publication: {link_id}\n\n{BASIS_MARKER}")
}

/// Normalizes one source document for publication-basis computation by
/// removing any present declared slot (in any state: absent, canonical,
/// stale, or migratable legacy) and inserting the fixed, link-ID-keyed
/// marker at the same boundary [`apply_slot`] would use for a real write.
///
/// For a fixed link ID, this produces byte-identical output for every input
/// document whose bytes are equal outside the slot's own span, regardless of
/// whether that document currently has no slot, a canonical current slot, or
/// a stale-but-well-formed (migratable legacy) slot: the publication basis
/// therefore never depends on the exact prior slot state, only on every
/// other page-affecting byte.
///
/// # Errors
///
/// Propagates [`SlotError::MixedLegacySection`] and every [`find_slot`] and
/// [`find_insertion_boundary`] error, exactly as [`apply_slot`] does.
pub fn normalize_for_basis(source: &str, link_id: &str) -> Result<String, SlotError> {
    apply_slot(source, link_id, &render_basis_marker(link_id))
}

fn section_has_unrelated_content(section: &str, link_id: &str) -> bool {
    let heading = format!("## Publication: {link_id}");
    let fence_open = format!("```{FENCE_INFO}");
    let mut inside_fence = false;
    for line in section.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed == heading.trim() {
            continue;
        }
        if trimmed.starts_with(&fence_open) {
            inside_fence = true;
            continue;
        }
        if inside_fence {
            if trimmed == "```" {
                inside_fence = false;
            }
            continue;
        }
        return true;
    }
    false
}

/// Rematerializes both the saved source and one candidate through the
/// normal `OpenSpec` adapter and requires every requirement's atom-revision
/// correspondence to stay identical.
///
/// This is the fail-closed proof that a publication-slot edit touched only
/// the declared nonauthority section: every stable requirement ID present
/// before the edit must still resolve to the exact same atom revision after
/// it, and no requirement ID may appear or disappear.
///
/// The comparison uses the requirement ID and atom revision pair rather
/// than the raw enclosing-document-coupled node reference: a legitimate
/// nonauthority byte change elsewhere in the document changes every
/// requirement node's enclosing-document digest by design, so raw node
/// identity is not the correct equality signal here.
///
/// # Errors
///
/// Returns [`SlotError::AuthorityMutation`] when either side fails to
/// parse, or when the candidate's requirement-ID-to-atom-revision set
/// differs from the saved source's.
pub fn verify_candidate_preserves_authority(
    active: buoy_core::ActiveAlgorithm,
    path: &buoy_core::RepositoryPath,
    source: &[u8],
    candidate: &[u8],
    configuration: &crate::adapters::OpenSpecConfiguration,
) -> Result<(), SlotError> {
    let before =
        crate::adapters::extract_openspec_configured_with(active, path, source, configuration);
    let after =
        crate::adapters::extract_openspec_configured_with(active, path, candidate, configuration);
    if !before.diagnostics.is_empty() || !after.diagnostics.is_empty() {
        return Err(SlotError::AuthorityMutation);
    }
    let before_atoms = before
        .authorities
        .iter()
        .map(|authority| authority.atom().clone())
        .collect::<std::collections::BTreeSet<_>>();
    let after_atoms = after
        .authorities
        .iter()
        .map(|authority| authority.atom().clone())
        .collect::<std::collections::BTreeSet<_>>();
    if before_atoms != after_atoms {
        return Err(SlotError::AuthorityMutation);
    }
    Ok(())
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::too_many_lines)]
mod tests {
    use super::*;
    use buoy_core::{
        ActiveAlgorithm, AtomRevisionRef, DigestAlgorithm, RepositoryPath, SemanticDigest,
        SourceAuthority, SourceNodeRef, StableId,
    };

    fn authority(id: &str, byte: u8) -> SourceAuthority {
        SourceAuthority::new(
            SourceNodeRef::new(SemanticDigest::xxh3_128([byte; 16])),
            AtomRevisionRef::new(
                StableId::new(id).expect("authority ID"),
                SemanticDigest::xxh3_128([byte.wrapping_add(1); 16]),
            ),
        )
    }

    fn configuration() -> crate::adapters::OpenSpecConfiguration {
        crate::adapters::OpenSpecConfiguration::parse(
            b"schema = \"buoy.openspec-authority/v1\"\n\n[[capabilities]]\nname = \"demo\"\nformat = \"openspec/standing-spec/v1\"\nscope = \"openspec.demo\"\natom_kind = \"behavioral\"\nlifecycle = \"proposed\"\nreserved_ids = [\"DEMO-001\"]\n",
        )
        .expect("OpenSpec configuration")
    }

    #[test]
    fn current_atom_selection_rejects_absent_and_ambiguous_authority() {
        let first = authority("DEMO-001", 1);
        assert_eq!(
            select_current_atom(core::slice::from_ref(&first), "MISSING-001"),
            Err(SlotError::UnknownRequirement)
        );
        assert!(select_current_atom(core::slice::from_ref(&first), "DEMO-001").is_ok());
        assert_eq!(
            select_current_atom(&[first, authority("DEMO-001", 2)], "DEMO-001"),
            Err(SlotError::AmbiguousAtom)
        );
    }

    #[test]
    fn malformed_slots_and_authority_edits_fail_closed() {
        assert_eq!(find_insertion_boundary("## Requirements\nBody\n"), Ok(21));
        assert_eq!(
            find_insertion_boundary("no normative heading"),
            Err(SlotError::NoNormativeSection)
        );
        assert!(
            find_insertion_boundary(
                "Quoted ## Requirements text\n\n## Requirements\nBody\n\n## Trace\n"
            )
            .is_ok()
        );
        let prefix_and_embedded = "prefix ## Publication: LINK-1\n\n## Publication: LINK-10\n";
        assert_eq!(find_slot(prefix_and_embedded, "LINK-1"), Ok(None));
        assert_eq!(
            find_slot("## Publication: LINK-1", "LINK-1"),
            Err(SlotError::MalformedFence)
        );

        let missing_fence = "# Demo\n\n## Requirements\n\n## Publication: LINK-1\n";
        assert_eq!(
            find_slot(missing_fence, "LINK-1"),
            Err(SlotError::MalformedFence)
        );
        let unterminated = "# Demo\n\n## Requirements\n\n## Publication: LINK-1\n\n```buoy-external-trace-v1\nid = \"LINK-1\"\n";
        assert_eq!(
            find_slot(unterminated, "LINK-1"),
            Err(SlotError::MalformedFence)
        );
        let missing_target = "# Demo\n\n## Requirements\n\n## Publication: LINK-1\n\n```buoy-external-trace-v1\nid = \"LINK-1\"\natoms = []\nrelation = \"documents\"\n```\n";
        assert_eq!(
            find_slot(missing_target, "LINK-1"),
            Err(SlotError::MalformedFence)
        );

        let source = "# Demo\n\n## Requirements\n\n";
        let rendered = render_slot(
            "LINK-1",
            "DEMO-001",
            "xxh3-128:11111111111111111111111111111111",
            "documents",
            "https://example.com/trace",
        );
        let applied = apply_slot(source, "LINK-1", &rendered).expect("inserted slot");
        assert!(applied.contains("## Publication: LINK-1"));
        let followed = format!("{rendered}\n## Later\n");
        assert!(
            find_slot(&followed, "LINK-1")
                .expect("followed slot")
                .is_some()
        );
        assert_eq!(fence_body("no fence"), None);
        let duplicate = format!("{rendered}\n{rendered}");
        assert_eq!(
            apply_slot(&duplicate, "LINK-1", &rendered),
            Err(SlotError::DuplicateSlot)
        );
        assert_eq!(
            apply_slot("no normative heading", "LINK-1", &rendered),
            Err(SlotError::NoNormativeSection)
        );

        let active = ActiveAlgorithm::new(DigestAlgorithm::Xxh3_128).expect("active member");
        let path = RepositoryPath::new("openspec/specs/demo/spec.md").expect("spec path");
        assert_eq!(
            verify_candidate_preserves_authority(
                active,
                &path,
                b"not an OpenSpec document",
                b"still not an OpenSpec document",
                &configuration(),
            ),
            Err(SlotError::AuthorityMutation)
        );
        let valid_source = b"# Demo\n\n## Requirements\n\n### Requirement: DEMO-001 Valid\nThe implementation MUST preserve this.\n";
        assert!(
            crate::adapters::extract_openspec_configured_with(
                active,
                &path,
                valid_source,
                &configuration(),
            )
            .diagnostics
            .is_empty()
        );
        assert_eq!(
            verify_candidate_preserves_authority(
                active,
                &path,
                valid_source,
                b"not an OpenSpec document",
                &configuration(),
            ),
            Err(SlotError::AuthorityMutation)
        );
        assert_eq!(
            resolve_current_atom(
                active,
                &path,
                b"not an OpenSpec document",
                &configuration(),
                "DEMO-001",
            ),
            Err(SlotError::UnknownRequirement)
        );
    }
}
