#![doc = "Test-only reference model for content-addressed `OpenSpec` implementation bindings."]
#![cfg(feature = "blake3-256")]
#![allow(
    clippy::expect_used,
    clippy::needless_pass_by_value,
    clippy::panic,
    clippy::too_many_lines
)]

use buoy::adapters::{OpenSpecConfiguration, extract_openspec_configured};
use buoy::core::{
    ActiveAlgorithm, CanonicalValue, DigestAlgorithm, RepositoryPath, SemanticDigest,
    semantic_digest_with,
};
use quote::ToTokens as _;
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::path::Path;
use syn::{Item, LitStr};

const SPEC_PATH: &str = "openspec/specs/demo/spec.md";
const RUST_PATH: &str = "src/lib.rs";
const RECORD_PATH: &str = "buoy/objects/v1/blake3-256/644c6ed5a45bbcd93e3c5f0c4b8c2fda8ea4de78078a3d1ddd74d13bac6fba25.toml";
const ZERO_BINDING: &str =
    "blake3-256:0000000000000000000000000000000000000000000000000000000000000000";

const SPEC_SOURCE: &str =
    include_str!("../fixtures/content-addressed-bindings/v1/positive/openspec/specs/demo/spec.md");
const RUST_SOURCE: &str = "#[buoy::implements(\"blake3-256:0000000000000000000000000000000000000000000000000000000000000000\")]\npub fn render_catalog(libraries: &[&str]) -> String {\n    libraries.join(\"\\n\")\n}\n";
const RECORD_SOURCE: &str =
    include_str!("../fixtures/content-addressed-bindings/v1/reference-spike-record.toml");

fn active() -> ActiveAlgorithm {
    ActiveAlgorithm::new(DigestAlgorithm::Blake3_256).expect("the spike enables BLAKE3-256")
}

fn digest(domain: &str, fields: Vec<(u16, CanonicalValue)>) -> SemanticDigest {
    semantic_digest_with(active(), domain, fields).expect("the bounded spike values encode")
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct SourceNode {
    digest: SemanticDigest,
    path: String,
    kind: String,
    address: String,
    children: Vec<SemanticDigest>,
}

impl SourceNode {
    fn new(
        path: &str,
        kind: &str,
        address: &str,
        payload: &[u8],
        children: Vec<SemanticDigest>,
    ) -> Self {
        let digest = digest(
            "buoy/source-node-spike/v1",
            vec![
                (1, CanonicalValue::Text(String::from("source-node/v1"))),
                (2, CanonicalValue::Text(path.to_owned())),
                (3, CanonicalValue::Text(kind.to_owned())),
                (4, CanonicalValue::Text(address.to_owned())),
                (5, CanonicalValue::Bytes(payload.to_vec())),
                (
                    6,
                    CanonicalValue::Sequence(
                        children
                            .iter()
                            .copied()
                            .map(CanonicalValue::Digest)
                            .collect(),
                    ),
                ),
            ],
        );
        Self {
            digest,
            path: path.to_owned(),
            kind: kind.to_owned(),
            address: address.to_owned(),
            children,
        }
    }
}

#[derive(Clone, Debug, Default)]
struct SourceGraph {
    nodes: BTreeMap<SemanticDigest, SourceNode>,
    roots: BTreeSet<SemanticDigest>,
}

impl SourceGraph {
    fn insert(&mut self, node: SourceNode) {
        if let Some(prior) = self.nodes.insert(node.digest, node.clone()) {
            assert_eq!(prior, node, "equal node digests must have equal preimages");
        }
    }

    fn add_root(&mut self, node: SourceNode) {
        self.roots.insert(node.digest);
        self.insert(node);
    }

    fn retain_historical_nodes(&mut self, historical: &Self) {
        for node in historical.nodes.values() {
            self.insert(node.clone());
        }
    }

    fn reachable(&self) -> BTreeSet<SemanticDigest> {
        let mut reached = BTreeSet::new();
        let mut pending = self.roots.iter().copied().collect::<Vec<_>>();
        while let Some(next) = pending.pop() {
            if !reached.insert(next) {
                continue;
            }
            if let Some(node) = self.nodes.get(&next) {
                pending.extend(node.children.iter().rev().copied());
            }
        }
        reached
    }

    /// Visits a start node in canonical order. An absent target is reported
    /// rather than fatal, so traversal output stays total and comparable.
    fn traverse(&self, start: SemanticDigest) -> Vec<Result<&SourceNode, SemanticDigest>> {
        fn visit<'graph>(
            graph: &'graph SourceGraph,
            digest: SemanticDigest,
            visited: &mut BTreeSet<SemanticDigest>,
            output: &mut Vec<Result<&'graph SourceNode, SemanticDigest>>,
        ) {
            if !visited.insert(digest) {
                return;
            }
            let Some(node) = graph.nodes.get(&digest) else {
                output.push(Err(digest));
                return;
            };
            output.push(Ok(node));
            for child in &node.children {
                visit(graph, *child, visited, output);
            }
        }

        let mut output = Vec::new();
        visit(self, start, &mut BTreeSet::new(), &mut output);
        output
    }

    fn traverse_kinds(&self, start: SemanticDigest) -> Vec<String> {
        self.traverse(start)
            .into_iter()
            .map(|node| match node {
                Ok(node) => node.kind.clone(),
                Err(digest) => format!("unresolved:{digest}"),
            })
            .collect()
    }
}

