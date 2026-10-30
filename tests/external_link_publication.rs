#![doc = "Contract tests for cycle-safe `OpenSpec` external-link generation."]
#![allow(missing_docs, clippy::expect_used)]

use buoy::adapters::OpenSpecConfiguration;
use buoy::core::ActiveAlgorithm;
use buoy::core::DigestAlgorithm;
use buoy::core::RepositoryPath;
use buoy::external_link_generation::{
    SlotError, apply_slot, find_insertion_boundary, find_slot, normalize_for_basis, render_slot,
    resolve_current_atom, verify_candidate_preserves_authority,
};
use buoy::publication::{
    BasisInputs, Selector, compute_publication_basis, join_target_base, render_path,
    validate_immutable_target,
};

fn openspec_configuration() -> OpenSpecConfiguration {
    OpenSpecConfiguration::parse(
        br#"schema = "buoy.openspec-authority/v1"

[[capabilities]]
name = "demo"
format = "openspec/standing-spec/v1"
scope = "openspec.demo"
atom_kind = "behavioral"
lifecycle = "proposed"
reserved_ids = ["DEMO-001"]
"#,
    )
    .expect("OpenSpec configuration")
}

fn fixture(name: &str) -> String {
    std::fs::read_to_string(format!("fixtures/trace-publication/v1/links/{name}"))
        .expect("link fixture")
}

#[test]
fn absent_slot_has_no_declared_section() {
    let source = fixture("absent-slot.md");
    assert_eq!(find_slot(&source, "demo-link"), Ok(None));
    let boundary = find_insertion_boundary(&source).expect("boundary");
    assert_eq!(boundary, source.len());
}

#[test]
fn insertion_boundary_ignores_an_inline_mention_that_does_not_start_a_line() {
    // Prose that mentions the heading text mid-line (not as its own heading
    // line) must never satisfy the real normative-section search. If the
    // search wrongly matched that inline mention, the boundary would land
    // at the real `## Requirements` heading that follows it (the next
    // `## ` line after the wrong match), rather than at the document end.
    let source = "# Demo\n\n## Purpose\n\nSee the ## Requirements section below for the normative list.\n\n## Requirements\n\n### Requirement: DEMO-001 Example\nBody.\n";
    let boundary = find_insertion_boundary(source).expect("boundary");
    assert_eq!(
        boundary,
        source.len(),
        "the inline mention must not be mistaken for the real heading"
    );
}

#[test]
fn canonical_slot_is_found_by_link_id() {
    let source = fixture("canonical-slot.md");
    let (start, end) = find_slot(&source, "demo-link")
        .expect("slot lookup")
        .expect("present slot");
    assert!(source[start..end].contains("buoy-external-trace-v1"));
    assert!(source[start..end].contains("blake3-256:bbbb"));
}

#[test]
fn slot_lookup_never_matches_a_link_id_that_is_a_prefix_of_another_declared_id() {
    // A link ID that is a literal prefix of another declared slot's ID must
    // never match that other slot: `demo-link` must not resolve inside a
    // document whose only declared slot is `demo-link-extended`.
    let rendered = render_slot(
        "demo-link-extended",
        "DEMO-001",
        "blake3-256:bbbb",
        "documents",
        "https://example.invalid/trace/blake3-256/current.md",
    );
    let source = format!(
        "# Demo\n\n## Requirements\n\n### Requirement: DEMO-001 Example\nBody.\n\n{rendered}"
    );
    assert_eq!(find_slot(&source, "demo-link").expect("slot lookup"), None);
    let (start, end) = find_slot(&source, "demo-link-extended")
        .expect("slot lookup")
        .expect("present slot");
    assert!(source[start..end].contains("demo-link-extended"));
}

#[test]
fn stale_slot_can_be_migrated_to_a_current_atom_revision() {
    let source = fixture("migratable-legacy.md");
    let rendered = render_slot(
        "demo-link",
        "DEMO-001",
        "blake3-256:bbbb",
        "documents",
        "https://example.invalid/trace/blake3-256/current.md",
    );
    let candidate = apply_slot(&source, "demo-link", &rendered).expect("migration candidate");
    assert!(candidate.contains("blake3-256:bbbb"));
    assert!(!candidate.contains("blake3-256:aaaa"));
    // Every normative requirement byte before the slot boundary is preserved.
    let boundary = find_insertion_boundary(&source).expect("boundary");
    assert_eq!(&candidate[..boundary], &source[..boundary]);
}

#[test]
fn mixed_legacy_section_fails_without_a_write() {
    let source = fixture("mixed-legacy-section.md");
    let rendered = render_slot(
        "demo-link",
        "DEMO-001",
        "blake3-256:bbbb",
        "documents",
        "https://example.invalid/trace/blake3-256/current.md",
    );
    assert_eq!(
        apply_slot(&source, "demo-link", &rendered),
        Err(SlotError::MixedLegacySection)
    );
}

