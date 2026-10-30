#![doc = "Full-graph closure integration for generated publication slots."]
#![allow(missing_docs, clippy::expect_used)]

use buoy::adapters::{OpenSpecConfiguration, extract_openspec_configured_with};
use buoy::check::check_snapshot;
use buoy::core::{
    ActiveAlgorithm, AtomRevisionRef, DigestAlgorithm, ExternalTraceLink, ExternalTraceRelation,
    ExternalTraceTarget, RepositoryPath, SpecGraph, StableId, validate_graph,
};
use buoy::external_link_generation::{
    apply_slot, render_slot, resolve_current_atom, verify_candidate_preserves_authority,
};
use buoy::repository::{load_config, snapshot_repository};
use std::fs;
use tempfile::TempDir;

const CONFIG: &str = r#"schema = "spec-provenance-config/v2"
digest_algorithms = ["xxh3-128"]

[adapter_configurations]
"openspec/v1" = '''
schema = "buoy.openspec-authority/v1"

[[capabilities]]
name = "demo"
format = "openspec/standing-spec/v1"
scope = "openspec.demo"
atom_kind = "behavioral"
lifecycle = "proposed"
reserved_ids = ["DEMO-001"]
'''

[[selectors]]
root = "openspec/specs"
adapter = "openspec/v1"
include = ["**/spec.md"]
exclude = []
authority_expectation = "required"
"#;

const SPEC: &str = "# Demo\n\n## Requirements\n\n### Requirement: DEMO-001 Typed kinds\n\nThe adapter MUST emit a typed kind.\n\n#### Scenario: Positive - valid kind\n\n- **WHEN** extraction runs\n- **THEN** one object exists\n";

/// Builds one minimal, complete `openspec`-only repository with no other
/// adapter selected, so this closure check exercises only the graph and
/// atom-revision facts publication generation is responsible for, with no
/// unrelated confound from a separately governed mechanism (such as a
/// compact Rust implementation binding, whose freshness is coupled to every
/// byte in its bound document by an existing, independent design and is not
/// a fact publication-slot generation claims to preserve).
fn closure_repository() -> (TempDir, std::path::PathBuf) {
    let root = TempDir::new().expect("closure control root");
    fs::create_dir_all(root.path().join("openspec/specs/demo")).expect("openspec directory");
    fs::write(root.path().join("openspec/specs/demo/spec.md"), SPEC).expect("spec source");
    let config_path = root.path().join("buoy.toml");
    fs::write(&config_path, CONFIG).expect("closure repository config");
    (root, config_path)
}

/// Task 8.16 (plus 8.13/8.14 end-to-end): a candidate that adds only the
/// declared publication slot to a real, complete `openspec` repository still
/// closes under the exact same full graph and semantic atom-revision checks
/// the unedited repository already passes.
///
/// This exercises the real production `OPENSPEC_EXTERNAL_TRACE_KIND` parser
/// (not a hand-rolled reimplementation): the rendered fence uses the exact
/// `buoy-external-trace-v1` info string the Markdown/`OpenSpec` adapters
/// already recognize, and the rendered `atoms` entry uses the real canonical
/// `<algorithm>:<payload>` digest text `resolve_current_atom` returns.
#[test]
fn candidate_with_only_a_new_publication_slot_still_closes_the_complete_graph() {
    let (repository, config_path) = closure_repository();
    let config = load_config(&config_path).expect("closure repository config parses");

    let baseline_snapshot =
        snapshot_repository(repository.path(), &config).expect("baseline snapshot");
    let baseline = check_snapshot(&baseline_snapshot, &config).expect("baseline closes");
    assert!(
        baseline.diagnostics.is_empty(),
        "the unedited fixture repository must already close: {:?}",
        baseline.diagnostics
    );

    let active = ActiveAlgorithm::new(DigestAlgorithm::Xxh3_128).expect("active member");
    let path = RepositoryPath::new("openspec/specs/demo/spec.md").expect("standing spec path");
    let spec_path = repository.path().join("openspec/specs/demo/spec.md");
    let source = fs::read_to_string(&spec_path).expect("read spec source");
    let configuration_bytes = config
        .adapter_configuration("openspec/v1")
        .expect("openspec adapter configuration bytes");
    let configuration =
        OpenSpecConfiguration::parse(configuration_bytes).expect("openspec adapter configuration");
    let (atom_id, atom_revision) =
        resolve_current_atom(active, &path, source.as_bytes(), &configuration, "DEMO-001")
            .expect("current atom resolves for the real fixture requirement");

    let rendered = render_slot(
        "closure-demo",
        &atom_id,
        &atom_revision,
        "documents",
        "https://example.invalid/trace/closure-demo.md",
    );
    let candidate =
        apply_slot(&source, "closure-demo", &rendered).expect("candidate with the new slot");
    verify_candidate_preserves_authority(
        active,
        &path,
        source.as_bytes(),
        candidate.as_bytes(),
        &configuration,
    )
    .expect("candidate preserves every authority");

    fs::write(&spec_path, &candidate).expect("write candidate spec");

    let candidate_snapshot =
        snapshot_repository(repository.path(), &config).expect("candidate snapshot");
    let candidate_result = check_snapshot(&candidate_snapshot, &config).expect("candidate check");
    assert!(
        candidate_result.diagnostics.is_empty(),
        "a candidate that adds only the declared publication slot must still close the complete graph: {:?}",
        candidate_result.diagnostics
    );
}

