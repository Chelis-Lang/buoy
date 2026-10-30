#![doc = "Mutation and parity contracts for total Markdown and Rust adapters."]
#![allow(clippy::expect_used, clippy::too_many_lines)]

use buoy::adapter_sdk::{
    ADAPTER_SDK_SCHEMA_ID, AdapterConfiguration, AdapterDescriptor, AdapterProduct,
    AdapterRegistry, AdapterSdkError, DeclaredSourceRoot, ExternalMaterializedSource,
    MaterializedSource, MaterializedSourceNode, MaterializedTraceLink, NonnormativeDisposition,
    SavedSnapshotInput, ShellAdapter, SourceProduct, SourceRootClass, built_in_configuration,
    built_in_registry, validate_parsed_item_protocol,
};
#[cfg(feature = "coverage-controls")]
use buoy::adapters::openspec_partition_control;
use buoy::adapters::{
    ADAPTER_SUPPORT_MATRIX, AdapterDiagnostic, AdapterSyntaxClass, AuthorityExpectation,
    MarkdownSemantic, OpenSpecConfiguration, OpenSpecOutput, RustSemantic, SourceSpan,
    SyntaxDispositionKind, extract_markdown, extract_openspec_configured, extract_rust,
};
use buoy::core::{
    ApprovalBinding, BindingAttachment, CanonicalStaticReport, CanonicalValue, EvidenceRole,
    ImplementationBindingRef, Oracle, OracleKind, ParsedItemIdentity, RepositoryPath, SchemaId,
    ScopeId, SemanticDigest, SourceItemKind, SourceNode, SpecGraph, ToolIdentity, TypedPayload,
    carrier_fresh, semantic_graph_digest,
};
use std::collections::BTreeSet;

fn extract_openspec(path: &RepositoryPath, bytes: &[u8]) -> OpenSpecOutput {
    let configuration = OpenSpecConfiguration::parse(
        b"schema = \"buoy.openspec-authority/v1\"\n\n[[capabilities]]\nname = \"demo\"\nformat = \"openspec/standing-spec/v1\"\nscope = \"openspec.demo\"\natom_kind = \"behavioral\"\nlifecycle = \"proposed\"\nreserved_ids = [\"DEMO-001\", \"DEMO-002\"]\n",
    )
    .expect("explicit OpenSpec test configuration");
    extract_openspec_configured(path, bytes, &configuration)
}
use std::fs;
use std::path::Path as HostPath;
use std::process::Command;

fn sid(value: &str) -> buoy::core::StableId {
    buoy::core::StableId::new(value).expect("valid stable ID")
}

fn path(value: &str) -> Option<RepositoryPath> {
    RepositoryPath::new(value).ok()
}

fn test_payload(schema: impl Into<String>, bytes: Vec<u8>) -> TypedPayload {
    let schema = schema.into();
    TypedPayload::new(SchemaId::new(&schema).expect("valid schema"), bytes)
        .expect("valid adapter test typed payload")
}

fn test_tool(
    id: impl Into<String>,
    version: impl Into<String>,
    executable: SemanticDigest,
    configuration: SemanticDigest,
) -> ToolIdentity {
    let id = id.into();
    ToolIdentity::new(
        ScopeId::new(&id).expect("valid scope"),
        version,
        executable,
        configuration,
    )
    .expect("valid adapter test tool identity")
}

const CONTROL_CONFIGURATION_SCHEMA_ID: &str = "host.configuration/v1";
const CONTROL_DECLARED_CAPABILITY: &str = "host.saved-syntax/v1";
const CONTROL_UNSUPPORTED_CAPABILITY: &str = "host.unsupported-syntax/v1";

struct ControlAdapter {
    descriptor: AdapterDescriptor,
    output_capability: Option<&'static str>,
}

impl ControlAdapter {
    fn new(adapter_id: &str, output_capability: Option<&'static str>) -> Self {
        Self {
            descriptor: AdapterDescriptor::new(
                adapter_id,
                "control-parser",
                "1.0.0",
                "host.syntax/v1",
                CONTROL_CONFIGURATION_SCHEMA_ID,
                vec![CONTROL_DECLARED_CAPABILITY.to_owned()],
                vec![SourceItemKind::new("host/item/v1").expect("control source item kind")],
            )
            .expect("valid control adapter descriptor"),
            output_capability,
        }
    }
}

impl ShellAdapter for ControlAdapter {
    fn descriptor(&self) -> &AdapterDescriptor {
        &self.descriptor
    }

    fn materialize(
        &self,
        input: &SavedSnapshotInput<'_>,
    ) -> Result<AdapterProduct, AdapterSdkError> {
        let nonnormative = if let Some(capability) = self.output_capability {
            vec![NonnormativeDisposition::new(
                SourceSpan {
                    path: input.path().clone(),
                    start: 0,
                    end: input.bytes().len() as u64,
                },
                capability,
            )?]
        } else {
            Vec::new()
        };
        AdapterProduct::new(SpecGraph::default(), Vec::new(), nonnormative, Vec::new())
    }
}

#[derive(Clone, Copy)]
enum ArtifactProductControl {
    Complete,
    UndeclaredKind,
    PartialSpan,
    UnsupportedCapability,
}

struct ArtifactControlAdapter {
    descriptor: AdapterDescriptor,
    control: ArtifactProductControl,
}

impl ArtifactControlAdapter {
    fn new(control: ArtifactProductControl) -> Self {
        Self {
            descriptor: AdapterDescriptor::new(
                "artifact/v1",
                "control-parser",
                "1.0.0",
                "artifact.opaque-bytes/v1",
                "artifact.control/v1",
                vec![String::from("artifact.opaque-bytes/v1")],
                vec![SourceItemKind::new("artifact/file/v1").expect("artifact control kind")],
            )
            .expect("artifact control descriptor"),
            control,
        }
    }
}

impl ShellAdapter for ArtifactControlAdapter {
    fn descriptor(&self) -> &AdapterDescriptor {
        &self.descriptor
    }

    fn materialize(
        &self,
        input: &SavedSnapshotInput<'_>,
    ) -> Result<AdapterProduct, AdapterSdkError> {
        let kind = if matches!(self.control, ArtifactProductControl::UndeclaredKind) {
            "host/item/v1"
        } else {
            "artifact/file/v1"
        };
        let semantic_bytes = CanonicalValue::Record(vec![
            (
                1,
                CanonicalValue::Text(String::from("application/octet-stream")),
            ),
            (2, CanonicalValue::Text(String::from("saved-bytes/v1"))),
            (3, CanonicalValue::Bytes(input.bytes().to_vec())),
        ])
        .encode()
        .expect("artifact control semantic bytes");
        let item = ParsedItemIdentity::new_with(
            input.active_algorithm(),
            SchemaId::new("artifact.opaque-bytes/v1").expect("artifact control schema"),
            input.path().clone(),
            SourceItemKind::new(kind).expect("artifact control item kind"),
            semantic_bytes,
            None,
        )
        .expect("artifact control parsed item");
        let node = SourceNode::new_with(input.active_algorithm(), item, Vec::new())
            .expect("artifact control source node");
        let reference = node.reference();
        let end = if matches!(self.control, ArtifactProductControl::PartialSpan) {
            input.bytes().len().saturating_sub(1) as u64
        } else {
            input.bytes().len() as u64
        };
        let capability = if matches!(self.control, ArtifactProductControl::UnsupportedCapability) {
            "artifact.unsupported/v1"
        } else {
            "artifact.opaque-bytes/v1"
        };
        let source = SourceProduct::new(
            vec![MaterializedSourceNode::new(
                node,
                SourceSpan {
                    path: input.path().clone(),
                    start: 0,
                    end,
                },
                capability,
            )?],
            Vec::new(),
            vec![DeclaredSourceRoot::new(
                reference,
                SourceRootClass::Navigational,
            )],
            Vec::new(),
            Vec::new(),
            Vec::new(),
        )?;
        AdapterProduct::new_with_source_product(
            SpecGraph::default(),
            Vec::new(),
            source,
            Vec::new(),
            Vec::new(),
            Vec::new(),
        )
    }
}

fn control_configuration(adapter_id: &str) -> AdapterConfiguration {
    AdapterConfiguration::new(
        adapter_id,
        CONTROL_CONFIGURATION_SCHEMA_ID,
        b"explicit-control".to_vec(),
    )
    .expect("valid control adapter configuration")
}

fn write_root_file(root: &HostPath, relative: &str, bytes: &[u8]) -> Vec<u8> {
    let location = root.join(relative);
    if let Some(parent) = location.parent() {
        fs::create_dir_all(parent).expect("create control fixture parent");
    }
    fs::write(&location, bytes).expect("write control fixture");
    fs::read(location).expect("read control fixture")
}

#[test]
fn shell_adapter_sdk_binds_descriptors_configuration_inputs_and_complete_outputs() {
    assert_eq!(ADAPTER_SDK_SCHEMA_ID, "buoy.adapter-sdk/v3");
    let registry = built_in_registry().expect("built-in adapter registry");
    let descriptors = registry.descriptors();
    assert_eq!(
        descriptors
            .iter()
            .map(|descriptor| descriptor.adapter_id())
            .collect::<Vec<_>>(),
        vec![
            "artifact/v1",
            "binding/v1",
            "markdown/v1",
            "openspec/v1",
            "rust/v1",
        ]
    );
    assert_eq!(descriptors[0].parser_id(), "opaque-bytes");
    assert_eq!(descriptors[1].parser_id(), "toml");
    assert_eq!(descriptors[2].parser_id(), "pulldown-cmark");
    assert_eq!(descriptors[2].syntax_schema_id(), "markdown/v1");
    assert!(descriptors[2].supports("markdown.fenced-object/v1"));
    assert_eq!(descriptors[3].parser_id(), "pulldown-cmark");
    assert_eq!(
        descriptors[3]
            .source_item_kinds()
            .iter()
            .map(SourceItemKind::as_str)
            .collect::<Vec<_>>(),
        vec![
            "openspec/document/v1",
            "openspec/external-trace/v1",
            "openspec/requirement/v1",
            "openspec/scenario/v1",
            "openspec/step/v1",
        ]
    );
    assert_eq!(descriptors[4].parser_id(), "syn");

    let path = path("spec/positive.md").expect("SDK input path");
    let bytes = include_bytes!("../fixtures/adapters/markdown/positive.md");
    let configuration = built_in_configuration("markdown/v1", AuthorityExpectation::Required)
        .expect("bound Markdown configuration");
    let input = SavedSnapshotInput::new(&path, bytes, &configuration)
        .expect("canonical saved snapshot input");
    assert_eq!(input.path(), &path);
    assert_eq!(input.bytes(), bytes);
    assert_eq!(input.configuration(), &configuration);
    assert_eq!(
        input.active_algorithm().algorithm(),
        input.digest().algorithm()
    );
    let explicit =
        SavedSnapshotInput::new_with(input.active_algorithm(), &path, bytes, &configuration)
            .expect("explicit-member saved snapshot input");
    assert_eq!(explicit, input);
    assert_eq!(
        SourceRootClass::StandingSpecification.as_str(),
        "standing-specification"
    );
    assert_eq!(SourceRootClass::Navigational.as_str(), "navigational");
    let first = registry
        .materialize(&input)
        .expect("complete Markdown materialization");
    let second = registry
        .materialize(&input)
        .expect("repeat Markdown materialization");
    assert_eq!(first, second);
    assert_eq!(first.configuration_digest(), configuration.digest());
    assert_eq!(first.input_digest(), input.digest());
    assert_eq!(first.product().graph().atoms.len(), 1);
    assert_eq!(first.product().sources().len(), 1);
    assert!(!first.product().nonnormative().is_empty());
    assert!(first.product().diagnostics().is_empty());
    assert_eq!(
        first.product().sources()[0].span().path.as_str(),
        "spec/positive.md"
    );
    assert_eq!(
        first.product().sources()[0].item().language_schema_id(),
        "markdown/v1"
    );
    assert_eq!(
        first.product().sources()[0].item().kind().as_str(),
        "markdown/fence/v1"
    );
    let source = &first.product().sources()[0];
    assert!(
        MaterializedSource::new(
            source.object().clone(),
            source.item().clone(),
            source.span().clone(),
            source.syntax_capability(),
        )
        .is_ok()
    );
    assert_eq!(
        MaterializedSource::new(
            source.object().clone(),
            source.item().clone(),
            source.span().clone(),
            "invalid",
        ),
        Err(AdapterSdkError::Capability)
    );

    assert_eq!(
        AdapterDescriptor::new(
            "duplicate-kind/v1",
            "control-parser",
            "1.0.0",
            "control.syntax/v1",
            "control.configuration/v1",
            vec![String::from("control.syntax/v1")],
            vec![
                SourceItemKind::new("control/item/v1").expect("control item kind"),
                SourceItemKind::new("control/item/v1").expect("control item kind"),
            ],
        ),
        Err(AdapterSdkError::UndeclaredSourceItemKind)
    );

    let openspec_path = RepositoryPath::new("openspec/specs/demo/spec.md").expect("OpenSpec path");
    let openspec_configuration = AdapterConfiguration::new(
        "openspec/v1",
        "buoy.adapter.configuration/v1",
        b"invalid".to_vec(),
    )
    .expect("structural OpenSpec configuration");
    let openspec_input = SavedSnapshotInput::new(
        &openspec_path,
        include_bytes!("../fixtures/openspec/v1/positive/standing-spec.md"),
        &openspec_configuration,
    )
    .expect("OpenSpec input");
    assert_eq!(
        registry.materialize(&openspec_input),
        Err(AdapterSdkError::Configuration)
    );
    let valid_openspec_configuration = AdapterConfiguration::new(
        "openspec/v1",
        "buoy.adapter.configuration/v1",
        include_bytes!("../fixtures/openspec/v1/positive/configuration.toml").to_vec(),
    )
    .expect("valid OpenSpec configuration");
    let valid_openspec_input = SavedSnapshotInput::new(
        &openspec_path,
        include_bytes!("../fixtures/openspec/v1/positive/standing-spec.md"),
        &valid_openspec_configuration,
    )
    .expect("valid OpenSpec input");
    let openspec_output = registry
        .materialize(&valid_openspec_input)
        .expect("OpenSpec materialization");
    let external = &openspec_output.product().external_sources()[0];
    assert!(
        ExternalMaterializedSource::new(
            external.object().clone(),
            external.item().clone(),
            external.span().clone(),
            external.syntax_capability(),
        )
        .is_ok()
    );
    assert_eq!(
        ExternalMaterializedSource::new(
            external.object().clone(),
            external.item().clone(),
            external.span().clone(),
            "invalid",
        ),
        Err(AdapterSdkError::Capability)
    );

    let trace_bytes = include_bytes!("../fixtures/openspec/v1/positive/external-trace.md");
    let trace_configuration = built_in_configuration("markdown/v1", AuthorityExpectation::Optional)
        .expect("trace configuration");
    let trace_input =
        SavedSnapshotInput::new(&path, trace_bytes, &trace_configuration).expect("trace input");
    let trace_output = registry
        .materialize(&trace_input)
        .expect("trace materialization");
    let trace = &trace_output.product().trace_links()[0];
    assert!(
        MaterializedTraceLink::new(
            trace.link().clone(),
            trace.item().clone(),
            trace.span().clone(),
            trace.syntax_capability(),
        )
        .is_ok()
    );
    assert_eq!(
        MaterializedTraceLink::new(
            trace.link().clone(),
            trace.item().clone(),
            trace.span().clone(),
            "invalid",
        ),
        Err(AdapterSdkError::Capability)
    );
}

#[test]
fn raw_artifact_registry() {
    let registry = built_in_registry().expect("built-in artifact registry");
    let descriptor = registry
        .descriptors()
        .into_iter()
        .find(|descriptor| descriptor.adapter_id() == "artifact/v1")
        .expect("artifact descriptor");
    assert_eq!(descriptor.syntax_schema_id(), "artifact.opaque-bytes/v1");
    assert_eq!(
        descriptor
            .source_item_kinds()
            .iter()
            .map(SourceItemKind::as_str)
            .collect::<Vec<_>>(),
        vec!["artifact/file/v1"]
    );
    assert_eq!(
        descriptor.syntax_capabilities(),
        [String::from("artifact.opaque-bytes/v1")]
    );

    let path = RepositoryPath::new("project/flake.nix").expect("artifact path");
    let bytes = [0xff, 0x00, 0xfe];
    let configuration = built_in_configuration("artifact/v1", AuthorityExpectation::Forbidden)
        .expect("artifact configuration");
    let input =
        SavedSnapshotInput::new(&path, &bytes, &configuration).expect("saved artifact input");
    let output = registry
        .materialize(&input)
        .expect("opaque artifact materialization");
    assert!(output.product().graph().object_meta().is_empty());
    assert!(output.product().sources().is_empty());
    assert_eq!(output.product().source().nodes().len(), 1);
    let retained = &output.product().source().nodes()[0];
    assert_eq!(retained.node().item().kind().as_str(), "artifact/file/v1");
    assert_eq!((retained.span().start, retained.span().end), (0, 3));
    assert!(output.product().source().authorities().is_empty());
    assert!(output.product().source().attachments().is_empty());
    assert!(output.product().source().bindings().is_empty());

    let optional = built_in_configuration("artifact/v1", AuthorityExpectation::Optional)
        .expect("structural optional artifact configuration");
    let optional_input =
        SavedSnapshotInput::new(&path, &bytes, &optional).expect("optional artifact input");
    assert_eq!(
        registry.materialize(&optional_input),
        Err(AdapterSdkError::Configuration)
    );

    let control_configuration =
        AdapterConfiguration::new("artifact/v1", "artifact.control/v1", b"control".to_vec())
            .expect("artifact control configuration");
    let control_input = SavedSnapshotInput::new(&path, &bytes, &control_configuration)
        .expect("artifact control input");
    for (control, expected) in [
        (ArtifactProductControl::Complete, Ok(())),
        (
            ArtifactProductControl::UndeclaredKind,
            Err(AdapterSdkError::UndeclaredSourceItemKind),
        ),
        (
            ArtifactProductControl::PartialSpan,
            Err(AdapterSdkError::Materialization),
        ),
        (
            ArtifactProductControl::UnsupportedCapability,
            Err(AdapterSdkError::Capability),
        ),
    ] {
        let mut controls = AdapterRegistry::new();
        controls
            .register(ArtifactControlAdapter::new(control))
            .expect("artifact control registration");
        assert_eq!(controls.materialize(&control_input).map(|_| ()), expected);
    }
    let malformed_configuration =
        AdapterConfiguration::new("artifact/v1", "host.configuration/v1", b"control".to_vec())
            .expect("malformed artifact configuration binding");
    let malformed_input = SavedSnapshotInput::new(&path, &bytes, &malformed_configuration)
        .expect("malformed artifact input");
    let mut controls = AdapterRegistry::new();
    controls
        .register(ArtifactControlAdapter::new(
            ArtifactProductControl::Complete,
        ))
        .expect("artifact malformed-configuration control");
    assert_eq!(
        controls.materialize(&malformed_input),
        Err(AdapterSdkError::Configuration)
    );
}

#[test]
fn adapter_sdk_is_cross_root_and_registration_order_invariant() {
    let first_root = tempfile::tempdir().expect("first absolute root");
    let second_root = tempfile::tempdir().expect("second absolute root");
    assert_ne!(first_root.path(), second_root.path());
    let relative = "host/source.txt";
    let first_bytes = write_root_file(first_root.path(), relative, b"saved host syntax\n");
    let second_bytes = write_root_file(second_root.path(), relative, b"saved host syntax\n");
    assert_eq!(first_bytes, second_bytes);

    let mut forward = AdapterRegistry::new();
    forward
        .register(ControlAdapter::new(
            "host-secondary/v1",
            Some(CONTROL_DECLARED_CAPABILITY),
        ))
        .expect("register secondary adapter first");
    forward
        .register(ControlAdapter::new(
            "host-primary/v1",
            Some(CONTROL_DECLARED_CAPABILITY),
        ))
        .expect("register primary adapter second");
    let mut reverse = AdapterRegistry::new();
    reverse
        .register(ControlAdapter::new(
            "host-primary/v1",
            Some(CONTROL_DECLARED_CAPABILITY),
        ))
        .expect("register primary adapter first");
    reverse
        .register(ControlAdapter::new(
            "host-secondary/v1",
            Some(CONTROL_DECLARED_CAPABILITY),
        ))
        .expect("register secondary adapter second");
    assert_eq!(forward.descriptors(), reverse.descriptors());

    let repository_path = path(relative).expect("canonical host path");
    let configuration = control_configuration("host-primary/v1");
    let first_input = SavedSnapshotInput::new(&repository_path, &first_bytes, &configuration)
        .expect("first root input");
    let second_input = SavedSnapshotInput::new(&repository_path, &second_bytes, &configuration)
        .expect("second root input");
    let forward_output = forward
        .materialize(&first_input)
        .expect("forward-order materialization");
    let reverse_output = reverse
        .materialize(&second_input)
        .expect("reverse-order materialization");
    assert_eq!(forward_output, reverse_output);
    assert_eq!(forward_output.digest(), reverse_output.digest());
}

