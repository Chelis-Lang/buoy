#![doc = "Contracts for extensible source items, `OpenSpec` extraction, and external trace links."]
#![allow(clippy::expect_used)]

use buoy::adapter_sdk::{
    AdapterConfiguration, AdapterDescriptor, SavedSnapshotInput, built_in_configuration,
    built_in_registry,
};
use buoy::adapters::{
    AuthorityExpectation, OpenSpecConfiguration, OpenSpecOutput, extract_markdown,
    extract_openspec_configured,
};
use buoy::check::check_snapshot;
use buoy::core::{
    Atom, AtomKind, AtomRevisionRef, ExternalTraceLink, ExternalTraceRelation, ExternalTraceTarget,
    LifecycleState, ObjectKind, RepositoryPath, ScopeId, SemanticDigest, SourceEdgeKind,
    SourceItemKind, SpecGraph, StableId, change_impacts, validate_graph,
};
use buoy::repository::{RepositoryConfig, RepositorySnapshot, SnapshotFile};
use buoy::traceability::ExternalTraceReport;
use hegel::generators as gs;
use hegel::{Hegel, TestCase};
use std::collections::BTreeSet;

#[path = "support/hegel_settings.rs"]
mod hegel_settings;
use hegel_settings::settings;

fn extract_openspec(path: &RepositoryPath, bytes: &[u8]) -> OpenSpecOutput {
    let configuration = OpenSpecConfiguration::parse(include_bytes!(
        "../fixtures/openspec/v1/positive/configuration.toml"
    ))
    .expect("explicit OpenSpec test configuration");
    extract_openspec_configured(path, bytes, &configuration)
}

#[test]
fn source_item_kinds_are_versioned_private_newtypes() {
    for valid in [
        "markdown/fence/v1",
        "openspec/requirement/v1",
        "chelis/surf-declaration/v1",
    ] {
        assert_eq!(
            SourceItemKind::new(valid).expect("valid kind").as_str(),
            valid
        );
    }
    for invalid in ["", "RustItem", "rust-item", "rust/item/v0", "rust//item/v1"] {
        assert_eq!(
            SourceItemKind::new(invalid)
                .expect_err("invalid kind")
                .code(),
            "SPG-SOURCE-ITEM-KIND"
        );
    }

    let custom = SourceItemKind::new("host/custom-node/v7").expect("custom kind");
    let descriptor = AdapterDescriptor::new(
        "host/v1",
        "host-parser",
        "7.0.0",
        "host/syntax/v1",
        "host/configuration/v1",
        vec![String::from("host.syntax/v1")],
        vec![custom.clone()],
    )
    .expect("extensible descriptor");
    assert!(descriptor.supports_source_item_kind(&custom));
    assert_eq!(
        AdapterDescriptor::new(
            "host/v1",
            "host-parser",
            "7.0.0",
            "host/syntax/v1",
            "host/configuration/v1",
            vec![String::from("host.syntax/v1")],
            vec![custom.clone(), custom],
        )
        .expect_err("duplicate source-item kind")
        .code(),
        "SPG-ADAPTER-UNDECLARED-SOURCE-ITEM-KIND"
    );
}

#[test]
fn source_tree_materializes_document_requirements_scenarios_and_steps() {
    let path = RepositoryPath::new("openspec/specs/demo/spec.md").expect("standing path");
    let bytes = include_bytes!("../fixtures/openspec/v1/positive/standing-spec.md");
    let output = extract_openspec(&path, bytes);
    assert!(output.diagnostics.is_empty());
    assert_eq!(output.semantic.len(), 3);
    assert_eq!(
        output
            .semantic
            .iter()
            .filter(|located| located.item.kind().as_str() == "openspec/requirement/v1")
            .count(),
        1
    );
    assert_eq!(
        output
            .semantic
            .iter()
            .filter(|located| located.item.kind().as_str() == "openspec/scenario/v1")
            .count(),
        2
    );
    assert_eq!(output.source_nodes.len(), 10);
    assert_eq!(output.source_edges.len(), 9);
    assert_eq!(output.authorities.len(), 1);
    assert!(output.document_root.is_some());
    assert_eq!(
        output
            .source_nodes
            .iter()
            .filter(|located| located.node.item().kind().as_str() == "openspec/step/v1")
            .count(),
        6
    );
    assert!(
        output
            .source_edges
            .iter()
            .all(|edge| edge.relation() == SourceEdgeKind::Contains && edge.position().is_some())
    );
    assert!(output.semantic.iter().all(|located| {
        let Ok(start) = usize::try_from(located.span.start) else {
            return false;
        };
        let Ok(end) = usize::try_from(located.span.end) else {
            return false;
        };
        &bytes[start..end] == located.value.body().as_bytes()
    }));
    let mut primary = output
        .semantic
        .iter()
        .filter(|located| located.value.atom().is_some())
        .map(|located| (located.span.start, located.span.end))
        .chain(
            output
                .nonnormative
                .iter()
                .map(|span| (span.start, span.end)),
        )
        .collect::<Vec<_>>();
    primary.sort_unstable();
    assert_eq!(primary.first().map(|span| span.0), Some(0));
    assert_eq!(primary.last().map(|span| span.1), Some(bytes.len() as u64));
    assert!(primary.windows(2).all(|pair| pair[0].1 == pair[1].0));
    let requirement_span = output
        .semantic
        .iter()
        .find(|located| located.value.atom().is_some())
        .map(|located| &located.span)
        .expect("requirement span");
    assert!(
        output
            .semantic
            .iter()
            .filter(|located| located.value.atom().is_none())
            .all(|scenario| {
                scenario.span.start >= requirement_span.start
                    && scenario.span.end <= requirement_span.end
            })
    );

    assert_openspec_configuration_behavior(&path, bytes);
    assert_openspec_identity_behavior(&path, bytes, &output);
    assert_openspec_negative_fixtures(&path, bytes);
}