#[test]
fn duplicate_slots_fail_before_edit_planning() {
    let source = fixture("duplicate-slot.md");
    assert_eq!(
        find_slot(&source, "demo-link"),
        Err(SlotError::DuplicateSlot)
    );
}

#[test]
fn malformed_fence_fails_closed() {
    let source = fixture("malformed-fence.md");
    assert_eq!(
        find_slot(&source, "demo-link"),
        Err(SlotError::MalformedFence)
    );
}

#[test]
fn first_generation_inserts_the_canonical_slot_at_the_fixed_boundary() {
    let source = fixture("absent-slot.md");
    let rendered = render_slot(
        "demo-link",
        "DEMO-001",
        "blake3-256:bbbb",
        "documents",
        "https://example.invalid/trace/blake3-256/first.md",
    );
    let candidate = apply_slot(&source, "demo-link", &rendered).expect("first insertion");
    assert!(candidate.starts_with(&source));
    assert!(candidate.contains("## Publication: demo-link"));
    assert!(candidate.contains("buoy-external-trace-v1"));

    // Applying again with equal inputs reproduces the same candidate bytes.
    let second = apply_slot(&candidate, "demo-link", &rendered).expect("idempotent insertion");
    assert_eq!(candidate, second);
}

#[test]
fn embedded_authority_slot_is_rejected() {
    // A fence placed inside the normative Requirements section is not a
    // declared nonauthority slot; it must not be treated as one, and any
    // edit plan that reaches into that span must fail before writing.
    let source = fixture("authority-embedding.md");
    let boundary = find_insertion_boundary(&source).expect("boundary");
    let requirement_heading = source
        .find("### Requirement:")
        .expect("requirement heading");
    assert!(
        boundary >= requirement_heading,
        "the boundary must never fall inside normative content"
    );
    assert_eq!(find_slot(&source, "demo-link"), Ok(None));
}

#[test]
fn immutable_target_requires_the_publication_basis_path() {
    let base = "https://example.invalid/trace/";
    let page_path = "docs/blake3-256/cccc.md";
    let target = join_target_base(base, page_path).expect("joined target");
    assert_eq!(
        target,
        "https://example.invalid/trace/docs/blake3-256/cccc.md"
    );
    assert!(validate_immutable_target(&target, "blake3-256/cccc").is_ok());
    assert!(validate_immutable_target(&target, "blake3-256/dddd").is_err());
}

#[test]
fn target_base_join_rejects_traversal_and_preserves_the_full_page_path() {
    assert!(join_target_base("https://example.invalid/trace/", "../escape.md").is_err());
    assert!(join_target_base("https://example.invalid/trace?x=1", "page.md").is_err());
    let joined =
        join_target_base("https://example.invalid/trace/", "a b/c.md").expect("encoded join");
    assert_eq!(joined, "https://example.invalid/trace/a%20b/c.md");
}

#[test]
fn basis_keyed_page_path_renders_and_joins_into_one_immutable_target() {
    let publication_basis = "blake3-256:cccc";
    let identities = buoy::publication::PathIdentities {
        basis: publication_basis,
        atom_revision: None,
        record_digest: None,
    };
    let page_path =
        render_path("docs/{publication_basis_path}.md", &identities).expect("page path");
    assert_eq!(page_path, "docs/blake3-256/cccc.md");
    let target = join_target_base("https://example.invalid/trace/", &page_path)
        .expect("joined immutable target");
    assert_eq!(
        target,
        "https://example.invalid/trace/docs/blake3-256/cccc.md"
    );
    assert!(validate_immutable_target(&target, "blake3-256/cccc").is_ok());
}

#[test]
fn candidate_that_adds_only_the_publication_slot_preserves_every_authority() {
    let active = ActiveAlgorithm::new(DigestAlgorithm::Xxh3_128).expect("active algorithm");
    let path = RepositoryPath::new("openspec/specs/demo/spec.md").expect("standing spec path");
    let configuration = openspec_configuration();
    let source = fixture("absent-slot.md");
    let rendered = render_slot(
        "demo-link",
        "DEMO-001",
        "xxh3-128:bbbb",
        "documents",
        "https://example.invalid/trace/first.md",
    );
    let candidate = apply_slot(&source, "demo-link", &rendered).expect("candidate bytes");
    assert!(
        verify_candidate_preserves_authority(
            active,
            &path,
            source.as_bytes(),
            candidate.as_bytes(),
            &configuration
        )
        .is_ok(),
        "adding only the declared nonauthority slot must preserve every authority"
    );
}