#[test]
fn adapter_sdk_excludes_daemon_editor_and_unsaved_state_from_saved_identity() {
    let root = tempfile::tempdir().expect("ambient-state root");
    let relative = "spec/positive.md";
    let saved = write_root_file(
        root.path(),
        relative,
        include_bytes!("../fixtures/adapters/markdown/positive.md"),
    );
    let daemon_before = write_root_file(root.path(), ".daemon/session", b"daemon-before");
    let editor_before = write_root_file(root.path(), ".editor/selection", b"selection-before");
    let unsaved_before = write_root_file(root.path(), ".editor/unsaved.md", b"unsaved-before\n");

    let registry = built_in_registry().expect("built-in adapter registry");
    let repository_path = path(relative).expect("canonical saved path");
    let configuration = built_in_configuration("markdown/v1", AuthorityExpectation::Required)
        .expect("Markdown configuration");
    let input = SavedSnapshotInput::new(&repository_path, &saved, &configuration)
        .expect("saved input without UX state");
    let before = registry
        .materialize(&input)
        .expect("materialization before ambient changes");

    let daemon_after = write_root_file(root.path(), ".daemon/session", b"daemon-after");
    let editor_after = write_root_file(root.path(), ".editor/selection", b"selection-after");
    let unsaved = write_root_file(
        root.path(),
        ".editor/unsaved.md",
        b"different unsaved editor bytes\n",
    );
    assert_ne!(daemon_before, daemon_after);
    assert_ne!(editor_before, editor_after);
    assert_ne!(unsaved_before, unsaved);
    assert_ne!(saved, unsaved);
    let after = registry
        .materialize(&input)
        .expect("materialization after ambient changes");
    assert_eq!(before, after);

    let unsaved_input = SavedSnapshotInput::new(&repository_path, &unsaved, &configuration)
        .expect("explicit unsaved bytes used only as a negative control");
    let unsaved_output = registry
        .materialize(&unsaved_input)
        .expect("explicit alternate bytes remain materializable");
    assert_ne!(before.input_digest(), unsaved_output.input_digest());
    assert_ne!(before.digest(), unsaved_output.digest());
}

#[test]
fn adapter_sdk_rejects_unsupported_and_duplicate_capabilities() {
    let duplicate_capabilities = AdapterDescriptor::new(
        "host-duplicate-capability/v1",
        "control-parser",
        "1.0.0",
        "host.syntax/v1",
        CONTROL_CONFIGURATION_SCHEMA_ID,
        vec![
            CONTROL_DECLARED_CAPABILITY.to_owned(),
            CONTROL_DECLARED_CAPABILITY.to_owned(),
        ],
        vec![SourceItemKind::new("host/item/v1").expect("control source item kind")],
    );
    assert_eq!(duplicate_capabilities, Err(AdapterSdkError::Capability));
    assert_eq!(
        AdapterDescriptor::new(
            "host-invalid-capability/v1",
            "control-parser",
            "1.0.0",
            "host.syntax/v1",
            CONTROL_CONFIGURATION_SCHEMA_ID,
            vec![String::from("invalid")],
            vec![SourceItemKind::new("host/item/v1").expect("control source item kind")],
        ),
        Err(AdapterSdkError::Capability)
    );
    assert_eq!(
        AdapterDescriptor::new(
            "invalid",
            "control-parser",
            "1.0.0",
            "host.syntax/v1",
            CONTROL_CONFIGURATION_SCHEMA_ID,
            vec![CONTROL_DECLARED_CAPABILITY.to_owned()],
            vec![SourceItemKind::new("host/item/v1").expect("control source item kind")],
        ),
        Err(AdapterSdkError::Identity)
    );
    assert_eq!(
        AdapterConfiguration::new("invalid", CONTROL_CONFIGURATION_SCHEMA_ID, Vec::new()),
        Err(AdapterSdkError::Configuration)
    );
    assert_eq!(
        AdapterConfiguration::new(
            "host-oversized/v1",
            CONTROL_CONFIGURATION_SCHEMA_ID,
            vec![0; 1_048_577],
        ),
        Err(AdapterSdkError::Configuration)
    );
    let diagnostic_path = path("host/diagnostic.txt").expect("diagnostic control path");
    let diagnostic_span = SourceSpan {
        path: diagnostic_path,
        start: 0,
        end: 1,
    };
    assert_eq!(
        NonnormativeDisposition::new(diagnostic_span.clone(), "invalid"),
        Err(AdapterSdkError::Capability)
    );
    let canonical_diagnostic = AdapterProduct::new(
        SpecGraph::default(),
        Vec::new(),
        Vec::new(),
        vec![
            AdapterDiagnostic {
                code: "CANONICAL-CONTEXT",
                span: diagnostic_span.clone(),
                context: String::from("canonical"),
            },
            AdapterDiagnostic {
                code: "CANONICAL-CONTEXT-2",
                span: diagnostic_span.clone(),
                context: String::from("canonical-2"),
            },
        ],
    )
    .expect("canonical diagnostic product");
    assert_eq!(canonical_diagnostic.diagnostics().len(), 2);
    assert_eq!(
        AdapterProduct::new(
            SpecGraph::default(),
            Vec::new(),
            Vec::new(),
            vec![AdapterDiagnostic {
                code: "NONCANONICAL-CONTEXT",
                span: diagnostic_span,
                context: String::from("e\u{301}"),
            }],
        ),
        Err(AdapterSdkError::Encoding)
    );

    let mut registry = AdapterRegistry::new();
    registry
        .register(ControlAdapter::new(
            "host-unsupported-output/v1",
            Some(CONTROL_UNSUPPORTED_CAPABILITY),
        ))
        .expect("register adapter with a frozen declared capability");
    let repository_path = path("host/unsupported.txt").expect("unsupported control path");
    let configuration = control_configuration("host-unsupported-output/v1");
    let input = SavedSnapshotInput::new(&repository_path, b"selected syntax", &configuration)
        .expect("unsupported control input");
    assert_eq!(
        registry.materialize(&input),
        Err(AdapterSdkError::Capability)
    );
}

#[test]
fn adapter_sdk_rejects_duplicate_identities_and_incomplete_disposition() {
    let mut duplicate_registry = AdapterRegistry::new();
    duplicate_registry
        .register(ControlAdapter::new(
            "host-duplicate/v1",
            Some(CONTROL_DECLARED_CAPABILITY),
        ))
        .expect("register first identity");
    assert_eq!(
        duplicate_registry.register(ControlAdapter::new(
            "host-duplicate/v1",
            Some(CONTROL_DECLARED_CAPABILITY),
        )),
        Err(AdapterSdkError::DuplicateAdapter)
    );

    let mut incomplete_registry = AdapterRegistry::new();
    incomplete_registry
        .register(ControlAdapter::new("host-incomplete/v1", None))
        .expect("register incomplete-output control adapter");
    let repository_path = path("host/incomplete.txt").expect("incomplete control path");
    let configuration = control_configuration("host-incomplete/v1");
    let input = SavedSnapshotInput::new(&repository_path, b"selected syntax", &configuration)
        .expect("incomplete control input");
    assert_eq!(
        incomplete_registry.materialize(&input),
        Err(AdapterSdkError::Materialization)
    );
}

#[test]
fn adapter_sdk_stays_shell_side_and_rejects_attempted_core_coupling() {
    let target_dir = "target/adapter-sdk-core-coupling-control";
    let uninstrumented_cargo = || {
        let mut command = Command::new(env!("CARGO"));
        for variable in [
            "CARGO_ENCODED_RUSTFLAGS",
            "CARGO_LLVM_COV",
            "CARGO_LLVM_COV_SHOW_ENV",
            "LLVM_PROFILE_FILE",
            "RUSTC_WRAPPER",
            "RUSTFLAGS",
            "RUSTDOCFLAGS",
            "__CARGO_LLVM_COV_RUSTC_WRAPPER",
            "__CARGO_LLVM_COV_RUSTC_WRAPPER_CRATE_NAMES",
            "__CARGO_LLVM_COV_RUSTC_WRAPPER_PRE_EXISTING",
            "__CARGO_LLVM_COV_RUSTC_WRAPPER_RUSTFLAGS",
        ] {
            command.env_remove(variable);
        }
        command
    };
    let positive = uninstrumented_cargo()
        .args([
            "check",
            "--locked",
            "--offline",
            "--package",
            "buoy-core",
            "--lib",
            "--target",
            "thumbv7em-none-eabi",
            "--target-dir",
            target_dir,
        ])
        .output()
        .expect("run positive bare-metal core control");
    assert!(
        positive.status.success(),
        "bare-metal core control failed: {}",
        String::from_utf8_lossy(&positive.stderr)
    );

    let negative_manifest = "fixtures/product-identity/v1/negative/buoy-core-effect/Cargo.toml";
    let negative = uninstrumented_cargo()
        .args([
            "check",
            "--locked",
            "--offline",
            "--manifest-path",
            negative_manifest,
            "--target",
            "thumbv7em-none-eabi",
            "--target-dir",
            target_dir,
        ])
        .output()
        .expect("run attempted core-coupling negative control");
    assert!(!negative.status.success());
    assert!(
        String::from_utf8_lossy(&negative.stderr).contains("can't find crate for `std`"),
        "core-coupling fixture failed for the wrong reason: {}",
        String::from_utf8_lossy(&negative.stderr)
    );
}

#[test]
fn parsed_item_protocol_accepts_versioned_extensible_kinds() {
    for (adapter, schema, kind) in [
        ("markdown/v1", "markdown/v1", "markdown/fence/v1"),
        ("rust/v1", "rust/v1", "rust/item/v1"),
        ("chelis/v1", "chelis/v1", "chelis/surf/v1"),
        (
            "openspec/v1",
            "openspec/standing-spec/v1",
            "openspec/requirement/v1",
        ),
    ] {
        let kind = SourceItemKind::new(kind).expect("versioned source item kind");
        assert_eq!(
            validate_parsed_item_protocol(adapter, schema, &kind),
            Ok(())
        );
    }
}

#[test]
fn markdown_extracts_exact_fence_and_marks_prose_nonnormative() {
    let source = include_bytes!("../fixtures/adapters/markdown/positive.md");
    let path = path("spec/positive.md");
    assert!(path.is_some());
    let Some(path) = path else { return };
    let output = extract_markdown(&path, source, AuthorityExpectation::Required);
    assert_eq!(output.diagnostics, Vec::new());
    assert_eq!(output.semantic.len(), 1);
    assert!(output.nonnormative_nodes > 0);
    let located = &output.semantic[0];
    assert_eq!(
        Some(located.span.start),
        source
            .windows(3)
            .position(|bytes| bytes == b"```")
            .and_then(|offset| u64::try_from(offset).ok())
    );
    let MarkdownSemantic::Atom(atom) = &located.value else {
        assert!(matches!(located.value, MarkdownSemantic::Atom(_)));
        return;
    };
    assert_eq!(atom.meta().id().as_str(), "SPG-ADAPTER-001");
    assert_eq!(
        atom.approval().map(ApprovalBinding::authority),
        Some("maintainers")
    );
}

#[test]
fn markdown_totality_handles_objects_fences_versions_mutations_and_empty_roots() {
    let path = path("spec/input.md");
    assert!(path.is_some());
    let Some(path) = path else { return };
    let empty = extract_markdown(&path, b"", AuthorityExpectation::Optional);
    assert!(empty.semantic.is_empty());
    assert!(empty.diagnostics.is_empty());
    assert_eq!(empty.nonnormative_nodes, 1);
    assert_eq!(empty.nonnormative.len(), 1);
    assert_eq!(empty.nonnormative[0].start, 0);
    assert_eq!(empty.nonnormative[0].end, 0);
    let forbidden_empty = extract_markdown(&path, b"", AuthorityExpectation::Forbidden);
    assert!(forbidden_empty.diagnostics.is_empty());
    assert_eq!(forbidden_empty.nonnormative_nodes, 1);

    let objects = extract_markdown(
        &path,
        include_bytes!("../fixtures/adapters/markdown/objects.md"),
        AuthorityExpectation::Optional,
    );
    assert_eq!(objects.diagnostics, Vec::new());
    assert_eq!(objects.semantic.len(), 10);
    assert!(objects.semantic.iter().all(|located| {
        located.item.language_schema_id() == "markdown/v1"
            && located.item.kind().as_str() == "markdown/fence/v1"
            && located.item.path() == &path
            && located.item.semantic_bytes()
                == format!(
                    "{}:{}:{}",
                    located.value.meta().kind() as u16,
                    located.value.meta().id().as_str(),
                    located.value.meta().digest()
                )
                .as_bytes()
    }));
    let supplied_materialization =
        String::from_utf8_lossy(include_bytes!("../fixtures/adapters/markdown/objects.md"))
            .replacen(
                "replacements = []\n```",
                "replacements = []\nmaterialized_subjects = []\n```",
                1,
            );
    let supplied = extract_markdown(
        &path,
        supplied_materialization.as_bytes(),
        AuthorityExpectation::Optional,
    );
    assert!(supplied.diagnostics.iter().any(|diagnostic| {
        diagnostic.code == "SPG-ADAPTER-UNSUPPORTED-FIELD" && diagnostic.context == "unknown-field"
    }));

    let fenced = extract_markdown(
        &path,
        include_bytes!("../fixtures/adapters/markdown/fenced-example.md"),
        AuthorityExpectation::Optional,
    );
    assert!(fenced.semantic.is_empty());
    assert!(fenced.diagnostics.is_empty());
    assert!(fenced.nonnormative_nodes > 0);

    for (bytes, code) in [
        (
            include_bytes!("../fixtures/adapters/markdown/unsupported.md").as_slice(),
            "SPG-ADAPTER-UNSUPPORTED-SYNTAX-VERSION",
        ),
        (
            include_bytes!("../fixtures/adapters/markdown/malformed.md").as_slice(),
            "SPG-ADAPTER-UNSUPPORTED-FIELD",
        ),
        (
            include_bytes!("../fixtures/adapters/markdown/empty-required.md").as_slice(),
            "SPG-ADAPTER-EXPECTED-AUTHORITY-EMPTY",
        ),
    ] {
        let output = extract_markdown(&path, bytes, AuthorityExpectation::Required);
        assert!(
            output
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == code)
        );
    }

    let duplicate_key = b"```spec-provenance-atom-v1\nid = \"SPG-001\"\nid = \"SPG-002\"\n```\n";
    let output = extract_markdown(&path, duplicate_key, AuthorityExpectation::Optional);
    assert_eq!(output.diagnostics[0].code, "SPG-ADAPTER-DUPLICATE-FIELD");

    let obsolete =
        String::from_utf8_lossy(include_bytes!("../fixtures/adapters/markdown/positive.md"))
            .replace("xxh3-128:", "sha256:");
    let output = extract_markdown(&path, obsolete.as_bytes(), AuthorityExpectation::Required);
    assert!(output.diagnostics.iter().any(|diagnostic| {
        diagnostic.code == "SPG-ADAPTER-MALFORMED-ATOM"
            && diagnostic.context == "DET-DIGEST-ALGORITHM"
    }));
}

#[test]
fn markdown_closed_parser_rejects_each_malformed_shape_and_authority_mismatch() {
    let path = path("spec/controls.md").expect("control path");
    for bytes in [&[0xff][..], b"text\rcontrol".as_slice()] {
        assert_eq!(
            extract_markdown(&path, bytes, AuthorityExpectation::Optional)
                .diagnostics
                .len(),
            1
        );
    }
    let nonnormative = extract_markdown(
        &path,
        b"plain `inline` text\n\n<div>html</div>\n",
        AuthorityExpectation::Optional,
    );
    assert!(nonnormative.semantic.is_empty());
    assert!(nonnormative.nonnormative_nodes >= 2);

    let atom = |body: &str| format!("```spec-provenance-atom-v1\n{body}\n```\n");
    let approval = "[approval]\nauthority = \"maintainers\"\nrecord = \"reviews/control.md\"\ndigest = \"xxh3-128:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\"";
    let valid = |state: &str, kind: &str, approval_text: &str| {
        atom(&format!(
            "id = \"CONTROL-1\"\nstate = \"{state}\"\nkind = \"{kind}\"\nscope = [\"spec.control\"]\nstatement = \"Control MUST hold.\"\n{approval_text}"
        ))
    };
    for source in [
        atom("not = [toml"),
        atom("id = \"x\"\nunknown = true"),
        atom(
            "state = \"proposed\"\nkind = \"behavioral\"\nscope = [\"x\"]\nstatement = \"X MUST hold.\"",
        ),
        atom("id = \"x\"\nkind = \"behavioral\"\nscope = [\"x\"]\nstatement = \"X MUST hold.\""),
        atom("id = \"x\"\nstate = \"proposed\"\nscope = [\"x\"]\nstatement = \"X MUST hold.\""),
        atom("id = \"x\"\nstate = \"proposed\"\nkind = \"behavioral\"\nscope = [\"x\"]"),
        atom(
            "id = \"x\"\nstate = \"unknown\"\nkind = \"behavioral\"\nscope = [\"x\"]\nstatement = \"X MUST hold.\"",
        ),
        atom(
            "id = \"x\"\nstate = \"proposed\"\nkind = \"unknown\"\nscope = [\"x\"]\nstatement = \"X MUST hold.\"",
        ),
        atom(
            "id = \"x\"\nstate = \"proposed\"\nkind = \"behavioral\"\nscope = [\"x\"]\nstatement = \"informative\"",
        ),
        atom(
            "id = \"\"\nstate = \"proposed\"\nkind = \"behavioral\"\nscope = [\"x\"]\nstatement = \"X MUST hold.\"",
        ),
        atom(
            "id = \"x\"\nstate = \"proposed\"\nkind = \"behavioral\"\nscope = \"x\"\nstatement = \"X MUST hold.\"",
        ),
        atom(
            "id = \"x\"\nstate = \"proposed\"\nkind = \"behavioral\"\nscope = [1]\nstatement = \"X MUST hold.\"",
        ),
        atom(
            "id = \"x\"\nstate = \"proposed\"\nkind = \"behavioral\"\nscope = []\nstatement = \"X MUST hold.\"",
        ),
        valid("proposed", "behavioral", approval),
        valid("ratified", "behavioral", ""),
        valid("ratified", "behavioral", "approval = \"wrong\""),
        valid("ratified", "behavioral", "[approval]\nauthority = \"x\""),
        valid(
            "ratified",
            "behavioral",
            "[approval]\nauthority = \"x\"\nrecord = \"reviews/x.md\"\nextra = \"x\"",
        ),
        valid(
            "ratified",
            "behavioral",
            "[approval]\nauthority = 1\nrecord = \"reviews/x.md\"\ndigest = \"xxh3-128:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\"",
        ),
        valid(
            "ratified",
            "behavioral",
            "[approval]\nauthority = \"x\"\nrecord = 1\ndigest = \"xxh3-128:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\"",
        ),
        valid(
            "ratified",
            "behavioral",
            "[approval]\nauthority = \"x\"\nrecord = \"reviews/x.md\"\ndigest = 1",
        ),
        valid(
            "ratified",
            "behavioral",
            "[approval]\nauthority = \"x\"\nrecord = \"/absolute\"\ndigest = \"xxh3-128:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\"",
        ),
        valid(
            "ratified",
            "behavioral",
            "[approval]\nauthority = \"x\"\nrecord = \"reviews/x.md\"\ndigest = \"wrong\"",
        ),
    ] {
        assert!(
            !extract_markdown(&path, source.as_bytes(), AuthorityExpectation::Optional)
                .diagnostics
                .is_empty()
        );
    }
    for kind in ["structural", "constraint"] {
        assert!(
            extract_markdown(
                &path,
                valid("proposed", kind, "").as_bytes(),
                AuthorityExpectation::Optional,
            )
            .diagnostics
            .is_empty()
        );
    }
    assert!(
        extract_markdown(
            &path,
            valid("ratified", "behavioral", approval).as_bytes(),
            AuthorityExpectation::Forbidden,
        )
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.code == "SPG-ADAPTER-AUTHORITY-FORBIDDEN")
    );
    for body in [
        "not = [toml",
        "name = \"missing-id\"",
        "id = \"x\"\nextra = true",
        "id = 1",
    ] {
        let source = format!("```spec-provenance-policy-v1\n{body}\n```\n");
        assert!(
            !extract_markdown(&path, source.as_bytes(), AuthorityExpectation::Optional)
                .diagnostics
                .is_empty()
        );
    }
    let unknown = b"```spec-provenance-policy-v1\nid = \"POL-UNKNOWN\"\nunknown = true\n```\n";
    assert_eq!(
        extract_markdown(&path, unknown, AuthorityExpectation::Optional).diagnostics[0].code,
        "SPG-ADAPTER-UNSUPPORTED-FIELD"
    );
}

