#![doc = "Differential tests between the bounded dependency model and production path rendering."]
#![allow(missing_docs, clippy::expect_used)]

use buoy::publication::render_path;
use buoy::publication_models::DependencyKind;

/// Maps each bounded-model dependency kind to the exact registered or
/// unregistered path-template substitution production code exposes for it.
fn substitution_for(kind: DependencyKind) -> &'static str {
    match kind {
        DependencyKind::PublicationBasis => "{publication_basis_path}",
        DependencyKind::AtomRevision => "{atom_revision_path}",
        DependencyKind::RecordDigest => "{record_digest_path}",
        DependencyKind::Graph => "{graph_digest}",
        DependencyKind::Page => "{page_digest}",
        DependencyKind::Link => "{link_digest}",
        DependencyKind::PageCommit => "{page_commit}",
        DependencyKind::BranchHead => "{branch_head}",
        DependencyKind::Provider => "{provider_id}",
        DependencyKind::PhysicalPath => "{physical_path}",
    }
}

#[test]
fn every_bounded_model_dependency_kind_matches_production_acceptance() {
    let identities = buoy::publication::PathIdentities {
        basis: "xxh3-128:aaaa",
        atom_revision: Some("xxh3-128:bbbb"),
        record_digest: Some("xxh3-128:cccc"),
    };
    for kind in DependencyKind::ALL {
        let template = format!("docs/{}.md", substitution_for(kind));
        let result = render_path(&template, &identities);
        assert_eq!(
            result.is_ok(),
            kind.is_pre_link(),
            "{kind:?}: bounded model says pre_link={}, but render_path returned {result:?}",
            kind.is_pre_link()
        );
    }
}
