#![doc = "Independent identity controls for the opt-in `OpenSpec` version 2 format."]
#![allow(clippy::expect_used, clippy::panic)]

use buoy::adapters::{OpenSpecConfiguration, OpenSpecOutput, extract_openspec_configured};
use buoy::core::{RepositoryPath, SpecGraph, validate_graph};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

fn repository_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf()
}

fn fixture(relative: &str) -> Vec<u8> {
    let path = repository_root().join("fixtures/openspec").join(relative);
    std::fs::read(&path).unwrap_or_else(|error| panic!("fixture {}: {error}", path.display()))
}

fn configuration(relative: &str) -> OpenSpecConfiguration {
    let bytes = fixture(relative);
    OpenSpecConfiguration::parse(&bytes)
        .unwrap_or_else(|code| panic!("configuration {relative} must parse, found {code}"))
}

fn spec_path() -> RepositoryPath {
    RepositoryPath::new("openspec/specs/demo/spec.md").expect("standing path")
}

fn extract(relative: &str, configuration: &OpenSpecConfiguration) -> OpenSpecOutput {
    extract_openspec_configured(&spec_path(), &fixture(relative), configuration)
}

/// Collects one independent map from source-item kind to its node digests.
///
/// The map is built from the retained node set, not from the adapter output
/// order, so a reordering cannot hide an identity change.
fn node_identities(output: &OpenSpecOutput) -> BTreeMap<String, Vec<String>> {
    let mut identities = BTreeMap::<String, Vec<String>>::new();
    for located in &output.source_nodes {
        identities
            .entry(located.node.item().kind().as_str().to_owned())
            .or_default()
            .push(located.node.digest().canonical_text());
    }
    for digests in identities.values_mut() {
        digests.sort();
    }
    identities
}

/// Collects one independent map from atom ID to its exact revision.
fn atom_identities(output: &OpenSpecOutput) -> BTreeMap<String, String> {
    output
        .authorities
        .iter()
        .map(|authority| {
            (
                authority.atom().id().as_str().to_owned(),
                authority.atom().revision().canonical_text(),
            )
        })
        .collect()
}

fn version_two_graph_with_external_atom_reference(use_object_digest: bool) -> SpecGraph {
    let configuration = configuration("v2/positive/configuration.toml");
    let base_bytes = fixture("v2/positive/no-trace-section.md");
    let baseline = extract_openspec_configured(&spec_path(), &base_bytes, &configuration);
    assert!(baseline.diagnostics.is_empty());
    let baseline_atom = baseline
        .semantic
        .iter()
        .find_map(|located| located.value.atom())
        .expect("baseline atom");
    assert_ne!(
        baseline_atom.revision(),
        baseline_atom.meta().digest(),
        "the integration regression requires distinct digest domains"
    );
    let reference = if use_object_digest {
        baseline_atom.meta().digest()
    } else {
        baseline_atom.revision()
    };
    let base = String::from_utf8(base_bytes).expect("standing specification text");
    let source = format!(
        "{base}\n## Implementation trace\n\n\
         This section is nonauthority navigation data.\n\n\
         ```buoy-external-trace-v1\n\
         id = \"TRACE-REVISION-PAGE\"\n\
         atoms = [\"OSA-900@{reference}\"]\n\
         relation = \"documents\"\n\
         target = \"https://example.com/traces/osa-900\"\n\
         ```\n"
    );
    let output = extract_openspec_configured(&spec_path(), source.as_bytes(), &configuration);
    assert!(
        output.diagnostics.is_empty(),
        "the dynamic version 2 source must materialize: {:?}",
        output.diagnostics
    );
    assert_eq!(output.external_links.len(), 1);
    let atom = output
        .semantic
        .iter()
        .find_map(|located| located.value.atom().cloned())
        .expect("materialized atom");
    assert_eq!(atom.revision(), baseline_atom.revision());

    SpecGraph {
        atoms: vec![atom],
        external_trace_links: vec![output.external_links[0].value.clone()],
        ..SpecGraph::default()
    }
}

#[test]
fn version_two_external_trace_link_accepts_the_semantic_atom_revision() {
    let graph = version_two_graph_with_external_atom_reference(false);
    let diagnostics = validate_graph(&graph);
    assert!(
        diagnostics.is_empty(),
        "a current version 2 link must be fresh: {diagnostics:?}"
    );
}

#[test]
fn version_two_external_trace_link_rejects_the_atom_object_digest() {
    let graph = version_two_graph_with_external_atom_reference(true);
    assert_eq!(
        validate_graph(&graph)
            .iter()
            .filter(|diagnostic| diagnostic.code == "SPG-REF-DANGLING-OR-STALE")
            .count(),
        1,
        "an atom object digest must not satisfy semantic revision closure"
    );
}