#[test]
fn markdown_unicode_and_crlf_preserve_original_spans_but_not_semantic_revision() {
    let path = path("spec/unicode.md");
    assert!(path.is_some());
    let Some(path) = path else { return };
    let lf = include_bytes!("../fixtures/adapters/markdown/unicode.md");
    let output = extract_markdown(&path, lf, AuthorityExpectation::Required);
    assert_eq!(output.diagnostics, Vec::new());
    assert_eq!(
        Some(output.semantic[0].span.start),
        lf.windows(3)
            .position(|bytes| bytes == b"```")
            .and_then(|offset| u64::try_from(offset).ok())
    );

    let lf_text = core::str::from_utf8(lf).unwrap_or("");
    let crlf_text = lf_text.replace('\n', "\r\n");
    let crlf = extract_markdown(&path, crlf_text.as_bytes(), AuthorityExpectation::Required);
    let (MarkdownSemantic::Atom(lf_atom), MarkdownSemantic::Atom(crlf_atom)) =
        (&output.semantic[0].value, &crlf.semantic[0].value)
    else {
        return;
    };
    assert_eq!(lf_atom.revision(), crlf_atom.revision());
    assert!(crlf.semantic[0].span.end > output.semantic[0].span.end);
}

#[test]
fn markdown_schema_errors_reach_each_nested_boundary_integration_path() {
    let Some(path) = path("spec/schema-controls.md") else {
        return;
    };
    let fixture = include_str!("../fixtures/adapters/markdown/objects.md");
    let body = |syntax: &str| {
        fixture.split("```").find_map(|block| {
            let (candidate, body) = block.split_once('\n')?;
            (candidate == syntax).then(|| body.to_owned())
        })
    };
    let atom = String::from(
        "id = \"ATOM-CONTROL\"\nstate = \"proposed\"\nkind = \"behavioral\"\nscope = [\"invalid scope\"]\nstatement = \"The adapter MUST reject an invalid scope.\"\n",
    );
    let mut controls = vec![("spec-provenance-atom-v1", atom)];
    let mut blocks = fixture.split("```").skip(1);
    while let Some(block) = blocks.next() {
        let Some((syntax, body)) = block.split_once('\n') else {
            return;
        };
        controls.push((syntax, body.replacen("id = \"", "id = \"invalid ", 1)));
        let _ = blocks.next();
    }
    for (syntax, old, new) in [
        (
            "spec-provenance-policy-v1",
            "required_roles = [\"positive\"]",
            "required_roles = [\"positive\", \"positive\"]",
        ),
        (
            "spec-provenance-debt-v1",
            "owner = \"adapter-owner\"",
            "owner = \"invalid scope\"",
        ),
        (
            "spec-provenance-waiver-v1",
            "owner = \"adapter-owner\"",
            "owner = \"invalid scope\"",
        ),
        (
            "spec-provenance-reaffirmation-v1",
            "affected_object_id = \"ATOM-ADAPTER-001\"",
            "affected_object_id = \"invalid id\"",
        ),
        (
            "spec-provenance-assumption-v1",
            "authority = \"maintainers\"",
            "authority = \"invalid scope\"",
        ),
        (
            "spec-provenance-policy-v1",
            "authority_selectors = []",
            "authority_selectors = [{ root = \"spec\", adapter_id = \"invalid schema\", include_patterns = [], exclude_patterns = [], authority_expectation = \"optional\" }]",
        ),
        (
            "spec-provenance-surface-v1",
            "schema = \"adapter.member-key/v1\"",
            "schema = \"invalid schema\"",
        ),
        (
            "spec-provenance-surface-v1",
            "authority = { id = \"ATOM-ADAPTER-001\"",
            "authority = { id = \"invalid id\"",
        ),
        (
            "spec-provenance-debt-v1",
            "objects = [{ kind = \"atom\", id = \"ATOM-ADAPTER-001\"",
            "objects = [{ kind = \"atom\", id = \"invalid id\"",
        ),
        (
            "spec-provenance-derivation-v1",
            "tool = { id = \"adapter-generator\"",
            "tool = { id = \"invalid scope\"",
        ),
        (
            "spec-provenance-derivation-v1",
            "path = \"src/input.rs\"",
            "path = \"../invalid\"",
        ),
        (
            "spec-provenance-debt-v1",
            "policy_id = \"POL-ADAPTER-001\"",
            "policy_id = \"invalid id\"",
        ),
    ] {
        let Some(original) = body(syntax) else {
            return;
        };
        controls.push((syntax, original.replace(old, new)));
    }
    for (syntax, body) in controls {
        let source = format!("```{syntax}\n{body}```\n");
        assert!(
            !extract_markdown(&path, source.as_bytes(), AuthorityExpectation::Optional)
                .diagnostics
                .is_empty(),
            "{syntax}"
        );
    }

    let revision = SemanticDigest::xxh3_128([1; 16]);
    let implementation = format!(
        r##"#[spec_provenance::implementation(
    syntax = "v1",
    body = r#"
id = "invalid id"
atoms = [{{ id = "ATOM-001", revision = "{revision}" }}]
navigation = "implements"
"#
)]
fn implementation_control() {{}}
"##
    );
    assert!(
        !extract_rust(&path, implementation.as_bytes())
            .diagnostics
            .is_empty()
    );
}

#[test]
fn openspec_configuration_errors_reach_each_integration_boundary() {
    let digest = SemanticDigest::xxh3_128([1; 16]);
    let configuration = |kind: &str,
                         lifecycle: &str,
                         scope: &str,
                         reserved: &str,
                         authority: &str,
                         record: &str,
                         approval_digest: &str| {
        format!(
            "schema = \"buoy.openspec-authority/v1\"\n\n[[capabilities]]\nname = \"demo\"\nformat = \"openspec/standing-spec/v1\"\nscope = \"{scope}\"\natom_kind = \"{kind}\"\nlifecycle = \"{lifecycle}\"\nreserved_ids = {reserved}\napproval = {{ authority = \"{authority}\", record = \"{record}\", digest = \"{approval_digest}\" }}\n"
        )
    };
    assert_eq!(
        OpenSpecConfiguration::parse(&[0xff]),
        Err("SPG-OPENSPEC-CONFIG-UTF8")
    );
    for source in [
        configuration(
            "invalid",
            "ratified",
            "openspec.demo",
            "[\"DEMO-001\"]",
            "maintainers",
            "reviews/demo.md",
            &digest.canonical_text(),
        ),
        configuration(
            "behavioral",
            "invalid",
            "openspec.demo",
            "[\"DEMO-001\"]",
            "maintainers",
            "reviews/demo.md",
            &digest.canonical_text(),
        ),
        configuration(
            "behavioral",
            "ratified",
            "invalid scope",
            "[\"DEMO-001\"]",
            "maintainers",
            "reviews/demo.md",
            &digest.canonical_text(),
        ),
        configuration(
            "behavioral",
            "ratified",
            "openspec.demo",
            "[]",
            "maintainers",
            "reviews/demo.md",
            &digest.canonical_text(),
        ),
        configuration(
            "behavioral",
            "ratified",
            "openspec.demo",
            "[\"DEMO-001\"]",
            "invalid scope",
            "reviews/demo.md",
            &digest.canonical_text(),
        ),
        configuration(
            "behavioral",
            "ratified",
            "openspec.demo",
            "[\"DEMO-001\"]",
            "maintainers",
            "../invalid",
            &digest.canonical_text(),
        ),
        configuration(
            "behavioral",
            "ratified",
            "openspec.demo",
            "[\"DEMO-001\"]",
            "maintainers",
            "reviews/demo.md",
            "invalid",
        ),
    ] {
        assert!(OpenSpecConfiguration::parse(source.as_bytes()).is_err());
    }

    let path = RepositoryPath::new("openspec/specs/demo/spec.md").expect("OpenSpec path");
    let multiple_diagnostics = b"# Demo\n\n## Requirements\n\n### Scenario: Wrong level\n\n### Requirement: DEMO-001 Valid\nThe adapter MUST parse this.\n\n#### Scenario: !!!\nBody.\n";
    assert_eq!(
        extract_openspec(&path, multiple_diagnostics)
            .diagnostics
            .len(),
        2
    );
    let invalid_key = b"# Demo\n\n## Requirements\n\n### Requirement: bad Invalid\nBody.\n";
    assert_eq!(
        extract_openspec(&path, invalid_key).diagnostics[0].code,
        "SPG-OPENSPEC-REQUIREMENT-ID"
    );
    let title_first_id =
        b"# Demo\n\n## Requirements\n\n### Requirement: [DEMO-001] Title first\nBody.\n";
    assert_eq!(
        extract_openspec(&path, title_first_id).diagnostics[0].code,
        "SPG-OPENSPEC-REQUIREMENT-ID"
    );
    let orphan_scenario = b"# Demo\n\n## Requirements\n\n#### Scenario: Orphan\nBody.\n\n### Requirement: DEMO-001 Valid\nThe adapter MUST parse this.\n";
    assert_eq!(
        extract_openspec(&path, orphan_scenario).diagnostics[0].code,
        "SPG-OPENSPEC-UNSUPPORTED-SYNTAX"
    );
    let requirement_boundaries = b"# Demo\n\n## Requirements\n\n### Requirement: DEMO-001 First\nThe adapter MUST parse this.\n\n#### Scenario: First\nBody.\n\n### Requirement: DEMO-002 Second\nThe adapter MUST parse this.\n\n#### Scenario: Second\nBody.\n";
    assert!(
        extract_openspec(&path, requirement_boundaries)
            .diagnostics
            .is_empty()
    );
    let ordinary_subheading = b"# Demo\n\n## Requirements\n\n### Requirement: DEMO-001 Valid\nThe adapter MUST parse this.\n\n#### Notes\nNonnormative detail.\n";
    assert!(
        extract_openspec(&path, ordinary_subheading)
            .diagnostics
            .is_empty()
    );
    for unsupported_heading in [
        "#### Requirement: DEMO-002 Wrong level",
        "##### Scenario: Wrong level",
    ] {
        let source = format!(
            "# Demo\n\n## Requirements\n\n### Requirement: DEMO-001 Valid\nThe adapter MUST parse this.\n\n{unsupported_heading}\nBody.\n"
        );
        assert_eq!(
            extract_openspec(&path, source.as_bytes()).diagnostics[0].code,
            "SPG-OPENSPEC-UNSUPPORTED-SYNTAX"
        );
    }
    #[cfg(feature = "coverage-controls")]
    assert!(openspec_partition_control());
    let valid = b"# Demo\n\n## Requirements\n\n### Requirement: DEMO-001 Valid\nThe adapter MUST parse this.\n";
    let output = extract_openspec(&path, valid);
    let object = &output.semantic[0].value;
    assert_eq!(object.title(), "DEMO-001 Valid");
    assert!(object.body().contains("MUST"));
    assert_eq!(object.digest(), object.external().revision());
}

#[test]
fn rust_adapter_attaches_only_eligible_items() {
    let path = path("tests/positive.rs");
    assert!(path.is_some());
    let Some(path) = path else { return };
    let positive = extract_rust(
        &path,
        include_bytes!("../fixtures/adapters/rust/positive.rs"),
    );
    assert_eq!(positive.semantic.len(), 1);
    assert!(positive.diagnostics.is_empty());
    let RustSemantic::EvidenceCarrier(carrier) = &positive.semantic[0].value else {
        assert!(matches!(
            positive.semantic[0].value,
            RustSemantic::EvidenceCarrier(_)
        ));
        return;
    };
    assert_eq!(carrier.meta().id().as_str(), "CAR-ADAPTER-001");
    assert_eq!(carrier.corpora().len(), 1);
    assert_eq!(carrier.assumptions().len(), 1);
    assert_eq!(carrier.exclusions(), ["Network behavior is excluded."]);
    assert_eq!(positive.semantic[0].item.language_schema_id(), "rust/v1");
    assert_eq!(positive.semantic[0].item.kind().as_str(), "rust/item/v1");
    assert_eq!(positive.semantic[0].item.path(), &path);
    assert_eq!(
        carrier.parsed_item_digest(),
        positive.semantic[0].item.digest()
    );

    let links = extract_rust(&path, include_bytes!("../fixtures/adapters/rust/links.rs"));
    assert_eq!(links.semantic.len(), 5);
    assert!(links.diagnostics.is_empty());
    assert!(
        links
            .semantic
            .iter()
            .any(|item| { matches!(item.value, RustSemantic::ImplementationLink(_)) })
    );
    assert!(
        links
            .semantic
            .iter()
            .any(|item| { matches!(item.value, RustSemantic::DecisionSurface(_)) })
    );
    assert!(
        links
            .semantic
            .iter()
            .any(|item| { matches!(item.value, RustSemantic::Derivation(_)) })
    );
    assert!(links.semantic.iter().any(|item| {
        matches!(
            &item.value,
            RustSemantic::EvidenceCarrier(carrier) if carrier.role() == EvidenceRole::Structural
        )
    }));
    assert!(links.semantic.iter().any(|item| {
        matches!(
            &item.value,
            RustSemantic::EvidenceCarrier(carrier)
                if carrier.role() == EvidenceRole::FormalProperty
        )
    }));

    let mentions = extract_rust(
        &path,
        include_bytes!("../fixtures/adapters/rust/mentions.rs"),
    );
    assert!(mentions.semantic.is_empty());
    assert!(
        mentions
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "SPG-ADAPTER-DISABLED-ITEM")
    );
}

#[test]
fn rust_adapter_fails_loudly_for_malformed_unsupported_and_file_level_metadata() {
    let path = path("tests/input.rs");
    assert!(path.is_some());
    let Some(path) = path else { return };
    for (bytes, code) in [
        (
            include_bytes!("../fixtures/adapters/rust/malformed.rs").as_slice(),
            "SPG-ADAPTER-MALFORMED-ATTRIBUTE",
        ),
        (
            include_bytes!("../fixtures/adapters/rust/unsupported.rs").as_slice(),
            "SPG-ADAPTER-UNSUPPORTED-SYNTAX-VERSION",
        ),
        (
            include_bytes!("../fixtures/adapters/rust/file-level.rs").as_slice(),
            "SPG-ADAPTER-UNATTACHED-ATTRIBUTE",
        ),
    ] {
        let output = extract_rust(&path, bytes);
        assert!(
            output
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == code)
        );
    }

    let mutation = br#"
#[spec_provenance::carrier(syntax = "v1", body = "id = 'CAR-MISSING'")]
#[test]
fn missing_fields() {}
"#;
    let output = extract_rust(&path, mutation);
    assert_eq!(output.diagnostics[0].code, "SPG-ADAPTER-MISSING-FIELD");

    let complete = String::from_utf8_lossy(include_bytes!("../fixtures/adapters/rust/positive.rs"));
    let without_exclusions =
        complete.replace("exclusions = [\"Network behavior is excluded.\"]\n", "");
    let output = extract_rust(&path, without_exclusions.as_bytes());
    assert!(output.diagnostics.iter().any(|diagnostic| {
        diagnostic.code == "SPG-ADAPTER-MISSING-FIELD" && diagnostic.context == "missing-field"
    }));

    let obsolete = complete.replace("xxh3-128:", "sha256:");
    let output = extract_rust(&path, obsolete.as_bytes());
    assert!(output.diagnostics.iter().any(|diagnostic| {
        diagnostic.code == "SPG-ADAPTER-MALFORMED-ATTRIBUTE" && diagnostic.context == "atoms"
    }));
}

#[test]
fn rust_adapter_closed_metadata_controls_cover_roles_digests_and_item_kinds() {
    let path = path("tests/controls.rs").expect("control path");
    assert_eq!(
        extract_rust(&path, &[0xff]).diagnostics[0].code,
        "SPG-ADAPTER-INVALID-UTF8"
    );
    assert_eq!(
        extract_rust(&path, b"fn {").diagnostics[0].code,
        "SPG-ADAPTER-RUST-PARSE"
    );
    let empty = extract_rust(&path, b"");
    assert!(empty.semantic.is_empty());
    assert!(empty.diagnostics.is_empty());
    assert_eq!(empty.nonnormative_items, 1);
    assert_eq!(empty.nonnormative.len(), 1);
    let external_module = extract_rust(&path, b"mod external;");
    assert_eq!(external_module.nonnormative_items, 1);
    assert!(external_module.diagnostics.is_empty());
    let verbatim_declaration = extract_rust(&path, b"fn declaration();");
    assert_eq!(verbatim_declaration.nonnormative_items, 1);
    assert!(verbatim_declaration.diagnostics.is_empty());

    for source in [
        "#[spec_provenance::carrier(syntax = \"v1\")] struct NotAFunction;",
        "#[spec_provenance::carrier(syntax = \"v1\")] fn not_a_test() {}",
        "#[spec_provenance::unknown(syntax = \"v1\", id = \"x\")] fn unknown() {}",
        "#[spec_provenance::unknown(syntax = \"v1\", body = \"ignored\")] fn unsupported() {}",
        "#[spec_provenance::implementation(syntax = \"v1\", id = \"x\")] fn missing_fields() {}",
        "#[spec_provenance::carrier(foo::bar = \"x\")] #[test] fn malformed_path() {}",
        "#[spec_provenance::carrier(syntax)] #[test] fn missing_value() {}",
        "#[spec_provenance::carrier(syntax = 1)] #[test] fn non_string_value() {}",
        "#[spec_provenance::carrier(syntax = \"v1\", syntax = \"v1\")] #[test] fn duplicate() {}",
    ] {
        assert!(
            !extract_rust(&path, source.as_bytes())
                .diagnostics
                .is_empty()
        );
    }
    let positive = String::from_utf8_lossy(include_bytes!("../fixtures/adapters/rust/positive.rs"));
    let ineligible = extract_rust(
        &path,
        positive
            .replace(
                "#[test]\nfn adapter_positive() {}",
                "struct AdapterPositive;",
            )
            .as_bytes(),
    );
    assert_eq!(
        ineligible.diagnostics[0].code,
        "SPG-ADAPTER-INELIGIBLE-CARRIER"
    );

    for role in [
        "negative",
        "property",
        "exhaustive-model",
        "formal-property",
        "structural",
    ] {
        assert_eq!(
            extract_rust(
                &path,
                positive
                    .replace("role = \"positive\"", &format!("role = \"{role}\""))
                    .as_bytes()
            )
            .semantic
            .len(),
            1,
            "{role}"
        );
    }
    assert!(
        extract_rust(
            &path,
            positive
                .replace("role = \"positive\"", "role = \"unknown\"")
                .as_bytes(),
        )
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.code == "SPG-ADAPTER-CARRIER-ROLE")
    );
    for (needle, context) in [
        ("xxh3-128:22222222222222222222222222222222", "oracle"),
        ("xxh3-128:33333333333333333333333333333333", "configuration"),
    ] {
        assert!(
            extract_rust(&path, positive.replacen(needle, "wrong", 1).as_bytes())
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.context == context)
        );
    }
    assert!(
        extract_rust(&path, b"#![allow(dead_code)]\nfn ordinary() {}\n")
            .diagnostics
            .is_empty()
    );
    let file_level_binding = extract_rust(
        &path,
        b"#![buoy::implements(\"xxh3-128:11111111111111111111111111111111\")]\n",
    );
    assert_eq!(
        file_level_binding.diagnostics[0].code,
        "SPG-ADAPTER-UNATTACHED-ATTRIBUTE"
    );
    assert!(file_level_binding.source_nodes.is_empty());
    assert_eq!(file_level_binding.nonnormative_items, 0);
    let item_kinds = r#"