#[test]
fn absent_canonical_and_migratable_legacy_slots_normalize_to_equal_basis_input() {
    // Task 2.5: for a fixed link ID, `normalize_for_basis` must produce
    // byte-identical output for absent, canonical, and stale-but-well-formed
    // (migratable legacy) slot states whenever every other byte in the
    // document is equal, because the publication basis must never depend on
    // which of those three states a prior slot happened to be in.
    let absent =
        normalize_for_basis(&fixture("absent-slot.md"), "demo-link").expect("absent normalizes");
    let canonical = normalize_for_basis(&fixture("canonical-slot.md"), "demo-link")
        .expect("canonical normalizes");
    let legacy = normalize_for_basis(&fixture("migratable-legacy.md"), "demo-link")
        .expect("migratable legacy normalizes");
    assert_eq!(absent, canonical);
    assert_eq!(canonical, legacy);

    // `migratable-legacy.md` differs from `canonical-slot.md` only in two
    // digest VALUES -- it is a stale canonical slot, not a structurally
    // different layout -- so on its own it cannot show that normalization
    // survives a genuinely legacy SHAPE. The v0-layout fixture supplies that:
    // original key order, compact `key="value"` spacing, a legacy-only
    // `generated_by` key the current renderer no longer emits, a multi-entry
    // atoms array, and comment lines inside the fence.
    let legacy_v0 = normalize_for_basis(&fixture("migratable-legacy-v0-layout.md"), "demo-link")
        .expect("migratable legacy v0 layout normalizes");
    assert_eq!(
        canonical, legacy_v0,
        "a structurally distinct legacy layout must normalize to the same basis input"
    );
}

/// The v0-layout fixture must be genuinely structurally distinct, not a
/// near-copy of the canonical slot. Without this control the fixture could
/// silently drift back into being another stale-digest variant, and the
/// migration evidence above would become vacuous again.
#[test]
fn the_legacy_v0_layout_fixture_is_structurally_distinct_from_the_canonical_slot() {
    let canonical = fixture("canonical-slot.md");
    let legacy_v0 = fixture("migratable-legacy-v0-layout.md");

    // Same document outside the slot: the normalization equality above is
    // only meaningful if every other byte already matches.
    let boundary = "## Publication: demo-link";
    assert_eq!(
        canonical.split(boundary).next(),
        legacy_v0.split(boundary).next(),
        "both fixtures must be byte-identical outside the declared slot"
    );

    // Structurally distinct INSIDE the slot.
    assert!(
        legacy_v0.contains("generated_by="),
        "the legacy layout must carry a legacy-only key the current renderer never emits"
    );
    assert!(
        !canonical.contains("generated_by"),
        "the canonical slot must not carry the legacy-only key"
    );
    assert!(
        legacy_v0.contains("id=\"demo-link\"") && canonical.contains("id = \"demo-link\""),
        "the legacy layout must use compact spacing where the canonical slot uses spaced keys"
    );
    let legacy_keys: Vec<&str> = ["target", "relation", "atoms", "id"]
        .into_iter()
        .filter(|key| legacy_v0.contains(&format!("{key}=")))
        .collect();
    assert_eq!(
        legacy_keys,
        vec!["target", "relation", "atoms", "id"],
        "the legacy layout must declare the original v0 key order"
    );

    // And it must still be MIGRATABLE, not merely different: a real slot
    // application replaces it with the canonical rendering.
    let migrated = buoy::external_link_generation::apply_slot(
        &legacy_v0,
        "demo-link",
        &buoy::external_link_generation::render_slot(
            "demo-link",
            "DEMO-001",
            "blake3-256:bbbb",
            "documents",
            "https://example.invalid/trace/blake3-256/cccc.md",
        ),
    )
    .expect("a legacy v0 layout must migrate");
    assert_eq!(
        migrated, canonical,
        "migrating the legacy layout must produce exactly the canonical slot document"
    );
}

#[test]
fn normalization_rejects_a_mixed_legacy_section_exactly_like_a_real_edit() {
    let source = fixture("mixed-legacy-section.md");
    assert_eq!(
        normalize_for_basis(&source, "demo-link"),
        Err(SlotError::MixedLegacySection)
    );
}