#[test]
fn version_one_source_and_authority_identities_are_frozen() {
    let path = RepositoryPath::new("openspec/specs/demo/spec.md").expect("standing path");
    let output = extract_openspec(
        &path,
        include_bytes!("../fixtures/openspec/v1/positive/standing-spec.md"),
    );
    assert!(output.diagnostics.is_empty());
    let mut nodes = output.source_nodes;
    nodes.sort_by(|left, right| {
        (
            left.node.item().kind().as_str(),
            left.span.start,
            left.span.end,
        )
            .cmp(&(
                right.node.item().kind().as_str(),
                right.span.start,
                right.span.end,
            ))
    });
    let mut actual = String::from("kind\tstart\tend\titem_digest\tnode_digest\n");
    for located in nodes {
        use std::fmt::Write as _;
        writeln!(
            actual,
            "{}\t{}\t{}\t{}\t{}",
            located.node.item().kind().as_str(),
            located.span.start,
            located.span.end,
            located.node.item().digest().canonical_text(),
            located.node.digest().canonical_text(),
        )
        .expect("identity row");
    }
    let mut authorities = output.authorities;
    authorities.sort_unstable();
    for authority in authorities {
        use std::fmt::Write as _;
        writeln!(
            actual,
            "authority\t0\t0\t{}\t{}",
            authority.atom().revision().canonical_text(),
            authority.node().digest().canonical_text(),
        )
        .expect("authority row");
    }
    assert_eq!(
        actual,
        include_str!("../fixtures/openspec/v1/positive/frozen-identities.tsv")
    );
}

#[test]
fn atom_correspondence_maps_only_the_exact_requirement_node() {
    let path = RepositoryPath::new("openspec/specs/demo/spec.md").expect("standing path");
    let output = extract_openspec(
        &path,
        include_bytes!("../fixtures/openspec/v1/positive/standing-spec.md"),
    );
    let requirement = output
        .source_nodes
        .iter()
        .find(|located| located.node.item().kind().as_str() == "openspec/requirement/v1")
        .expect("one requirement node");
    let atom = output
        .semantic
        .iter()
        .find_map(|located| located.value.atom())
        .expect("one requirement atom");
    assert_eq!(output.authorities.len(), 1);
    assert_eq!(output.authorities[0].node(), requirement.node.reference());
    assert_eq!(output.authorities[0].atom().id(), atom.meta().id());
    assert_eq!(output.authorities[0].atom().revision(), atom.revision());
    assert!(output.source_nodes.iter().all(|located| {
        located.node.item().kind().as_str() == "openspec/requirement/v1"
            || !output
                .authorities
                .iter()
                .any(|authority| authority.node() == located.node.reference())
    }));
}

fn assert_openspec_configuration_behavior(path: &RepositoryPath, bytes: &[u8]) {
    let ratified_configuration = OpenSpecConfiguration::parse(include_bytes!(
        "../fixtures/openspec/v1/positive/ratified-configuration.toml"
    ))
    .expect("ratified configuration");
    let ratified = extract_openspec_configured(path, bytes, &ratified_configuration);
    let ratified_atom = ratified
        .semantic
        .iter()
        .find_map(|located| located.value.atom())
        .expect("ratified requirement atom");
    assert_eq!(ratified_atom.state(), LifecycleState::Ratified);
    assert!(ratified_atom.approval().is_some());

    let unreserved_source = include_str!("../fixtures/openspec/v1/positive/configuration.toml")
        .replace("OSA-900", "OSA-901");
    let unreserved_configuration = OpenSpecConfiguration::parse(unreserved_source.as_bytes())
        .expect("unreserved control configuration");
    assert!(
        extract_openspec_configured(path, bytes, &unreserved_configuration)
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "SPG-OPENSPEC-REQUIREMENT-UNRESERVED")
    );
}

