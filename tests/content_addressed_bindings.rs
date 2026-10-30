#![doc = "Repository-level contracts for content-addressed implementation bindings."]
#![allow(clippy::expect_used, clippy::panic, clippy::too_many_lines)]

use buoy::core::{ActiveAlgorithm, DigestAlgorithm};
use buoy::repository::RepositoryConfig;

/// A compact locator carries no algorithm agility, so the object store, the
/// aggregate, and the record identities the repository selects must all address
/// one collision-resistant member.
///
/// The member is a repository selection, not a build constant, so these
/// contracts run under every build that contains one collision-resistant
/// adapter, including the all-feature acceptance build.
#[cfg(any(feature = "blake3-256", feature = "sha-256"))]
mod collision_resistant {
    use buoy::adapter_sdk::{
        AdapterProduct, DeclaredSourceRoot, MaterializedSourceNode, SavedSnapshotInput,
        SourceProduct, SourceRootClass, built_in_binding_configuration, built_in_registry,
    };
    use buoy::adapters::{
        AuthorityExpectation, SourceSpan, binding_object_locator, binding_object_path,
        canonical_binding_record, extract_rust_with,
    };
    use buoy::check::{CheckError, aggregate_source_products, check_snapshot};
    use buoy::core::{
        ActiveAlgorithm, AtomRevisionRef, BindingAttachment, ImplementationBinding,
        ImplementationBindingRef, NavigationKind, ParsedItemIdentity, RepositoryPath, SchemaId,
        SemanticDigest, SourceAuthority, SourceBudgets, SourceChildRef, SourceDepthBudget,
        SourceEdge, SourceEdgeBudget, SourceEdgeKind, SourceGraphError, SourceItemKind, SourceNode,
        SourceNodeBudget, SourceNodeRef, SourceVisitedBudget, SpecGraph, StableId, StoredBinding,
        semantic_digest_with, source_traversal,
    };
    use buoy::repository::{RepositoryConfig, snapshot_repository};
    use serde::Serialize;
    use serde_json::Value;
    use std::fs;
    use std::path::Path;
    use std::process::Command;

    const OPENSPEC_SCHEMA: &str = "openspec/standing-spec/v1";
    const OPENSPEC_CAPABILITY: &str = "openspec.standing-spec/v1";
    const RUST_SCHEMA: &str = "rust/v1";
    const RUST_CAPABILITY: &str = "rust.attribute/v1";
    const SPEC_PATH: &str = "openspec/specs/demo/spec.md";
    const RUST_PATH: &str = "src/lib.rs";
    const SUBJECT_START: u64 = 12;
    const SUBJECT_END: u64 = 48;
    const BINDING_GOLDEN_SCHEMA: &str = "buoy/binding-diagnostic-golden/v1";