#[derive(Clone, Debug)]
struct OpenSpecTree {
    graph: SourceGraph,
    requirement: SemanticDigest,
    atom_id: String,
    atom_revision: SemanticDigest,
}

fn step_nodes(path: &str, scenario_address: &str, scenario_body: &str) -> Vec<SourceNode> {
    scenario_body
        .lines()
        .filter_map(|line| line.strip_prefix("- **"))
        .filter_map(|line| line.split_once("** "))
        .enumerate()
        .map(|(index, (keyword, text))| {
            SourceNode::new(
                path,
                "openspec/step/v1",
                &format!("{scenario_address}/step/{index}/{keyword}"),
                text.as_bytes(),
                Vec::new(),
            )
        })
        .collect()
}

fn openspec_tree(document_path: &str, source: &str) -> OpenSpecTree {
    let configuration = OpenSpecConfiguration::parse(include_bytes!(
        "../fixtures/content-addressed-bindings/v1/positive/configuration.toml"
    ))
    .expect("spike OpenSpec configuration");
    let path = RepositoryPath::new(SPEC_PATH).expect("spike OpenSpec path");
    let output = extract_openspec_configured(&path, source.as_bytes(), &configuration);
    assert!(output.diagnostics.is_empty(), "{:?}", output.diagnostics);

    let requirement = output
        .semantic
        .iter()
        .find(|located| located.value.atom().is_some())
        .expect("one requirement");
    let atom = requirement.value.atom().expect("requirement atom");
    let atom_id = atom.meta().id().as_str().to_owned();
    let atom_revision = SemanticDigest::parse_canonical(
        "blake3-256:a352b07c143fff7a0aa8e2a57d65fba2ee2d59635fdb20191f68fe93ef21c87f",
    )
    .expect("frozen independent atom revision");

    let mut graph = SourceGraph::default();
    let scenario_nodes = output
        .semantic
        .iter()
        .filter(|located| located.item.kind().as_str() == "openspec/scenario/v1")
        .enumerate()
        .map(|(scenario_index, located)| {
            let scenario_address = format!(
                "requirement/{atom_id}/scenario/{scenario_index}/{}",
                located.value.title()
            );
            let steps = step_nodes(document_path, &scenario_address, located.value.body());
            let children = steps.iter().map(|node| node.digest).collect();
            for step in steps {
                graph.insert(step);
            }
            SourceNode::new(
                document_path,
                "openspec/scenario/v1",
                &scenario_address,
                located.item.semantic_bytes(),
                children,
            )
        })
        .collect::<Vec<_>>();
    let scenario_digests = scenario_nodes.iter().map(|node| node.digest).collect();
    for scenario in scenario_nodes {
        graph.insert(scenario);
    }

    let requirement_node = SourceNode::new(
        document_path,
        "openspec/requirement/v1",
        &format!("requirement/{atom_id}"),
        requirement.item.semantic_bytes(),
        scenario_digests,
    );
    let requirement_digest = requirement_node.digest;
    graph.insert(requirement_node);

    let document = SourceNode::new(
        document_path,
        "openspec/document/v1",
        "document",
        b"generated-lint-catalog",
        vec![requirement_digest],
    );
    graph.add_root(document);

    OpenSpecTree {
        graph,
        requirement: requirement_digest,
        atom_id,
        atom_revision,
    }
}