fn assert_openspec_identity_behavior(
    path: &RepositoryPath,
    bytes: &[u8],
    output: &buoy::adapters::OpenSpecOutput,
) {
    assert_eq!(output, &extract_openspec(path, bytes));
    let edited = extract_openspec(
        path,
        include_bytes!("../fixtures/openspec/v1/positive/trailing-notes-revision.md"),
    );
    let requirement_digest = |result: &buoy::adapters::OpenSpecOutput| {
        result
            .semantic
            .iter()
            .find(|located| located.item.kind().as_str() == "openspec/requirement/v1")
            .expect("requirement identity")
            .item
            .digest()
    };
    assert_eq!(requirement_digest(output), requirement_digest(&edited));
    let atom_digest = |result: &buoy::adapters::OpenSpecOutput| {
        result
            .semantic
            .iter()
            .find_map(|located| located.value.atom())
            .expect("requirement atom")
            .meta()
            .digest()
    };
    assert_eq!(atom_digest(output), atom_digest(&edited));
    let semantic_edit = String::from_utf8(bytes.to_vec())
        .expect("fixture text")
        .replace(
            "scenario identities are exact",
            "scenario identities remain exact",
        );
    let semantic_edit = extract_openspec(path, semantic_edit.as_bytes());
    assert_ne!(atom_digest(output), atom_digest(&semantic_edit));
}

#[test]
fn child_propagation_changes_containers_but_not_sibling_requirements() {
    let path = RepositoryPath::new("openspec/specs/demo/spec.md").expect("standing path");
    let configuration = OpenSpecConfiguration::parse(
        br#"schema = "buoy.openspec-authority/v1"

[[capabilities]]
name = "demo"
format = "openspec/standing-spec/v1"
scope = "openspec.demo"
atom_kind = "behavioral"
lifecycle = "proposed"
reserved_ids = ["OSA-900", "OSA-901"]
"#,
    )
    .expect("two-requirement configuration");
    let base = "# Demo\n\n## Requirements\n\n### Requirement: OSA-900 First\nThe adapter MUST keep the first requirement.\n\n#### Scenario: First\n- **WHEN** the first step runs\n- **THEN** the first result is exact\n\n### Requirement: OSA-901 Second\nThe adapter MUST keep the second requirement.\n\n#### Scenario: Second\n- **WHEN** the second step runs\n- **THEN** the second result is exact\n";
    let changed = base.replace(
        "the first result is exact",
        "the first result remains exact",
    );
    let nodes = |source: &str| {
        extract_openspec_configured(&path, source.as_bytes(), &configuration)
            .source_nodes
            .into_iter()
            .filter(|located| located.node.item().kind().as_str() == "openspec/requirement/v1")
            .map(|located| {
                (
                    String::from_utf8(located.node.item().semantic_bytes().to_vec())
                        .expect("requirement text"),
                    located.node.digest(),
                )
            })
            .collect::<Vec<_>>()
    };
    let base_nodes = nodes(base);
    let changed_nodes = nodes(&changed);
    assert_ne!(base_nodes[0].1, changed_nodes[0].1);
    assert_eq!(base_nodes[1].1, changed_nodes[1].1);
}

fn assert_openspec_negative_fixtures(path: &RepositoryPath, bytes: &[u8]) {
    let duplicate = String::from_utf8(bytes.to_vec())
        .expect("standing spec text")
        .replace(
            "#### Scenario: Negative - active change path",
            "### Requirement: OSA-900 Duplicate fixture requirement\nThe adapter MUST reject this duplicate.\n\n#### Scenario: Negative - active change path",
        );
    assert!(
        extract_openspec(path, duplicate.as_bytes())
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "SPG-OPENSPEC-DUPLICATE-OBJECT")
    );
    let malformed = extract_openspec(
        path,
        include_bytes!("../fixtures/openspec/v1/negative/malformed-requirement.md"),
    );
    assert_eq!(malformed.diagnostics[0].code, "SPG-OPENSPEC-REQUIREMENT-ID");
    let malformed_scenario = extract_openspec(
        path,
        include_bytes!("../fixtures/openspec/v1/negative/three-hash-scenario.md"),
    );
    assert!(
        malformed_scenario
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "SPG-OPENSPEC-SCENARIO-HEADING")
    );
    assert!(malformed_scenario.semantic.is_empty());
    for fixture in [
        include_bytes!("../fixtures/openspec/v1/negative/unsupported-authority-heading.md")
            .as_slice(),
        include_bytes!("../fixtures/openspec/v1/negative/title-first-heading.md").as_slice(),
    ] {
        let rejected = extract_openspec(path, fixture);
        assert_eq!(
            rejected.diagnostics[0].code,
            "SPG-OPENSPEC-UNSUPPORTED-SYNTAX"
        );
        assert!(rejected.semantic.is_empty());
    }
    let duplicate_sections = extract_openspec(
        path,
        include_bytes!("../fixtures/openspec/v1/negative/duplicate-requirements.md"),
    );
    assert_eq!(
        duplicate_sections.diagnostics[0].code,
        "SPG-OPENSPEC-REQUIREMENTS-DUPLICATE"
    );
    assert!(duplicate_sections.semantic.is_empty());
    assert_eq!(
        OpenSpecConfiguration::parse(include_bytes!(
            "../fixtures/openspec/v1/negative/missing-format.toml"
        )),
        Err("SPG-OPENSPEC-CONFIG-FORMAT")
    );
    assert_eq!(
        OpenSpecConfiguration::parse(include_bytes!(
            "../fixtures/openspec/v1/negative/unsupported-format.toml"
        )),
        Err("SPG-OPENSPEC-CONFIG-FORMAT")
    );
    let active_path = RepositoryPath::new("openspec/changes/demo/spec.md").expect("change path");
    assert_eq!(
        extract_openspec(&active_path, bytes).diagnostics[0].code,
        "SPG-OPENSPEC-STANDING-PATH"
    );
}