#[spec_provenance::unknown(syntax = "v1", id = "c")] const C: u8 = 1;
#[spec_provenance::unknown(syntax = "v1", id = "e")] enum E { A }
#[spec_provenance::unknown(syntax = "v1", id = "x")] extern crate core;
#[spec_provenance::unknown(syntax = "v1", id = "f")] extern "C" { fn foreign(); }
#[spec_provenance::unknown(syntax = "v1", id = "i")] impl E {}
#[spec_provenance::unknown(syntax = "v1", id = "m")] macro_rules! m { () => {} }
#[spec_provenance::unknown(syntax = "v1", id = "o")] mod o {}
#[spec_provenance::unknown(syntax = "v1", id = "s")] static S: u8 = 1;
#[spec_provenance::unknown(syntax = "v1", id = "t")] struct T;
#[spec_provenance::unknown(syntax = "v1", id = "r")] trait R {}
#[spec_provenance::unknown(syntax = "v1", id = "a")] trait Alias = R;
#[spec_provenance::unknown(syntax = "v1", id = "y")] type Y = u8;
#[spec_provenance::unknown(syntax = "v1", id = "u")] union U { value: u8 }
#[spec_provenance::carrier::extra]
use std::fmt;
#[other::unknown]
fn other_attribute() {}
#[buoy::unknown]
fn buoy_non_binding_attribute() {}
#[cfg(test)]
#[spec_provenance::unknown(syntax = "v1", id = "cfg")]
fn cfg_enabled_metadata() {}
"#;
    let item_output = extract_rust(&path, item_kinds.as_bytes());
    assert_eq!(item_output.diagnostics.len(), 14);
    assert_eq!(item_output.nonnormative_items, 3);
}

#[test]
fn adapters_are_source_root_independent_and_close_through_the_same_core() {
    let markdown_path = path("spec/positive.md");
    let rust_path = path("tests/generated.rs");
    assert!(markdown_path.is_some() && rust_path.is_some());
    let (Some(markdown_path), Some(rust_path)) = (markdown_path, rust_path) else {
        return;
    };
    let markdown = extract_markdown(
        &markdown_path,
        include_bytes!("../fixtures/adapters/markdown/positive.md"),
        AuthorityExpectation::Required,
    );
    let MarkdownSemantic::Atom(atom) = markdown.semantic[0].value.clone() else {
        return;
    };
    let oracle_configuration = SemanticDigest::xxh3_128([44; 16]);
    let oracle = Oracle::new(
        sid("ORC-CROSS-001"),
        OracleKind::Test,
        test_tool(
            "cross-adapter-runner",
            "1",
            oracle_configuration,
            oracle_configuration,
        ),
        test_payload("oracle-selection/v1", Vec::new()),
        test_payload("verdict-normalization/v1", Vec::new()),
        oracle_configuration,
        test_payload("oracle-scope/v1", Vec::new()),
        Vec::new(),
    )
    .expect("complete cross-adapter oracle");
    let oracle_digest = oracle.meta().digest();
    let source = format!(
        r##"
#[spec_provenance::carrier(
 syntax = "v1",
 body = r#"
id = "CAR-CROSS-001"
role = "positive"
atoms = [{{ id = "{}", revision = "{}" }}]
oracle = {{ kind = "oracle", id = "ORC-CROSS-001", digest = "{}" }}
configuration = "xxh3-128:33333333333333333333333333333333"
corpora = []
scope = {{ schema = "spec-provenance.scope.exact-corpus/v1", bytes_hex = "636173652d61" }}
assumptions = []
exclusions = ["Network behavior is excluded."]
"#
)]
#[test]
fn cross_adapter() {{}}
"##,
        atom.meta().id().as_str(),
        atom.revision(),
        oracle_digest,
    );
    let first = extract_rust(&rust_path, source.as_bytes());
    let second = extract_rust(&rust_path, source.as_bytes());
    assert_eq!(first, second);
    assert_eq!(first.semantic.len(), 1);
    let RustSemantic::EvidenceCarrier(carrier) = &first.semantic[0].value else {
        assert!(matches!(
            first.semantic[0].value,
            RustSemantic::EvidenceCarrier(_)
        ));
        return;
    };
    let carrier = carrier.clone();
    let graph = SpecGraph {
        atoms: vec![atom.clone()],
        evidence_carriers: vec![carrier.clone()],
        oracles: vec![oracle.clone()],
        ..SpecGraph::default()
    };
    assert!(carrier_fresh(&graph, &carrier));

    let permuted = SpecGraph {
        oracles: vec![oracle],
        evidence_carriers: vec![carrier],
        atoms: vec![atom],
        ..SpecGraph::default()
    };
    assert_eq!(
        semantic_graph_digest(&graph).expect("adapter graph must encode"),
        semantic_graph_digest(&permuted).expect("permuted adapter graph must encode")
    );
}

#[test]
fn adapter_support_matrix_is_closed_unique_and_total() {
    let classes = ADAPTER_SUPPORT_MATRIX
        .iter()
        .map(|row| row.syntax_class)
        .collect::<BTreeSet<_>>();
    assert_eq!(classes.len(), ADAPTER_SUPPORT_MATRIX.len());
    assert_eq!(classes.len(), 15);
    for class in [
        AdapterSyntaxClass::MarkdownSemanticFence,
        AdapterSyntaxClass::RustSemanticAttribute,
    ] {
        assert!(ADAPTER_SUPPORT_MATRIX.iter().any(|row| {
            row.syntax_class == class && row.disposition == SyntaxDispositionKind::Semantic
        }));
    }
    for class in [
        AdapterSyntaxClass::BindingObjectRecord,
        AdapterSyntaxClass::MarkdownCommonMark,
        AdapterSyntaxClass::RustOrdinaryItem,
        AdapterSyntaxClass::RustMacroInvocation,
    ] {
        assert!(ADAPTER_SUPPORT_MATRIX.iter().any(|row| {
            row.syntax_class == class && row.disposition == SyntaxDispositionKind::Nonnormative
        }));
    }
    for class in classes {
        assert!(ADAPTER_SUPPORT_MATRIX.iter().any(|row| {
            row.syntax_class == class
                && matches!(
                    row.disposition,
                    SyntaxDispositionKind::Semantic
                        | SyntaxDispositionKind::Nonnormative
                        | SyntaxDispositionKind::Diagnostic
                )
        }));
    }
}

#[test]
fn rust_nested_items_have_exact_enclosing_identities_and_macro_tokens_stay_nonnormative() {
    let source = r##"
mod nested {
    #[spec_provenance::implementation(
        syntax = "v1",
        body = r#"
id = "IMPL-NESTED-001"
atoms = [{ id = "ATOM-NESTED-001", revision = "xxh3-128:11111111111111111111111111111111" }]
navigation = "implements"
"#
    )]
    fn implementation() {}
}

macro_rules! retained_tokens {
    ($($tokens:tt)*) => {};
}
retained_tokens! {
    #[spec_provenance::carrier(syntax = "v1", body = "not parsed as an expanded item")]
    fn generated() {}
}
"##;
    let path = path("src/nested.rs").expect("nested Rust path");
    let output = extract_rust(&path, source.as_bytes());
    assert!(output.diagnostics.is_empty());
    assert_eq!(output.semantic.len(), 1);
    assert!(output.nonnormative_items >= 2);
    assert!(output.semantic[0].item.enclosing_item().is_some());
    assert_eq!(output.semantic[0].item.path(), &path);
    let RustSemantic::ImplementationLink(link) = &output.semantic[0].value else {
        assert!(matches!(
            output.semantic[0].value,
            RustSemantic::ImplementationLink(_)
        ));
        return;
    };
    assert_eq!(link.parsed_item_digest(), output.semantic[0].item.digest());
}

fn hexd(pair: &str) -> String {
    format!("xxh3-128:{}", pair.repeat(16))
}

/// Asserts that corrupting `needle` to a malformed value inside an otherwise
/// valid Markdown fence produces at least one diagnostic, proving the
/// corrupted field's own fallible build path is reachable.
fn assert_markdown_field_error(path: &buoy::core::RepositoryPath, valid: &str, needle: &str) {
    assert!(
        valid.contains(needle),
        "fixture must contain the needle to corrupt: {needle}"
    );
    let corrupted = valid.replacen(needle, "not-a-valid-digest", 1);
    let output = extract_markdown(path, corrupted.as_bytes(), AuthorityExpectation::Optional);
    assert!(
        !output.diagnostics.is_empty(),
        "expected a diagnostic after corrupting {needle} in: {valid}"
    );
}

/// As [`assert_markdown_field_error`], but for Rust attribute bodies.
fn assert_rust_field_error(path: &buoy::core::RepositoryPath, valid: &str, needle: &str) {
    assert!(
        valid.contains(needle),
        "fixture must contain the needle to corrupt: {needle}"
    );
    let corrupted = valid.replacen(needle, "not-a-valid-digest", 1);
    let output = extract_rust(path, corrupted.as_bytes());
    assert!(
        !output.diagnostics.is_empty(),
        "expected a diagnostic after corrupting {needle} in: {valid}"
    );
}