fn is_buoy_implementation(attribute: &syn::Attribute) -> bool {
    let mut segments = attribute.path().segments.iter();
    matches!(
        (segments.next(), segments.next(), segments.next()),
        (Some(first), Some(second), None)
            if first.ident == "buoy" && second.ident == "implements"
    )
}

fn rust_subject(item_path: &str, source: &str) -> SourceNode {
    let file = syn::parse_file(source).expect("spike Rust source");
    let mut function = file
        .items
        .into_iter()
        .find_map(|item| match item {
            Item::Fn(function) => Some(function),
            _ => None,
        })
        .expect("one Rust function");
    function
        .attrs
        .retain(|attribute| !is_buoy_implementation(attribute));
    SourceNode::new(
        item_path,
        "rust/item/v1",
        "fn/render_catalog",
        function.to_token_stream().to_string().as_bytes(),
        Vec::new(),
    )
}

fn compact_binding_digest(source: &str) -> Result<SemanticDigest, &'static str> {
    let file = syn::parse_file(source).map_err(|_| "rust-syntax")?;
    let function = file
        .items
        .iter()
        .find_map(|item| match item {
            Item::Fn(function) => Some(function),
            _ => None,
        })
        .ok_or("rust-item")?;
    let attributes = function
        .attrs
        .iter()
        .filter(|attribute| is_buoy_implementation(attribute))
        .collect::<Vec<_>>();
    let [attribute] = attributes.as_slice() else {
        return Err("attribute-count");
    };
    let value = attribute
        .parse_args::<LitStr>()
        .map_err(|_| "attribute-fields")?
        .value();
    SemanticDigest::parse_canonical(&value).map_err(|_| "attribute-digest")
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct BindingRecord {
    id: String,
    subject: SemanticDigest,
    requirement: SemanticDigest,
    atom_id: String,
    atom_revision: SemanticDigest,
}

impl BindingRecord {
    fn digest(&self) -> SemanticDigest {
        digest(
            "buoy/implementation-binding/v1",
            vec![
                (1, CanonicalValue::Text(self.id.clone())),
                (2, CanonicalValue::Text(String::from("implements"))),
                (3, CanonicalValue::Digest(self.subject)),
                (4, CanonicalValue::Digest(self.requirement)),
                (5, CanonicalValue::Text(self.atom_id.clone())),
                (6, CanonicalValue::Digest(self.atom_revision)),
            ],
        )
    }