#[test]
fn openspec_registry_materializes_external_objects_without_assurance() {
    let registry = built_in_registry().expect("built-in registry");
    let path = RepositoryPath::new("openspec/specs/demo/spec.md").expect("standing path");
    let bytes = include_bytes!("../fixtures/openspec/v1/positive/standing-spec.md");
    let configuration = AdapterConfiguration::new(
        "openspec/v1",
        "buoy.adapter.configuration/v1",
        include_bytes!("../fixtures/openspec/v1/positive/configuration.toml").to_vec(),
    )
    .expect("OpenSpec configuration");
    let input = SavedSnapshotInput::new(&path, bytes, &configuration).expect("saved input");
    let output = registry
        .materialize(&input)
        .expect("OpenSpec materialization");
    assert_eq!(output.product().graph().atoms.len(), 1);
    assert_eq!(output.product().external_sources().len(), 2);
    assert_eq!(output.product().source().nodes().len(), 10);
    assert_eq!(output.product().source().edges().len(), 9);
    assert_eq!(output.product().source().roots().len(), 1);
    assert_eq!(output.product().source().authorities().len(), 1);
    assert!(output.product().trace_links().is_empty());
}

#[test]
fn finite_openspec_parser_state_model_matches_production() {
    let standing = RepositoryPath::new("openspec/specs/demo/spec.md").expect("standing path");
    let active = RepositoryPath::new("openspec/changes/demo/spec.md").expect("active path");
    let valid = include_bytes!("../fixtures/openspec/v1/positive/standing-spec.md");
    let cases: [(&RepositoryPath, &[u8], Option<&str>, usize); 5] = [
        (&standing, valid, None, 3),
        (
            &standing,
            b"# Demo\n\n## Purpose\nNo requirements.\n",
            Some("SPG-OPENSPEC-REQUIREMENTS-MISSING"),
            0,
        ),
        (
            &standing,
            include_bytes!("../fixtures/openspec/v1/negative/malformed-requirement.md"),
            Some("SPG-OPENSPEC-REQUIREMENT-ID"),
            0,
        ),
        (
            &standing,
            include_bytes!("../fixtures/openspec/v1/negative/three-hash-scenario.md"),
            Some("SPG-OPENSPEC-SCENARIO-HEADING"),
            0,
        ),
        (&active, valid, Some("SPG-OPENSPEC-STANDING-PATH"), 0),
    ];
    for (path, bytes, diagnostic, semantic_count) in cases {
        let output = extract_openspec(path, bytes);
        assert_eq!(output.semantic.len(), semantic_count);
        assert_eq!(
            output.diagnostics.first().map(|value| value.code),
            diagnostic
        );
    }
}

#[test]
fn openspec_layout_is_nonsemantic_but_scenario_text_changes_revision() {
    let path = RepositoryPath::new("openspec/specs/demo/spec.md").expect("standing path");
    let base = "# Demo\n\n## Requirements\n\n### Requirement: OSA-900 Exact range\nThe adapter MUST preserve the range.\n\n#### Scenario: Range\n- **GIVEN** one saved file\n- **WHEN** the adapter parses it\n- **THEN** it preserves the original byte range\n";
    let layout = base.replace("## Requirements\n\n", "## Requirements\n\n\n\n");
    let changed = base.replace("original byte range", "canonical byte range");
    let revision = |source: &str| {
        extract_openspec(&path, source.as_bytes())
            .semantic
            .into_iter()
            .find_map(|located| located.value.atom().map(Atom::revision))
            .expect("requirement atom")
    };
    assert_eq!(revision(base), revision(&layout));
    assert_ne!(revision(base), revision(&changed));
}