/// Task 8.19: `resolve_current_atom` must always return the requirement's
/// semantic atom *revision* (schema-v2 domain `atom-revision`, bound only to
/// kind/scopes/statement), never its distinct atom-*object* digest (domain
/// `atom-object`, bound to id/revision/state/approval). This ties that real
/// production function's output directly to the frozen, already-covered core
/// closure oracle (`buoy_core::validate_graph`): an `ExternalTraceLink` built
/// from `resolve_current_atom`'s exact output closes cleanly, and the same
/// link built from the object digest instead fails with the exact
/// `SPG-REF-DANGLING-OR-STALE` code ETL-006's negative scenario requires.
#[test]
fn resolved_atom_revision_closes_but_the_object_digest_would_not() {
    let (repository, config_path) = closure_repository();
    let config = load_config(&config_path).expect("closure repository config parses");
    let active = ActiveAlgorithm::new(DigestAlgorithm::Xxh3_128).expect("active member");
    let path = RepositoryPath::new("openspec/specs/demo/spec.md").expect("standing spec path");
    let spec_path = repository.path().join("openspec/specs/demo/spec.md");
    let source = fs::read_to_string(&spec_path).expect("read spec source");
    let configuration_bytes = config
        .adapter_configuration("openspec/v1")
        .expect("openspec adapter configuration bytes");
    let configuration =
        OpenSpecConfiguration::parse(configuration_bytes).expect("openspec adapter configuration");

    let output = extract_openspec_configured_with(active, &path, source.as_bytes(), &configuration);
    assert!(output.diagnostics.is_empty());
    let atom = output
        .semantic
        .iter()
        .find_map(|located| located.value.atom())
        .expect("the fixture requirement materializes one atom")
        .clone();
    assert_ne!(
        atom.revision(),
        atom.meta().digest(),
        "the atom-object and atom-revision domains must produce genuinely distinct digests"
    );

    let (_, resolved_revision_text) =
        resolve_current_atom(active, &path, source.as_bytes(), &configuration, "DEMO-001")
            .expect("current atom resolves");
    assert_eq!(resolved_revision_text, atom.revision().canonical_text());

    let target =
        ExternalTraceTarget::new("https://example.invalid/trace/closure-demo.md").expect("target");
    let correct_link = ExternalTraceLink::new(
        StableId::new("CLOSURE-DEMO").expect("link id"),
        vec![AtomRevisionRef::new(
            atom.meta().id().clone(),
            atom.revision(),
        )],
        ExternalTraceRelation::Documents,
        target.clone(),
    )
    .expect("correct external trace link");
    let correct_graph = SpecGraph {
        atoms: vec![atom.clone()],
        external_trace_links: vec![correct_link],
        ..SpecGraph::default()
    };
    assert!(
        validate_graph(&correct_graph).is_empty(),
        "a link built from resolve_current_atom's exact output must close cleanly"
    );

    let defective_link = ExternalTraceLink::new(
        StableId::new("CLOSURE-DEMO").expect("link id"),
        vec![AtomRevisionRef::new(
            atom.meta().id().clone(),
            atom.meta().digest(),
        )],
        ExternalTraceRelation::Documents,
        target,
    )
    .expect("defective external trace link constructs (bytes are still well-formed)");
    let defective_graph = SpecGraph {
        atoms: vec![atom],
        external_trace_links: vec![defective_link],
        ..SpecGraph::default()
    };
    let defective_diagnostics = validate_graph(&defective_graph);
    assert!(
        defective_diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "SPG-REF-DANGLING-OR-STALE"),
        "a link built from the atom-object digest must fail closure with \
         SPG-REF-DANGLING-OR-STALE: {defective_diagnostics:?}"
    );
}