    fn canonical_toml(&self) -> String {
        format!(
            "schema = \"buoy/implementation-binding/v1\"\nid = \"{}\"\nrelation = \"implements\"\nsubject = \"{}\"\nrequirement = \"{}\"\n\n[atom]\nid = \"{}\"\nrevision = \"{}\"\n",
            self.id, self.subject, self.requirement, self.atom_id, self.atom_revision,
        )
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ExpectedDigests {
    document: String,
    requirement: String,
    atom_revision: String,
    subject: String,
    binding: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct StoredBinding {
    schema: String,
    id: String,
    relation: String,
    subject: String,
    requirement: String,
    atom: StoredAtom,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct StoredAtom {
    id: String,
    revision: String,
}

fn parse_record(source: &str) -> Result<BindingRecord, &'static str> {
    let stored = toml::from_str::<StoredBinding>(source).map_err(|_| "record-syntax")?;
    if stored.schema != "buoy/implementation-binding/v1" || stored.relation != "implements" {
        return Err("record-schema");
    }
    Ok(BindingRecord {
        id: stored.id,
        subject: SemanticDigest::parse_canonical(&stored.subject).map_err(|_| "record-subject")?,
        requirement: SemanticDigest::parse_canonical(&stored.requirement)
            .map_err(|_| "record-requirement")?,
        atom_id: stored.atom.id,
        atom_revision: SemanticDigest::parse_canonical(&stored.atom.revision)
            .map_err(|_| "record-atom")?,
    })
}

fn object_path(digest: SemanticDigest) -> String {
    let canonical = digest.canonical_text();
    let (algorithm, payload) = canonical
        .split_once(':')
        .expect("canonical digest contains an algorithm separator");
    format!("buoy/objects/v1/{algorithm}/{payload}.toml")
}

fn digest_from_object_path(path: &str) -> Result<SemanticDigest, &'static str> {
    let path = path
        .strip_prefix("buoy/objects/v1/")
        .ok_or("object-prefix")?;
    let (algorithm, filename) = path.split_once('/').ok_or("object-algorithm")?;
    let payload = filename.strip_suffix(".toml").ok_or("object-suffix")?;
    SemanticDigest::parse_canonical(&format!("{algorithm}:{payload}")).map_err(|_| "object-digest")
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Freshness {
    Fresh,
    MissingRecord,
    StaleImplementation,
    StaleRequirement,
    WrongRequirementKind,
    AuthorityMismatch,
}

fn binding_freshness(
    attribute_digest: SemanticDigest,
    record: Option<&BindingRecord>,
    attached_subject: SemanticDigest,
    graph: &SourceGraph,
    authorities: &BTreeMap<SemanticDigest, (String, SemanticDigest)>,
) -> Freshness {
    let Some(record) = record.filter(|record| record.digest() == attribute_digest) else {
        return Freshness::MissingRecord;
    };
    let reachable = graph.reachable();
    if record.subject != attached_subject || !reachable.contains(&record.subject) {
        return Freshness::StaleImplementation;
    }
    if !reachable.contains(&record.requirement) {
        return Freshness::StaleRequirement;
    }
    if graph
        .nodes
        .get(&record.requirement)
        .map(|node| node.kind.as_str())
        != Some("openspec/requirement/v1")
    {
        return Freshness::WrongRequirementKind;
    }
    if authorities.get(&record.requirement) != Some(&(record.atom_id.clone(), record.atom_revision))
    {
        return Freshness::AuthorityMismatch;
    }
    Freshness::Fresh
}

struct BoundFixture {
    tree: OpenSpecTree,
    subject: SourceNode,
    record: BindingRecord,
    record_digest: SemanticDigest,
    rust_source: String,
    graph: SourceGraph,
    authorities: BTreeMap<SemanticDigest, (String, SemanticDigest)>,
}

fn bound_fixture() -> BoundFixture {
    let spec = SPEC_SOURCE;
    let rust_template = RUST_SOURCE;
    let tree = openspec_tree(SPEC_PATH, spec);
    let subject = rust_subject(RUST_PATH, rust_template);
    let record = BindingRecord {
        id: String::from("BGLC-RENDER-CATALOG"),
        subject: subject.digest,
        requirement: tree.requirement,
        atom_id: tree.atom_id.clone(),
        atom_revision: tree.atom_revision,
    };
    let record_digest = record.digest();
    let rust_source = rust_template.replace(ZERO_BINDING, &record_digest.canonical_text());
    let mut graph = tree.graph.clone();
    graph.add_root(subject.clone());
    let authorities =
        BTreeMap::from([(tree.requirement, (tree.atom_id.clone(), tree.atom_revision))]);
    BoundFixture {
        tree,
        subject,
        record,
        record_digest,
        rust_source,
        graph,
        authorities,
    }
}

/// One repository file addressed by an absolute root and a separate repository-relative path.
#[derive(Clone, Copy)]
struct SourceLocation<'root> {
    root: &'root Path,
    relative: &'root str,
}

impl SourceLocation<'_> {
    fn absolute(self) -> std::path::PathBuf {
        self.root.join(self.relative)
    }

    fn read(self) -> String {
        std::fs::read_to_string(self.absolute()).expect("materialized repository file")
    }
}

/// Selects which path text reaches canonical identity values.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum IdentityPath {
    /// The rule under test: only the repository-relative path is canonical.
    RepositoryRelative,
    /// The non-vacuity control: the absolute root leaks into canonical identity.
    AbsoluteRootControl,
}

impl IdentityPath {
    fn resolve(self, location: SourceLocation<'_>) -> String {
        match self {
            Self::RepositoryRelative => location.relative.to_owned(),
            Self::AbsoluteRootControl => location.absolute().to_string_lossy().into_owned(),
        }
    }
}