#[test]
fn repository_selectors_bind_explicit_openspec_configuration() {
    let adapter_configuration = include_str!("../fixtures/openspec/v1/positive/configuration.toml");
    let repository_configuration = format!(
        "schema = \"spec-provenance-config/v2\"\n\
         digest_algorithms = [\"xxh3-128\"]\n\
         [adapter_configurations]\n\
         \"openspec/v1\" = '''{adapter_configuration}'''\n\
         [[selectors]]\n\
         root = \"openspec/specs\"\n\
         adapter = \"openspec/v1\"\n\
         include = [\"**/spec.md\"]\n\
         authority_expectation = \"required\"\n"
    );
    let configuration = RepositoryConfig::parse(repository_configuration.as_bytes())
        .expect("repository configuration");
    let snapshot = RepositorySnapshot {
        files: vec![SnapshotFile {
            path: RepositoryPath::new("openspec/specs/demo/spec.md").expect("standing path"),
            bytes: include_bytes!("../fixtures/openspec/v1/positive/standing-spec.md").to_vec(),
            adapter: String::from("openspec/v1"),
            authority_expectation: AuthorityExpectation::Required,
            selector_root: String::from("openspec/specs"),
        }],
    };
    let result = check_snapshot(&snapshot, &configuration).expect("OpenSpec repository check");
    assert_eq!(result.graph.atoms.len(), 1);
    assert!(result.diagnostics.is_empty());

    assert!(
        RepositoryConfig::parse(
            repository_configuration
                .replace("\"openspec/v1\" = '''", "\"unknown/v1\" = '''")
                .as_bytes()
        )
        .is_err()
    );
    let invalid_adapter_bytes = "schema = \"spec-provenance-config/v2\"\n\
         digest_algorithms = [\"xxh3-128\"]\n\
         [adapter_configurations]\n\
         \"openspec/v1\" = \"not a configuration\"\n\
         [[selectors]]\n\
         root = \"openspec/specs\"\n\
         adapter = \"openspec/v1\"\n\
         include = [\"**/spec.md\"]\n\
         authority_expectation = \"required\"\n";
    assert!(RepositoryConfig::parse(invalid_adapter_bytes.as_bytes()).is_err());
    let missing_adapter_configuration = "schema = \"spec-provenance-config/v2\"\n\
         digest_algorithms = [\"xxh3-128\"]\n\
         [[selectors]]\n\
         root = \"openspec/specs\"\n\
         adapter = \"openspec/v1\"\n\
         include = [\"**/spec.md\"]\n\
         authority_expectation = \"required\"\n";
    assert!(RepositoryConfig::parse(missing_adapter_configuration.as_bytes()).is_err());
}

#[test]
fn repository_check_retains_version_two_trace_links_and_source_mappings() {
    let adapter_configuration = include_str!("../fixtures/openspec/v2/positive/configuration.toml");
    let parsed = OpenSpecConfiguration::parse(adapter_configuration.as_bytes())
        .expect("version 2 adapter configuration");
    let path = RepositoryPath::new("openspec/specs/demo/spec.md").expect("standing path");
    let source = include_str!("../fixtures/openspec/v2/positive/standing-spec.md");
    let extracted = extract_openspec_configured(&path, source.as_bytes(), &parsed);
    let atom = extracted
        .semantic
        .iter()
        .find_map(|located| located.value.atom())
        .expect("version 2 atom");
    let source = source.replace(
        "xxh3-128:02020202020202020202020202020202",
        &atom.revision().canonical_text(),
    );
    let repository_configuration = format!(
        "schema = \"spec-provenance-config/v2\"\n\
         digest_algorithms = [\"xxh3-128\"]\n\
         [adapter_configurations]\n\
         \"openspec/v1\" = '''{adapter_configuration}'''\n\
         [[selectors]]\n\
         root = \"openspec/specs\"\n\
         adapter = \"openspec/v1\"\n\
         include = [\"**/spec.md\"]\n\
         authority_expectation = \"required\"\n"
    );
    let configuration = RepositoryConfig::parse(repository_configuration.as_bytes())
        .expect("version 2 repository configuration");
    let snapshot = RepositorySnapshot {
        files: vec![SnapshotFile {
            path,
            bytes: source.into_bytes(),
            adapter: String::from("openspec/v1"),
            authority_expectation: AuthorityExpectation::Required,
            selector_root: String::from("openspec/specs"),
        }],
    };
    let result = check_snapshot(&snapshot, &configuration).expect("version 2 repository check");
    assert_eq!(result.graph.external_trace_links.len(), 1);
    assert!(
        result.diagnostics.is_empty(),
        "the complete graph keeps the exact trace source mapping: {:?}",
        result.diagnostics
    );
}