    #[derive(Serialize)]
    struct BindingGolden<'a> {
        schema: &'static str,
        case: &'a str,
        accepted: bool,
        source_graph: Option<String>,
        binding: Option<String>,
        diagnostics: Vec<String>,
    }

    /// Returns the collision-resistant member this build can select.
    pub(super) fn active() -> ActiveAlgorithm {
        ActiveAlgorithm::new(super::collision_resistant_member())
            .expect("this build contains one collision-resistant adapter")
    }

    #[test]
    fn compact_attribute_has_one_digest_and_stable_subject_identity() {
        let first = format!(
            "#[buoy::implements(\"{}\")]\n#[inline]\npub fn render() -> &'static str {{ \"first\" }}\n",
            semantic_digest_with(active(), "test-record", vec![])
                .expect("first record digest")
                .canonical_text(),
        );
        let second = format!(
            "#[buoy::implements(\"{}\")]\n#[inline]\npub fn render() -> &'static str {{ \"first\" }}\n",
            semantic_digest_with(active(), "test-record-second", vec![],)
                .expect("second record digest")
                .canonical_text(),
        );
        let changed = first.replace("\"first\"", "\"changed\"");
        let path = RepositoryPath::new(RUST_PATH).expect("Rust path");
        let extract = |source: &str| extract_rust_with(active(), &path, source.as_bytes());
        let first_output = extract(&first);
        let second_output = extract(&second);
        let changed_output = extract(&changed);
        assert!(first_output.diagnostics.is_empty());
        assert_eq!(first_output.bindings.len(), 1);
        assert_eq!(first_output.source_nodes.len(), 1);
        assert_eq!(
            first_output.source_nodes[0].node.digest(),
            second_output.source_nodes[0].node.digest()
        );
        assert_ne!(
            first_output.source_nodes[0].node.digest(),
            changed_output.source_nodes[0].node.digest()
        );
        assert!(
            first_output.source_nodes[0]
                .node
                .item()
                .semantic_bytes()
                .windows(b"inline".len())
                .any(|window| window == b"inline")
        );

        for invalid in [
            "#[buoy::implements()] fn invalid() {}",
            "#[buoy::implements(\"x\", \"y\")] fn invalid() {}",
            "#[buoy::implements(id = \"x\")] fn invalid() {}",
        ] {
            assert_eq!(
                extract(invalid).diagnostics[0].code,
                "CAB-COMPACT-ATTRIBUTE-SYNTAX"
            );
        }
        assert_eq!(
            extract("#[buoy::implements(\"not-a-digest\")] fn invalid() {}").diagnostics[0].code,
            "CAB-COMPACT-ATTRIBUTE-DIGEST"
        );
        let foreign = SemanticDigest::xxh3_128([9; 16]).canonical_text();
        assert_eq!(
            extract(&format!(
                "#[buoy::implements(\"{foreign}\")] fn invalid() {{}}"
            ))
            .diagnostics[0]
                .code,
            "CAB-COMPACT-DIGEST-MEMBER"
        );
    }

    #[test]
    fn nested_compact_attributes_do_not_create_an_enclosing_digest_cycle() {
        let first = semantic_digest_with(active(), "nested-record-first", vec![])
            .expect("first nested record digest")
            .canonical_text();
        let second = semantic_digest_with(active(), "nested-record-second", vec![])
            .expect("second nested record digest")
            .canonical_text();
        let source = |record: &str| {
            format!(
                "mod nested {{\n    #[buoy::implements(\"{record}\")]\n    pub fn render() -> &'static str {{ \"same\" }}\n}}\n"
            )
        };
        let path = RepositoryPath::new(RUST_PATH).expect("Rust path");
        let first = extract_rust_with(active(), &path, source(&first).as_bytes());
        let second = extract_rust_with(active(), &path, source(&second).as_bytes());
        assert!(first.diagnostics.is_empty());
        assert_eq!(first.bindings.len(), 1);
        assert_eq!(first.source_nodes.len(), 2);
        assert_eq!(
            first
                .source_nodes
                .iter()
                .map(|located| located.node.digest())
                .collect::<Vec<_>>(),
            second
                .source_nodes
                .iter()
                .map(|located| located.node.digest())
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn verbose_implementation_metadata_is_not_part_of_subject_identity() {
        let revision = semantic_digest_with(active(), "legacy-atom", vec![])
            .expect("legacy atom revision")
            .canonical_text();
        let source = |id: &str| {
            format!(
                concat!(
                    r"#[spec_",
                    r##"provenance::implementation(
    syntax = "v1",
    body = r#"
id = "{}"
atoms = [{{ id = "DEMO-001", revision = "{}" }}]
navigation = "implements"
"#
)]
pub fn render() -> &'static str {{ "same" }}
"##
                ),
                id, revision
            )
        };
        let path = RepositoryPath::new(RUST_PATH).expect("Rust path");
        let first = extract_rust_with(active(), &path, source("LEGACY-FIRST").as_bytes());
        let second = extract_rust_with(active(), &path, source("LEGACY-SECOND").as_bytes());

        assert!(first.diagnostics.is_empty(), "{:?}", first.diagnostics);
        assert_eq!(first.semantic.len(), 1, "legacy syntax remains readable");
        assert_eq!(first.source_nodes.len(), 1);
        assert_eq!(
            first.source_nodes[0].node.digest(),
            second.source_nodes[0].node.digest(),
            "legacy binding metadata is not part of the implementation subject"
        );
    }

    #[test]
    fn compact_and_verbose_implementation_attributes_fail_closed() {
        let record = semantic_digest_with(active(), "mixed-record", vec![])
            .expect("mixed record digest")
            .canonical_text();
        let revision = semantic_digest_with(active(), "mixed-atom", vec![])
            .expect("mixed atom revision")
            .canonical_text();
        let source = format!(
            concat!(
                r##"#[buoy::implements("{}")]
#[spec_"##,
                r##"provenance::implementation(
    syntax = "v1",
    body = r#"
id = "LEGACY-MIXED"
atoms = [{{ id = "DEMO-001", revision = "{}" }}]
navigation = "implements"
"#
)]
fn mixed() {{}}
"##
            ),
            record, revision
        );
        let path = RepositoryPath::new(RUST_PATH).expect("Rust path");
        let output = extract_rust_with(active(), &path, source.as_bytes());

        assert!(output.bindings.is_empty());
        assert!(output.semantic.is_empty());
        assert_eq!(output.diagnostics.len(), 2);
        assert!(
            output
                .diagnostics
                .iter()
                .all(|diagnostic| diagnostic.code == "CAB-MIXED-BINDING-SYNTAX")
        );
    }

    #[test]
    fn duplicate_compact_attributes_fail_closed() {
        let digest = semantic_digest_with(active(), "test-record", vec![])
            .expect("record digest")
            .canonical_text();
        let source = format!(
            "#[buoy::implements(\"{digest}\")]\n#[buoy::implements(\"{digest}\")]\nfn duplicate() {{}}\n"
        );
        let path = RepositoryPath::new(RUST_PATH).expect("Rust path");
        let output = extract_rust_with(active(), &path, source.as_bytes());
        assert!(output.bindings.is_empty());
        assert_eq!(output.diagnostics.len(), 2);
        assert!(
            output
                .diagnostics
                .iter()
                .all(|diagnostic| diagnostic.code == "CAB-COMPACT-ATTRIBUTE-COUNT")
        );
    }

    fn budgets() -> SourceBudgets {
        SourceBudgets::new(
            SourceNodeBudget::new(64).expect("positive node budget"),
            SourceEdgeBudget::new(64).expect("positive edge budget"),
            SourceDepthBudget::new(16).expect("positive depth budget"),
            SourceVisitedBudget::new(64).expect("positive visited budget"),
        )
    }

    fn node(
        schema: &str,
        path: &str,
        kind: &str,
        payload: &[u8],
        children: Vec<SourceChildRef>,
    ) -> SourceNode {
        let item = ParsedItemIdentity::new_with(
            active(),
            SchemaId::new(schema).expect("pinned language schema"),
            RepositoryPath::new(path).expect("repository-relative path"),
            SourceItemKind::new(kind).expect("declared source item kind"),
            payload.to_vec(),
            None,
        )
        .expect("canonical parsed item identity");
        SourceNode::new_with(active(), item, children).expect("canonical source node")
    }

    fn retained(node: SourceNode, path: &str, capability: &str) -> MaterializedSourceNode {
        span_retained(node, path, capability, 0, 1)
    }

    fn span_retained(
        node: SourceNode,
        path: &str,
        capability: &str,
        start: u64,
        end: u64,
    ) -> MaterializedSourceNode {
        MaterializedSourceNode::new(
            node,
            SourceSpan {
                path: RepositoryPath::new(path).expect("repository-relative path"),
                start,
                end,
            },
            capability,
        )
        .expect("declared syntax capability")
    }

    struct Fixture {
        specification: SourceProduct,
        implementation: SourceProduct,
        store: SourceProduct,
        requirement: SourceNodeRef,
        scenario: SourceNodeRef,
        subject: SourceNodeRef,
        record: ImplementationBinding,
        atom: AtomRevisionRef,
    }

    /// One `OpenSpec` document root, one Rust subject root, and one stored record.
    fn fixture() -> Fixture {
        let step = node(
            OPENSPEC_SCHEMA,
            SPEC_PATH,
            "openspec/step/v1",
            b"GIVEN one current requirement",
            Vec::new(),
        );
        let scenario = node(
            OPENSPEC_SCHEMA,
            SPEC_PATH,
            "openspec/scenario/v1",
            b"Positive - current requirement is reachable",
            vec![SourceChildRef::new(
                SourceEdgeKind::Contains,
                0,
                step.reference(),
            )],
        );
        let requirement = node(
            OPENSPEC_SCHEMA,
            SPEC_PATH,
            "openspec/requirement/v1",
            b"DEMO-001 the catalog renders every library",
            vec![SourceChildRef::new(
                SourceEdgeKind::Contains,
                0,
                scenario.reference(),
            )],
        );
        let document = node(
            OPENSPEC_SCHEMA,
            SPEC_PATH,
            "openspec/document/v1",
            b"demo",
            vec![SourceChildRef::new(
                SourceEdgeKind::Contains,
                0,
                requirement.reference(),
            )],
        );
        let subject = node(
            RUST_SCHEMA,
            RUST_PATH,
            "rust/item/v1",
            b"pub fn render_catalog(libraries: &[&str]) -> String { libraries.join(\"\\n\") }",
            Vec::new(),
        );

        let atom = AtomRevisionRef::new(
            StableId::new("DEMO-001").expect("stable atom ID"),
            requirement.digest(),
        );
        let record = ImplementationBinding::new_with(
            active(),
            StableId::new("DEMO-RENDER-CATALOG").expect("stable relation ID"),
            NavigationKind::Implements,
            subject.reference(),
            requirement.reference(),
            atom.clone(),
        )
        .expect("one collision-resistant binding record");

        let specification = SourceProduct::new(
            vec![
                retained(document.clone(), SPEC_PATH, OPENSPEC_CAPABILITY),
                retained(requirement.clone(), SPEC_PATH, OPENSPEC_CAPABILITY),
                retained(scenario.clone(), SPEC_PATH, OPENSPEC_CAPABILITY),
                retained(step, SPEC_PATH, OPENSPEC_CAPABILITY),
            ],
            Vec::new(),
            vec![DeclaredSourceRoot::new(
                document.reference(),
                SourceRootClass::StandingSpecification,
            )],
            vec![SourceAuthority::new(requirement.reference(), atom.clone())],
            Vec::new(),
            Vec::new(),
        )
        .expect("retained OpenSpec source product");
        let implementation = SourceProduct::new(
            vec![span_retained(
                subject.clone(),
                RUST_PATH,
                RUST_CAPABILITY,
                SUBJECT_START,
                SUBJECT_END,
            )],
            Vec::new(),
            vec![DeclaredSourceRoot::new(
                subject.reference(),
                SourceRootClass::Navigational,
            )],
            Vec::new(),
            vec![BindingAttachment::new(subject.reference(), record.record())],
            Vec::new(),
        )
        .expect("retained Rust source product");
        let store = SourceProduct::new(
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            vec![StoredBinding::new(record.record(), record.clone())],
        )
        .expect("retained object-store product");

        Fixture {
            specification,
            implementation,
            store,
            requirement: requirement.reference(),
            scenario: scenario.reference(),
            subject: subject.reference(),
            record,
            atom,
        }
    }

    fn store_input_bytes(record: &ImplementationBinding) -> (String, String) {
        (
            binding_object_path(record.record()),
            canonical_binding_record(record),
        )
    }

    fn materialize_store(path: &str, bytes: &[u8]) -> Result<AdapterProduct, String> {
        let registry = built_in_registry().expect("built-in adapter registry");
        let path = RepositoryPath::new(path).expect("repository-relative object path");
        let configuration =
            built_in_binding_configuration(active(), AuthorityExpectation::Forbidden)
                .expect("bound object-store configuration");
        let input =
            SavedSnapshotInput::new(&path, bytes, &configuration).expect("saved store input");
        registry
            .materialize(&input)
            .map(|output| output.product().clone())
            .map_err(|error| error.code().to_owned())
    }

    fn store_diagnostics(path: &str, bytes: &[u8]) -> Vec<String> {
        let product = materialize_store(path, bytes).expect("total object-store materialization");
        product
            .diagnostics()
            .iter()
            .map(|diagnostic| diagnostic.code.to_owned())
            .collect()
    }

    fn store_prefix() -> String {
        format!("buoy/objects/v1/{}/", active().algorithm().canonical_name())
    }

    #[test]
    fn object_store_resolves_one_canonical_record_at_its_digest_derived_path() {
        let fixture = fixture();
        let (path, source) = store_input_bytes(&fixture.record);
        assert_eq!(
            path,
            format!(
                "{}{}.toml",
                store_prefix(),
                fixture
                    .record
                    .record()
                    .canonical_text()
                    .split_once(':')
                    .expect("canonical digest separator")
                    .1
            )
        );
        assert!(!path.contains(':'), "the store path carries no colon");

        let product = materialize_store(&path, source.as_bytes())
            .expect("total object-store materialization");
        assert!(
            product.diagnostics().is_empty(),
            "{:?}",
            product.diagnostics()
        );
        assert_eq!(product.source().bindings().len(), 1);
        let stored = &product.source().bindings()[0];
        assert_eq!(stored.locator(), fixture.record.record());
        assert_eq!(stored.record(), &fixture.record);
        assert!(product.graph().implementation_bindings.is_empty());
    }

    #[test]
    fn object_store_computes_record_identity_under_the_configured_member() {
        let fixture = fixture();
        assert_eq!(
            fixture.record.record().digest().algorithm(),
            active().algorithm(),
            "the record identity computes under the repository selection"
        );
        assert_eq!(
            fixture.record.meta().digest().algorithm(),
            active().algorithm()
        );
        let build_default = ActiveAlgorithm::build_default()
            .expect("a digest-producing build")
            .algorithm();
        if build_default != active().algorithm() {
            let unselected = ImplementationBinding::new(
                StableId::new("DEMO-RENDER-CATALOG").expect("stable relation ID"),
                NavigationKind::Implements,
                fixture.subject,
                fixture.requirement,
                fixture.atom.clone(),
            )
            .expect("a build-default record over the same references");
            assert_eq!(
                canonical_binding_record(&unselected),
                canonical_binding_record(&fixture.record),
                "the canonical record text names no digest member"
            );
            assert_ne!(
                unselected.record().digest().algorithm(),
                fixture.record.record().digest().algorithm(),
            );
            assert_ne!(
                unselected.record(),
                fixture.record.record(),
                "a build-default identity never addresses the configured record"
            );
        }
        let (path, source) = store_input_bytes(&fixture.record);
        assert!(
            store_diagnostics(&path, source.as_bytes()).is_empty(),
            "the configured member alone settles the digest-derived store path"
        );
    }

    #[test]
    fn object_store_rejects_paths_schemas_relations_and_noncanonical_records() {
        let fixture = fixture();
        let (path, source) = store_input_bytes(&fixture.record);
        let prefix = store_prefix();
        let payload = path
            .strip_prefix(prefix.as_str())
            .expect("digest-derived store path");

        assert_eq!(
            store_diagnostics(&format!("{prefix}binding.toml"), source.as_bytes()),
            vec![String::from("CAB-OBJECT-PATH")]
        );
        let empty_payload = RepositoryPath::new("buoy/objects/v1/blake3-256/.toml")
            .expect("repository object path");
        assert_eq!(binding_object_locator(&empty_payload), None);
        assert_eq!(
            store_diagnostics(
                &path.replace("buoy/objects/v1/", "buoy/objects/v2/"),
                source.as_bytes()
            ),
            vec![String::from("CAB-OBJECT-PATH")]
        );
        assert_eq!(
            store_diagnostics(&format!("{prefix}nested/{payload}"), source.as_bytes()),
            vec![String::from("CAB-OBJECT-PATH")]
        );
        assert_eq!(
            store_diagnostics(&format!("buoy/objects/v1/{payload}"), source.as_bytes()),
            vec![String::from("CAB-OBJECT-PATH")]
        );
        assert_eq!(
            store_diagnostics(
                &format!("{prefix}{}", payload.replace(".toml", ".txt")),
                source.as_bytes()
            ),
            vec![String::from("CAB-OBJECT-PATH")]
        );
        assert_eq!(
            store_diagnostics(&path, &[0xff, 0xfe]),
            vec![String::from("CAB-OBJECT-SYNTAX")]
        );
        assert_eq!(
            store_diagnostics(&path, b"schema = "),
            vec![String::from("CAB-OBJECT-SYNTAX")]
        );
        assert_eq!(
            store_diagnostics(
                &path,
                source
                    .replace(
                        "buoy/implementation-binding/v1",
                        "buoy/implementation-binding/v2"
                    )
                    .as_bytes()
            ),
            vec![String::from("CAB-OBJECT-SCHEMA")]
        );
        assert_eq!(
            store_diagnostics(
                &path,
                source
                    .replace("relation = \"implements\"", "relation = \"owns\"")
                    .as_bytes()
            ),
            vec![String::from("CAB-OBJECT-RELATION")]
        );
        assert_eq!(
            store_diagnostics(&path, format!("# comment\n{source}").as_bytes()),
            vec![String::from("CAB-OBJECT-CANONICAL")]
        );
        assert_eq!(
            store_diagnostics(
                &path,
                source
                    .replace("DEMO-RENDER-CATALOG", "DEMO-RENDER-CATALOG-TAMPERED")
                    .as_bytes()
            ),
            vec![String::from("CAB-OBJECT-PATH-DIGEST")]
        );
        assert_eq!(
            store_diagnostics(
                &path,
                source.replace("id = \"DEMO-001\"", "id = \"\"").as_bytes(),
            ),
            vec![String::from("CAB-OBJECT-FIELD")]
        );
        for malformed in [
            source.replace("id = \"DEMO-RENDER-CATALOG\"", "id = \"\""),
            source.replace("requirement = \"", "requirement = \"x"),
            source.replace("revision = \"", "revision = \"x"),
        ] {
            assert_eq!(
                store_diagnostics(&path, malformed.as_bytes()),
                vec![String::from("CAB-OBJECT-FIELD")]
            );
        }

        let rejected = materialize_store(&path, b"schema = ").expect("total materialization");
        assert!(
            rejected.source().bindings().is_empty(),
            "a rejected record never reaches the aggregate"
        );
    }

    #[test]
    fn object_store_rejects_every_reference_outside_the_selected_member() {
        let fixture = fixture();
        let (path, source) = store_input_bytes(&fixture.record);
        let selected = active().algorithm().canonical_name();
        assert_eq!(
            store_diagnostics(&path, source.replace(selected, "blake3-255").as_bytes()),
            vec![String::from("CAB-OBJECT-FIELD")],
            "an unregistered member name never parses"
        );
        let change_detection = SemanticDigest::xxh3_128([7; 16]).canonical_text();
        assert_eq!(
            store_diagnostics(
                &path,
                source
                    .replace(&fixture.requirement.canonical_text(), &change_detection)
                    .as_bytes()
            ),
            vec![String::from("CAB-DIGEST-CLASS")],
            "a change-detection reference never addresses one compact locator"
        );
        if let Some(other) = super::other_collision_resistant_member(active().algorithm()) {
            let foreign = SemanticDigest::parse_canonical(&format!(
                "{}:{}",
                other.canonical_name(),
                "7".repeat(other.output_length() * 2)
            ))
            .expect("one canonical foreign-member digest");
            assert_eq!(
                store_diagnostics(
                    &path,
                    source
                        .replace(
                            &fixture.requirement.canonical_text(),
                            &foreign.canonical_text()
                        )
                        .as_bytes()
                ),
                vec![String::from("CAB-DIGEST-MEMBER")],
                "every reference shares the one member the repository selected"
            );
            let foreign_path = format!(
                "buoy/objects/v1/{}/{}.toml",
                foreign.algorithm().canonical_name(),
                "7".repeat(foreign.algorithm().output_length() * 2)
            );
            assert_eq!(
                store_diagnostics(&foreign_path, source.as_bytes()),
                vec![String::from("CAB-DIGEST-MEMBER")],
                "the claimed locator must use the selected member"
            );
        }
    }

    #[cfg(feature = "blake3-256")]
    #[test]
    fn repository_binding_budgets_fail_at_each_post_materialization_boundary() {
        let root = Path::new("fixtures/content-addressed-bindings/v1/positive");
        let source = fs::read_to_string(root.join("buoy.toml")).expect("positive configuration");
        let config = |source_nodes: u64, graph_objects: u64, diagnostics: u64| {
            RepositoryConfig::parse(
                format!(
                    "{source}\n[budgets]\npath_depth = 64\ndirectory_entries = 100000\nselected_files = 10000\nfile_bytes = 16777216\ntotal_bytes = 536870912\ngraph_objects = {graph_objects}\ngraph_references = 2000000\ndiagnostics = {diagnostics}\nsource_nodes = {source_nodes}\nsource_edges = 400000\nsource_depth = 64\nsource_visited = 200000\n"
                )
                .as_bytes(),
            )
            .expect("binding budget configuration")
        };
        let complete = config(200_000, 200_000, 100_000);
        let snapshot = snapshot_repository(root, &complete).expect("positive binding snapshot");
        assert_eq!(
            check_snapshot(&snapshot, &config(1, 200_000, 100_000)),
            Err(CheckError::SourceGraph(SourceGraphError::NodeBudget))
        );
        assert_eq!(
            check_snapshot(&snapshot, &config(200_000, 1, 100_000)),
            Err(CheckError::GraphObjectsBudget)
        );

        let mut stale = snapshot;
        let rust = stale
            .files
            .iter_mut()
            .find(|file| file.path.as_str() == RUST_PATH)
            .expect("selected Rust source");
        let missing = "blake3-256:0000000000000000000000000000000000000000000000000000000000000000";
        rust.bytes = format!(
            "#[buoy::implements(\"{missing}\")]\npub fn first() {{}}\n#[buoy::implements(\"{missing}\")]\npub fn second() {{}}\n"
        )
        .into_bytes();
        assert_eq!(
            check_snapshot(&stale, &config(200_000, 200_000, 1)),
            Err(CheckError::DiagnosticsBudget)
        );
    }

    #[test]
    fn aggregate_joins_separate_adapter_products_before_reference_resolution() {
        let fixture = fixture();
        let aggregate = aggregate_source_products(
            active(),
            &[
                fixture.specification.clone(),
                fixture.implementation.clone(),
                fixture.store.clone(),
            ],
            budgets(),
        )
        .expect("closed repository source aggregate");

        assert_eq!(aggregate.graph().nodes().len(), 5);
        assert_eq!(aggregate.graph().roots().len(), 2);
        assert_eq!(
            aggregate.standing_roots().len(),
            1,
            "only a standing specification selector establishes a specification root"
        );
        assert!(aggregate.closure().contains(fixture.requirement));
        assert!(aggregate.closure().contains(fixture.scenario));
        assert!(aggregate.closure().contains(fixture.subject));
        assert_eq!(aggregate.attachments().len(), 1);
        assert_eq!(aggregate.store().len(), 1);
        assert!(
            aggregate.binding_diagnostics().is_empty(),
            "{:?}",
            aggregate.binding_diagnostics()
        );
    }

    #[test]
    fn declared_navigation_edges_extend_bounded_binding_traversal() {
        let fixture = fixture();
        let implements = SourceEdge::new(
            fixture.subject,
            SourceEdgeKind::Implements,
            None,
            fixture.requirement,
        )
        .expect("one unpositioned navigation edge");
        let authority = SourceEdge::new(
            fixture.requirement,
            SourceEdgeKind::Authority,
            None,
            fixture.scenario,
        )
        .expect("one unpositioned authority edge");
        let navigating = SourceProduct::new(
            fixture.implementation.nodes().to_vec(),
            vec![implements],
            fixture.implementation.roots().to_vec(),
            Vec::new(),
            fixture.implementation.attachments().to_vec(),
            Vec::new(),
        )
        .expect("a product declares edges out of nodes it retained");
        let corresponding = SourceProduct::new(
            fixture.specification.nodes().to_vec(),
            vec![authority],
            fixture.specification.roots().to_vec(),
            fixture.specification.authorities().to_vec(),
            Vec::new(),
            Vec::new(),
        )
        .expect("a specification product declares its authority correspondence");

        let aggregate = aggregate_source_products(
            active(),
            &[corresponding, navigating, fixture.store.clone()],
            budgets(),
        )
        .expect("declared navigation edges aggregate");
        assert!(aggregate.graph().edges().contains(&implements));
        assert!(aggregate.graph().edges().contains(&authority));
        let reached = source_traversal(aggregate.graph(), fixture.subject)
            .expect("bounded traversal from the implementation subject");
        assert!(
            reached.contains(fixture.requirement),
            "an Implements edge reaches the requirement it names"
        );
        assert!(
            reached.contains(fixture.scenario),
            "an Authority edge continues the same traversal"
        );
    }

    #[test]
    fn declared_containment_must_repeat_one_retained_node_preimage() {
        let fixture = fixture();
        let unbacked = SourceEdge::new(
            fixture.subject,
            SourceEdgeKind::Contains,
            Some(0),
            fixture.requirement,
        )
        .expect("one positioned containment edge");
        let claiming = SourceProduct::new(
            fixture.implementation.nodes().to_vec(),
            vec![unbacked],
            fixture.implementation.roots().to_vec(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
        )
        .expect("a product may declare any edge out of a retained node");
        assert_eq!(
            aggregate_source_products(
                active(),
                &[fixture.specification.clone(), claiming],
                budgets()
            )
            .map(|_| ()),
            Err(CheckError::SourceGraph(
                SourceGraphError::UnbackedContainment
            )),
            "local containment stays Merkle-backed"
        );
    }

    #[test]
    fn a_product_declares_edges_only_out_of_nodes_it_retained() {
        let fixture = fixture();
        let foreign = SourceEdge::new(
            fixture.requirement,
            SourceEdgeKind::Implements,
            None,
            fixture.subject,
        )
        .expect("one unpositioned navigation edge");
        assert!(
            SourceProduct::new(
                fixture.implementation.nodes().to_vec(),
                vec![foreign],
                Vec::new(),
                Vec::new(),
                Vec::new(),
                Vec::new(),
            )
            .is_err(),
            "an edge origin must name one retained node"
        );
    }

    #[test]
    fn aggregate_order_and_repetition_do_not_change_the_result() {
        let fixture = fixture();
        let forward = aggregate_source_products(
            active(),
            &[
                fixture.specification.clone(),
                fixture.implementation.clone(),
                fixture.store.clone(),
            ],
            budgets(),
        )
        .expect("forward aggregate");
        let reverse = aggregate_source_products(
            active(),
            &[
                fixture.store.clone(),
                fixture.implementation.clone(),
                fixture.specification.clone(),
            ],
            budgets(),
        )
        .expect("reverse aggregate");
        let repeated = aggregate_source_products(
            active(),
            &[
                fixture.specification.clone(),
                fixture.store.clone(),
                fixture.implementation.clone(),
                fixture.specification.clone(),
                fixture.store.clone(),
            ],
            budgets(),
        )
        .expect("repeated aggregate");
        assert_eq!(forward, reverse);
        assert_eq!(forward, repeated);
        assert_eq!(forward.graph().digest(), repeated.graph().digest());
    }

    #[test]
    fn binding_freshness_fails_closed_on_absent_stale_wrong_kind_and_ambiguous_references() {
        let fixture = fixture();
        let unresolved = aggregate_source_products(
            active(),
            &[
                fixture.specification.clone(),
                fixture.implementation.clone(),
            ],
            budgets(),
        )
        .expect("aggregate without the object store");
        assert_eq!(
            unresolved
                .binding_diagnostics()
                .iter()
                .map(|diagnostic| diagnostic.code.clone())
                .collect::<Vec<_>>(),
            vec![String::from("CAB-RECORD-ABSENT")]
        );

        let historical = aggregate_source_products(
            active(),
            &[
                SourceProduct::new(
                    fixture.specification.nodes().to_vec(),
                    Vec::new(),
                    Vec::new(),
                    fixture.specification.authorities().to_vec(),
                    Vec::new(),
                    Vec::new(),
                )
                .expect("noncurrent OpenSpec product"),
                fixture.implementation.clone(),
                fixture.store.clone(),
            ],
            budgets(),
        )
        .expect("aggregate without a current document root");
        assert!(
            historical
                .binding_diagnostics()
                .iter()
                .any(|diagnostic| diagnostic.code == "CAB-REQUIREMENT-STALE"),
            "{:?}",
            historical.binding_diagnostics()
        );

        let wrong_kind_record = ImplementationBinding::new_with(
            active(),
            StableId::new("DEMO-RENDER-CATALOG").expect("stable relation ID"),
            NavigationKind::Implements,
            fixture.subject,
            fixture.scenario,
            fixture.atom.clone(),
        )
        .expect("binding to one scenario node");
        let wrong_kind = aggregate_source_products(
            active(),
            &[
                fixture.specification.clone(),
                SourceProduct::new(
                    fixture.implementation.nodes().to_vec(),
                    Vec::new(),
                    fixture.implementation.roots().to_vec(),
                    Vec::new(),
                    vec![BindingAttachment::new(
                        fixture.subject,
                        wrong_kind_record.record(),
                    )],
                    Vec::new(),
                )
                .expect("Rust product bound to one scenario"),
                SourceProduct::new(
                    Vec::new(),
                    Vec::new(),
                    Vec::new(),
                    Vec::new(),
                    Vec::new(),
                    vec![StoredBinding::new(
                        wrong_kind_record.record(),
                        wrong_kind_record.clone(),
                    )],
                )
                .expect("wrong-kind store product"),
            ],
            budgets(),
        )
        .expect("aggregate with a wrong-kind requirement reference");
        assert!(
            wrong_kind
                .binding_diagnostics()
                .iter()
                .any(|diagnostic| diagnostic.code == "CAB-REQUIREMENT-KIND"),
            "{:?}",
            wrong_kind.binding_diagnostics()
        );

        let ambiguous = aggregate_source_products(
            active(),
            &[
                fixture.specification.clone(),
                fixture.implementation.clone(),
                fixture.store.clone(),
                SourceProduct::new(
                    Vec::new(),
                    Vec::new(),
                    Vec::new(),
                    Vec::new(),
                    Vec::new(),
                    vec![StoredBinding::new(
                        fixture.record.record(),
                        wrong_kind_record,
                    )],
                )
                .expect("second store product under one locator"),
            ],
            budgets(),
        )
        .expect("aggregate with one ambiguous locator");
        let codes = ambiguous
            .binding_diagnostics()
            .iter()
            .map(|diagnostic| diagnostic.code.clone())
            .collect::<Vec<_>>();
        assert!(
            codes.contains(&String::from("CAB-OBJECT-DUPLICATE")),
            "{codes:?}"
        );
        assert!(
            codes.contains(&String::from("CAB-RECORD-AMBIGUOUS")),
            "{codes:?}"
        );
    }

    fn diagnostic_codes(aggregate: &buoy::check::RepositorySourceAggregate) -> Vec<String> {
        aggregate
            .binding_diagnostics()
            .into_iter()
            .map(|diagnostic| diagnostic.code)
            .collect()
    }

    fn golden_bytes(
        case: &str,
        result: Result<
            (
                buoy::check::RepositorySourceAggregate,
                Option<ImplementationBindingRef>,
            ),
            CheckError,
        >,
    ) -> String {
        let report = match result {
            Ok((aggregate, binding)) => {
                let diagnostics = diagnostic_codes(&aggregate);
                BindingGolden {
                    schema: BINDING_GOLDEN_SCHEMA,
                    case,
                    accepted: diagnostics.is_empty(),
                    source_graph: Some(aggregate.graph().digest().canonical_text()),
                    binding: binding.map(ImplementationBindingRef::canonical_text),
                    diagnostics,
                }
            }
            Err(error) => BindingGolden {
                schema: BINDING_GOLDEN_SCHEMA,
                case,
                accepted: false,
                source_graph: None,
                binding: None,
                diagnostics: vec![String::from(error.code())],
            },
        };
        format!(
            "{}\n",
            serde_json::to_string(&report).expect("binding golden serialization")
        )
    }

    fn golden_path(case: &str) -> std::path::PathBuf {
        std::path::Path::new("fixtures/content-addressed-bindings/v1/goldens")
            .join(format!("{case}.json"))
    }

    fn write_binding_golden(case: &str, actual: &str) {
        if std::env::var_os("BUOY_WRITE_BINDING_GOLDENS").is_none() {
            return;
        }
        std::fs::create_dir_all(
            golden_path(case)
                .parent()
                .expect("binding golden directory"),
        )
        .expect("create binding golden directory");
        std::fs::write(golden_path(case), actual).expect("write binding golden");
    }

    fn assert_binding_golden(
        case: &str,
        result: Result<
            (
                buoy::check::RepositorySourceAggregate,
                Option<ImplementationBindingRef>,
            ),
            CheckError,
        >,
    ) {
        let actual = golden_bytes(case, result);
        write_binding_golden(case, &actual);
        let expected = std::fs::read_to_string(golden_path(case)).expect("binding golden");
        assert_eq!(actual, expected, "{case}");
    }

    fn aggregate_fixture(
        fixture: &Fixture,
    ) -> Result<buoy::check::RepositorySourceAggregate, CheckError> {
        aggregate_source_products(
            active(),
            &[
                fixture.specification.clone(),
                fixture.implementation.clone(),
                fixture.store.clone(),
            ],
            budgets(),
        )
    }

    #[test]
    fn deterministic_binding_goldens_cover_valid_stale_historical_cycle_and_wrong_kind_cases() {
        let fixture = fixture();
        assert_binding_golden(
            "valid",
            aggregate_fixture(&fixture).map(|aggregate| (aggregate, Some(fixture.record.record()))),
        );

        let current_specification = fixture.specification.clone();
        let historical_specification = SourceProduct::new(
            fixture.specification.nodes().to_vec(),
            Vec::new(),
            Vec::new(),
            fixture.specification.authorities().to_vec(),
            Vec::new(),
            Vec::new(),
        )
        .expect("historical OpenSpec nodes");
        for case in ["stale-requirement", "historical-node"] {
            assert_binding_golden(
                case,
                aggregate_source_products(
                    active(),
                    &[
                        historical_specification.clone(),
                        fixture.implementation.clone(),
                        fixture.store.clone(),
                    ],
                    budgets(),
                )
                .map(|aggregate| (aggregate, Some(fixture.record.record()))),
            );
        }

        let current_subject = node(
            RUST_SCHEMA,
            RUST_PATH,
            "rust/item/v1",
            b"pub fn render_catalog(libraries: &[&str]) -> String { libraries.concat() }",
            Vec::new(),
        );
        let stale_implementation = SourceProduct::new(
            vec![
                span_retained(
                    fixture
                        .implementation
                        .nodes()
                        .first()
                        .expect("historical subject")
                        .node()
                        .clone(),
                    RUST_PATH,
                    RUST_CAPABILITY,
                    SUBJECT_START,
                    SUBJECT_END,
                ),
                span_retained(
                    current_subject.clone(),
                    RUST_PATH,
                    RUST_CAPABILITY,
                    SUBJECT_START,
                    SUBJECT_END,
                ),
            ],
            Vec::new(),
            vec![DeclaredSourceRoot::new(
                current_subject.reference(),
                SourceRootClass::Navigational,
            )],
            Vec::new(),
            vec![BindingAttachment::new(
                current_subject.reference(),
                fixture.record.record(),
            )],
            Vec::new(),
        )
        .expect("stale implementation product");
        assert_binding_golden(
            "stale-implementation",
            aggregate_source_products(
                active(),
                &[
                    current_specification.clone(),
                    stale_implementation,
                    fixture.store.clone(),
                ],
                budgets(),
            )
            .map(|aggregate| (aggregate, Some(fixture.record.record()))),
        );

        let wrong_kind_record = ImplementationBinding::new_with(
            active(),
            StableId::new("DEMO-RENDER-CATALOG").expect("stable relation ID"),
            NavigationKind::Implements,
            fixture.subject,
            fixture.scenario,
            fixture.atom.clone(),
        )
        .expect("wrong-kind record");
        let wrong_kind_implementation = SourceProduct::new(
            fixture.implementation.nodes().to_vec(),
            Vec::new(),
            fixture.implementation.roots().to_vec(),
            Vec::new(),
            vec![BindingAttachment::new(
                fixture.subject,
                wrong_kind_record.record(),
            )],
            Vec::new(),
        )
        .expect("wrong-kind attachment");
        let wrong_kind_store = SourceProduct::new(
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            vec![StoredBinding::new(
                wrong_kind_record.record(),
                wrong_kind_record.clone(),
            )],
        )
        .expect("wrong-kind store");
        assert_binding_golden(
            "wrong-kind",
            aggregate_source_products(
                active(),
                &[
                    current_specification,
                    wrong_kind_implementation,
                    wrong_kind_store,
                ],
                budgets(),
            )
            .map(|aggregate| (aggregate, Some(wrong_kind_record.record()))),
        );

        let cycle = SourceProduct::new(
            fixture.specification.nodes().to_vec(),
            vec![
                SourceEdge::new(
                    fixture.requirement,
                    SourceEdgeKind::Implements,
                    None,
                    fixture.subject,
                )
                .expect("requirement-to-subject edge"),
            ],
            fixture.specification.roots().to_vec(),
            fixture.specification.authorities().to_vec(),
            Vec::new(),
            Vec::new(),
        )
        .expect("cycle contributor");
        let reverse = SourceProduct::new(
            fixture.implementation.nodes().to_vec(),
            vec![
                SourceEdge::new(
                    fixture.subject,
                    SourceEdgeKind::Implements,
                    None,
                    fixture.requirement,
                )
                .expect("subject-to-requirement edge"),
            ],
            fixture.implementation.roots().to_vec(),
            Vec::new(),
            fixture.implementation.attachments().to_vec(),
            Vec::new(),
        )
        .expect("reverse cycle contributor");
        assert_binding_golden(
            "cycle",
            aggregate_source_products(active(), &[cycle, reverse, fixture.store], budgets())
                .map(|aggregate| (aggregate, None)),
        );
    }

    #[test]
    fn binding_diagnostics_carry_the_exact_attachment_subject_span() {
        let fixture = fixture();
        let unresolved = aggregate_source_products(
            active(),
            &[
                fixture.specification.clone(),
                fixture.implementation.clone(),
            ],
            budgets(),
        )
        .expect("aggregate without the object store");
        let reported = unresolved.binding_diagnostics();
        assert_eq!(reported.len(), 1, "{reported:?}");
        assert_eq!(reported[0].path.as_deref(), Some(RUST_PATH));
        assert_eq!(reported[0].start, SUBJECT_START);
        assert_eq!(reported[0].end, SUBJECT_END);
        assert_eq!(
            unresolved
                .span(fixture.subject)
                .map(|span| (span.start, span.end)),
            Some((SUBJECT_START, SUBJECT_END))
        );
    }

    #[test]
    fn aggregate_fails_closed_above_every_configured_source_budget() {
        let fixture = fixture();
        let products = [
            fixture.specification.clone(),
            fixture.implementation.clone(),
            fixture.store.clone(),
        ];
        let node_bound = SourceBudgets::new(
            SourceNodeBudget::new(4).expect("positive node budget"),
            SourceEdgeBudget::new(64).expect("positive edge budget"),
            SourceDepthBudget::new(16).expect("positive depth budget"),
            SourceVisitedBudget::new(64).expect("positive visited budget"),
        );
        assert_eq!(
            aggregate_source_products(active(), &products, node_bound).map(|_| ()),
            Err(CheckError::SourceGraph(SourceGraphError::NodeBudget))
        );
        let edge_bound = SourceBudgets::new(
            SourceNodeBudget::new(64).expect("positive node budget"),
            SourceEdgeBudget::new(2).expect("positive edge budget"),
            SourceDepthBudget::new(16).expect("positive depth budget"),
            SourceVisitedBudget::new(64).expect("positive visited budget"),
        );
        assert_eq!(
            aggregate_source_products(active(), &products, edge_bound).map(|_| ()),
            Err(CheckError::SourceGraph(SourceGraphError::EdgeBudget))
        );
        let depth_bound = SourceBudgets::new(
            SourceNodeBudget::new(64).expect("positive node budget"),
            SourceEdgeBudget::new(64).expect("positive edge budget"),
            SourceDepthBudget::new(1).expect("positive depth budget"),
            SourceVisitedBudget::new(64).expect("positive visited budget"),
        );
        assert_eq!(
            aggregate_source_products(active(), &products, depth_bound).map(|_| ()),
            Err(CheckError::SourceGraph(SourceGraphError::DepthBudget))
        );
        let visited_bound = SourceBudgets::new(
            SourceNodeBudget::new(64).expect("positive node budget"),
            SourceEdgeBudget::new(64).expect("positive edge budget"),
            SourceDepthBudget::new(16).expect("positive depth budget"),
            SourceVisitedBudget::new(2).expect("positive visited budget"),
        );
        assert_eq!(
            aggregate_source_products(active(), &products, visited_bound).map(|_| ()),
            Err(CheckError::SourceGraph(SourceGraphError::VisitedBudget))
        );
    }

    #[test]
    fn aggregation_counts_every_reference_row_against_its_budget() {
        let fixture = fixture();
        // Two roots, one authority, one attachment, and one record are five
        // reference rows, so a four-row limit fails closed before the aggregate
        // graph is built.
        let reference_bound = SourceBudgets::new(
            SourceNodeBudget::new(64).expect("positive node budget"),
            SourceEdgeBudget::new(4).expect("positive edge budget"),
            SourceDepthBudget::new(16).expect("positive depth budget"),
            SourceVisitedBudget::new(64).expect("positive visited budget"),
        );
        assert_eq!(
            aggregate_source_products(
                active(),
                &[
                    fixture.specification.clone(),
                    fixture.implementation.clone(),
                    fixture.store.clone(),
                ],
                reference_bound
            )
            .map(|_| ()),
            Err(CheckError::SourceGraph(SourceGraphError::EdgeBudget))
        );
    }

    #[test]
    fn a_retained_node_needs_no_semantic_object() {
        let fixture = fixture();
        let product = AdapterProduct::new_with_source_product(
            SpecGraph::default(),
            Vec::new(),
            fixture.specification.clone(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
        )
        .expect("a product may retain nodes without semantic objects");
        assert!(product.graph().atoms.is_empty());
        assert_eq!(product.source().nodes().len(), 4);
        assert!(
            product
                .source()
                .nodes()
                .iter()
                .any(|retained| retained.node().item().kind().as_str() == "openspec/scenario/v1")
        );
    }

    #[test]
    fn a_source_product_declares_only_nodes_it_retains() {
        let fixture = fixture();
        let empty = SourceProduct::default();
        assert!(empty.is_empty());
        assert!(!fixture.specification.is_empty());

        let foreign = SourceProduct::new(
            fixture.specification.nodes().to_vec(),
            Vec::new(),
            vec![DeclaredSourceRoot::new(
                fixture.subject,
                SourceRootClass::Navigational,
            )],
            Vec::new(),
            Vec::new(),
            Vec::new(),
        );
        assert!(foreign.is_err(), "a root must name one retained node");
        let duplicate = fixture.specification.nodes()[0].clone();
        assert!(
            SourceProduct::new(
                vec![duplicate.clone(), duplicate],
                Vec::new(),
                Vec::new(),
                Vec::new(),
                Vec::new(),
                Vec::new(),
            )
            .is_err(),
            "a product rejects duplicate retained nodes"
        );
        let foreign_attachment = SourceProduct::new(
            fixture.specification.nodes().to_vec(),
            Vec::new(),
            vec![DeclaredSourceRoot::new(
                fixture.requirement,
                SourceRootClass::StandingSpecification,
            )],
            Vec::new(),
            vec![BindingAttachment::new(
                fixture.subject,
                ImplementationBindingRef::new(fixture.record.record().digest()),
            )],
            Vec::new(),
        );
        assert!(
            foreign_attachment.is_err(),
            "an attachment must name one retained subject"
        );
        let foreign_authority = SourceProduct::new(
            fixture.implementation.nodes().to_vec(),
            Vec::new(),
            Vec::new(),
            fixture.specification.authorities().to_vec(),
            Vec::new(),
            Vec::new(),
        );
        assert!(
            foreign_authority.is_err(),
            "an authority must name one retained node"
        );

        let duplicate_edge = SourceEdge::new(
            fixture.requirement,
            SourceEdgeKind::Authority,
            None,
            fixture.scenario,
        )
        .expect("duplicate-edge control");
        assert!(
            SourceProduct::new(
                fixture.specification.nodes().to_vec(),
                vec![duplicate_edge, duplicate_edge],
                Vec::new(),
                Vec::new(),
                Vec::new(),
                Vec::new(),
            )
            .is_err(),
            "a product rejects duplicate edges"
        );
        let duplicate_root = fixture.specification.roots()[0];
        assert!(
            SourceProduct::new(
                fixture.specification.nodes().to_vec(),
                Vec::new(),
                vec![duplicate_root, duplicate_root],
                Vec::new(),
                Vec::new(),
                Vec::new(),
            )
            .is_err(),
            "a product rejects duplicate roots"
        );
        let duplicate_authority = fixture.specification.authorities()[0].clone();
        assert!(
            SourceProduct::new(
                fixture.specification.nodes().to_vec(),
                Vec::new(),
                Vec::new(),
                vec![duplicate_authority.clone(), duplicate_authority],
                Vec::new(),
                Vec::new(),
            )
            .is_err(),
            "a product rejects duplicate authorities"
        );
        let duplicate_attachment = fixture.implementation.attachments()[0];
        assert!(
            SourceProduct::new(
                fixture.implementation.nodes().to_vec(),
                Vec::new(),
                Vec::new(),
                Vec::new(),
                vec![duplicate_attachment, duplicate_attachment],
                Vec::new(),
            )
            .is_err(),
            "a product rejects duplicate attachments"
        );
        let duplicate_binding = fixture.store.bindings()[0].clone();
        assert!(
            SourceProduct::new(
                Vec::new(),
                Vec::new(),
                Vec::new(),
                Vec::new(),
                Vec::new(),
                vec![duplicate_binding.clone(), duplicate_binding],
            )
            .is_err(),
            "a product rejects duplicate stored bindings"
        );
    }

    fn store_config() -> String {
        format!(
            r#"schema = "spec-provenance-config/v2"
digest_algorithms = ["{}"]

[[selectors]]
root = "buoy/objects"
adapter = "binding/v1"
include = ["**/*.toml"]
exclude = []
authority_expectation = "forbidden"
"#,
            active().algorithm().canonical_name()
        )
    }

    #[test]
    fn positive_fixture_resolves_the_complete_binding_without_diagnostics() {
        let root = std::path::Path::new("fixtures/content-addressed-bindings/v1/positive");
        let config = buoy::repository::load_config(&root.join("buoy.toml"))
            .expect("positive fixture configuration");
        let snapshot = buoy::repository::snapshot_repository(root, &config)
            .expect("positive fixture snapshot");
        let result = check_snapshot(&snapshot, &config).expect("positive fixture check");
        assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
        assert_eq!(result.source.attachments().len(), 1);
        assert_eq!(result.source.authorities().len(), 1);
        assert_eq!(result.source.store().len(), 1);
        assert_eq!(result.source.standing_roots().len(), 1);
        assert_eq!(result.source.fresh_bindings().len(), 1);
        assert_eq!(result.graph.implementation_bindings.len(), 1);
    }

    fn copy_positive_fixture(prefix: &str) -> tempfile::TempDir {
        let source = std::path::Path::new("fixtures/content-addressed-bindings/v1/positive");
        let target = tempfile::Builder::new()
            .prefix(prefix)
            .tempdir()
            .expect("binding command root");
        for path in [
            "buoy.toml",
            "openspec/specs/demo/spec.md",
            "src/lib.rs",
            "buoy/objects/v1/blake3-256/0e857e33e387325fbf457e426bc42286d9d99926b19f3efa3c203795cfeeb6c4.toml",
        ] {
            let destination = target.path().join(path);
            std::fs::create_dir_all(destination.parent().expect("fixture parent"))
                .expect("fixture directory");
            std::fs::copy(source.join(path), destination).expect("copied binding fixture");
        }
        target
    }

    fn run_bind(
        root: &std::path::Path,
        requirement: &str,
        implementation: &str,
    ) -> std::process::Output {
        run_bind_format(root, requirement, implementation, "machine")
    }

    fn run_bind_format(
        root: &std::path::Path,
        requirement: &str,
        implementation: &str,
        format: &str,
    ) -> std::process::Output {
        Command::new(env!("CARGO_BIN_EXE_buoy"))
            .args([
                "bind",
                "--root",
                root.to_str().expect("UTF-8 root"),
                "--config",
                root.join("buoy.toml").to_str().expect("UTF-8 config"),
                "--requirement",
                requirement,
                "--implementation",
                implementation,
                "--relation",
                "BGLC-RENDER-CATALOG-V2",
                "--format",
                format,
            ])
            .output()
            .expect("binding command")
    }

    /// One fresh relation must read the same facts through both reports.
    ///
    /// The successful-bind report and the trace projection share one internal
    /// fresh-link builder, so this control fails whenever one view drifts.
    #[test]
    fn binding_tree_report_and_fresh_trace_row_agree_on_one_relation() {
        let root = copy_positive_fixture("buoy-trace-parity-");
        let config = buoy::repository::load_config(&root.path().join("buoy.toml"))
            .expect("binding configuration");
        let snapshot =
            buoy::repository::snapshot_repository(root.path(), &config).expect("binding snapshot");
        let checked = check_snapshot(&snapshot, &config).expect("binding source graph");
        let old = checked.source.fresh_bindings()[0].clone();
        let bind = run_bind(
            root.path(),
            &old.requirement().canonical_text(),
            &old.subject().canonical_text(),
        );
        assert!(
            bind.status.success(),
            "{}",
            String::from_utf8_lossy(&bind.stderr)
        );
        let bind_report: Value =
            serde_json::from_slice(&bind.stdout).expect("machine traversal report");
        assert_eq!(bind_report["schema"], "buoy/binding-tree-report/v1");

        let binding = bind_report["binding"].as_str().expect("binding digest");
        let trace = Command::new(env!("CARGO_BIN_EXE_buoy"))
            .args([
                "trace",
                "--root",
                root.path().to_str().expect("UTF-8 root"),
                "--config",
                root.path()
                    .join("buoy.toml")
                    .to_str()
                    .expect("UTF-8 config"),
                "--record",
                binding,
                "--format",
                "machine",
            ])
            .output()
            .expect("trace command");
        assert_eq!(
            trace.status.code(),
            Some(0),
            "{}",
            String::from_utf8_lossy(&trace.stderr)
        );
        let trace_report: Value =
            serde_json::from_slice(&trace.stdout).expect("machine trace report");
        assert_eq!(trace_report["schema"], "buoy/trace-report/v1");
        let rows = trace_report["rows"].as_array().expect("trace rows");
        assert_eq!(rows.len(), 1, "one exact record selects one row");
        let row = &rows[0];
        assert_eq!(row["state"], "current");

        for (bind_field, trace_field) in [
            ("relation", "relation"),
            ("binding", "record"),
            ("requirement_id", "requirement_id"),
            ("atom_revision", "atom_revision"),
            ("implementation_path", "implementation_path"),
            ("requirement_path", "requirement_path"),
        ] {
            assert_eq!(
                bind_report[bind_field], row[trace_field],
                "the two reports disagree about {bind_field}"
            );
        }
        let bind_nodes = bind_report["nodes"].as_array().expect("bind nodes");
        let trace_nodes = row["traversal"].as_array().expect("trace nodes");
        assert_eq!(bind_nodes.len(), trace_nodes.len());
        for (bind_node, trace_node) in bind_nodes.iter().zip(trace_nodes) {
            for field in ["position", "digest", "kind", "path"] {
                assert_eq!(
                    bind_node[field], trace_node[field],
                    "the two reports disagree about traversal field {field}"
                );
            }
            assert!(
                trace_node.get("semantic").is_none(),
                "trace reports must not copy complete source bytes"
            );
        }
        assert!(
            row["freshness"]
                .as_object()
                .expect("trace freshness facts")
                .values()
                .all(|value| value == true),
            "a fresh trace row reports every freshness fact"
        );
        // The frozen bind schema keeps its four published freshness facts.
        assert_eq!(
            bind_report["freshness"]
                .as_object()
                .expect("bind freshness facts")
                .len(),
            4
        );
    }

    #[test]
    fn bind_command_uses_explicit_selectors_and_creates_one_immutable_update() {
        let root = copy_positive_fixture("buoy-bind-positive-");
        let config = buoy::repository::load_config(&root.path().join("buoy.toml"))
            .expect("binding configuration");
        let snapshot =
            buoy::repository::snapshot_repository(root.path(), &config).expect("binding snapshot");
        let checked = check_snapshot(&snapshot, &config).expect("binding source graph");
        let old = checked.source.fresh_bindings()[0].clone();
        let output = run_bind(
            root.path(),
            &old.requirement().canonical_text(),
            &old.subject().canonical_text(),
        );
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let report: Value =
            serde_json::from_slice(&output.stdout).expect("machine traversal report");
        assert_eq!(report["schema"], "buoy/binding-tree-report/v1");
        assert_eq!(report["relation"], "BGLC-RENDER-CATALOG-V2");
        assert_eq!(report["requirement_id"], "BGLC-003");
        assert_eq!(report["implementation_path"], RUST_PATH);
        assert_eq!(report["requirement_path"], SPEC_PATH);
        assert!(
            report["nodes"]
                .as_array()
                .is_some_and(|nodes| nodes.len() == 10)
        );
        assert!(
            report["nodes"]
                .as_array()
                .expect("traversal nodes")
                .iter()
                .any(|node| node["kind"] == "openspec/scenario/v1")
        );
        assert!(
            report["freshness"]
                .as_object()
                .expect("freshness facts")
                .values()
                .all(|value| value == true)
        );

        let new_binding = report["binding"].as_str().expect("binding digest");
        let new_path = root.path().join(binding_object_path(
            ImplementationBindingRef::parse(new_binding).expect("binding reference"),
        ));
        assert!(new_path.is_file());
        assert!(
            root.path()
                .join(binding_object_path(old.record()))
                .is_file()
        );
        let rust =
            std::fs::read_to_string(root.path().join(RUST_PATH)).expect("updated Rust source");
        assert!(rust.contains(new_binding));
        assert!(!rust.contains(&old.record().canonical_text()));
        let updated =
            buoy::repository::snapshot_repository(root.path(), &config).expect("updated snapshot");
        let checked = check_snapshot(&updated, &config).expect("updated binding graph");
        assert!(checked.diagnostics.is_empty(), "{:?}", checked.diagnostics);
        assert_eq!(checked.source.fresh_bindings().len(), 1);
        assert_eq!(checked.graph.implementation_bindings.len(), 1);
    }

    #[test]
    fn bind_command_human_output_shows_navigation_and_freshness_facts() {
        let root = copy_positive_fixture("buoy-bind-human-");
        let config = buoy::repository::load_config(&root.path().join("buoy.toml"))
            .expect("binding configuration");
        let snapshot =
            buoy::repository::snapshot_repository(root.path(), &config).expect("binding snapshot");
        let checked = check_snapshot(&snapshot, &config).expect("binding source graph");
        let old = &checked.source.fresh_bindings()[0];
        let output = run_bind_format(
            root.path(),
            &old.requirement().canonical_text(),
            &old.subject().canonical_text(),
            "human",
        );
        assert!(output.status.success());
        let human = String::from_utf8(output.stdout).expect("UTF-8 human report");
        for fact in [
            "relation: BGLC-RENDER-CATALOG-V2",
            "requirement: BGLC-003",
            "implementation path: src/lib.rs",
            "requirement path: openspec/specs/demo/spec.md",
            "freshness: record=true implementation=true requirement=true authority=true",
            "openspec/scenario/v1",
        ] {
            assert!(human.contains(fact), "{fact}: {human}");
        }
    }

    #[test]
    fn bind_command_errors_leave_source_and_store_unchanged() {
        let root = copy_positive_fixture("buoy-bind-negative-");
        let before_source =
            std::fs::read(root.path().join(RUST_PATH)).expect("source before failure");
        let before_store = std::fs::read_dir(root.path().join("buoy/objects/v1/blake3-256"))
            .expect("store before failure")
            .count();
        let output = run_bind(root.path(), "not-a-digest", "also-not-a-digest");
        assert!(!output.status.success());
        let config = buoy::repository::load_config(&root.path().join("buoy.toml"))
            .expect("binding configuration");
        let snapshot =
            buoy::repository::snapshot_repository(root.path(), &config).expect("binding snapshot");
        let checked = check_snapshot(&snapshot, &config).expect("binding graph");
        let old = &checked.source.fresh_bindings()[0];
        let stale = SemanticDigest::parse_canonical(&format!(
            "{}:{}",
            active().algorithm().canonical_name(),
            "0".repeat(active().algorithm().output_length() * 2)
        ))
        .expect("same-member stale node")
        .canonical_text();
        for (requirement, implementation, format, exit) in [
            (
                old.requirement().canonical_text(),
                stale.clone(),
                String::from("machine"),
                70,
            ),
            (
                old.subject().canonical_text(),
                old.subject().canonical_text(),
                String::from("machine"),
                70,
            ),
            (
                old.requirement().canonical_text(),
                old.subject().canonical_text(),
                String::from("invalid"),
                64,
            ),
        ] {
            assert_eq!(
                run_bind_format(root.path(), &requirement, &implementation, &format)
                    .status
                    .code(),
                Some(exit),
                "requirement={requirement} implementation={implementation} format={format}"
            );
        }
        assert_eq!(
            std::fs::read(root.path().join(RUST_PATH)).expect("source after failure"),
            before_source
        );
        assert_eq!(
            std::fs::read_dir(root.path().join("buoy/objects/v1/blake3-256"))
                .expect("store after failure")
                .count(),
            before_store
        );
        let root_text = root.path().to_str().expect("UTF-8 root");
        let config_text = root
            .path()
            .join("buoy.toml")
            .to_str()
            .expect("UTF-8 config")
            .to_owned();
        for arguments in [
            vec!["bind"],
            vec!["bind", "--root", root_text],
            vec![
                "bind",
                "--root",
                root_text,
                "--config",
                config_text.as_str(),
            ],
        ] {
            let missing_selector = Command::new(env!("CARGO_BIN_EXE_buoy"))
                .args(arguments)
                .output()
                .expect("missing-selector command");
            assert_eq!(missing_selector.status.code(), Some(64));
        }
        let missing_config = root.path().join("missing.toml");
        let missing_config_output = Command::new(env!("CARGO_BIN_EXE_buoy"))
            .args([
                "bind",
                "--root",
                root_text,
                "--config",
                missing_config.to_str().expect("UTF-8 missing config"),
                "--requirement",
                &old.requirement().canonical_text(),
                "--implementation",
                &old.subject().canonical_text(),
                "--relation",
                "BGLC-RENDER-CATALOG-V2",
            ])
            .output()
            .expect("missing-config command");
        assert_eq!(missing_config_output.status.code(), Some(70));
    }

    fn materialize_repository(prefix: &str, record: &str, source: &str) -> tempfile::TempDir {
        let directory = tempfile::Builder::new()
            .prefix(prefix)
            .tempdir()
            .expect("separate repository root");
        for (relative, contents) in [("buoy.toml", store_config().as_str()), (record, source)] {
            let absolute = directory.path().join(relative);
            std::fs::create_dir_all(absolute.parent().expect("repository file has a parent"))
                .expect("materialized repository directory");
            std::fs::write(&absolute, contents).expect("materialized repository file");
        }
        directory
    }

    fn root_text(path: &std::path::Path) -> String {
        path.to_str().expect("UTF-8 repository root").to_owned()
    }

    #[test]
    fn static_change_and_execute_paths_keep_the_retained_source_context() {
        let fixture = fixture();
        let (record_path, source) = store_input_bytes(&fixture.record);
        let tampered = source.replace("DEMO-RENDER-CATALOG", "DEMO-RENDER-CATALOG-TAMPERED");

        for (prefix, contents, expected_codes) in [
            ("buoy-store-current-", source.as_str(), Vec::new()),
            (
                "buoy-store-tampered-",
                tampered.as_str(),
                vec![String::from("CAB-OBJECT-PATH-DIGEST")],
            ),
        ] {
            let root = materialize_repository(prefix, &record_path, contents);
            let config = buoy::repository::load_config(&root.path().join("buoy.toml"))
                .expect("collision-resistant store configuration");
            let snapshot = buoy::repository::snapshot_repository(root.path(), &config)
                .expect("materialized object-store snapshot");
            let result =
                buoy::check::check_snapshot(&snapshot, &config).expect("shared snapshot check");
            assert_eq!(
                result
                    .diagnostics
                    .iter()
                    .map(|diagnostic| diagnostic.code.clone())
                    .collect::<Vec<_>>(),
                expected_codes
            );
            assert_eq!(
                result.source.store().len(),
                usize::from(expected_codes.is_empty())
            );

            let manifest = root.path().join("manifest.toml");
            std::fs::write(
                &manifest,
                format!(
                    "schema = \"spec-provenance-change-manifest/v1\"\nid = \"CHANGE-STORE\"\nbase_graph = \"{0}\"\nhead_graph = \"{0}\"\ndispositions = []\n",
                    result.graph_digest
                ),
            )
            .expect("change manifest");
            let selected_root = root_text(root.path());
            let config_text = root_text(&root.path().join("buoy.toml"));
            let manifest_text = root_text(&manifest);
            let expected_exit = if expected_codes.is_empty() { 0 } else { 2 };
            for command in [
                vec![
                    String::from("static"),
                    String::from("--root"),
                    selected_root.clone(),
                    String::from("--config"),
                    config_text.clone(),
                ],
                vec![
                    String::from("execute"),
                    String::from("--root"),
                    selected_root.clone(),
                    String::from("--config"),
                    config_text.clone(),
                ],
            ] {
                assert_eq!(
                    buoy::cli::run(&command),
                    expected_exit,
                    "{command:?} must report the retained source context"
                );
            }
            assert_eq!(
                buoy::cli::run(&[
                    String::from("change"),
                    String::from("--base"),
                    selected_root.clone(),
                    String::from("--head"),
                    selected_root,
                    String::from("--config"),
                    config_text,
                    String::from("--manifest"),
                    manifest_text,
                ]),
                if expected_codes.is_empty() { 0 } else { 3 },
                "the change path must report the retained source context"
            );
        }
    }

    #[test]
    fn unresolved_typed_references_survive_per_file_materialization() {
        let fixture = fixture();
        assert_eq!(fixture.implementation.attachments().len(), 1);
        assert_eq!(
            fixture.implementation.attachments()[0].record(),
            fixture.record.record()
        );
        let alone = aggregate_source_products(
            active(),
            std::slice::from_ref(&fixture.implementation),
            budgets(),
        )
        .expect("one adapter product aggregates alone");
        assert_eq!(
            alone
                .binding_diagnostics()
                .iter()
                .map(|diagnostic| diagnostic.code.clone())
                .collect::<Vec<_>>(),
            vec![String::from("CAB-RECORD-ABSENT")],
            "one adapter never reports closure for another adapter's target"
        );
        assert_eq!(
            SemanticDigest::parse_canonical(&fixture.record.record().canonical_text()),
            Ok(fixture.record.record().digest())
        );
    }
}

/// The shared check path validates the graph against the exact retained
/// repository context rather than against an empty snapshot.
///
/// These contracts need no collision-resistant member, so they run in every
/// build, including the default one.
mod graph_context {
    use buoy::core::ActiveAlgorithm;

    /// One repository whose only Rust item nests inside an inline module.
    ///
    /// The selected adapter retains the parsed item that carries the attribute
    /// and names its enclosing module item, but retains no identity for that
    /// module. The retained parsed-item snapshot is therefore not closed under
    /// enclosure, which only the explicit graph-validation context can report.
    const NESTED_SOURCE_PARTS: [&str; 2] = [
        r"mod inner {
    #[spec_",
        r##"provenance::implementation(
        syntax = "v1",
        body = r#"
id = "IMPL-CONTEXT-001"
atoms = [{ id = "ATOM-CONTEXT-001", revision = "REVISION" }]
navigation = "implements"
"#
    )]
    pub fn implemented() {}
}
"##,
    ];

    /// The same item, and the same semantic object, without one enclosing item.
    const FLAT_SOURCE_PARTS: [&str; 2] = [
        r"#[spec_",
        r##"provenance::implementation(
    syntax = "v1",
    body = r#"
id = "IMPL-CONTEXT-001"
atoms = [{ id = "ATOM-CONTEXT-001", revision = "REVISION" }]
navigation = "implements"
"#
)]
pub fn implemented() {}
"##,
    ];

    fn revision_text() -> String {
        let member = ActiveAlgorithm::build_default()
            .expect("a digest-producing build")
            .algorithm();
        format!(
            "{}:{}",
            member.canonical_name(),
            "1".repeat(member.output_length() * 2)
        )
    }

    fn config_text() -> String {
        format!(
            r#"schema = "spec-provenance-config/v2"
digest_algorithms = ["{}"]

[[selectors]]
root = "src"
adapter = "rust/v1"
include = ["**/*.rs"]
exclude = []
authority_expectation = "forbidden"
"#,
            ActiveAlgorithm::build_default()
                .expect("a digest-producing build")
                .algorithm()
                .canonical_name()
        )
    }

    fn diagnostic_codes(prefix: &str, source: &str) -> Vec<String> {
        let directory = tempfile::Builder::new()
            .prefix(prefix)
            .tempdir()
            .expect("separate repository root");
        std::fs::create_dir_all(directory.path().join("src")).expect("selected source directory");
        std::fs::write(directory.path().join("buoy.toml"), config_text())
            .expect("repository configuration");
        std::fs::write(
            directory.path().join("src/lib.rs"),
            source.replace("REVISION", &revision_text()),
        )
        .expect("selected Rust source");

        let config = buoy::repository::load_config(&directory.path().join("buoy.toml"))
            .expect("repository configuration");
        let snapshot = buoy::repository::snapshot_repository(directory.path(), &config)
            .expect("materialized snapshot");
        let result =
            buoy::check::check_snapshot(&snapshot, &config).expect("shared snapshot check");
        let codes = result
            .diagnostics
            .iter()
            .map(|diagnostic| diagnostic.code.clone())
            .collect::<Vec<_>>();
        assert_eq!(
            buoy::cli::run(&[
                String::from("static"),
                String::from("--root"),
                directory
                    .path()
                    .to_str()
                    .expect("UTF-8 repository root")
                    .to_owned(),
                String::from("--config"),
                directory
                    .path()
                    .join("buoy.toml")
                    .to_str()
                    .expect("UTF-8 configuration path")
                    .to_owned(),
            ]),
            2,
            "the static path reports the same retained context"
        );
        codes
    }

    #[test]
    fn the_shared_check_path_reports_one_context_only_diagnostic() {
        let nested_source = NESTED_SOURCE_PARTS.concat();
        let flat_source = FLAT_SOURCE_PARTS.concat();
        let nested = diagnostic_codes("buoy-context-nested-", &nested_source);
        let flat = diagnostic_codes("buoy-context-flat-", &flat_source);
        assert!(
            nested.contains(&String::from("SCF-SOURCE-MAP-ENCLOSING")),
            "{nested:?}"
        );
        assert!(
            !flat.contains(&String::from("SCF-SOURCE-MAP-ENCLOSING")),
            "{flat:?}"
        );
        assert_eq!(
            nested
                .iter()
                .filter(|code| code.as_str() != "SCF-SOURCE-MAP-ENCLOSING")
                .collect::<Vec<_>>(),
            flat.iter().collect::<Vec<_>>(),
            "enclosure is the one fact that separates the two repositories"
        );
    }
}