#[test]
fn markdown_decode_errors_cover_every_remaining_semantic_kind() {
    let path = path("spec/gap.md").expect("gap control path");
    let cases = [
        r#"```spec-provenance-surface-v1
id = "SURFACE-GAP-001"
member_key_schema = { schema = "gap.member-key/v1", bytes_hex = "6b6579" }
disposition_vocabulary = { schema = "gap.disposition/v1", bytes_hex = "766f63" }
member_keys = [{ schema = "gap.member/v1", bytes_hex = "61" }]
generation = 1
policies = []
dispositions = []
unknown_field = true
```
"#
        .to_string(),
        format!(
            r#"```spec-provenance-assumption-v1
id = "ASSUMPTION-GAP-001"
statement = "Gap assumption statement."
scope = {{ schema = "gap.assumption-scope/v1", bytes_hex = "61" }}
exclusions = []
approval = {{ authority = "maintainers", record = "reviews/gap.md", digest = "{}" }}
unknown_field = true
```
"#,
            hexd("41")
        ),
        format!(
            r#"```spec-provenance-derivation-v1
id = "DERIV-GAP-001"
sources = []
targets = []
tool = {{ id = "gap-tool", version = "1.0.0", executable = "{}", configuration = "{}" }}
configuration = "{}"
oracle = {{ kind = "oracle", id = "ORACLE-GAP-001", digest = "{}" }}
relation = "generated-from"
relation_scope = {{ schema = "gap.relation-scope/v1", bytes_hex = "6578616374" }}
exclusions = []
unknown_field = true
```
"#,
            hexd("42"),
            hexd("43"),
            hexd("44"),
            hexd("45")
        ),
        r#"```spec-provenance-finite-model-v1
id = "MODEL-GAP-001"
state_space = { schema = "gap.model-state/v1", bytes_hex = "73" }
event_alphabet = { schema = "gap.model-event/v1", bytes_hex = "65" }
transition_relation = { schema = "gap.model-transition/v1", bytes_hex = "74" }
initial_states = []
bound = { schema = "gap.model-bound/v1", bytes_hex = "62" }
checked_invariants = []
assumptions = []
exclusions = []
unknown_field = true
```
"#
        .to_string(),
        format!(
            r#"```spec-provenance-proof-record-v1
id = "PROOF-GAP-001"
atoms = []
property = {{ kind = "formal-property", id = "PROPERTY-GAP-001", digest = "{}" }}
implementation = {{ kind = "repository-artifact", path = "src/gap.rs", raw_domain = "raw-artifact", media_type = "text/x-rust", representation = "saved-bytes/v1", digest = "{}" }}
assumptions = []
translators = []
verifier = {{ id = "gap-verifier", version = "1.0.0", executable = "{}", configuration = "{}" }}
verifier_configuration = "{}"
external_models = []
generated = []
proof_sources = []
verdict = "pass"
scope = {{ schema = "gap.proof-scope/v1", bytes_hex = "6b" }}
exclusions = []
unknown_field = true
```
"#,
            hexd("46"),
            hexd("47"),
            hexd("48"),
            hexd("49"),
            hexd("4a")
        ),
        format!(
            r#"```spec-provenance-reaffirmation-v1
id = "REAFFIRM-GAP-001"
affected_object_kind = "atom"
affected_object_id = "ATOM-GAP-001"
base_object_digest = "{}"
head_object_digest = "{}"
review_approval = {{ authority = "maintainers", record = "reviews/gap.md", digest = "{}" }}
review_statement = "Gap reaffirmation statement."
unknown_field = true
```
"#,
            hexd("4b"),
            hexd("4c"),
            hexd("4d")
        ),
        format!(
            r#"```spec-provenance-formal-property-v1
id = "PROPERTY-GAP-002"
atoms = []
property = {{ schema = "gap.formal-property/v1", bytes_hex = "70" }}
implementation_abstraction = {{ schema = "gap.implementation-abstraction/v1", bytes_hex = "69" }}
assumptions = []
exclusions = []
correspondence_approval = {{ authority = "maintainers", record = "reviews/gap.md", digest = "{}" }}
unknown_field = true
```
"#,
            hexd("4e")
        ),
        r#"```spec-provenance-debt-v1
id = "DEBT-GAP-001"
obligations = []
objects = []
rationale = "Gap debt rationale."
owner = "gap-owner"
remediation = "Gap remediation."
discharge_condition = { schema = "gap.discharge/v1", bytes_hex = "72" }
external_references = []
lifecycle = "active"
discharge_evidence = []
replacements = []
unknown_field = true
```
"#
        .to_string(),
        format!(
            r#"```spec-provenance-waiver-v1
id = "WAIVER-GAP-001"
obligations = []
objects = []
rationale = "Gap waiver rationale."
owner = "gap-owner"
discharge_condition = {{ schema = "gap.discharge/v1", bytes_hex = "77" }}
active_approval = {{ authority = "maintainers", record = "reviews/gap.md", digest = "{}" }}
external_references = []
lifecycle = "active"
discharge_evidence = []
replacements = []
unknown_field = true
```
"#,
            hexd("4f")
        ),
    ];
    for source in cases {
        let output = extract_markdown(&path, source.as_bytes(), AuthorityExpectation::Optional);
        assert!(
            output
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == "SPG-ADAPTER-UNSUPPORTED-FIELD"),
            "expected an unsupported-field diagnostic for: {source}"
        );
    }
}

#[test]
fn markdown_surface_completeness_rejects_length_and_key_mismatches() {
    let path = path("spec/gap-surface.md").expect("gap surface control path");
    let length_mismatch = format!(
        r#"```spec-provenance-surface-v1
id = "SURFACE-GAP-LEN-001"
member_key_schema = {{ schema = "gap.member-key/v1", bytes_hex = "6b6579" }}
disposition_vocabulary = {{ schema = "gap.disposition/v1", bytes_hex = "766f63" }}
member_keys = [{{ schema = "gap.member/v1", bytes_hex = "61" }}, {{ schema = "gap.member/v1", bytes_hex = "62" }}]
generation = 1
policies = []
dispositions = [{{ key = {{ schema = "gap.member/v1", bytes_hex = "61" }}, disposition = "normative", payload = {{ schema = "gap.disposition/v1", bytes_hex = "6e" }}, authority = {{ id = "ATOM-GAP-001", revision = "{}" }} }}]
```
"#,
        hexd("50")
    );
    let key_mismatch = format!(
        r#"```spec-provenance-surface-v1
id = "SURFACE-GAP-KEY-001"
member_key_schema = {{ schema = "gap.member-key/v1", bytes_hex = "6b6579" }}
disposition_vocabulary = {{ schema = "gap.disposition/v1", bytes_hex = "766f63" }}
member_keys = [{{ schema = "gap.member/v1", bytes_hex = "61" }}]
generation = 1
policies = []
dispositions = [{{ key = {{ schema = "gap.member/v1", bytes_hex = "62" }}, disposition = "normative", payload = {{ schema = "gap.disposition/v1", bytes_hex = "6e" }}, authority = {{ id = "ATOM-GAP-001", revision = "{}" }} }}]
```
"#,
        hexd("51")
    );
    for source in [length_mismatch, key_mismatch] {
        let output = extract_markdown(&path, source.as_bytes(), AuthorityExpectation::Optional);
        assert!(
            output.diagnostics.iter().any(|diagnostic| {
                diagnostic.code == "SPG-ADAPTER-MALFORMED-OBJECT"
                    && diagnostic.context == "surface-completeness"
            }),
            "expected a surface-completeness diagnostic for: {source}"
        );
    }
}

#[test]
fn markdown_object_kind_covers_every_variant_and_rejects_unknown() {
    let path = path("spec/gap-kind.md").expect("gap kind control path");
    let every_kind = format!(
        r#"```spec-provenance-debt-v1
id = "DEBT-GAP-KIND-001"
obligations = [{{ policy_id = "POL-GAP-001", kind = "freshness", subject = {{ kind = "atom", id = "ATOM-GAP-001", digest = "{}" }} }}]
objects = [
{{ kind = "proof-record", id = "X-1", digest = "{}" }},
{{ kind = "parsed-item", id = "X-2", digest = "{}" }},
{{ kind = "reaffirmation", id = "X-3", digest = "{}" }},
{{ kind = "change-manifest", id = "X-4", digest = "{}" }},
{{ kind = "change-disposition", id = "X-5", digest = "{}" }},
]
rationale = "Gap debt covering every object kind."
owner = "gap-owner"
remediation = "Gap remediation."
discharge_condition = {{ schema = "gap.discharge/v1", bytes_hex = "72" }}
external_references = []
lifecycle = "active"
discharge_evidence = []
replacements = []
```
"#,
        hexd("62"),
        hexd("52"),
        hexd("53"),
        hexd("54"),
        hexd("55"),
        hexd("56")
    );
    let output = extract_markdown(&path, every_kind.as_bytes(), AuthorityExpectation::Optional);
    assert_eq!(output.diagnostics, Vec::new());
    assert_eq!(output.semantic.len(), 1);

    let unknown_kind = format!(
        r#"```spec-provenance-debt-v1
id = "DEBT-GAP-KIND-002"
obligations = []
objects = [{{ kind = "not-a-real-kind", id = "X-1", digest = "{}" }}]
rationale = "Gap debt with an invalid object kind."
owner = "gap-owner"
remediation = "Gap remediation."
discharge_condition = {{ schema = "gap.discharge/v1", bytes_hex = "72" }}
external_references = []
lifecycle = "active"
discharge_evidence = []
replacements = []
```
"#,
        hexd("57")
    );
    let output = extract_markdown(
        &path,
        unknown_kind.as_bytes(),
        AuthorityExpectation::Optional,
    );
    assert!(output.diagnostics.iter().any(|diagnostic| {
        diagnostic.code == "SPG-ADAPTER-MALFORMED-OBJECT" && diagnostic.context == "object-kind"
    }));
}

#[test]
fn markdown_payload_hex_rejects_odd_length_and_invalid_characters() {
    let path = path("spec/gap-hex.md").expect("gap hex control path");
    let odd_length = r#"```spec-provenance-debt-v1
id = "DEBT-GAP-HEX-001"
obligations = []
objects = []
rationale = "Gap debt with odd-length hex."
owner = "gap-owner"
remediation = "Gap remediation."
discharge_condition = { schema = "gap.discharge/v1", bytes_hex = "abc" }
external_references = []
lifecycle = "active"
discharge_evidence = []
replacements = []
```
"#;
    let invalid_character = r#"```spec-provenance-debt-v1
id = "DEBT-GAP-HEX-002"
obligations = []
objects = []
rationale = "Gap debt with an invalid hex character."
owner = "gap-owner"
remediation = "Gap remediation."
discharge_condition = { schema = "gap.discharge/v1", bytes_hex = "zz" }
external_references = []
lifecycle = "active"
discharge_evidence = []
replacements = []
```
"#;
    for source in [odd_length, invalid_character] {
        let output = extract_markdown(&path, source.as_bytes(), AuthorityExpectation::Optional);
        assert!(
            output.diagnostics.iter().any(|diagnostic| {
                diagnostic.code == "SPG-ADAPTER-MALFORMED-OBJECT" && diagnostic.context == "hex"
            }),
            "expected a hex diagnostic for: {source}"
        );
    }
}

#[test]
fn markdown_policy_authority_selectors_cover_expectations_and_reject_invalid_root_and_kind() {
    let path = path("spec/gap-policy.md").expect("gap policy control path");
    let every_expectation = format!(
        r#"```spec-provenance-policy-v1
id = "POL-GAP-001"
scope = {{ schema = "gap.policy-scope/v1", bytes_hex = "616c6c" }}
required_roles = ["positive"]
enforcement = "blocking"
ratchet_id = "gap-ratchet"
authority_selectors = [
{{ root = "spec", adapter_id = "markdown/v1", include_patterns = ["**"], exclude_patterns = [], authority_expectation = "forbidden" }},
{{ root = "spec", adapter_id = "markdown/v1", include_patterns = ["**"], exclude_patterns = [], authority_expectation = "optional" }},
{{ root = "spec", adapter_id = "markdown/v1", include_patterns = ["**"], exclude_patterns = [], authority_expectation = "required" }},
]
active_approval = {{ authority = "maintainers", record = "reviews/policy.md", digest = "{}" }}
lifecycle = "active"
replacements = []
```
"#,
        hexd("58")
    );
    let output = extract_markdown(
        &path,
        every_expectation.as_bytes(),
        AuthorityExpectation::Optional,
    );
    assert_eq!(output.diagnostics, Vec::new());
    assert_eq!(output.semantic.len(), 1);

    let invalid_root = format!(
        r#"```spec-provenance-policy-v1
id = "POL-GAP-002"
scope = {{ schema = "gap.policy-scope/v1", bytes_hex = "616c6c" }}
required_roles = ["positive"]
enforcement = "blocking"
ratchet_id = "gap-ratchet"
authority_selectors = [{{ root = "/absolute", adapter_id = "markdown/v1", include_patterns = ["**"], exclude_patterns = [], authority_expectation = "optional" }}]
active_approval = {{ authority = "maintainers", record = "reviews/policy.md", digest = "{}" }}
lifecycle = "active"
replacements = []
```
"#,
        hexd("59")
    );
    let output = extract_markdown(
        &path,
        invalid_root.as_bytes(),
        AuthorityExpectation::Optional,
    );
    assert!(output.diagnostics.iter().any(|diagnostic| {
        diagnostic.code == "SPG-ADAPTER-MALFORMED-OBJECT"
            && diagnostic.context == "DET-PATH-NOT-RELATIVE"
    }));

    let invalid_expectation = format!(
        r#"```spec-provenance-policy-v1
id = "POL-GAP-003"
scope = {{ schema = "gap.policy-scope/v1", bytes_hex = "616c6c" }}
required_roles = ["positive"]
enforcement = "blocking"
ratchet_id = "gap-ratchet"
authority_selectors = [{{ root = "spec", adapter_id = "markdown/v1", include_patterns = ["**"], exclude_patterns = [], authority_expectation = "sometimes" }}]
active_approval = {{ authority = "maintainers", record = "reviews/policy.md", digest = "{}" }}
lifecycle = "active"
replacements = []
```
"#,
        hexd("5a")
    );
    let output = extract_markdown(
        &path,
        invalid_expectation.as_bytes(),
        AuthorityExpectation::Optional,
    );
    assert!(output.diagnostics.iter().any(|diagnostic| {
        diagnostic.code == "SPG-ADAPTER-MALFORMED-OBJECT"
            && diagnostic.context == "authority-expectation"
    }));
}

#[test]
fn markdown_derivation_rejects_an_invalid_repository_artifact_path() {
    let path = path("spec/gap-derivation.md").expect("gap derivation control path");
    let invalid_path = format!(
        r#"```spec-provenance-derivation-v1
id = "DERIV-GAP-PATH-001"
sources = [{{ kind = "repository-artifact", path = "/absolute", raw_domain = "raw-artifact", media_type = "text/x-rust", representation = "saved-bytes/v1", digest = "{}" }}]
targets = []
tool = {{ id = "gap-tool", version = "1.0.0", executable = "{}", configuration = "{}" }}
configuration = "{}"
oracle = {{ kind = "oracle", id = "ORACLE-GAP-PATH-001", digest = "{}" }}
relation = "generated-from"
relation_scope = {{ schema = "gap.relation-scope/v1", bytes_hex = "6578616374" }}
exclusions = []
```
"#,
        hexd("5b"),
        hexd("5c"),
        hexd("5d"),
        hexd("5e"),
        hexd("5f")
    );
    let output = extract_markdown(
        &path,
        invalid_path.as_bytes(),
        AuthorityExpectation::Optional,
    );
    assert!(output.diagnostics.iter().any(|diagnostic| {
        diagnostic.code == "SPG-ADAPTER-MALFORMED-OBJECT"
            && diagnostic.context == "DET-PATH-NOT-RELATIVE"
    }));
}

#[test]
fn markdown_closed_vocabulary_variants_are_all_reachable() {
    let path = path("spec/gap-vocabulary.md").expect("gap vocabulary control path");

    let surface_dispositions = format!(
        r#"```spec-provenance-surface-v1
id = "SURFACE-VOCAB-001"
member_key_schema = {{ schema = "gap.member-key/v1", bytes_hex = "6b6579" }}
disposition_vocabulary = {{ schema = "gap.disposition/v1", bytes_hex = "766f63" }}
member_keys = [{{ schema = "gap.member/v1", bytes_hex = "61" }}, {{ schema = "gap.member/v1", bytes_hex = "62" }}, {{ schema = "gap.member/v1", bytes_hex = "63" }}, {{ schema = "gap.member/v1", bytes_hex = "64" }}]
generation = 1
policies = []
dispositions = [
{{ key = {{ schema = "gap.member/v1", bytes_hex = "61" }}, disposition = "normative", payload = {{ schema = "gap.disposition/v1", bytes_hex = "6e" }}, authority = {{ id = "ATOM-VOCAB-001", revision = "{}" }} }},
{{ key = {{ schema = "gap.member/v1", bytes_hex = "62" }}, disposition = "prohibited", payload = {{ schema = "gap.disposition/v1", bytes_hex = "70" }}, authority = {{ id = "ATOM-VOCAB-001", revision = "{}" }} }},
{{ key = {{ schema = "gap.member/v1", bytes_hex = "63" }}, disposition = "deferred", payload = {{ schema = "gap.disposition/v1", bytes_hex = "64" }}, authority = {{ id = "ATOM-VOCAB-001", revision = "{}" }} }},
{{ key = {{ schema = "gap.member/v1", bytes_hex = "64" }}, disposition = "not-applicable" }},
]
```
"#,
        hexd("63"),
        hexd("63"),
        hexd("63")
    );
    let output = extract_markdown(
        &path,
        surface_dispositions.as_bytes(),
        AuthorityExpectation::Optional,
    );
    assert_eq!(output.diagnostics, Vec::new());
    assert_eq!(output.semantic.len(), 1);

    let derivation_object_variant_and_structural_equivalence = format!(
        r#"```spec-provenance-derivation-v1
id = "DERIV-VOCAB-001"
sources = [{{ kind = "object", object = {{ kind = "atom", id = "ATOM-VOCAB-001", digest = "{}" }} }}]
targets = [{{ kind = "repository-artifact", path = "generated/vocab-output.rs", raw_domain = "raw-artifact", media_type = "text/x-rust", representation = "saved-bytes/v1", digest = "{}" }}]
tool = {{ id = "vocab-tool", version = "1.0.0", executable = "{}", configuration = "{}" }}
configuration = "{}"
oracle = {{ kind = "oracle", id = "ORACLE-VOCAB-001", digest = "{}" }}
relation = "structurally-equivalent"
relation_scope = {{ schema = "gap.relation-scope/v1", bytes_hex = "6578616374" }}
exclusions = []
```
"#,
        hexd("64"),
        hexd("93"),
        hexd("65"),
        hexd("66"),
        hexd("67"),
        hexd("68")
    );
    let output = extract_markdown(
        &path,
        derivation_object_variant_and_structural_equivalence.as_bytes(),
        AuthorityExpectation::Optional,
    );
    assert_eq!(output.diagnostics, Vec::new());
    assert_eq!(output.semantic.len(), 1);

    let derivation_host_consistency = format!(
        r#"```spec-provenance-derivation-v1
id = "DERIV-VOCAB-002"
sources = [{{ kind = "repository-artifact", path = "src/vocab.rs", raw_domain = "raw-artifact", media_type = "text/x-rust", representation = "saved-bytes/v1", digest = "{}" }}]
targets = [{{ kind = "repository-artifact", path = "generated/vocab-output-2.rs", raw_domain = "raw-artifact", media_type = "text/x-rust", representation = "saved-bytes/v1", digest = "{}" }}]
tool = {{ id = "vocab-tool", version = "1.0.0", executable = "{}", configuration = "{}" }}
configuration = "{}"
oracle = {{ kind = "oracle", id = "ORACLE-VOCAB-002", digest = "{}" }}
relation = "host-consistency"
relation_scope = {{ schema = "gap.relation-scope/v1", bytes_hex = "6578616374" }}
exclusions = []
```
"#,
        hexd("69"),
        hexd("94"),
        hexd("6a"),
        hexd("6b"),
        hexd("6c"),
        hexd("6d")
    );
    let output = extract_markdown(
        &path,
        derivation_host_consistency.as_bytes(),
        AuthorityExpectation::Optional,
    );
    assert_eq!(output.diagnostics, Vec::new());
    assert_eq!(output.semantic.len(), 1);

    let obligation_kind_extra_variants = format!(
        r#"```spec-provenance-debt-v1
id = "DEBT-VOCAB-001"
obligations = [
{{ policy_id = "POL-VOCAB-001", kind = "derivation-consistency", subject = {{ kind = "atom", id = "ATOM-VOCAB-V1", digest = "{}" }} }},
{{ policy_id = "POL-VOCAB-001", kind = "proof", subject = {{ kind = "atom", id = "ATOM-VOCAB-V2", digest = "{}" }} }},
{{ policy_id = "POL-VOCAB-001", kind = "change-impact", subject = {{ kind = "atom", id = "ATOM-VOCAB-V3", digest = "{}" }} }},
]
objects = []
rationale = "Gap debt covering the remaining obligation kinds."
owner = "gap-owner"
remediation = "Gap remediation."
discharge_condition = {{ schema = "gap.discharge/v1", bytes_hex = "72" }}
external_references = []
lifecycle = "active"
discharge_evidence = []
replacements = []
```
"#,
        hexd("6e"),
        hexd("6f"),
        hexd("70")
    );
    let output = extract_markdown(
        &path,
        obligation_kind_extra_variants.as_bytes(),
        AuthorityExpectation::Optional,
    );
    assert_eq!(output.diagnostics, Vec::new());
    assert_eq!(output.semantic.len(), 1);

    let obligation_kind_invalid = format!(
        r#"```spec-provenance-debt-v1
id = "DEBT-VOCAB-002"
obligations = [{{ policy_id = "POL-VOCAB-001", kind = "not-a-real-obligation-kind", subject = {{ kind = "atom", id = "ATOM-VOCAB-V4", digest = "{}" }} }}]
objects = []
rationale = "Gap debt with an invalid obligation kind."
owner = "gap-owner"
remediation = "Gap remediation."
discharge_condition = {{ schema = "gap.discharge/v1", bytes_hex = "72" }}
external_references = []
lifecycle = "active"
discharge_evidence = []
replacements = []
```
"#,
        hexd("71")
    );
    let output = extract_markdown(
        &path,
        obligation_kind_invalid.as_bytes(),
        AuthorityExpectation::Optional,
    );
    assert!(output.diagnostics.iter().any(|diagnostic| {
        diagnostic.code == "SPG-ADAPTER-MALFORMED-OBJECT" && diagnostic.context == "obligation-kind"
    }));

    let debt_discharged = format!(
        r#"```spec-provenance-debt-v1
id = "DEBT-VOCAB-DISCHARGED-001"
obligations = [{{ policy_id = "POL-VOCAB-001", kind = "freshness", subject = {{ kind = "atom", id = "ATOM-VOCAB-V5", digest = "{}" }} }}]
objects = []
rationale = "Gap discharged debt."
owner = "gap-owner"
remediation = "Gap remediation."
discharge_condition = {{ schema = "gap.discharge/v1", bytes_hex = "72" }}
external_references = []
lifecycle = "discharged"
discharge_evidence = [{{ kind = "atom", id = "ATOM-VOCAB-V6", digest = "{}" }}]
replacements = []
terminal_approval = {{ authority = "maintainers", record = "reviews/vocab.md", digest = "{}" }}
```
"#,
        hexd("72"),
        hexd("73"),
        hexd("74")
    );
    let output = extract_markdown(
        &path,
        debt_discharged.as_bytes(),
        AuthorityExpectation::Optional,
    );
    assert_eq!(output.diagnostics, Vec::new());
    assert_eq!(output.semantic.len(), 1);

    let debt_superseded = format!(
        r#"```spec-provenance-debt-v1
id = "DEBT-VOCAB-SUPERSEDED-001"
obligations = [{{ policy_id = "POL-VOCAB-001", kind = "freshness", subject = {{ kind = "atom", id = "ATOM-VOCAB-V7", digest = "{}" }} }}]
objects = []
rationale = "Gap superseded debt."
owner = "gap-owner"
remediation = "Gap remediation."
discharge_condition = {{ schema = "gap.discharge/v1", bytes_hex = "72" }}
external_references = []
lifecycle = "superseded"
discharge_evidence = []
replacements = [{{ kind = "debt-record", id = "DEBT-VOCAB-OTHER-001", digest = "{}" }}]
terminal_approval = {{ authority = "maintainers", record = "reviews/vocab.md", digest = "{}" }}
```
"#,
        hexd("75"),
        hexd("76"),
        hexd("77")
    );
    let output = extract_markdown(
        &path,
        debt_superseded.as_bytes(),
        AuthorityExpectation::Optional,
    );
    assert_eq!(output.diagnostics, Vec::new());
    assert_eq!(output.semantic.len(), 1);

    let waiver_discharged = format!(
        r#"```spec-provenance-waiver-v1
id = "WAIVER-VOCAB-DISCHARGED-001"
obligations = [{{ policy_id = "POL-VOCAB-001", kind = "freshness", subject = {{ kind = "atom", id = "ATOM-VOCAB-V8", digest = "{}" }} }}]
objects = []
rationale = "Gap discharged waiver."
owner = "gap-owner"
discharge_condition = {{ schema = "gap.discharge/v1", bytes_hex = "77" }}
active_approval = {{ authority = "maintainers", record = "reviews/vocab.md", digest = "{}" }}
external_references = []
lifecycle = "discharged"
discharge_evidence = [{{ kind = "atom", id = "ATOM-VOCAB-V9", digest = "{}" }}]
replacements = []
terminal_approval = {{ authority = "maintainers", record = "reviews/vocab.md", digest = "{}" }}
```
"#,
        hexd("78"),
        hexd("79"),
        hexd("7a"),
        hexd("90")
    );
    let output = extract_markdown(
        &path,
        waiver_discharged.as_bytes(),
        AuthorityExpectation::Optional,
    );
    assert_eq!(output.diagnostics, Vec::new());
    assert_eq!(output.semantic.len(), 1);

    let waiver_revoked = format!(
        r#"```spec-provenance-waiver-v1
id = "WAIVER-VOCAB-REVOKED-001"
obligations = [{{ policy_id = "POL-VOCAB-001", kind = "freshness", subject = {{ kind = "atom", id = "ATOM-VOCAB-VA", digest = "{}" }} }}]
objects = []
rationale = "Gap revoked waiver."
owner = "gap-owner"
discharge_condition = {{ schema = "gap.discharge/v1", bytes_hex = "77" }}
active_approval = {{ authority = "maintainers", record = "reviews/vocab.md", digest = "{}" }}
external_references = []
lifecycle = "revoked"
discharge_evidence = []
replacements = []
terminal_approval = {{ authority = "maintainers", record = "reviews/vocab.md", digest = "{}" }}
```
"#,
        hexd("7b"),
        hexd("7c"),
        hexd("91")
    );
    let output = extract_markdown(
        &path,
        waiver_revoked.as_bytes(),
        AuthorityExpectation::Optional,
    );
    assert_eq!(output.diagnostics, Vec::new());
    assert_eq!(output.semantic.len(), 1);

    let waiver_superseded = format!(
        r#"```spec-provenance-waiver-v1
id = "WAIVER-VOCAB-SUPERSEDED-001"
obligations = [{{ policy_id = "POL-VOCAB-001", kind = "freshness", subject = {{ kind = "atom", id = "ATOM-VOCAB-VB", digest = "{}" }} }}]
objects = []
rationale = "Gap superseded waiver."
owner = "gap-owner"
discharge_condition = {{ schema = "gap.discharge/v1", bytes_hex = "77" }}
active_approval = {{ authority = "maintainers", record = "reviews/vocab.md", digest = "{}" }}
external_references = []
lifecycle = "superseded"
discharge_evidence = []
replacements = [{{ kind = "waiver", id = "WAIVER-VOCAB-OTHER-001", digest = "{}" }}]
terminal_approval = {{ authority = "maintainers", record = "reviews/vocab.md", digest = "{}" }}
```
"#,
        hexd("7d"),
        hexd("7e"),
        hexd("7f"),
        hexd("92")
    );
    let output = extract_markdown(
        &path,
        waiver_superseded.as_bytes(),
        AuthorityExpectation::Optional,
    );
    assert_eq!(output.diagnostics, Vec::new());
    assert_eq!(output.semantic.len(), 1);

    let policy_advisory = format!(
        r#"```spec-provenance-policy-v1
id = "POL-VOCAB-ADVISORY-001"
scope = {{ schema = "gap.policy-scope/v1", bytes_hex = "616c6c" }}
required_roles = ["positive"]
enforcement = "advisory"
ratchet_id = "gap-ratchet"
authority_selectors = []
active_approval = {{ authority = "maintainers", record = "reviews/vocab.md", digest = "{}" }}
lifecycle = "active"
replacements = []
```
"#,
        hexd("80")
    );
    let output = extract_markdown(
        &path,
        policy_advisory.as_bytes(),
        AuthorityExpectation::Optional,
    );
    assert_eq!(output.diagnostics, Vec::new());
    assert_eq!(output.semantic.len(), 1);

    let policy_enforcement_invalid = format!(
        r#"```spec-provenance-policy-v1
id = "POL-VOCAB-ENFORCEMENT-001"
scope = {{ schema = "gap.policy-scope/v1", bytes_hex = "616c6c" }}
required_roles = ["positive"]
enforcement = "sometimes"
ratchet_id = "gap-ratchet"
authority_selectors = []
active_approval = {{ authority = "maintainers", record = "reviews/vocab.md", digest = "{}" }}
lifecycle = "active"
replacements = []
```
"#,
        hexd("81")
    );
    let output = extract_markdown(
        &path,
        policy_enforcement_invalid.as_bytes(),
        AuthorityExpectation::Optional,
    );
    assert!(output.diagnostics.iter().any(|diagnostic| {
        diagnostic.code == "SPG-ADAPTER-MALFORMED-OBJECT" && diagnostic.context == "enforcement"
    }));

    let policy_superseded = format!(
        r#"```spec-provenance-policy-v1
id = "POL-VOCAB-SUPERSEDED-001"
scope = {{ schema = "gap.policy-scope/v1", bytes_hex = "616c6c" }}
required_roles = ["positive"]
enforcement = "blocking"
ratchet_id = "gap-ratchet"
authority_selectors = []
active_approval = {{ authority = "maintainers", record = "reviews/vocab.md", digest = "{}" }}
lifecycle = "superseded"
replacements = [{{ kind = "coverage-policy", id = "POL-VOCAB-OTHER-001", digest = "{}" }}]
terminal_approval = {{ authority = "maintainers", record = "reviews/vocab.md", digest = "{}" }}
```
"#,
        hexd("82"),
        hexd("83"),
        hexd("84")
    );
    let output = extract_markdown(
        &path,
        policy_superseded.as_bytes(),
        AuthorityExpectation::Optional,
    );
    assert_eq!(output.diagnostics, Vec::new());
    assert_eq!(output.semantic.len(), 1);

    let policy_retired = format!(
        r#"```spec-provenance-policy-v1
id = "POL-VOCAB-RETIRED-001"
scope = {{ schema = "gap.policy-scope/v1", bytes_hex = "616c6c" }}
required_roles = ["positive"]
enforcement = "blocking"
ratchet_id = "gap-ratchet"
authority_selectors = []
active_approval = {{ authority = "maintainers", record = "reviews/vocab.md", digest = "{}" }}
lifecycle = "retired"
replacements = []
terminal_approval = {{ authority = "maintainers", record = "reviews/vocab.md", digest = "{}" }}
```
"#,
        hexd("85"),
        hexd("86")
    );
    let output = extract_markdown(
        &path,
        policy_retired.as_bytes(),
        AuthorityExpectation::Optional,
    );
    assert_eq!(output.diagnostics, Vec::new());
    assert_eq!(output.semantic.len(), 1);

    let proof_record = |verdict: &str, id: &str, pair: &str| {
        format!(
            r#"```spec-provenance-proof-record-v1
id = "{id}"
atoms = [{{ id = "ATOM-VOCAB-001", revision = "{}" }}]
property = {{ kind = "formal-property", id = "PROPERTY-VOCAB-001", digest = "{}" }}
implementation = {{ kind = "repository-artifact", path = "src/vocab-proof.rs", raw_domain = "raw-artifact", media_type = "text/x-rust", representation = "saved-bytes/v1", digest = "{}" }}
assumptions = []
translators = []
verifier = {{ id = "vocab-verifier", version = "1.0.0", executable = "{}", configuration = "{}" }}
verifier_configuration = "{}"
external_models = []
generated = []
proof_sources = [{{ kind = "repository-artifact", path = "proofs/vocab.rs", raw_domain = "raw-artifact", media_type = "text/x-rust", representation = "saved-bytes/v1", digest = "{}" }}]
verdict = "{verdict}"
scope = {{ schema = "gap.proof-scope/v1", bytes_hex = "6b" }}
exclusions = []
```
"#,
            hexd(pair),
            hexd(pair),
            hexd(pair),
            hexd(pair),
            hexd(pair),
            hexd(pair),
            hexd(pair),
        )
    };
    for (verdict, pair) in [("fail", "87"), ("error", "88"), ("timeout", "89")] {
        let source = proof_record(
            verdict,
            &format!("PROOF-VOCAB-{}", verdict.to_uppercase()),
            pair,
        );
        let output = extract_markdown(&path, source.as_bytes(), AuthorityExpectation::Optional);
        assert_eq!(output.diagnostics, Vec::new(), "verdict {verdict}");
        assert_eq!(output.semantic.len(), 1, "verdict {verdict}");
    }
    let invalid_verdict = proof_record("sometimes", "PROOF-VOCAB-INVALID", "8a");
    let output = extract_markdown(
        &path,
        invalid_verdict.as_bytes(),
        AuthorityExpectation::Optional,
    );
    assert!(output.diagnostics.iter().any(|diagnostic| {
        diagnostic.code == "SPG-ADAPTER-MALFORMED-OBJECT" && diagnostic.context == "verdict"
    }));

    let surface_disposition_invalid = r#"```spec-provenance-surface-v1
id = "SURFACE-VOCAB-INVALID"
member_key_schema = { schema = "gap.member-key/v1", bytes_hex = "6b6579" }
disposition_vocabulary = { schema = "gap.disposition/v1", bytes_hex = "766f63" }
member_keys = [{ schema = "gap.member/v1", bytes_hex = "61" }]
generation = 1
policies = []
dispositions = [{ key = { schema = "gap.member/v1", bytes_hex = "61" }, disposition = "sometimes" }]
```
"#;
    let output = extract_markdown(
        &path,
        surface_disposition_invalid.as_bytes(),
        AuthorityExpectation::Optional,
    );
    assert!(output.diagnostics.iter().any(|diagnostic| {
        diagnostic.code == "SPG-ADAPTER-MALFORMED-OBJECT"
            && diagnostic.context == "surface-disposition"
    }));

    let policy_lifecycle_invalid = format!(
        r#"```spec-provenance-policy-v1
id = "POL-VOCAB-LIFECYCLE-INVALID"
scope = {{ schema = "gap.policy-scope/v1", bytes_hex = "616c6c" }}
required_roles = ["positive"]
enforcement = "blocking"
ratchet_id = "gap-ratchet"
authority_selectors = []
active_approval = {{ authority = "maintainers", record = "reviews/vocab.md", digest = "{}" }}
lifecycle = "sometimes"
replacements = []
```
"#,
        hexd("9e")
    );
    let output = extract_markdown(
        &path,
        policy_lifecycle_invalid.as_bytes(),
        AuthorityExpectation::Optional,
    );
    assert!(output.diagnostics.iter().any(|diagnostic| {
        diagnostic.code == "SPG-ADAPTER-MALFORMED-OBJECT"
            && diagnostic.context == "policy-lifecycle"
    }));

    let debt_lifecycle_invalid = r#"```spec-provenance-debt-v1
id = "DEBT-VOCAB-LIFECYCLE-INVALID"
obligations = []
objects = []
rationale = "Gap debt with an invalid lifecycle."
owner = "gap-owner"
remediation = "Gap remediation."
discharge_condition = { schema = "gap.discharge/v1", bytes_hex = "72" }
external_references = []
lifecycle = "sometimes"
discharge_evidence = []
replacements = []
```
"#;
    let output = extract_markdown(
        &path,
        debt_lifecycle_invalid.as_bytes(),
        AuthorityExpectation::Optional,
    );
    assert!(output.diagnostics.iter().any(|diagnostic| {
        diagnostic.code == "SPG-ADAPTER-MALFORMED-OBJECT" && diagnostic.context == "debt-lifecycle"
    }));

    let waiver_lifecycle_invalid = format!(
        r#"```spec-provenance-waiver-v1
id = "WAIVER-VOCAB-LIFECYCLE-INVALID"
obligations = []
objects = []
rationale = "Gap waiver with an invalid lifecycle."
owner = "gap-owner"
discharge_condition = {{ schema = "gap.discharge/v1", bytes_hex = "77" }}
active_approval = {{ authority = "maintainers", record = "reviews/vocab.md", digest = "{}" }}
external_references = []
lifecycle = "sometimes"
discharge_evidence = []
replacements = []
```
"#,
        hexd("9f")
    );
    let output = extract_markdown(
        &path,
        waiver_lifecycle_invalid.as_bytes(),
        AuthorityExpectation::Optional,
    );
    assert!(output.diagnostics.iter().any(|diagnostic| {
        diagnostic.code == "SPG-ADAPTER-MALFORMED-OBJECT"
            && diagnostic.context == "waiver-lifecycle"
    }));

    let derivation_relation_invalid = format!(
        r#"```spec-provenance-derivation-v1
id = "DERIV-VOCAB-RELATION-INVALID"
sources = []
targets = []
tool = {{ id = "vocab-tool", version = "1.0.0", executable = "{}", configuration = "{}" }}
configuration = "{}"
oracle = {{ kind = "oracle", id = "ORACLE-VOCAB-RELATION-INVALID", digest = "{}" }}
relation = "sometimes"
relation_scope = {{ schema = "gap.relation-scope/v1", bytes_hex = "6578616374" }}
exclusions = []
```
"#,
        hexd("a1"),
        hexd("a2"),
        hexd("a3"),
        hexd("a4")
    );
    let output = extract_markdown(
        &path,
        derivation_relation_invalid.as_bytes(),
        AuthorityExpectation::Optional,
    );
    assert!(output.diagnostics.iter().any(|diagnostic| {
        diagnostic.code == "SPG-ADAPTER-MALFORMED-OBJECT"
            && diagnostic.context == "derivation-relation"
    }));
}

#[test]
fn rust_implementation_navigation_covers_every_remaining_variant() {
    let path = path("tests/gap-navigation.rs").expect("gap navigation control path");
    for (navigation, id) in [
        ("owns", "IMPL-VOCAB-OWNS"),
        ("consumes", "IMPL-VOCAB-CONSUMES"),
        ("configures", "IMPL-VOCAB-CONFIGURES"),
    ] {
        let source = format!(
            r##"
#[spec_provenance::implementation(
 syntax = "v1",
 body = r#"
id = "{id}"
atoms = [{{ id = "ATOM-VOCAB-NAV", revision = "{}" }}]
navigation = "{navigation}"
"#
)]
fn gap_navigation() {{}}
"##,
            hexd("8b")
        );
        let output = extract_rust(&path, source.as_bytes());
        assert!(output.diagnostics.is_empty(), "navigation {navigation}");
        assert_eq!(output.semantic.len(), 1, "navigation {navigation}");
    }

    let invalid_navigation = format!(
        r##"
#[spec_provenance::implementation(
 syntax = "v1",
 body = r#"
id = "IMPL-VOCAB-INVALID"
atoms = [{{ id = "ATOM-VOCAB-NAV", revision = "{}" }}]
navigation = "sometimes"
"#
)]
fn gap_invalid_navigation() {{}}
"##,
        hexd("8c")
    );
    let output = extract_rust(&path, invalid_navigation.as_bytes());
    assert!(output.diagnostics.iter().any(|diagnostic| {
        diagnostic.code == "SPG-ADAPTER-MALFORMED-OBJECT" && diagnostic.context == "navigation"
    }));

    let bad_atom_digest = r##"
#[spec_provenance::implementation(
 syntax = "v1",
 body = r#"
id = "IMPL-VOCAB-BAD-ATOM"
atoms = [{ id = "ATOM-VOCAB-NAV", revision = "not-a-valid-digest" }]
navigation = "implements"
"#
)]
fn gap_implementation_bad_atom() {}
"##;
    let output = extract_rust(&path, bad_atom_digest.as_bytes());
    assert!(!output.diagnostics.is_empty());
}

#[test]
fn rust_carrier_rejects_attachment_role_mismatch_malformed_body_and_empty_implementation_atoms() {
    let path = path("tests/gap.rs").expect("gap Rust control path");
    let role_mismatch = format!(
        r##"
#[spec_provenance::structural(
 syntax = "v1",
 body = r#"
id = "CAR-GAP-ROLE-001"
role = "positive"
atoms = []
oracle = {{ kind = "oracle", id = "ORACLE-GAP-ROLE-001", digest = "{}" }}
configuration = "{}"
corpora = []
scope = {{ schema = "gap.structural-scope/v1", bytes_hex = "73" }}
assumptions = []
exclusions = []
"#
)]
#[test]
fn gap_role_mismatch() {{}}
"##,
        hexd("60"),
        hexd("61")
    );
    let output = extract_rust(&path, role_mismatch.as_bytes());
    assert!(output.diagnostics.iter().any(|diagnostic| {
        diagnostic.code == "SPG-ADAPTER-CARRIER-ROLE" && diagnostic.context == "attachment-role"
    }));

    let malformed_body = br#"
#[spec_provenance::carrier(syntax = "v1", body = "not = [toml")]
#[test]
fn gap_malformed_body() {}
"#;
    let output = extract_rust(&path, malformed_body);
    assert!(output.diagnostics.iter().any(|diagnostic| {
        diagnostic.code == "SPG-ADAPTER-MALFORMED-ATTRIBUTE" && diagnostic.context == "toml"
    }));

    let malformed_implementation_body = br#"
#[spec_provenance::implementation(syntax = "v1", body = "not = [toml")]
fn gap_malformed_implementation() {}
"#;
    let output = extract_rust(&path, malformed_implementation_body);
    assert!(output.diagnostics.iter().any(|diagnostic| {
        diagnostic.code == "SPG-ADAPTER-MALFORMED-ATTRIBUTE" && diagnostic.context == "toml"
    }));

    let malformed_corpora = format!(
        r##"
#[spec_provenance::structural(
 syntax = "v1",
 body = r#"
id = "CAR-GAP-CORPORA-001"
role = "structural"
atoms = [{{ id = "ATOM-GAP-CORPORA-001", revision = "{}" }}]
oracle = {{ kind = "oracle", id = "ORACLE-GAP-CORPORA-001", digest = "{}" }}
configuration = "{}"
corpora = ["not-a-valid-digest"]
scope = {{ schema = "gap.structural-scope/v1", bytes_hex = "73" }}
assumptions = []
exclusions = []
"#
)]
#[test]
fn gap_malformed_corpora() {{}}
"##,
        hexd("95"),
        hexd("96"),
        hexd("97")
    );
    let output = extract_rust(&path, malformed_corpora.as_bytes());
    assert!(output.diagnostics.iter().any(|diagnostic| {
        diagnostic.code == "SPG-ADAPTER-MALFORMED-ATTRIBUTE" && diagnostic.context == "corpora"
    }));

    let malformed_scope = format!(
        r##"
#[spec_provenance::structural(
 syntax = "v1",
 body = r#"
id = "CAR-GAP-SCOPE-001"
role = "structural"
atoms = [{{ id = "ATOM-GAP-SCOPE-001", revision = "{}" }}]
oracle = {{ kind = "oracle", id = "ORACLE-GAP-SCOPE-001", digest = "{}" }}
configuration = "{}"
corpora = []
scope = {{ schema = "gap.structural-scope/v1", bytes_hex = "zz" }}
assumptions = []
exclusions = []
"#
)]
#[test]
fn gap_malformed_scope() {{}}
"##,
        hexd("98"),
        hexd("99"),
        hexd("9a")
    );
    let output = extract_rust(&path, malformed_scope.as_bytes());
    assert!(output.diagnostics.iter().any(|diagnostic| {
        diagnostic.code == "SPG-ADAPTER-MALFORMED-ATTRIBUTE" && diagnostic.context == "scope"
    }));

    let malformed_assumptions = format!(
        r##"
#[spec_provenance::structural(
 syntax = "v1",
 body = r#"
id = "CAR-GAP-ASSUMPTIONS-001"
role = "structural"
atoms = [{{ id = "ATOM-GAP-ASSUMPTIONS-001", revision = "{}" }}]
oracle = {{ kind = "oracle", id = "ORACLE-GAP-ASSUMPTIONS-001", digest = "{}" }}
configuration = "{}"
corpora = []
scope = {{ schema = "gap.structural-scope/v1", bytes_hex = "73" }}
assumptions = [{{ kind = "assumption", id = "ASSUMPTION-GAP-001", digest = "not-a-valid-digest" }}]
exclusions = []
"#
)]
#[test]
fn gap_malformed_assumptions() {{}}
"##,
        hexd("9b"),
        hexd("9c"),
        hexd("9d")
    );
    let output = extract_rust(&path, malformed_assumptions.as_bytes());
    assert!(output.diagnostics.iter().any(|diagnostic| {
        diagnostic.code == "SPG-ADAPTER-MALFORMED-ATTRIBUTE" && diagnostic.context == "assumptions"
    }));

    let empty_atoms = r##"
#[spec_provenance::implementation(
 syntax = "v1",
 body = r#"
id = "IMPL-GAP-EMPTY-001"
atoms = []
navigation = "implements"
"#
)]
fn gap_empty_atoms() {}
"##;
    let output = extract_rust(&path, empty_atoms.as_bytes());
    assert!(output.diagnostics.iter().any(|diagnostic| {
        diagnostic.code == "SCF-CONSTRUCT-FIELD-EMPTY"
            && diagnostic.context == "implementation-link"
    }));
}

#[test]
fn markdown_surface_every_field_level_build_error_is_reachable() {
    let path = path("spec/gap-field-surface.md").expect("gap field surface control path");
    let base = format!(
        r#"```spec-provenance-surface-v1
id = "SURFACE-FIELD-001"
member_key_schema = {{ schema = "s", bytes_hex = "c0" }}
disposition_vocabulary = {{ schema = "s", bytes_hex = "c1" }}
member_keys = [{{ schema = "s", bytes_hex = "c2" }}]
generation = 1
policies = [{{ kind = "coverage-policy", id = "POL-FIELD-001", digest = "{}" }}]
dispositions = [{{ key = {{ schema = "s", bytes_hex = "c2" }}, disposition = "not-applicable" }}]
```
"#,
        hexd("b0")
    );
    assert!(
        extract_markdown(&path, base.as_bytes(), AuthorityExpectation::Optional)
            .diagnostics
            .is_empty()
    );
    assert_markdown_field_error(&path, &base, "c0");
    assert_markdown_field_error(&path, &base, "c1");
    assert_markdown_field_error(&path, &base, "c2");
    assert_markdown_field_error(&path, &base, &hexd("b0"));

    let zero_generation = base.replace("generation = 1", "generation = 0");
    let output = extract_markdown(
        &path,
        zero_generation.as_bytes(),
        AuthorityExpectation::Optional,
    );
    assert!(!output.diagnostics.is_empty());
}

#[test]
fn markdown_derivation_every_field_level_build_error_is_reachable() {
    let path = path("spec/gap-field-derivation.md").expect("gap field derivation control path");
    let base = format!(
        r#"```spec-provenance-derivation-v1
id = "DERIV-FIELD-001"
sources = [{{ kind = "repository-artifact", path = "src/field.rs", raw_domain = "raw-artifact", media_type = "text/x-rust", representation = "saved-bytes/v1", digest = "{}" }}]
targets = [{{ kind = "repository-artifact", path = "generated/field.rs", raw_domain = "raw-artifact", media_type = "text/x-rust", representation = "saved-bytes/v1", digest = "{}" }}]
tool = {{ id = "field-tool", version = "1.0.0", executable = "{}", configuration = "{}" }}
configuration = "{}"
oracle = {{ kind = "oracle", id = "ORACLE-FIELD-001", digest = "{}" }}
relation = "generated-from"
relation_scope = {{ schema = "s", bytes_hex = "c3" }}
exclusions = []
```
"#,
        hexd("b1"),
        hexd("b2"),
        hexd("b3"),
        hexd("b4"),
        hexd("b5"),
        hexd("b6")
    );
    assert!(
        extract_markdown(&path, base.as_bytes(), AuthorityExpectation::Optional)
            .diagnostics
            .is_empty()
    );
    assert_markdown_field_error(&path, &base, &hexd("b2"));
    assert_markdown_field_error(&path, &base, &hexd("b3"));
    assert_markdown_field_error(&path, &base, &hexd("b5"));
    assert_markdown_field_error(&path, &base, &hexd("b6"));
    assert_markdown_field_error(&path, &base, "c3");

    let bad_configuration = base.replacen(&hexd("b4"), "not-a-valid-digest", 1);
    let output = extract_markdown(
        &path,
        bad_configuration.as_bytes(),
        AuthorityExpectation::Optional,
    );
    assert!(!output.diagnostics.is_empty());
}

#[test]
fn rust_carrier_atoms_field_build_error_is_reachable() {
    let path = path("tests/gap-field-carrier.rs").expect("gap field carrier control path");
    let base = format!(
        r##"
#[spec_provenance::structural(
 syntax = "v1",
 body = r#"
id = "CAR-FIELD-001"
role = "structural"
atoms = [{{ id = "ATOM-FIELD-001", revision = "{}" }}]
oracle = {{ kind = "oracle", id = "ORACLE-FIELD-CARRIER-001", digest = "{}" }}
configuration = "{}"
corpora = []
scope = {{ schema = "s", bytes_hex = "73" }}
assumptions = []
exclusions = []
"#
)]
#[test]
fn gap_field_carrier() {{}}
"##,
        hexd("b7"),
        hexd("b8"),
        hexd("b9")
    );
    assert!(extract_rust(&path, base.as_bytes()).diagnostics.is_empty());
    assert_rust_field_error(&path, &base, &hexd("b7"));
}

#[test]
fn markdown_policy_every_field_level_build_error_is_reachable() {
    let path = path("spec/gap-field-policy.md").expect("gap field policy control path");
    let base = format!(
        r#"```spec-provenance-policy-v1
id = "POL-FIELD-001"
scope = {{ schema = "s", bytes_hex = "c4" }}
required_roles = ["positive"]
enforcement = "blocking"
ratchet_id = "gap-ratchet"
authority_selectors = []
derivation_requirements = {{ schema = "s", bytes_hex = "c5" }}
active_approval = {{ authority = "maintainers", record = "reviews/field.md", digest = "{}" }}
lifecycle = "retired"
replacements = []
terminal_approval = {{ authority = "maintainers", record = "reviews/field.md", digest = "{}" }}
```
"#,
        hexd("ba"),
        hexd("bb")
    );
    assert!(
        extract_markdown(&path, base.as_bytes(), AuthorityExpectation::Optional)
            .diagnostics
            .is_empty()
    );
    assert_markdown_field_error(&path, &base, "c4");
    assert_markdown_field_error(&path, &base, "c5");
    assert_markdown_field_error(&path, &base, &hexd("ba"));
    assert_markdown_field_error(&path, &base, &hexd("bb"));

    let bad_required_role = base.replace(
        "required_roles = [\"positive\"]",
        "required_roles = [\"not-a-real-role\"]",
    );
    let output = extract_markdown(
        &path,
        bad_required_role.as_bytes(),
        AuthorityExpectation::Optional,
    );
    assert!(!output.diagnostics.is_empty());

    let replacements_error = base.replace(
        "replacements = []",
        &format!(
            "replacements = [{{ kind = \"coverage-policy\", id = \"POL-OTHER\", digest = \"{}\" }}]",
            "not-a-valid-digest"
        ),
    );
    let output = extract_markdown(
        &path,
        replacements_error.as_bytes(),
        AuthorityExpectation::Optional,
    );
    assert!(!output.diagnostics.is_empty());

    let bad_ratchet_id = base.replace(
        "ratchet_id = \"gap-ratchet\"",
        "ratchet_id = \"Invalid Ratchet ID!\"",
    );
    let output = extract_markdown(
        &path,
        bad_ratchet_id.as_bytes(),
        AuthorityExpectation::Optional,
    );
    assert!(!output.diagnostics.is_empty());
}

#[test]
fn markdown_debt_discharge_evidence_replacements_and_terminal_approval_errors_are_reachable() {
    let path = path("spec/gap-field-debt.md").expect("gap field debt control path");
    let discharged = format!(
        r#"```spec-provenance-debt-v1
id = "DEBT-FIELD-DISCHARGED-001"
obligations = [{{ policy_id = "POL-FIELD-001", kind = "freshness", subject = {{ kind = "atom", id = "ATOM-FIELD-DEBT-001", digest = "{}" }} }}]
objects = []
rationale = "Field-level debt rationale."
owner = "gap-owner"
remediation = "Gap remediation."
discharge_condition = {{ schema = "s", bytes_hex = "72" }}
external_references = []
lifecycle = "discharged"
discharge_evidence = [{{ kind = "atom", id = "ATOM-FIELD-DEBT-002", digest = "{}" }}]
replacements = []
terminal_approval = {{ authority = "maintainers", record = "reviews/field.md", digest = "{}" }}
```
"#,
        hexd("11"),
        hexd("12"),
        hexd("13")
    );
    assert!(
        extract_markdown(&path, discharged.as_bytes(), AuthorityExpectation::Optional)
            .diagnostics
            .is_empty()
    );
    assert_markdown_field_error(&path, &discharged, &hexd("12"));
    assert_markdown_field_error(&path, &discharged, &hexd("13"));

    let superseded = format!(
        r#"```spec-provenance-debt-v1
id = "DEBT-FIELD-SUPERSEDED-001"
obligations = [{{ policy_id = "POL-FIELD-001", kind = "freshness", subject = {{ kind = "atom", id = "ATOM-FIELD-DEBT-003", digest = "{}" }} }}]
objects = []
rationale = "Field-level debt rationale."
owner = "gap-owner"
remediation = "Gap remediation."
discharge_condition = {{ schema = "s", bytes_hex = "72" }}
external_references = []
lifecycle = "superseded"
discharge_evidence = []
replacements = [{{ kind = "debt-record", id = "DEBT-FIELD-OTHER-001", digest = "{}" }}]
terminal_approval = {{ authority = "maintainers", record = "reviews/field.md", digest = "{}" }}
```
"#,
        hexd("14"),
        hexd("15"),
        hexd("16")
    );
    assert!(
        extract_markdown(&path, superseded.as_bytes(), AuthorityExpectation::Optional)
            .diagnostics
            .is_empty()
    );
    assert_markdown_field_error(&path, &superseded, &hexd("15"));
}

#[test]
fn markdown_waiver_every_field_level_build_error_is_reachable() {
    let path = path("spec/gap-field-waiver.md").expect("gap field waiver control path");
    let base = format!(
        r#"```spec-provenance-waiver-v1
id = "WAIVER-FIELD-001"
obligations = [{{ policy_id = "POL-FIELD-001", kind = "freshness", subject = {{ kind = "atom", id = "ATOM-FIELD-001", digest = "{}" }} }}]
objects = [{{ kind = "atom", id = "ATOM-FIELD-002", digest = "{}" }}]
rationale = "Field-level waiver rationale."
owner = "gap-owner"
discharge_condition = {{ schema = "s", bytes_hex = "c6" }}
active_approval = {{ authority = "maintainers", record = "reviews/field.md", digest = "{}" }}
external_references = []
lifecycle = "discharged"
discharge_evidence = [{{ kind = "atom", id = "ATOM-FIELD-003", digest = "{}" }}]
replacements = []
terminal_approval = {{ authority = "maintainers", record = "reviews/field.md", digest = "{}" }}
```
"#,
        hexd("bc"),
        hexd("bd"),
        hexd("be"),
        hexd("bf"),
        hexd("c8")
    );
    assert!(
        extract_markdown(&path, base.as_bytes(), AuthorityExpectation::Optional)
            .diagnostics
            .is_empty()
    );
    assert_markdown_field_error(&path, &base, &hexd("bc"));
    assert_markdown_field_error(&path, &base, &hexd("bd"));
    assert_markdown_field_error(&path, &base, "c6");
    assert_markdown_field_error(&path, &base, &hexd("be"));
    assert_markdown_field_error(&path, &base, &hexd("bf"));
    assert_markdown_field_error(&path, &base, &hexd("c8"));

    let superseded = format!(
        r#"```spec-provenance-waiver-v1
id = "WAIVER-FIELD-SUPERSEDED-001"
obligations = [{{ policy_id = "POL-FIELD-001", kind = "freshness", subject = {{ kind = "atom", id = "ATOM-FIELD-004", digest = "{}" }} }}]
objects = []
rationale = "Field-level superseded waiver rationale."
owner = "gap-owner"
discharge_condition = {{ schema = "s", bytes_hex = "c6" }}
active_approval = {{ authority = "maintainers", record = "reviews/field.md", digest = "{}" }}
external_references = []
lifecycle = "superseded"
discharge_evidence = []
replacements = [{{ kind = "waiver", id = "WAIVER-FIELD-OTHER-001", digest = "{}" }}]
terminal_approval = {{ authority = "maintainers", record = "reviews/field.md", digest = "{}" }}
```
"#,
        hexd("17"),
        hexd("18"),
        hexd("19"),
        hexd("1a")
    );
    assert!(
        extract_markdown(&path, superseded.as_bytes(), AuthorityExpectation::Optional)
            .diagnostics
            .is_empty()
    );
    assert_markdown_field_error(&path, &superseded, &hexd("19"));
}

#[test]
fn markdown_reaffirmation_and_assumption_every_field_level_build_error_is_reachable() {
    let path =
        path("spec/gap-field-reaffirmation.md").expect("gap field reaffirmation control path");
    let base = format!(
        r#"```spec-provenance-reaffirmation-v1
id = "REAFFIRM-FIELD-001"
affected_object_kind = "atom"
affected_object_id = "ATOM-FIELD-001"
base_object_digest = "{}"
head_object_digest = "{}"
review_approval = {{ authority = "maintainers", record = "reviews/field.md", digest = "{}" }}
review_statement = "Field-level reaffirmation statement."
```
"#,
        hexd("c9"),
        hexd("ca"),
        hexd("cb")
    );
    assert!(
        extract_markdown(&path, base.as_bytes(), AuthorityExpectation::Optional)
            .diagnostics
            .is_empty()
    );
    let bad_kind = base.replace(
        "affected_object_kind = \"atom\"",
        "affected_object_kind = \"unknown\"",
    );
    assert!(
        !extract_markdown(&path, bad_kind.as_bytes(), AuthorityExpectation::Optional)
            .diagnostics
            .is_empty()
    );
    assert_markdown_field_error(&path, &base, &hexd("c9"));
    assert_markdown_field_error(&path, &base, &hexd("ca"));
    assert_markdown_field_error(&path, &base, &hexd("cb"));

    let assumption_base = format!(
        r#"```spec-provenance-assumption-v1
id = "ASSUMPTION-FIELD-001"
statement = "Field-level assumption statement."
scope = {{ schema = "s", bytes_hex = "c7" }}
exclusions = []
approval = {{ authority = "maintainers", record = "reviews/field.md", digest = "{}" }}
```
"#,
        hexd("cc")
    );
    assert!(
        extract_markdown(
            &path,
            assumption_base.as_bytes(),
            AuthorityExpectation::Optional
        )
        .diagnostics
        .is_empty()
    );
    assert_markdown_field_error(&path, &assumption_base, "c7");
    assert_markdown_field_error(&path, &assumption_base, &hexd("cc"));
}

#[test]
fn markdown_finite_model_every_field_level_build_error_is_reachable() {
    let path = path("spec/gap-field-finite-model.md").expect("gap field finite model control path");
    let base = format!(
        r#"```spec-provenance-finite-model-v1
id = "MODEL-FIELD-001"
state_space = {{ schema = "s", bytes_hex = "d0" }}
event_alphabet = {{ schema = "s", bytes_hex = "d1" }}
transition_relation = {{ schema = "s", bytes_hex = "d2" }}
initial_states = [{{ schema = "s", bytes_hex = "d3" }}]
bound = {{ schema = "s", bytes_hex = "d4" }}
checked_invariants = [{{ id = "ATOM-FIELD-001", revision = "{}" }}]
assumptions = [{{ kind = "assumption", id = "ASSUMPTION-FIELD-001", digest = "{}" }}]
exclusions = []
```
"#,
        hexd("cd"),
        hexd("ce")
    );
    assert!(
        extract_markdown(&path, base.as_bytes(), AuthorityExpectation::Optional)
            .diagnostics
            .is_empty()
    );
    assert_markdown_field_error(&path, &base, "d0");
    assert_markdown_field_error(&path, &base, "d1");
    assert_markdown_field_error(&path, &base, "d2");
    assert_markdown_field_error(&path, &base, "d3");
    assert_markdown_field_error(&path, &base, "d4");
    assert_markdown_field_error(&path, &base, &hexd("cd"));
    assert_markdown_field_error(&path, &base, &hexd("ce"));
}

#[test]
fn markdown_formal_property_every_field_level_build_error_is_reachable() {
    let path =
        path("spec/gap-field-formal-property.md").expect("gap field formal property control path");
    let base = format!(
        r#"```spec-provenance-formal-property-v1
id = "PROPERTY-FIELD-001"
atoms = [{{ id = "ATOM-FIELD-001", revision = "{}" }}]
property = {{ schema = "s", bytes_hex = "d5" }}
implementation_abstraction = {{ schema = "s", bytes_hex = "d6" }}
assumptions = [{{ kind = "assumption", id = "ASSUMPTION-FIELD-001", digest = "{}" }}]
exclusions = []
correspondence_approval = {{ authority = "maintainers", record = "reviews/field.md", digest = "{}" }}
```
"#,
        hexd("cf"),
        hexd("d7"),
        hexd("d8")
    );
    assert!(
        extract_markdown(&path, base.as_bytes(), AuthorityExpectation::Optional)
            .diagnostics
            .is_empty()
    );
    assert_markdown_field_error(&path, &base, &hexd("cf"));
    assert_markdown_field_error(&path, &base, "d5");
    assert_markdown_field_error(&path, &base, "d6");
    assert_markdown_field_error(&path, &base, &hexd("d7"));
    assert_markdown_field_error(&path, &base, &hexd("d8"));
}

#[test]
fn markdown_proof_record_every_field_level_build_error_is_reachable() {
    let path = path("spec/gap-field-proof-record.md").expect("gap field proof record control path");
    let base = format!(
        r#"```spec-provenance-proof-record-v1
id = "PROOF-FIELD-001"
atoms = [{{ id = "ATOM-FIELD-001", revision = "{}" }}]
property = {{ kind = "formal-property", id = "PROPERTY-FIELD-001", digest = "{}" }}
implementation = {{ kind = "repository-artifact", path = "src/field-impl.rs", raw_domain = "raw-artifact", media_type = "text/x-rust", representation = "saved-bytes/v1", digest = "{}" }}
assumptions = [{{ kind = "assumption", id = "ASSUMPTION-FIELD-001", digest = "{}" }}]
translators = [{{ id = "field-translator", version = "1.0.0", executable = "{}", configuration = "{}" }}]
verifier = {{ id = "field-verifier", version = "1.0.0", executable = "{}", configuration = "{}" }}
verifier_configuration = "{}"
external_models = [{{ kind = "repository-artifact", path = "models/field.rs", raw_domain = "raw-artifact", media_type = "text/x-rust", representation = "saved-bytes/v1", digest = "{}" }}]
generated = [{{ kind = "repository-artifact", path = "generated/field-proof.rs", raw_domain = "raw-artifact", media_type = "text/x-rust", representation = "saved-bytes/v1", digest = "{}" }}]
proof_sources = [{{ kind = "repository-artifact", path = "proofs/field.rs", raw_domain = "raw-artifact", media_type = "text/x-rust", representation = "saved-bytes/v1", digest = "{}" }}]
verdict = "pass"
scope = {{ schema = "s", bytes_hex = "d9" }}
exclusions = []
```
"#,
        hexd("da"),
        hexd("db"),
        hexd("dc"),
        hexd("dd"),
        hexd("de"),
        hexd("df"),
        hexd("e0"),
        hexd("e1"),
        hexd("e2"),
        hexd("e3"),
        hexd("e4"),
        hexd("e5")
    );
    assert!(
        extract_markdown(&path, base.as_bytes(), AuthorityExpectation::Optional)
            .diagnostics
            .is_empty()
    );
    assert_markdown_field_error(&path, &base, &hexd("da"));
    assert_markdown_field_error(&path, &base, &hexd("db"));
    assert_markdown_field_error(&path, &base, &hexd("dc"));
    assert_markdown_field_error(&path, &base, &hexd("dd"));
    assert_markdown_field_error(&path, &base, &hexd("df"));
    assert_markdown_field_error(&path, &base, &hexd("e1"));
    assert_markdown_field_error(&path, &base, &hexd("e2"));
    assert_markdown_field_error(&path, &base, &hexd("e3"));
    assert_markdown_field_error(&path, &base, &hexd("e4"));
    assert_markdown_field_error(&path, &base, &hexd("e5"));
    assert_markdown_field_error(&path, &base, "d9");
}

#[test]
fn markdown_tool_executable_and_configuration_digest_errors_are_reachable() {
    let path = path("spec/gap-field-tool.md").expect("gap field tool control path");
    let bad_executable = format!(
        r#"```spec-provenance-derivation-v1
id = "DERIV-TOOL-001"
sources = [{{ kind = "repository-artifact", path = "src/tool.rs", raw_domain = "raw-artifact", media_type = "text/x-rust", representation = "saved-bytes/v1", digest = "{}" }}]
targets = [{{ kind = "repository-artifact", path = "generated/tool.rs", raw_domain = "raw-artifact", media_type = "text/x-rust", representation = "saved-bytes/v1", digest = "{}" }}]
tool = {{ id = "tool-a", version = "1.0.0", executable = "not-a-valid-digest", configuration = "{}" }}
configuration = "{}"
oracle = {{ kind = "oracle", id = "ORACLE-TOOL-001", digest = "{}" }}
relation = "generated-from"
relation_scope = {{ schema = "s", bytes_hex = "d0" }}
exclusions = []
```
"#,
        hexd("e6"),
        hexd("e7"),
        hexd("e8"),
        hexd("e9"),
        hexd("ea")
    );
    assert!(
        !extract_markdown(
            &path,
            bad_executable.as_bytes(),
            AuthorityExpectation::Optional
        )
        .diagnostics
        .is_empty()
    );

    let bad_configuration = format!(
        r#"```spec-provenance-derivation-v1
id = "DERIV-TOOL-002"
sources = [{{ kind = "repository-artifact", path = "src/tool2.rs", raw_domain = "raw-artifact", media_type = "text/x-rust", representation = "saved-bytes/v1", digest = "{}" }}]
targets = [{{ kind = "repository-artifact", path = "generated/tool2.rs", raw_domain = "raw-artifact", media_type = "text/x-rust", representation = "saved-bytes/v1", digest = "{}" }}]
tool = {{ id = "tool-b", version = "1.0.0", executable = "{}", configuration = "not-a-valid-digest" }}
configuration = "{}"
oracle = {{ kind = "oracle", id = "ORACLE-TOOL-002", digest = "{}" }}
relation = "generated-from"
relation_scope = {{ schema = "s", bytes_hex = "d1" }}
exclusions = []
```
"#,
        hexd("eb"),
        hexd("ec"),
        hexd("ed"),
        hexd("ee"),
        hexd("ef")
    );
    assert!(
        !extract_markdown(
            &path,
            bad_configuration.as_bytes(),
            AuthorityExpectation::Optional
        )
        .diagnostics
        .is_empty()
    );
}

#[test]
fn markdown_disposition_and_obligation_every_field_level_build_error_is_reachable() {
    let path = path("spec/gap-field-disposition.md").expect("gap field disposition control path");
    let base = format!(
        r#"```spec-provenance-surface-v1
id = "SURFACE-DISPOSITION-001"
member_key_schema = {{ schema = "s", bytes_hex = "60" }}
disposition_vocabulary = {{ schema = "s", bytes_hex = "61" }}
member_keys = [{{ schema = "s", bytes_hex = "f0" }}]
generation = 1
policies = []
dispositions = [{{ key = {{ schema = "s", bytes_hex = "f0" }}, disposition = "normative", payload = {{ schema = "s", bytes_hex = "f1" }}, authority = {{ id = "ATOM-DISPOSITION-001", revision = "{}" }} }}]
```
"#,
        hexd("f2")
    );
    assert!(
        extract_markdown(&path, base.as_bytes(), AuthorityExpectation::Optional)
            .diagnostics
            .is_empty()
    );
    // Corrupt only the *last* occurrence of "f0" (the disposition row's own
    // key), leaving `member_keys` untouched, to isolate `DispositionDef::build`'s
    // own key-build error from the earlier `member_keys` build error.
    let last = base
        .rfind("\"f0\"")
        .expect("disposition key literal present");
    let mut bad_key = base.clone();
    bad_key.replace_range(last..last + 4, "\"zz\"");
    assert!(
        !extract_markdown(&path, bad_key.as_bytes(), AuthorityExpectation::Optional)
            .diagnostics
            .is_empty()
    );
    assert_markdown_field_error(&path, &base, "f1");
    assert_markdown_field_error(&path, &base, &hexd("f2"));

    let obligation_base = format!(
        r#"```spec-provenance-debt-v1
id = "DEBT-OBLIGATION-001"
obligations = [{{ policy_id = "POL-OBLIGATION-001", kind = "required-role", subject = {{ kind = "atom", id = "ATOM-OBLIGATION-001", digest = "{}" }}, required_role = "positive" }}]
objects = []
rationale = "Obligation field-level rationale."
owner = "gap-owner"
remediation = "Gap remediation."
discharge_condition = {{ schema = "s", bytes_hex = "72" }}
external_references = []
lifecycle = "active"
discharge_evidence = []
replacements = []
```
"#,
        hexd("f3")
    );
    assert!(
        extract_markdown(
            &path,
            obligation_base.as_bytes(),
            AuthorityExpectation::Optional
        )
        .diagnostics
        .is_empty()
    );
    assert_markdown_field_error(&path, &obligation_base, &hexd("f3"));
    let bad_required_role = obligation_base.replace(
        "required_role = \"positive\"",
        "required_role = \"not-a-real-role\"",
    );
    assert!(
        !extract_markdown(
            &path,
            bad_required_role.as_bytes(),
            AuthorityExpectation::Optional
        )
        .diagnostics
        .is_empty()
    );
}

#[test]
fn markdown_object_kind_and_closed_vocabulary_remaining_variants_are_reachable() {
    let path = path("spec/gap-remaining-kinds.md").expect("gap remaining kinds control path");
    let every_remaining_kind = format!(
        r#"```spec-provenance-debt-v1
id = "DEBT-REMAINING-KIND-001"
obligations = [
{{ policy_id = "POL-REMAINING-001", kind = "authority", subject = {{ kind = "atom", id = "ATOM-REMAINING-A1", digest = "{}" }} }},
{{ policy_id = "POL-REMAINING-001", kind = "surface-completeness", subject = {{ kind = "atom", id = "ATOM-REMAINING-A2", digest = "{}" }} }},
{{ policy_id = "POL-REMAINING-001", kind = "surface-authority", subject = {{ kind = "atom", id = "ATOM-REMAINING-A3", digest = "{}" }} }},
{{ policy_id = "POL-REMAINING-001", kind = "oracle-selection", subject = {{ kind = "atom", id = "ATOM-REMAINING-A4", digest = "{}" }} }},
]
objects = [
{{ kind = "tombstone", id = "X-1", digest = "{}" }},
{{ kind = "decision-surface", id = "X-2", digest = "{}" }},
{{ kind = "evidence-carrier", id = "X-3", digest = "{}" }},
{{ kind = "implementation-link", id = "X-4", digest = "{}" }},
{{ kind = "derivation", id = "X-5", digest = "{}" }},
{{ kind = "finite-model", id = "X-6", digest = "{}" }},
]
rationale = "Gap debt covering every remaining closed-vocabulary variant."
owner = "gap-owner"
remediation = "Gap remediation."
discharge_condition = {{ schema = "s", bytes_hex = "72" }}
external_references = []
lifecycle = "active"
discharge_evidence = []
replacements = []
```
"#,
        hexd("f4"),
        hexd("f5"),
        hexd("f6"),
        hexd("f7"),
        hexd("f8"),
        hexd("f9"),
        hexd("fa"),
        hexd("fb"),
        hexd("fc"),
        hexd("fd")
    );
    let output = extract_markdown(
        &path,
        every_remaining_kind.as_bytes(),
        AuthorityExpectation::Optional,
    );
    assert_eq!(output.diagnostics, Vec::new());
    assert_eq!(output.semantic.len(), 1);

    let byte_equal_derivation = format!(
        r#"```spec-provenance-derivation-v1
id = "DERIV-BYTE-EQUAL-001"
sources = [{{ kind = "repository-artifact", path = "src/byte-equal.rs", raw_domain = "raw-artifact", media_type = "text/x-rust", representation = "saved-bytes/v1", digest = "{}" }}]
targets = [{{ kind = "repository-artifact", path = "generated/byte-equal.rs", raw_domain = "raw-artifact", media_type = "text/x-rust", representation = "saved-bytes/v1", digest = "{}" }}]
tool = {{ id = "byte-equal-tool", version = "1.0.0", executable = "{}", configuration = "{}" }}
configuration = "{}"
oracle = {{ kind = "oracle", id = "ORACLE-BYTE-EQUAL-001", digest = "{}" }}
relation = "byte-equal"
relation_scope = {{ schema = "s", bytes_hex = "6578616374" }}
exclusions = []
```
"#,
        hexd("fe"),
        hexd("ff"),
        hexd("00"),
        hexd("01"),
        hexd("02"),
        hexd("03")
    );
    let output = extract_markdown(
        &path,
        byte_equal_derivation.as_bytes(),
        AuthorityExpectation::Optional,
    );
    assert_eq!(output.diagnostics, Vec::new());
    assert_eq!(output.semantic.len(), 1);

    let not_run_proof_record = format!(
        r#"```spec-provenance-proof-record-v1
id = "PROOF-NOT-RUN-001"
atoms = [{{ id = "ATOM-NOT-RUN-001", revision = "{}" }}]
property = {{ kind = "formal-property", id = "PROPERTY-NOT-RUN-001", digest = "{}" }}
implementation = {{ kind = "repository-artifact", path = "src/not-run.rs", raw_domain = "raw-artifact", media_type = "text/x-rust", representation = "saved-bytes/v1", digest = "{}" }}
assumptions = []
translators = []
verifier = {{ id = "not-run-verifier", version = "1.0.0", executable = "{}", configuration = "{}" }}
verifier_configuration = "{}"
external_models = []
generated = []
proof_sources = [{{ kind = "repository-artifact", path = "proofs/not-run.rs", raw_domain = "raw-artifact", media_type = "text/x-rust", representation = "saved-bytes/v1", digest = "{}" }}]
verdict = "not-run"
scope = {{ schema = "s", bytes_hex = "6b" }}
exclusions = []
```
"#,
        hexd("04"),
        hexd("05"),
        hexd("06"),
        hexd("07"),
        hexd("08"),
        hexd("09"),
        hexd("0a")
    );
    let output = extract_markdown(
        &path,
        not_run_proof_record.as_bytes(),
        AuthorityExpectation::Optional,
    );
    assert_eq!(output.diagnostics, Vec::new());
    assert_eq!(output.semantic.len(), 1);
}

fn canonical_adapter_report_bytes() -> Option<Vec<u8>> {
    canonical_adapter_report_bytes_from(
        include_bytes!("../fixtures/adapters/markdown/objects.md"),
        include_bytes!("../fixtures/adapters/rust/links.rs"),
    )
}

fn canonical_adapter_report_bytes_from(
    markdown_bytes: &[u8],
    rust_bytes: &[u8],
) -> Option<Vec<u8>> {
    let markdown_path = path("spec/objects.md")?;
    let rust_path = path("src/objects.rs")?;
    let markdown = extract_markdown(
        &markdown_path,
        markdown_bytes,
        AuthorityExpectation::Optional,
    );
    let rust = extract_rust(&rust_path, rust_bytes);
    if !markdown.diagnostics.is_empty() || !rust.diagnostics.is_empty() {
        return None;
    }
    let markdown_surface = markdown
        .semantic
        .iter()
        .find_map(|located| match &located.value {
            MarkdownSemantic::DecisionSurface(value) => Some(value.clone()),
            _ => None,
        })?;
    let markdown_derivation =
        markdown
            .semantic
            .iter()
            .find_map(|located| match &located.value {
                MarkdownSemantic::Derivation(value) => Some(value.clone()),
                _ => None,
            })?;
    let rust_surface = rust
        .semantic
        .iter()
        .find_map(|located| match &located.value {
            RustSemantic::DecisionSurface(value) => Some(value.clone()),
            _ => None,
        })?;
    let rust_derivation = rust
        .semantic
        .iter()
        .find_map(|located| match &located.value {
            RustSemantic::Derivation(value) => Some(value.clone()),
            _ => None,
        })?;
    if markdown_surface != rust_surface || markdown_derivation != rust_derivation {
        return None;
    }
    let graph = SpecGraph {
        decision_surfaces: vec![markdown_surface],
        derivations: vec![markdown_derivation],
        ..SpecGraph::default()
    };
    let graph_digest = semantic_graph_digest(&graph).ok()?;
    let report = static_report(&graph)?;
    let mut bytes = graph_digest.bytes().to_vec();
    bytes.extend_from_slice(report.bytes());
    Some(bytes)
}

fn static_report(graph: &SpecGraph) -> Option<CanonicalStaticReport> {
    CanonicalStaticReport::new(
        semantic_graph_digest(graph).ok()?,
        Vec::new(),
        Vec::new(),
        Vec::new(),
        Vec::new(),
        Vec::new(),
        Vec::new(),
        Vec::new(),
        Vec::new(),
        Vec::new(),
    )
    .ok()
}

fn hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push(char::from(DIGITS[usize::from(byte >> 4)]));
        output.push(char::from(DIGITS[usize::from(byte & 0x0f)]));
    }
    output
}

const PORTABLE_ADAPTER_REPORT_GOLDEN: &str = "45197630c3dba58ebb5e6ea3fd1665370600000000000000d2000000000000000a00010c00000000000000140002001045197630c3dba58ebb5e6ea3fd1665370002080000000000000008000000000000000000030800000000000000080000000000000000000408000000000000000800000000000000000005080000000000000008000000000000000000060800000000000000080000000000000000000708000000000000000800000000000000000008080000000000000008000000000000000000090800000000000000080000000000000000000a0800000000000000080000000000000000";

#[test]
fn canonical_adapter_environment_child() {
    if std::env::var_os("BUOY_ADAPTER_ENV_CHILD").is_none() {
        return;
    }
    let bytes = canonical_adapter_report_bytes().expect("canonical adapter report corpus");
    assert_eq!(hex(&bytes), PORTABLE_ADAPTER_REPORT_GOLDEN);
}

#[test]
fn cross_adapter_order_root_environment_and_platform_report_bytes_are_canonical() {
    let bytes = canonical_adapter_report_bytes().expect("cross-adapter canonical corpus");
    assert_eq!(hex(&bytes), PORTABLE_ADAPTER_REPORT_GOLDEN);

    let atom_fence = |id: &str, statement: &str| {
        format!(
            "```spec-provenance-atom-v1\nid = \"{id}\"\nstate = \"proposed\"\nkind = \"behavioral\"\nscope = [\"adapter.order\"]\nstatement = \"{statement}\"\n```\n"
        )
    };
    let first_atom = atom_fence("ATOM-ORDER-001", "First order atom MUST remain stable.");
    let second_atom = atom_fence("ATOM-ORDER-002", "Second order atom MUST remain stable.");
    let order_path = path("spec/order.md").expect("order fixture path");
    let ordered = extract_markdown(
        &order_path,
        format!("{first_atom}{second_atom}").as_bytes(),
        AuthorityExpectation::Optional,
    );
    let reversed = extract_markdown(
        &order_path,
        format!("{second_atom}{first_atom}").as_bytes(),
        AuthorityExpectation::Optional,
    );
    let graph_for = |output: &buoy::adapters::MarkdownOutput| SpecGraph {
        atoms: output
            .semantic
            .iter()
            .filter_map(|located| match &located.value {
                MarkdownSemantic::Atom(atom) => Some(atom.clone()),
                _ => None,
            })
            .collect(),
        ..SpecGraph::default()
    };
    let ordered_graph = graph_for(&ordered);
    let reversed_graph = graph_for(&reversed);
    assert_eq!(
        semantic_graph_digest(&ordered_graph).expect("ordered adapter graph"),
        semantic_graph_digest(&reversed_graph).expect("reversed adapter graph")
    );
    assert_eq!(
        static_report(&ordered_graph)
            .expect("ordered adapter report")
            .bytes(),
        static_report(&reversed_graph)
            .expect("reversed adapter report")
            .bytes()
    );

    let path = path("spec/objects.md").expect("canonical object path");
    let configuration = built_in_configuration("markdown/v1", AuthorityExpectation::Optional)
        .expect("Markdown configuration");
    let input = SavedSnapshotInput::new(
        &path,
        include_bytes!("../fixtures/adapters/markdown/objects.md"),
        &configuration,
    )
    .expect("saved Markdown corpus");
    let output = built_in_registry()
        .expect("built-in registry")
        .materialize(&input)
        .expect("complete Markdown corpus");
    let mut permuted = output.product().graph().clone();
    permuted.decision_surfaces.reverse();
    permuted.derivations.reverse();
    permuted.coverage_policies.reverse();
    permuted.debt_records.reverse();
    permuted.waivers.reverse();
    permuted.finite_models.reverse();
    permuted.formal_properties.reverse();
    permuted.assumptions.reverse();
    permuted.proof_records.reverse();
    permuted.reaffirmations.reverse();
    assert_eq!(
        semantic_graph_digest(output.product().graph()).expect("canonical graph"),
        semantic_graph_digest(&permuted).expect("permuted graph")
    );
    assert_eq!(
        static_report(output.product().graph())
            .expect("canonical static report")
            .bytes(),
        static_report(&permuted)
            .expect("permuted static report")
            .bytes()
    );

    let first_root = tempfile::tempdir().expect("first canonical root");
    let second_root = tempfile::tempdir().expect("second canonical root");
    let first_markdown = write_root_file(
        first_root.path(),
        "spec/objects.md",
        include_bytes!("../fixtures/adapters/markdown/objects.md"),
    );
    let first_rust = write_root_file(
        first_root.path(),
        "src/objects.rs",
        include_bytes!("../fixtures/adapters/rust/links.rs"),
    );
    let second_markdown = write_root_file(
        second_root.path(),
        "spec/objects.md",
        include_bytes!("../fixtures/adapters/markdown/objects.md"),
    );
    let second_rust = write_root_file(
        second_root.path(),
        "src/objects.rs",
        include_bytes!("../fixtures/adapters/rust/links.rs"),
    );
    assert_ne!(first_root.path(), second_root.path());
    assert_eq!(
        canonical_adapter_report_bytes_from(&first_markdown, &first_rust),
        canonical_adapter_report_bytes_from(&second_markdown, &second_rust)
    );

    let executable = std::env::current_exe().expect("current adapter test executable");
    for (locale, timezone) in [("C", "UTC"), ("tr_TR.UTF-8", "Pacific/Kiritimati")] {
        let child = Command::new(&executable)
            .args([
                "--exact",
                "canonical_adapter_environment_child",
                "--nocapture",
            ])
            .env("BUOY_ADAPTER_ENV_CHILD", "1")
            .env("LC_ALL", locale)
            .env("LANG", locale)
            .env("TZ", timezone)
            .output()
            .expect("run locale/timezone canonical child");
        assert!(
            child.status.success(),
            "canonical child failed for {locale}/{timezone}: {}",
            String::from_utf8_lossy(&child.stderr)
        );
    }
}