#[test]
fn external_trace_fences_are_strict_navigation_only_values() {
    let path = RepositoryPath::new("docs/external-trace.md").expect("trace path");
    let bytes = include_bytes!("../fixtures/openspec/v1/positive/external-trace.md");
    let extracted = extract_markdown(&path, bytes, AuthorityExpectation::Optional);
    assert!(extracted.diagnostics.is_empty());
    assert_eq!(extracted.external_links.len(), 1);
    let report = ExternalTraceReport::new(vec![extracted.external_links[0].value.clone()])
        .expect("trace report");
    assert_eq!(
        ExternalTraceReport::parse_tsv(&report.encode_tsv()).expect("read trace report"),
        report
    );
    assert!(report.render_human().contains("TRACE-FIXTURE [tracks]"));
    let provider_augmented = report.encode_tsv().replace('\n', "\tprovider_status\n");
    assert!(ExternalTraceReport::parse_tsv(&provider_augmented).is_err());

    let registry = built_in_registry().expect("built-in registry");
    let configuration = built_in_configuration("markdown/v1", AuthorityExpectation::Optional)
        .expect("Markdown configuration");
    let input = SavedSnapshotInput::new(&path, bytes, &configuration).expect("saved input");
    let output = registry.materialize(&input).expect("trace materialization");
    assert_eq!(output.product().trace_links().len(), 1);
    assert_eq!(output.product().graph().external_trace_links.len(), 1);
    assert!(output.product().graph().coverage_policies.is_empty());
    assert!(output.product().graph().evidence_carriers.is_empty());

    let all_relations = extract_markdown(
        &path,
        include_bytes!("../fixtures/openspec/v1/positive/all-external-relations.md"),
        AuthorityExpectation::Optional,
    );
    assert!(all_relations.diagnostics.is_empty());
    assert_eq!(
        all_relations
            .external_links
            .iter()
            .map(|located| located.value.relation().as_str())
            .collect::<BTreeSet<_>>(),
        BTreeSet::from(["discusses", "documents", "reviews", "tracks"])
    );

    let invalid = extract_markdown(
        &path,
        include_bytes!("../fixtures/openspec/v1/negative/external-trace.md"),
        AuthorityExpectation::Optional,
    );
    assert_eq!(invalid.diagnostics[0].code, "SPG-EXTERNAL-TRACE-DOCUMENT");
}

#[test]
fn external_trace_links_participate_in_graph_closure_and_impact() {
    let atom = Atom::new(
        StableId::new("TRACE-ATOM").expect("atom ID"),
        LifecycleState::Proposed,
        AtomKind::Behavioral,
        vec![ScopeId::new("trace").expect("scope")],
        "The trace target MUST remain exact.",
        None,
    )
    .expect("base atom");
    let link = ExternalTraceLink::new(
        StableId::new("TRACE-LINK").expect("link ID"),
        vec![AtomRevisionRef::new(
            atom.meta().id().clone(),
            atom.revision(),
        )],
        ExternalTraceRelation::Documents,
        ExternalTraceTarget::new("urn:buoy:document:v1").expect("target"),
    )
    .expect("trace link");
    let base = SpecGraph {
        atoms: vec![atom],
        external_trace_links: vec![link.clone()],
        ..SpecGraph::default()
    };
    assert!(validate_graph(&base).is_empty());

    let revised = Atom::new(
        StableId::new("TRACE-ATOM").expect("atom ID"),
        LifecycleState::Proposed,
        AtomKind::Behavioral,
        vec![ScopeId::new("trace").expect("scope")],
        "The revised trace target MUST remain exact.",
        None,
    )
    .expect("revised atom");
    let head = SpecGraph {
        atoms: vec![revised],
        external_trace_links: vec![link],
        ..SpecGraph::default()
    };
    assert!(
        validate_graph(&head)
            .iter()
            .any(|diagnostic| diagnostic.code == "SPG-REF-DANGLING-OR-STALE")
    );
    assert!(change_impacts(&base, &head).iter().any(|impact| {
        impact.kind == ObjectKind::ExternalTraceLink && impact.id.as_str() == "TRACE-LINK"
    }));
}

#[test]
fn provider_state_is_absent_from_adapter_configuration() {
    let bytes = include_bytes!("../fixtures/openspec/v1/positive/configuration.toml");
    let configuration = AdapterConfiguration::new(
        "openspec/v1",
        "buoy.adapter.configuration/v1",
        bytes.to_vec(),
    )
    .expect("offline configuration");
    assert!(OpenSpecConfiguration::parse(configuration.bytes()).is_ok());
    assert_eq!(
        OpenSpecConfiguration::parse(include_bytes!(
            "../fixtures/openspec/v1/negative/configuration.toml"
        )),
        Err("SPG-OPENSPEC-CONFIG-SYNTAX")
    );
    let text = String::from_utf8(configuration.bytes().to_vec()).expect("configuration text");
    for forbidden in [
        "token",
        "provider_",
        "issue_status",
        "pull_request_status",
        "fetched_title",
        "network",
    ] {
        assert!(!text.contains(forbidden));
    }
}