#[test]
fn version_two_configuration_is_explicit_and_accepted() {
    let configuration = configuration("v2/positive/configuration.toml");
    let output = extract("v2/positive/standing-spec.md", &configuration);
    assert!(
        output.diagnostics.is_empty(),
        "a version 2 standing spec with a later trace section must materialize: {:?}",
        output.diagnostics
    );
}

#[test]
fn unsupported_format_versions_still_fail_closed() {
    let bytes = fixture("v2/negative/unsupported-format.toml");
    assert_eq!(
        OpenSpecConfiguration::parse(&bytes).expect_err("an unknown format must fail"),
        "SPG-OPENSPEC-CONFIG-FORMAT"
    );
}

#[test]
fn equal_authority_bytes_keep_equal_identities_across_both_formats() {
    let version_one = configuration("v1/positive/configuration.toml");
    let version_two = configuration("v2/positive/configuration.toml");

    // The version 1 fixture and the version 2 fixture share byte-identical
    // requirement, scenario, and step spans. Only the trailing nonauthority
    // section differs.
    let first = extract("v1/positive/standing-spec.md", &version_one);
    let second = extract("v2/positive/no-trace-section.md", &version_two);
    assert!(first.diagnostics.is_empty());
    assert!(second.diagnostics.is_empty());

    assert_eq!(
        node_identities(&first),
        node_identities(&second),
        "equal authority bytes must keep equal source-node identities"
    );
    assert_eq!(
        atom_identities(&first),
        atom_identities(&second),
        "equal authority bytes must keep equal atom revisions"
    );
}

#[test]
fn a_trace_section_edit_changes_no_authority_identity() {
    let configuration = configuration("v2/positive/configuration.toml");
    let first = extract("v2/positive/standing-spec.md", &configuration);
    let second = extract("v2/positive/other-target.md", &configuration);
    assert!(first.diagnostics.is_empty());
    assert!(second.diagnostics.is_empty());

    assert_eq!(
        node_identities(&first),
        node_identities(&second),
        "a trace-target edit must not change an authority node"
    );
    assert_eq!(
        atom_identities(&first),
        atom_identities(&second),
        "a trace-target edit must not change an atom revision"
    );
    assert_ne!(
        fixture("v2/positive/standing-spec.md"),
        fixture("v2/positive/other-target.md"),
        "the two fixtures must differ in their trace bytes"
    );
}

#[test]
fn a_requirement_edit_changes_the_requirement_and_atom_identity() {
    let configuration = configuration("v2/positive/configuration.toml");
    let baseline = extract("v2/positive/standing-spec.md", &configuration);
    let edited_bytes = String::from_utf8(fixture("v2/positive/standing-spec.md"))
        .expect("fixture text")
        .replace(
            "The adapter MUST materialize this requirement.",
            "The adapter MUST materialize this exact requirement.",
        );
    let edited = extract_openspec_configured(&spec_path(), edited_bytes.as_bytes(), &configuration);
    assert!(edited.diagnostics.is_empty());

    assert_ne!(
        node_identities(&baseline).get("openspec/requirement/v1"),
        node_identities(&edited).get("openspec/requirement/v1"),
        "a requirement edit must change the requirement node"
    );
    assert_ne!(
        atom_identities(&baseline),
        atom_identities(&edited),
        "a requirement edit must change the atom revision"
    );
}

#[test]
fn a_trace_fence_inside_an_authority_span_fails_closed() {
    let configuration = configuration("v2/positive/configuration.toml");
    for relative in [
        "v2/negative/trace-in-requirement.md",
        "v2/negative/trace-in-scenario.md",
    ] {
        let output = extract(relative, &configuration);
        assert_eq!(output.diagnostics.len(), 1, "{relative}");
        assert_eq!(
            output.diagnostics[0].code, "SPG-OPENSPEC-TRACE-IN-AUTHORITY",
            "{relative}"
        );
        assert!(
            output.source_nodes.is_empty() && output.semantic.is_empty(),
            "{relative}: a rejected file must produce no partial graph"
        );
    }
}

/// The primary partition stays total and each trace mapping nests inside it.
#[test]
fn a_trace_mapping_nests_inside_one_nonnormative_span() {
    let configuration = configuration("v2/positive/configuration.toml");
    let bytes = fixture("v2/positive/standing-spec.md");
    let output = extract("v2/positive/standing-spec.md", &configuration);
    assert!(output.diagnostics.is_empty());
    assert_eq!(output.external_links.len(), 1);

    // Primary requirement and nonnormative spans still partition every byte.
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

    // The secondary trace mapping nests inside exactly one nonnormative span.
    let trace = &output.external_links[0].span;
    let hosts = output
        .nonnormative
        .iter()
        .filter(|span| span.start <= trace.start && trace.end <= span.end)
        .count();
    assert_eq!(hosts, 1, "one trace mapping nests in one nonnormative span");

    // The mapping adds no primary partition entry.
    assert!(
        !output
            .nonnormative
            .iter()
            .any(|span| span.start == trace.start && span.end == trace.end),
        "a secondary mapping never replaces one primary entry"
    );
}