const RETAINED_NODE_KIND: &str = "host/node/v1";

/// One control adapter that retains source nodes and one unresolved locator.
struct RetainingAdapter {
    descriptor: AdapterDescriptor,
    node_kind: &'static str,
    node_capability: &'static str,
    span_end: Option<u64>,
    root_class: SourceRootClass,
}

impl RetainingAdapter {
    fn new(node_kind: &'static str, node_capability: &'static str, span_end: Option<u64>) -> Self {
        Self::classified(
            node_kind,
            node_capability,
            span_end,
            SourceRootClass::Navigational,
        )
    }

    fn classified(
        node_kind: &'static str,
        node_capability: &'static str,
        span_end: Option<u64>,
        root_class: SourceRootClass,
    ) -> Self {
        Self {
            descriptor: AdapterDescriptor::new(
                "retaining/v1",
                "control-parser",
                "1.0.0",
                "host.syntax/v1",
                CONTROL_CONFIGURATION_SCHEMA_ID,
                vec![CONTROL_DECLARED_CAPABILITY.to_owned()],
                vec![SourceItemKind::new(RETAINED_NODE_KIND).expect("retained source item kind")],
            )
            .expect("valid retaining adapter descriptor"),
            node_kind,
            node_capability,
            span_end,
            root_class,
        }
    }
}