#[test]
fn finite_external_target_and_relation_model_matches_production() {
    for (target, accepted) in [
        ("https://example.com/issues/1", true),
        ("https://example.com?view=1", true),
        ("urn:buoy:document:v1", true),
        ("http://example.com/issues/1", false),
        ("https://user@example.com/issues/1", false),
        ("https://example.com/issues/1#state", false),
        ("https://EXAMPLE.com/issues/1", false),
        ("https://bad_host/issues/1", false),
        ("https://-example.com/issues/1", false),
        ("urn:buoy:v1:document", false),
        ("urn:buoy:document:v0", false),
    ] {
        assert_eq!(
            ExternalTraceTarget::new(target).is_ok(),
            accepted,
            "{target}"
        );
    }
    for (relation, accepted) in [
        ("tracks", true),
        ("discusses", true),
        ("documents", true),
        ("reviews", true),
        ("closes", false),
        ("", false),
    ] {
        assert_eq!(
            ExternalTraceRelation::parse(relation).is_ok(),
            accepted,
            "{relation}"
        );
    }
}

#[test]
fn hegel_external_relation_parser_accepts_only_the_closed_set() {
    const ACCEPTED: [&str; 4] = ["tracks", "discusses", "documents", "reviews"];
    Hegel::new(|test_case: TestCase| {
        let bytes = test_case.draw(gs::vecs(gs::integers::<u8>()).max_size(24));
        let candidate = bytes
            .iter()
            .map(|byte| char::from(b'a' + (byte % 26)))
            .collect::<String>();
        assert_eq!(
            ExternalTraceRelation::parse(&candidate).is_ok(),
            ACCEPTED.contains(&candidate.as_str())
        );
    })
    .settings(settings(1))
    .run();
}

#[test]
fn hegel_openspec_source_spans_remain_exact_after_prefix_changes() {
    Hegel::new(|test_case: TestCase| {
        let prefix_length = usize::from(test_case.draw(gs::integers::<u8>()) % 16);
        let statement_length = usize::from(test_case.draw(gs::integers::<u8>()) % 24) + 1;
        let notes_length = usize::from(test_case.draw(gs::integers::<u8>()) % 24) + 1;
        let source = format!(
            "# Demo Specification\n\n{}\n\n## Requirements\n\nOrdinary prose.\n\n### Requirement: OSA-900 Demo\nThe café system MUST preserve {}exact source ranges.\n\n#### Scenario: Exact café range\n- **GIVEN** a saved specification\n- **WHEN** the adapter parses it\n- **THEN** each item has an original byte range\n\n## Notes\n{}\n",
            "café λ ".repeat(prefix_length),
            "very ".repeat(statement_length),
            "note ".repeat(notes_length),
        );
        let path = RepositoryPath::new("openspec/specs/demo/spec.md").expect("path");
        let first = extract_openspec(&path, source.as_bytes());
        let second = extract_openspec(&path, source.as_bytes());
        assert_eq!(first, second);
        assert_eq!(first.semantic.len(), 2);
        for located in &first.semantic {
            let start = usize::try_from(located.span.start).expect("span start");
            let end = usize::try_from(located.span.end).expect("span end");
            assert!(start < end && end <= source.len());
            assert!(!source[start..end].is_empty());
        }
        let requirement = first
            .semantic
            .iter()
            .find(|located| located.value.atom().is_some())
            .expect("requirement");
        assert!(first.semantic.iter().filter(|located| located.value.atom().is_none()).all(
            |scenario| {
                scenario.span.start >= requirement.span.start
                    && scenario.span.end <= requirement.span.end
            }
        ));
        let mut primary = first
            .nonnormative
            .iter()
            .map(|span| (span.start, span.end))
            .chain(core::iter::once((
                requirement.span.start,
                requirement.span.end,
            )))
            .collect::<Vec<_>>();
        primary.sort_unstable();
        assert_eq!(primary[0].0, 0);
        assert_eq!(primary.last().map(|span| span.1), Some(source.len() as u64));
        assert!(primary.windows(2).all(|pair| pair[0].1 == pair[1].0));
    })
    .settings(settings(1))
    .run();
}

#[test]
fn hegel_openspec_diagnostic_order_is_deterministic() {
    Hegel::new(|test_case: TestCase| {
        let prefix_length = usize::from(test_case.draw(gs::integers::<u8>()) % 16);
        let source = format!(
            "# Demo\n\n{}\n\n## Requirements\n\n### Scenario: Wrong level\n\n### Requirement: OSA-900 Valid\nThe adapter MUST reject malformed scenarios.\n\n#### Scenario: !!!\nBody.\n",
            "λ".repeat(prefix_length),
        );
        let path = RepositoryPath::new("openspec/specs/demo/spec.md").expect("path");
        let first = extract_openspec(&path, source.as_bytes());
        let second = extract_openspec(&path, source.as_bytes());
        assert_eq!(first, second);
        assert!(first.semantic.is_empty());
        assert_eq!(first.diagnostics.len(), 2);
        assert!(first.diagnostics.windows(2).all(|pair| {
            (
                pair[0].span.start,
                pair[0].span.end,
                pair[0].code,
                &pair[0].context,
            ) <= (
                pair[1].span.start,
                pair[1].span.end,
                pair[1].code,
                &pair[1].context,
            )
        }));
    })
    .settings(settings(1))
    .run();
}