#[test]
fn current_atom_resolves_from_the_saved_snapshot_before_closure_checking() {
    // Task 8.2: resolution must find the exact current atom bound to the
    // declared requirement ID, and reject an unknown requirement or a
    // configuration with parse diagnostics before any slot rendering runs.
    let active = ActiveAlgorithm::new(DigestAlgorithm::Xxh3_128).expect("active algorithm");
    let path = RepositoryPath::new("openspec/specs/demo/spec.md").expect("standing spec path");
    let configuration = openspec_configuration();
    let source = fixture("absent-slot.md");
    let (atom_id, atom_revision) =
        resolve_current_atom(active, &path, source.as_bytes(), &configuration, "DEMO-001")
            .expect("current atom resolves");
    assert_eq!(atom_id, "DEMO-001");
    assert!(!atom_revision.is_empty());
    assert_eq!(
        resolve_current_atom(
            active,
            &path,
            source.as_bytes(),
            &configuration,
            "UNKNOWN-999"
        ),
        Err(SlotError::UnknownRequirement)
    );
}

#[test]
fn end_to_end_slot_generation_computes_one_stable_basis_from_the_resolved_atom() {
    // Tasks 8.4, 8.5, 8.6, 8.15: resolve the current atom, normalize the
    // declared publication slot to the fixed basis marker, compute the
    // publication basis from that normalized snapshot, then render and apply
    // the real slot. The basis computed from the pre-slot and post-slot
    // documents (each independently normalized) must be exactly equal,
    // because normalization removes the one span that generation changes.
    let active = ActiveAlgorithm::new(DigestAlgorithm::Xxh3_128).expect("active algorithm");
    let path = RepositoryPath::new("openspec/specs/demo/spec.md").expect("standing spec path");
    let configuration = openspec_configuration();
    let source = fixture("absent-slot.md");
    let link_id = "demo-link";

    let (atom_id, atom_revision) =
        resolve_current_atom(active, &path, source.as_bytes(), &configuration, "DEMO-001")
            .expect("current atom resolves");

    let normalized_before = normalize_for_basis(&source, link_id).expect("normalizes before");
    let selector = Selector::RequirementId(String::from("DEMO-001"));
    let rows = Vec::new();
    let basis_before = compute_publication_basis(&BasisInputs {
        plan_schema: "buoy/trace-publication-plan/v1",
        renderer_schema: "buoy/readable-trace-markdown/v1",
        active_member: "xxh3-128",
        selector: &selector,
        trace_rows: &rows,
        normalized_snapshot: &[(
            String::from("openspec/specs/demo/spec.md"),
            normalized_before.into_bytes(),
        )],
    })
    .expect("basis before generation");

    // Only after the basis is fixed does slot rendering use it, keeping the
    // page path (and therefore the target) dependent on this exact value.
    let basis_before_identities = buoy::publication::PathIdentities {
        basis: &basis_before,
        atom_revision: None,
        record_digest: None,
    };
    let page_path = render_path("docs/{publication_basis_path}.md", &basis_before_identities)
        .expect("page path from the fixed basis");
    let target =
        join_target_base("https://example.invalid/trace/", &page_path).expect("joined target");
    assert!(validate_immutable_target(&target, &basis_before.replace(':', "/")).is_ok());

    let rendered = render_slot(link_id, &atom_id, &atom_revision, "documents", &target);
    let candidate = apply_slot(&source, link_id, &rendered).expect("candidate with the new slot");
    verify_candidate_preserves_authority(
        active,
        &path,
        source.as_bytes(),
        candidate.as_bytes(),
        &configuration,
    )
    .expect("candidate preserves every authority");

    // Task 8.15: normalizing the post-generation candidate reproduces the
    // exact pre-generation normalized bytes, so a second basis computed from
    // the candidate equals the first.
    let normalized_after = normalize_for_basis(&candidate, link_id).expect("normalizes after");
    let basis_after = compute_publication_basis(&BasisInputs {
        plan_schema: "buoy/trace-publication-plan/v1",
        renderer_schema: "buoy/readable-trace-markdown/v1",
        active_member: "xxh3-128",
        selector: &selector,
        trace_rows: &rows,
        normalized_snapshot: &[(
            String::from("openspec/specs/demo/spec.md"),
            normalized_after.into_bytes(),
        )],
    })
    .expect("basis after generation");
    assert_eq!(basis_before, basis_after);
}

#[test]
fn candidate_that_mutates_a_requirement_fails_the_authority_comparison() {
    let active = ActiveAlgorithm::new(DigestAlgorithm::Xxh3_128).expect("active algorithm");
    let path = RepositoryPath::new("openspec/specs/demo/spec.md").expect("standing spec path");
    let configuration = openspec_configuration();
    let source = fixture("absent-slot.md");
    let mutated = source.replace("MUST emit a typed kind", "MUST emit a different typed kind");
    assert_eq!(
        verify_candidate_preserves_authority(
            active,
            &path,
            source.as_bytes(),
            mutated.as_bytes(),
            &configuration
        ),
        Err(SlotError::AuthorityMutation)
    );
}