fn materialize_repository(prefix: &str) -> tempfile::TempDir {
    let directory = tempfile::Builder::new()
        .prefix(prefix)
        .tempdir()
        .expect("separate spike repository root");
    let binding = digest_from_object_path(RECORD_PATH).expect("committed record path digest");
    for (relative, contents) in [
        (SPEC_PATH, SPEC_SOURCE.to_owned()),
        (
            RUST_PATH,
            RUST_SOURCE.replace(ZERO_BINDING, &binding.canonical_text()),
        ),
        (RECORD_PATH, RECORD_SOURCE.to_owned()),
    ] {
        let absolute = directory.path().join(relative);
        std::fs::create_dir_all(absolute.parent().expect("repository file has a parent"))
            .expect("materialized repository directory");
        std::fs::write(&absolute, contents).expect("materialized repository file");
    }
    directory
}

fn canonical_binding_report(root: &Path, identity: IdentityPath) -> String {
    let specification = SourceLocation {
        root,
        relative: SPEC_PATH,
    };
    let implementation = SourceLocation {
        root,
        relative: RUST_PATH,
    };
    let rust_source = implementation.read();

    let tree = openspec_tree(&identity.resolve(specification), &specification.read());
    let subject = rust_subject(&identity.resolve(implementation), &rust_source);
    let binding = compact_binding_digest(&rust_source).expect("one compact attribute digest");

    let record_relative = object_path(binding);
    let record = parse_record(
        &SourceLocation {
            root,
            relative: &record_relative,
        }
        .read(),
    )
    .expect("stored binding record");
    assert_eq!(record.digest(), binding);

    let mut graph = tree.graph.clone();
    graph.add_root(subject.clone());
    let authorities =
        BTreeMap::from([(tree.requirement, (tree.atom_id.clone(), tree.atom_revision))]);
    let freshness = binding_freshness(binding, Some(&record), subject.digest, &graph, &authorities);

    let mut report = String::new();
    let mut line = |text: &str| writeln!(report, "{text}").expect("in-memory report line");
    line("binding-report/v1");
    line(&format!("relation\t{}\timplements", record.id));
    line(&format!("binding\t{binding}"));
    line(&format!("record-path\t{record_relative}"));
    line(&format!(
        "atom\t{}\t{}",
        record.atom_id, record.atom_revision
    ));
    line(&format!("freshness\t{freshness:?}"));
    let from_current_roots = graph
        .roots
        .iter()
        .flat_map(|root| graph.traverse(*root))
        .collect::<Vec<_>>();
    let from_binding = graph
        .traverse(record.subject)
        .into_iter()
        .chain(graph.traverse(record.requirement))
        .collect::<Vec<_>>();
    for (section, visited) in [("root", from_current_roots), ("binding", from_binding)] {
        for (position, node) in visited.into_iter().enumerate() {
            match node {
                Ok(node) => line(&format!(
                    "{section}\t{position}\t{}\t{}\t{}\t{}",
                    node.kind, node.path, node.address, node.digest
                )),
                Err(absent) => line(&format!("{section}\t{position}\tunresolved\t{absent}")),
            }
        }
    }
    report
}