/// Returns the collision-resistant member a build with one such adapter selects.
#[cfg(any(feature = "blake3-256", feature = "sha-256"))]
const fn collision_resistant_member() -> DigestAlgorithm {
    if cfg!(feature = "blake3-256") {
        DigestAlgorithm::Blake3_256
    } else {
        DigestAlgorithm::Sha256
    }
}

/// Returns a second available collision-resistant member, when the build has one.
#[cfg(any(feature = "blake3-256", feature = "sha-256"))]
const fn other_collision_resistant_member(selected: DigestAlgorithm) -> Option<DigestAlgorithm> {
    match selected {
        DigestAlgorithm::Blake3_256 if cfg!(feature = "sha-256") => Some(DigestAlgorithm::Sha256),
        DigestAlgorithm::Sha256 if cfg!(feature = "blake3-256") => {
            Some(DigestAlgorithm::Blake3_256)
        }
        _ => None,
    }
}

fn configuration(algorithm: &str, adapter: &str) -> Result<RepositoryConfig, String> {
    RepositoryConfig::parse(
        format!(
            r#"schema = "spec-provenance-config/v2"
digest_algorithms = ["{algorithm}"]

[[selectors]]
root = "buoy"
adapter = "{adapter}"
include = ["**/*.toml"]
exclude = []
authority_expectation = "forbidden"
"#
        )
        .as_bytes(),
    )
    .map_err(|error| error.code().to_owned())
}