impl ShellAdapter for RetainingAdapter {
    fn descriptor(&self) -> &AdapterDescriptor {
        &self.descriptor
    }

    fn materialize(
        &self,
        input: &SavedSnapshotInput<'_>,
    ) -> Result<AdapterProduct, AdapterSdkError> {
        let item = ParsedItemIdentity::new(
            SchemaId::new("host.syntax/v1").expect("retained language schema"),
            input.path().clone(),
            SourceItemKind::new(self.node_kind).expect("retained source item kind"),
            input.bytes().to_vec(),
            None,
        )
        .expect("canonical retained parsed item");
        let node = SourceNode::new(item, Vec::new()).expect("canonical retained source node");
        let reference = node.reference();
        let span = SourceSpan {
            path: input.path().clone(),
            start: 0,
            end: self.span_end.unwrap_or(input.bytes().len() as u64),
        };
        let source = SourceProduct::new(
            vec![MaterializedSourceNode::new(
                node,
                span,
                self.node_capability,
            )?],
            Vec::new(),
            vec![DeclaredSourceRoot::new(reference, self.root_class)],
            Vec::new(),
            vec![BindingAttachment::new(
                reference,
                ImplementationBindingRef::new(reference.digest()),
            )],
            Vec::new(),
        )?;
        AdapterProduct::new_with_source_product(
            SpecGraph::default(),
            Vec::new(),
            source,
            Vec::new(),
            Vec::new(),
            Vec::new(),
        )
    }
}