#[test]
fn separate_absolute_roots_produce_equal_canonical_output() {
    let first = materialize_repository("buoy-spike-a-");
    let second = materialize_repository("buoy-spike-second-root-longer-");
    let (first, second) = (first.path(), second.path());
    assert_ne!(first, second);
    assert_ne!(
        first.as_os_str().len(),
        second.as_os_str().len(),
        "the two roots must differ in prefix length, not only in random suffix bytes"
    );

    let left = canonical_binding_report(first, IdentityPath::RepositoryRelative);
    let right = canonical_binding_report(second, IdentityPath::RepositoryRelative);
    assert_eq!(left, right);

    let expected = toml::from_str::<ExpectedDigests>(include_str!(
        "../fixtures/content-addressed-bindings/v1/reference-spike-expected.toml"
    ))
    .expect("expected spike digests");
    assert!(left.contains(&expected.binding), "{left}");
    assert!(left.contains(&expected.subject), "{left}");
    for relative in [SPEC_PATH, RUST_PATH] {
        assert!(
            left.contains(&format!("\t{relative}\t")),
            "canonical output must carry the repository-relative path: {left}"
        );
    }
    assert_eq!(
        left.matches("openspec/step/v1").count(),
        12,
        "both root and binding traversals must visit every ordered step: {left}"
    );
    for root in [first, second] {
        assert!(
            !left.contains(&root.to_string_lossy().into_owned()),
            "canonical output must not contain an absolute root"
        );
    }

    let leaked_left = canonical_binding_report(first, IdentityPath::AbsoluteRootControl);
    let leaked_right = canonical_binding_report(second, IdentityPath::AbsoluteRootControl);
    assert_ne!(
        leaked_left, leaked_right,
        "the comparison is vacuous unless a leaked absolute root changes canonical output"
    );
    assert_ne!(leaked_left, left);

    assert!(
        RepositoryPath::new(
            &SourceLocation {
                root: first,
                relative: SPEC_PATH,
            }
            .absolute()
            .to_string_lossy()
        )
        .is_err(),
        "an absolute path is not a repository path"
    );
    assert!(RepositoryPath::new(SPEC_PATH).is_ok());
}

#[test]
fn one_hash_resolves_and_traverses_the_openspec_tree() {
    let fixture = bound_fixture();
    let expected = toml::from_str::<ExpectedDigests>(include_str!(
        "../fixtures/content-addressed-bindings/v1/reference-spike-expected.toml"
    ))
    .expect("expected spike digests");
    let document = fixture
        .tree
        .graph
        .roots
        .first()
        .expect("one OpenSpec document root");
    assert_eq!(document.canonical_text(), expected.document);
    assert_eq!(
        fixture.tree.requirement.canonical_text(),
        expected.requirement
    );
    assert_eq!(
        fixture.tree.atom_revision.canonical_text(),
        expected.atom_revision
    );
    assert_eq!(fixture.subject.digest.canonical_text(), expected.subject);
    assert_eq!(fixture.record_digest.canonical_text(), expected.binding);
    assert_eq!(
        compact_binding_digest(&fixture.rust_source),
        Ok(fixture.record_digest)
    );
    assert_eq!(
        rust_subject(RUST_PATH, &fixture.rust_source),
        fixture.subject
    );
    assert_eq!(
        binding_freshness(
            fixture.record_digest,
            Some(&fixture.record),
            fixture.subject.digest,
            &fixture.graph,
            &fixture.authorities,
        ),
        Freshness::Fresh
    );

    let record_path = object_path(fixture.record_digest);
    let path_digest = digest_from_object_path(&record_path).expect("record path digest");
    assert_eq!(
        path_digest,
        compact_binding_digest(&fixture.rust_source).expect("attribute digest")
    );
    let record = parse_record(RECORD_SOURCE).expect("stored binding record");
    assert_eq!(record.digest(), path_digest);
    assert_eq!(
        binding_freshness(
            path_digest,
            Some(&record),
            fixture.subject.digest,
            &fixture.graph,
            &fixture.authorities,
        ),
        Freshness::Fresh
    );

    let mut kinds = fixture.graph.traverse_kinds(record.subject);
    kinds.extend(fixture.graph.traverse_kinds(record.requirement));
    assert_eq!(
        kinds,
        [
            "rust/item/v1",
            "openspec/requirement/v1",
            "openspec/scenario/v1",
            "openspec/step/v1",
            "openspec/step/v1",
            "openspec/step/v1",
            "openspec/scenario/v1",
            "openspec/step/v1",
            "openspec/step/v1",
            "openspec/step/v1",
        ]
    );

    let stored = fixture.record.canonical_toml();
    assert_eq!(stored, RECORD_SOURCE);
    assert_eq!(parse_record(RECORD_SOURCE), Ok(fixture.record.clone()));
    assert_eq!(record_path, RECORD_PATH);
    assert_eq!(
        digest_from_object_path(&record_path),
        Ok(fixture.record_digest)
    );
}

