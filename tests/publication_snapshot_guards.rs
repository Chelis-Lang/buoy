#![doc = "Retained-snapshot and source-guard contract tests for publication operations."]
#![allow(missing_docs, clippy::expect_used)]

use buoy::adapters::OpenSpecConfiguration;
use buoy::core::{ActiveAlgorithm, DigestAlgorithm, RepositoryPath};
use buoy::external_link_generation::{apply_slot, resolve_current_atom};
use buoy::publication::{
    SourceGuardError, enforce_source_set_policy, require_source_edit_conforms,
    reverify_selected_paths,
};

#[test]
fn source_edit_absent_from_snapshot_is_rejected() {
    let snapshot = vec![("openspec/specs/demo/spec.md".to_owned(), b"a".to_vec())];
    assert_eq!(
        require_source_edit_conforms(&snapshot, "openspec/specs/other/spec.md", b"a"),
        Err(SourceGuardError::Absent)
    );
}

#[test]
fn source_edit_bytes_must_equal_candidate() {
    let snapshot = vec![("openspec/specs/demo/spec.md".to_owned(), b"a".to_vec())];
    assert_eq!(
        require_source_edit_conforms(&snapshot, "openspec/specs/demo/spec.md", b"b"),
        Err(SourceGuardError::Stale)
    );
    assert!(require_source_edit_conforms(&snapshot, "openspec/specs/demo/spec.md", b"a").is_ok());
}

#[test]
fn reverify_detects_change_removal_and_new_arrival() {
    let retained = vec![("docs/spec.md".to_owned(), b"a".to_vec())];
    let unchanged = retained.clone();
    assert!(reverify_selected_paths(&retained, &unchanged).is_ok());

    let changed = vec![("docs/spec.md".to_owned(), b"b".to_vec())];
    assert_eq!(
        reverify_selected_paths(&retained, &changed),
        Err(SourceGuardError::Stale)
    );

    let removed: Vec<(String, Vec<u8>)> = Vec::new();
    assert_eq!(
        reverify_selected_paths(&retained, &removed),
        Err(SourceGuardError::Stale)
    );

    let grown = vec![
        ("docs/spec.md".to_owned(), b"a".to_vec()),
        ("docs/extra.md".to_owned(), b"c".to_vec()),
    ];
    assert_eq!(
        reverify_selected_paths(&retained, &grown),
        Err(SourceGuardError::Stale)
    );
}

#[test]
fn equal_selected_bytes_under_different_roots_produce_equal_guard_results() {
    // Only logical paths and bytes participate in guard comparison; the
    // physical root that produced them is not canonical input.
    let root_one = vec![("docs/spec.md".to_owned(), b"same".to_vec())];
    let root_two = vec![("docs/spec.md".to_owned(), b"same".to_vec())];
    assert_eq!(
        reverify_selected_paths(&root_one, &root_two),
        reverify_selected_paths(&root_one, &root_one)
    );
}

#[test]
fn exact_source_set_policy_detects_recursive_selector_growth() {
    let policy = vec!["openspec/specs/demo/spec.md".to_owned()];
    let exact = vec!["openspec/specs/demo/spec.md".to_owned()];
    assert!(enforce_source_set_policy(&policy, &exact).is_ok());

    let grown = vec![
        "openspec/specs/demo/spec.md".to_owned(),
        "openspec/specs/demo/nested/extra.md".to_owned(),
    ];
    assert_eq!(
        enforce_source_set_policy(&policy, &grown),
        Err(SourceGuardError::Stale)
    );
}

/// TPT: a declared source-set policy MUST constrain selector results; an
/// empty policy is exact equality with the empty selection, never a bypass
/// of the check. An empty policy against a genuinely empty selection still
/// passes (both sides equal the empty set), but an empty policy against any
/// nonempty selection is scope drift and fails closed exactly like any
/// other declared-vs-resolved mismatch.
#[test]
fn empty_policy_is_exact_equality_with_the_empty_set_not_a_bypass() {
    assert!(enforce_source_set_policy(&[], &[]).is_ok());
    let selected = vec!["any/path.md".to_owned()];
    assert_eq!(
        enforce_source_set_policy(&[], &selected),
        Err(SourceGuardError::Stale)
    );
}

/// Task 8.18: page generation for one repository must be able to require
/// exactly the link-generation candidate bytes produced for a *separate*
/// authority repository, before that candidate has actually reached
/// authority. This proves the maintainer-controlled two-step order design.md
/// section 13 requires: the external-link source edit is generated and
/// saved first (in the authority worktree), and only a *new* snapshot that
/// already contains those exact saved candidate bytes can satisfy page
/// generation's source-edit guard — a page-generation attempt against a
/// snapshot from *before* the link candidate was saved is rejected exactly
/// like any other absent or stale declared source edit.
#[test]
fn page_generation_requires_the_saved_link_candidate_from_a_prior_snapshot() {
    let active = ActiveAlgorithm::new(DigestAlgorithm::Xxh3_128).expect("active algorithm");
    let path = RepositoryPath::new("openspec/specs/demo/spec.md").expect("standing spec path");
    let configuration = OpenSpecConfiguration::parse(
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
    .expect("OpenSpec configuration");
    let source = std::fs::read_to_string("fixtures/trace-publication/v1/links/absent-slot.md")
        .expect("absent-slot fixture");

    // Step 1: the maintainer generates and saves the link candidate in the
    // authority worktree, before any page operation runs.
    let (atom_id, atom_revision) =
        resolve_current_atom(active, &path, source.as_bytes(), &configuration, "DEMO-001")
            .expect("current atom resolves");
    let rendered = buoy::external_link_generation::render_slot(
        "demo-link",
        &atom_id,
        &atom_revision,
        "documents",
        "https://example.invalid/trace/first.md",
    );
    let saved_link_candidate =
        apply_slot(&source, "demo-link", &rendered).expect("saved link candidate bytes");

    // A page-generation snapshot captured *before* that save exists (the
    // unedited source) cannot satisfy the source-edit guard for the
    // now-declared candidate: the declared source edit is simply absent from
    // that earlier snapshot.
    let pre_save_snapshot = vec![(
        String::from("openspec/specs/demo/spec.md"),
        source.clone().into_bytes(),
    )];
    assert_eq!(
        require_source_edit_conforms(
            &pre_save_snapshot,
            "openspec/specs/demo/spec.md",
            saved_link_candidate.as_bytes(),
        ),
        Err(SourceGuardError::Stale),
        "an earlier snapshot's unedited bytes must not satisfy the saved candidate"
    );

    // Step 2: page generation starts a *new* snapshot, taken after the link
    // candidate was saved to the authority worktree. Only now does the
    // source-edit guard accept it, and page rendering may proceed.
    let post_save_snapshot = vec![(
        String::from("openspec/specs/demo/spec.md"),
        saved_link_candidate.clone().into_bytes(),
    )];
    assert!(
        require_source_edit_conforms(
            &post_save_snapshot,
            "openspec/specs/demo/spec.md",
            saved_link_candidate.as_bytes(),
        )
        .is_ok(),
        "a new snapshot containing the saved candidate must satisfy the source-edit guard"
    );
}