fn retaining_output(adapter: RetainingAdapter) -> Result<AdapterProduct, AdapterSdkError> {
    let mut registry = AdapterRegistry::new();
    registry.register(adapter)?;
    let path = path("host/retained.txt").expect("retained control path");
    let configuration = control_configuration("retaining/v1");
    let input = SavedSnapshotInput::new(&path, b"retained bytes", &configuration)
        .expect("canonical retained input");
    registry
        .materialize(&input)
        .map(|output| output.product().clone())
}

#[test]
fn retained_source_nodes_keep_declared_kinds_and_exact_spans() {
    let product = retaining_output(RetainingAdapter::new(
        RETAINED_NODE_KIND,
        CONTROL_DECLARED_CAPABILITY,
        None,
    ))
    .expect("retained control materialization");
    assert!(
        product.graph().object_meta().is_empty(),
        "a retained node creates no semantic object"
    );
    assert_eq!(product.source().nodes().len(), 1);
    let retained = &product.source().nodes()[0];
    assert_eq!(retained.node().item().kind().as_str(), RETAINED_NODE_KIND);
    assert_eq!(retained.span().start, 0);
    assert_eq!(retained.syntax_capability(), CONTROL_DECLARED_CAPABILITY);
    assert!(
        MaterializedSourceNode::new(retained.node().clone(), retained.span().clone(), "invalid",)
            .is_err()
    );
    assert_eq!(
        retained.span().end,
        u64::try_from(b"retained bytes".len()).expect("bounded span")
    );
    assert_eq!(
        product.source().roots(),
        &[DeclaredSourceRoot::new(
            retained.node().reference(),
            SourceRootClass::Navigational
        )]
    );

    assert_eq!(
        retaining_output(RetainingAdapter::new(
            "host/undeclared/v1",
            CONTROL_DECLARED_CAPABILITY,
            None
        )),
        Err(AdapterSdkError::UndeclaredSourceItemKind),
        "a retained node must declare its source-item kind"
    );
    assert_eq!(
        retaining_output(RetainingAdapter::new(
            RETAINED_NODE_KIND,
            CONTROL_UNSUPPORTED_CAPABILITY,
            None
        )),
        Err(AdapterSdkError::Capability)
    );
    assert_eq!(
        retaining_output(RetainingAdapter::new(
            RETAINED_NODE_KIND,
            CONTROL_DECLARED_CAPABILITY,
            Some(4096)
        )),
        Err(AdapterSdkError::Materialization),
        "a retained node must carry one exact parser-backed span"
    );
}

/// Only the standing specification adapter establishes a specification root.
///
/// The registry freezes every descriptor identity, so an adapter cannot claim
/// the standing specification class for the selector that invoked it.
#[test]
fn only_the_standing_specification_adapter_declares_a_specification_root() {
    assert_eq!(
        retaining_output(RetainingAdapter::classified(
            RETAINED_NODE_KIND,
            CONTROL_DECLARED_CAPABILITY,
            None,
            SourceRootClass::StandingSpecification,
        )),
        Err(AdapterSdkError::Materialization),
        "a navigational selector never establishes a specification root"
    );
    let navigational = retaining_output(RetainingAdapter::classified(
        RETAINED_NODE_KIND,
        CONTROL_DECLARED_CAPABILITY,
        None,
        SourceRootClass::Navigational,
    ))
    .expect("a navigational root remains available to every adapter");
    assert_eq!(
        navigational.source().roots()[0].class(),
        SourceRootClass::Navigational
    );
}

#[test]
fn unresolved_reference_rows_survive_per_file_materialization() {
    let product = retaining_output(RetainingAdapter::new(
        RETAINED_NODE_KIND,
        CONTROL_DECLARED_CAPABILITY,
        None,
    ))
    .expect("retained control materialization");
    let subject = product.source().nodes()[0].node().reference();
    assert_eq!(product.source().attachments().len(), 1);
    assert_eq!(product.source().attachments()[0].subject(), subject);
    assert_eq!(
        product.source().attachments()[0].record().digest(),
        subject.digest(),
        "the adapter retains the locator it parsed rather than a resolved target"
    );
    assert!(
        product.source().bindings().is_empty(),
        "one adapter never materializes another adapter's store"
    );
}