#[test]
fn source_edits_make_the_old_binding_stale_on_the_correct_side() {
    let fixture = bound_fixture();
    let changed_spec = SPEC_SOURCE.replace("byte-identical", "text-identical");
    let changed_tree = openspec_tree(SPEC_PATH, &changed_spec);
    assert_ne!(changed_tree.requirement, fixture.tree.requirement);
    let mut changed_requirement_graph = changed_tree.graph.clone();
    changed_requirement_graph.add_root(fixture.subject.clone());
    changed_requirement_graph.retain_historical_nodes(&fixture.tree.graph);
    let changed_authorities = BTreeMap::from([(
        changed_tree.requirement,
        (changed_tree.atom_id, changed_tree.atom_revision),
    )]);
    assert_eq!(
        binding_freshness(
            fixture.record_digest,
            Some(&fixture.record),
            fixture.subject.digest,
            &changed_requirement_graph,
            &changed_authorities,
        ),
        Freshness::StaleRequirement
    );

    let changed_rust = fixture
        .rust_source
        .replace("libraries.join(\"\\n\")", "libraries.join(\",\")");
    let changed_subject = rust_subject(RUST_PATH, &changed_rust);
    assert_ne!(changed_subject.digest, fixture.subject.digest);
    let mut changed_implementation_graph = fixture.tree.graph.clone();
    changed_implementation_graph.add_root(changed_subject.clone());
    changed_implementation_graph.insert(fixture.subject.clone());
    assert_eq!(
        binding_freshness(
            fixture.record_digest,
            Some(&fixture.record),
            changed_subject.digest,
            &changed_implementation_graph,
            &fixture.authorities,
        ),
        Freshness::StaleImplementation
    );
}

#[test]
fn binding_attributes_are_nonsemantic_to_the_rust_subject() {
    let fixture = bound_fixture();
    let replacement = "blake3-256:ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff";
    let changed_attribute = fixture
        .rust_source
        .replace(&fixture.record_digest.canonical_text(), replacement);
    assert_eq!(rust_subject(RUST_PATH, &changed_attribute), fixture.subject);
    assert_eq!(
        compact_binding_digest(&changed_attribute)
            .expect("replacement is one canonical compact digest")
            .canonical_text(),
        replacement
    );

    let extra_field = fixture.rust_source.replace(
        &format!("\"{}\"", fixture.record_digest),
        &format!("\"{}\", \"extra\"", fixture.record_digest),
    );
    assert_eq!(
        compact_binding_digest(&extra_field),
        Err("attribute-fields")
    );
}

#[test]
fn invalid_records_and_targets_fail_closed() {
    let fixture = bound_fixture();
    assert_eq!(
        binding_freshness(
            fixture.record_digest,
            None,
            fixture.subject.digest,
            &fixture.graph,
            &fixture.authorities,
        ),
        Freshness::MissingRecord
    );

    let mut tampered = fixture.record.clone();
    tampered.id.push_str("-TAMPERED");
    assert_ne!(tampered.digest(), fixture.record_digest);
    assert_eq!(
        binding_freshness(
            fixture.record_digest,
            Some(&tampered),
            fixture.subject.digest,
            &fixture.graph,
            &fixture.authorities,
        ),
        Freshness::MissingRecord
    );

    let scenario = fixture
        .graph
        .nodes
        .values()
        .find(|node| node.kind == "openspec/scenario/v1")
        .expect("one scenario");
    let mut wrong_kind = fixture.record.clone();
    wrong_kind.requirement = scenario.digest;
    let wrong_kind_digest = wrong_kind.digest();
    assert_eq!(
        binding_freshness(
            wrong_kind_digest,
            Some(&wrong_kind),
            fixture.subject.digest,
            &fixture.graph,
            &fixture.authorities,
        ),
        Freshness::WrongRequirementKind
    );

    let mut wrong_authority = fixture.record.clone();
    wrong_authority.atom_revision = fixture.subject.digest;
    let wrong_authority_digest = wrong_authority.digest();
    assert_eq!(
        binding_freshness(
            wrong_authority_digest,
            Some(&wrong_authority),
            fixture.subject.digest,
            &fixture.graph,
            &fixture.authorities,
        ),
        Freshness::AuthorityMismatch
    );

    let original_path = object_path(fixture.record_digest);
    let tampered_path = object_path(tampered.digest());
    assert_ne!(original_path, tampered_path);
    assert_eq!(
        digest_from_object_path(&original_path),
        Ok(fixture.record_digest)
    );
}