#[test]
fn external_trace_reports_fail_closed_on_duplicates_and_malformed_rows() {
    use buoy::traceability::TraceabilityError;
    for (error, code) in [
        (
            TraceabilityError::ExternalObjectId,
            "SPG-EXTERNAL-OBJECT-ID",
        ),
        (TraceabilityError::Report, "SPG-EXTERNAL-TRACE-REPORT"),
        (TraceabilityError::Duplicate, "SPG-EXTERNAL-TRACE-DUPLICATE"),
        (TraceabilityError::Encoding, "DET-ENC-EXTERNAL-TRACE"),
    ] {
        assert_eq!(error.code(), code);
    }
    let make_link = |id: &str, byte: u8| {
        ExternalTraceLink::new(
            StableId::new(id).expect("link ID"),
            vec![AtomRevisionRef::new(
                StableId::new("OSA-900").expect("atom ID"),
                SemanticDigest::xxh3_128([byte; 16]),
            )],
            ExternalTraceRelation::Tracks,
            ExternalTraceTarget::new(&format!("https://example.com/issues/{byte}"))
                .expect("target"),
        )
        .expect("link")
    };
    assert_eq!(
        ExternalTraceReport::new(vec![make_link("TRACE-A", 1), make_link("TRACE-A", 1)])
            .expect_err("duplicate link identity")
            .code(),
        "SPG-EXTERNAL-TRACE-DUPLICATE"
    );
    let report = ExternalTraceReport::new(vec![make_link("TRACE-A", 1)]).expect("report");
    assert_eq!(report.links().len(), 1);
    let encoded = report.encode_tsv();
    let mut lines = encoded.lines();
    let header = lines.next().expect("encoded header");
    let row = lines.next().expect("encoded row");
    let fields = row.split('\t').map(str::to_owned).collect::<Vec<_>>();
    let parse_fields = |fields: &[String]| {
        ExternalTraceReport::parse_tsv(&format!("{header}\n{}\n", fields.join("\t")))
    };
    let short_row = format!("{encoded}external-trace/v1\tonly\tthree\n");
    assert!(ExternalTraceReport::parse_tsv(&short_row).is_err());
    for (index, value) in [
        (0, String::from("wrong/v1")),
        (1, String::new()),
        (2, String::new()),
        (2, String::from("missing-separator")),
        (2, format!("bad@{}", SemanticDigest::xxh3_128([1; 16]))),
        (2, String::from("OSA-900@not-a-digest")),
        (3, String::from("closes")),
        (4, String::from("http://example.com")),
        (5, String::from("not-a-digest")),
    ] {
        let mut malformed = fields.clone();
        malformed[index] = value;
        assert!(parse_fields(&malformed).is_err());
    }
    let mut duplicate_atoms = fields.clone();
    duplicate_atoms[2] = format!("{};{}", fields[2], fields[2]);
    assert!(parse_fields(&duplicate_atoms).is_err());
    let mut mismatched = fields.clone();
    mismatched[5] = format!("{}", SemanticDigest::xxh3_128([9; 16]));
    assert!(parse_fields(&mismatched).is_err());
}

#[test]
fn hegel_external_trace_report_is_order_independent() {
    Hegel::new(|test_case: TestCase| {
        let reverse = test_case.draw(gs::booleans());
        let make_link = |id: &str, byte: u8| {
            ExternalTraceLink::new(
                StableId::new(id).expect("link ID"),
                vec![AtomRevisionRef::new(
                    StableId::new("OSA-900").expect("atom ID"),
                    SemanticDigest::xxh3_128([byte; 16]),
                )],
                ExternalTraceRelation::Tracks,
                ExternalTraceTarget::new(&format!("https://example.com/issues/{byte}"))
                    .expect("target"),
            )
            .expect("link")
        };
        let mut links = vec![make_link("TRACE-A", 1), make_link("TRACE-B", 2)];
        if reverse {
            links.reverse();
        }
        let actual = ExternalTraceReport::new(links)
            .expect("report")
            .encode_tsv();
        let expected =
            ExternalTraceReport::new(vec![make_link("TRACE-A", 1), make_link("TRACE-B", 2)])
                .expect("report")
                .encode_tsv();
        assert_eq!(actual, expected);
    })
    .settings(settings(1))
    .run();
}