#[cfg(feature = "xxh3-128")]
#[test]
fn digest_class_configuration_rejects_change_detection_members() {
    assert_eq!(
        configuration("xxh3-128", "binding/v1").map(|_| ()),
        Err(String::from("CAB-DIGEST-CLASS")),
        "a compact locator requires a collision-resistant member"
    );
    assert!(
        configuration("xxh3-128", "markdown/v1").is_ok(),
        "the class rule applies only to compact binding selection"
    );
    let active = ActiveAlgorithm::new(DigestAlgorithm::Xxh3_128)
        .expect("this build contains the xxh3-128 adapter");
    let path = buoy::core::RepositoryPath::new("src/lib.rs").expect("Rust path");
    let output = buoy::adapters::extract_rust_with(
        active,
        &path,
        b"#[buoy::implements(\"xxh3-128:11111111111111111111111111111111\")]\nfn subject() {}\n",
    );
    assert_eq!(output.diagnostics[0].code, "CAB-DIGEST-CLASS");
    assert!(output.bindings.is_empty());
}

/// Every available collision-resistant member is selectable, in every build.
///
/// The repository selection, not the first member this build contains, decides
/// which member computes binding record identities.
#[cfg(any(feature = "blake3-256", feature = "sha-256"))]
#[test]
fn digest_class_configuration_accepts_every_available_collision_resistant_member() {
    for member in [DigestAlgorithm::Blake3_256, DigestAlgorithm::Sha256] {
        if ActiveAlgorithm::new(member).is_err() {
            continue;
        }
        assert!(
            configuration(member.canonical_name(), "binding/v1").is_ok(),
            "{} must be selectable for compact bindings",
            member.canonical_name()
        );
    }
}

/// The repository never selects a member this build cannot compute.
#[cfg(not(feature = "sha-256"))]
#[test]
fn digest_class_configuration_rejects_an_unavailable_member() {
    assert_eq!(
        configuration("sha-256", "binding/v1").map(|_| ()),
        Err(String::from("DIG-ALGORITHM-UNAVAILABLE"))
    );
}