/// A version 2 format change alone keeps one current binding fresh.
#[test]
fn adopting_version_two_preserves_every_authority_identity() {
    let version_one = configuration("v1/positive/configuration.toml");
    let version_two = configuration("v2/positive/configuration.toml");
    let bytes = fixture("v1/positive/standing-spec.md");
    let first = extract_openspec_configured(&spec_path(), &bytes, &version_one);
    let second = extract_openspec_configured(&spec_path(), &bytes, &version_two);
    assert!(first.diagnostics.is_empty());
    assert!(second.diagnostics.is_empty());

    // The same bytes under either format keep every authority identity, so one
    // format change alone cannot stale an existing implementation binding.
    assert_eq!(node_identities(&first), node_identities(&second));
    assert_eq!(atom_identities(&first), atom_identities(&second));
    assert_eq!(first.document_root, second.document_root);
    assert_eq!(first.source_edges, second.source_edges);
    assert_eq!(first.authorities, second.authorities);
    assert!(first.external_links.is_empty());
    assert!(second.external_links.is_empty());
}

/// Both formats keep the version 1 source-item schema for authority nodes.
#[test]
fn both_formats_keep_the_version_one_source_item_schema() {
    for (relative, config) in [
        (
            "v1/positive/standing-spec.md",
            "v1/positive/configuration.toml",
        ),
        (
            "v2/positive/standing-spec.md",
            "v2/positive/configuration.toml",
        ),
    ] {
        let output = extract(relative, &configuration(config));
        assert!(output.diagnostics.is_empty());
        for located in &output.source_nodes {
            assert_eq!(
                located.node.item().language_schema_id(),
                "openspec/standing-spec/v1",
                "{relative}: every authority node keeps the version 1 schema"
            );
        }
    }
}

#[test]
fn duplicate_version_two_trace_fences_fail_without_a_partial_product() {
    let configuration = configuration("v2/positive/configuration.toml");
    let source = String::from_utf8(fixture("v2/positive/standing-spec.md")).expect("fixture text");
    let fence_start = source
        .find("```buoy-external-trace-v1")
        .expect("trace fence");
    let fence = &source[fence_start..];
    let duplicate = format!("{source}\n{fence}");
    let output = extract_openspec_configured(&spec_path(), duplicate.as_bytes(), &configuration);
    assert_eq!(output.diagnostics.len(), 1);
    assert_eq!(output.diagnostics[0].code, "SPG-EXTERNAL-TRACE-DUPLICATE");
    assert!(output.semantic.is_empty());
    assert!(output.source_nodes.is_empty());
    assert!(output.external_links.is_empty());
}

#[test]
fn version_two_rejects_a_trace_fence_outside_a_later_level_two_section() {
    let configuration = configuration("v2/positive/configuration.toml");
    let base = String::from_utf8(fixture("v2/positive/no-trace-section.md")).expect("fixture text");
    let fence = r#"
```buoy-external-trace-v1
id = "TRACE-WRONG-SECTION"
atoms = ["OSA-900@xxh3-128:02020202020202020202020202020202"]
relation = "documents"
target = "https://example.com/traces/wrong-section"
```
"#;
    let outside = format!("{base}\n# Appendix\n{fence}");
    let output = extract_openspec_configured(&spec_path(), outside.as_bytes(), &configuration);
    assert_eq!(output.diagnostics.len(), 1);
    assert_eq!(output.diagnostics[0].code, "SPG-OPENSPEC-TRACE-SECTION");
    assert!(output.source_nodes.is_empty() && output.external_links.is_empty());
}

#[test]
fn version_one_does_not_parse_or_reject_an_authority_span_fence() {
    let version_one = configuration("v1/positive/configuration.toml");
    let output = extract("v2/negative/trace-in-requirement.md", &version_one);
    assert!(
        output.diagnostics.is_empty(),
        "version 1 must retain its prior nonauthority-parser behavior"
    );
    assert!(output.external_links.is_empty());
}

#[test]
fn version_one_still_treats_a_later_fence_as_nonnormative() {
    let version_one = configuration("v1/positive/configuration.toml");
    let output = extract_openspec_configured(
        &spec_path(),
        &fixture("v2/positive/standing-spec.md"),
        &version_one,
    );
    assert!(
        output.diagnostics.is_empty(),
        "version 1 must accept the same bytes without new meaning"
    );
    let nonnormative_bytes = output
        .nonnormative
        .iter()
        .map(|span| span.end - span.start)
        .sum::<u64>();
    assert!(
        nonnormative_bytes > 0,
        "version 1 must keep the later fence nonnormative"
    );
}
