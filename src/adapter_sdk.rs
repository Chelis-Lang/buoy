//! Versioned shell-side adapter contract and deterministic registry.
//!
//! This module is intentionally part of the imperative `std` shell. Adapters
//! receive only canonical repository-relative paths, saved bytes, and explicit
//! configuration. No parser callback, host-language variant, or UX state is
//! passed into `buoy-core`.

use crate::adapters::{
    ARTIFACT_ADAPTER_ID, ARTIFACT_CAPABILITY, ARTIFACT_FILE_KIND, ARTIFACT_MEDIA_TYPE,
    ARTIFACT_REPRESENTATION, AdapterDiagnostic, AuthorityExpectation, MarkdownSemantic,
    OPENSPEC_EXTERNAL_TRACE_KIND, OpenSpecConfiguration, RustSemantic, SourceSpan,
    extract_artifact, extract_binding_in_mount, extract_markdown, extract_openspec_configured_with,
    extract_rust_with,
};
use crate::traceability::{ExternalObjectRef, ExternalTraceLink};
use buoy_core::{
    ActiveAlgorithm, ArtifactBindingAttachment, BindingAttachment, CanonicalValue, DigestAlgorithm,
    ObjectKind, ObjectRef, ParsedItemIdentity, RepositoryPath, SchemaId, ScopeId, SemanticDigest,
    SourceAuthority, SourceEdge, SourceItemKind, SourceNode, SourceNodeRef, SpecGraph,
    StoredBinding, semantic_digest, semantic_digest_with, semantic_graph_digest,
};
use std::collections::{BTreeMap, BTreeSet};

/// Published shell-side adapter contract identity.
pub const ADAPTER_SDK_SCHEMA_ID: &str = "buoy.adapter-sdk/v3";

const BUILTIN_CONFIGURATION_SCHEMA_ID: &str = "buoy.adapter.configuration/v1";
const MARKDOWN_SEMANTIC_CAPABILITY: &str = "markdown.fenced-object/v1";
const MARKDOWN_NONNORMATIVE_CAPABILITY: &str = "markdown.commonmark/v1";
const RUST_SEMANTIC_CAPABILITY: &str = "rust.attribute/v1";
const RUST_NONNORMATIVE_CAPABILITY: &str = "rust.item/v1";
const OPENSPEC_SEMANTIC_CAPABILITY: &str = "openspec.standing-spec/v1";
const OPENSPEC_NONNORMATIVE_CAPABILITY: &str = "openspec.prose/v1";
const BINDING_NONNORMATIVE_CAPABILITY: &str = "binding.object-file/v1";
const STANDING_SPECIFICATION_ADAPTER: &str = "openspec/v1";
const CONFIGURATION_BYTES_MAXIMUM: usize = 1_048_576;

/// Stable shell-side adapter SDK failure category.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AdapterSdkError {
    /// An adapter, parser, or syntax-schema identity is malformed or changed after registration.
    Identity,
    /// A declared syntax capability is malformed, absent, or duplicated.
    Capability,
    /// Explicit adapter configuration is malformed, oversized, or bound to another adapter.
    Configuration,
    /// The immutable adapter identity is already registered.
    DuplicateAdapter,
    /// No adapter is registered for the requested identity.
    UnsupportedAdapter,
    /// Materialized values, source mappings, dispositions, or diagnostics are incomplete.
    Materialization,
    /// A materialized source item kind is absent from the adapter declaration.
    UndeclaredSourceItemKind,
    /// A canonical adapter contract value could not be encoded.
    Encoding,
}

impl AdapterSdkError {
    /// Stable machine diagnostic code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::Identity => "SPG-ADAPTER-SDK-IDENTITY",
            Self::Capability => "SPG-ADAPTER-SDK-CAPABILITY",
            Self::Configuration => "SPG-ADAPTER-SDK-CONFIGURATION",
            Self::DuplicateAdapter => "SPG-ADAPTER-SDK-DUPLICATE",
            Self::UnsupportedAdapter => "SPG-ADAPTER-UNSUPPORTED",
            Self::Materialization => "SPG-ADAPTER-SDK-MATERIALIZATION",
            Self::UndeclaredSourceItemKind => "SPG-ADAPTER-UNDECLARED-SOURCE-ITEM-KIND",
            Self::Encoding => "DET-ENC-ADAPTER-SDK",
        }
    }
}

/// Immutable versioned identity and capability declaration for one shell adapter.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AdapterDescriptor {
    adapter_id: String,
    parser_id: String,
    parser_version: String,
    syntax_schema_id: String,
    configuration_schema_id: String,
    syntax_capabilities: Vec<String>,
    source_item_kinds: Vec<SourceItemKind>,
    digest: SemanticDigest,
}

impl AdapterDescriptor {
    /// Constructs a descriptor with an explicit source item kind set.
    ///
    /// # Errors
    ///
    /// Rejects malformed identities, empty sets, duplicates, or encoding failure.
    pub fn new(
        adapter_id: impl Into<String>,
        parser_id: impl Into<String>,
        parser_version: impl Into<String>,
        syntax_schema_id: impl Into<String>,
        configuration_schema_id: impl Into<String>,
        syntax_capabilities: Vec<String>,
        source_item_kinds: Vec<SourceItemKind>,
    ) -> Result<Self, AdapterSdkError> {
        Self::from_parts(
            adapter_id.into(),
            parser_id.into(),
            parser_version.into(),
            syntax_schema_id.into(),
            configuration_schema_id.into(),
            syntax_capabilities,
            source_item_kinds,
        )
    }

    #[allow(clippy::needless_bitwise_bool, clippy::too_many_arguments)]
    fn from_parts(
        adapter_id: String,
        parser_id: String,
        parser_version: String,
        syntax_schema_id: String,
        configuration_schema_id: String,
        mut syntax_capabilities: Vec<String>,
        mut source_item_kinds: Vec<SourceItemKind>,
    ) -> Result<Self, AdapterSdkError> {
        if !versioned_schema_id(&adapter_id)
            | !scope_id(&parser_id)
            | !exact_version(&parser_version)
            | !versioned_schema_id(&syntax_schema_id)
            | !versioned_schema_id(&configuration_schema_id)
        {
            return Err(AdapterSdkError::Identity);
        }
        if syntax_capabilities.is_empty()
            | syntax_capabilities
                .iter()
                .any(|capability| !versioned_schema_id(capability))
        {
            return Err(AdapterSdkError::Capability);
        }
        syntax_capabilities = canonicalize_rows(syntax_capabilities, |capability| {
            CanonicalValue::Text(capability.clone())
        })
        .map_err(|_| AdapterSdkError::Capability)?;
        if source_item_kinds.is_empty() {
            return Err(AdapterSdkError::UndeclaredSourceItemKind);
        }
        source_item_kinds.sort();
        if source_item_kinds.windows(2).any(|pair| pair[0] == pair[1]) {
            return Err(AdapterSdkError::UndeclaredSourceItemKind);
        }
        let digest = validated_semantic_digest(
            "shell-adapter-descriptor",
            vec![
                (1, CanonicalValue::Text(ADAPTER_SDK_SCHEMA_ID.to_owned())),
                (2, CanonicalValue::Text(adapter_id.clone())),
                (3, CanonicalValue::Text(parser_id.clone())),
                (4, CanonicalValue::Text(parser_version.clone())),
                (5, CanonicalValue::Text(syntax_schema_id.clone())),
                (6, CanonicalValue::Text(configuration_schema_id.clone())),
                (
                    7,
                    CanonicalValue::Set(
                        syntax_capabilities
                            .iter()
                            .cloned()
                            .map(CanonicalValue::Text)
                            .collect(),
                    ),
                ),
                (
                    8,
                    CanonicalValue::Set(
                        source_item_kinds
                            .iter()
                            .map(|kind| CanonicalValue::Text(kind.as_str().to_owned()))
                            .collect(),
                    ),
                ),
            ],
        );
        Ok(Self {
            adapter_id,
            parser_id,
            parser_version,
            syntax_schema_id,
            configuration_schema_id,
            syntax_capabilities,
            source_item_kinds,
            digest,
        })
    }

    /// Immutable versioned adapter identity used by selectors and registry lookup.
    #[must_use]
    pub fn adapter_id(&self) -> &str {
        &self.adapter_id
    }

    /// Exact parser package/tool identity.
    #[must_use]
    pub fn parser_id(&self) -> &str {
        &self.parser_id
    }

    /// Exact parser package/tool version.
    #[must_use]
    pub fn parser_version(&self) -> &str {
        &self.parser_version
    }

    /// Versioned syntax schema interpreted by the adapter.
    #[must_use]
    pub fn syntax_schema_id(&self) -> &str {
        &self.syntax_schema_id
    }

    /// Versioned schema for explicit adapter configuration bytes.
    #[must_use]
    pub fn configuration_schema_id(&self) -> &str {
        &self.configuration_schema_id
    }

    /// Canonically ordered supported syntax capabilities.
    #[must_use]
    pub fn syntax_capabilities(&self) -> &[String] {
        &self.syntax_capabilities
    }

    /// Canonically ordered source item kinds that the adapter can emit.
    #[must_use]
    pub fn source_item_kinds(&self) -> &[SourceItemKind] {
        &self.source_item_kinds
    }

    /// Reports whether the descriptor declares one exact source item kind.
    #[must_use]
    pub fn supports_source_item_kind(&self, kind: &SourceItemKind) -> bool {
        self.source_item_kinds.binary_search(kind).is_ok()
    }

    /// Complete immutable descriptor identity.
    #[must_use]
    pub const fn digest(&self) -> SemanticDigest {
        self.digest
    }

    /// Reports whether the descriptor declares one exact versioned capability.
    #[must_use]
    pub fn supports(&self, capability: &str) -> bool {
        self.syntax_capabilities
            .iter()
            .any(|candidate| candidate == capability)
    }
}

/// Exact explicit configuration binding for one adapter invocation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AdapterConfiguration {
    adapter_id: String,
    schema_id: String,
    bytes: Vec<u8>,
    digest: SemanticDigest,
}

impl AdapterConfiguration {
    /// Binds explicit configuration bytes to one adapter and configuration schema.
    ///
    /// # Errors
    ///
    /// Rejects malformed versioned IDs, oversized bytes, or encoding failure.
    pub fn new(adapter_id: &str, schema_id: &str, bytes: Vec<u8>) -> Result<Self, AdapterSdkError> {
        Self::from_strings(adapter_id.to_owned(), schema_id.to_owned(), bytes)
    }

    #[allow(clippy::needless_bitwise_bool)]
    fn from_strings(
        adapter_id: String,
        schema_id: String,
        bytes: Vec<u8>,
    ) -> Result<Self, AdapterSdkError> {
        if !versioned_schema_id(&adapter_id) | !versioned_schema_id(&schema_id) {
            return Err(AdapterSdkError::Configuration);
        }
        if bytes.len() > CONFIGURATION_BYTES_MAXIMUM {
            return Err(AdapterSdkError::Configuration);
        }
        let digest = validated_semantic_digest(
            "shell-adapter-configuration",
            vec![
                (1, CanonicalValue::Text(ADAPTER_SDK_SCHEMA_ID.to_owned())),
                (2, CanonicalValue::Text(adapter_id.clone())),
                (3, CanonicalValue::Text(schema_id.clone())),
                (4, CanonicalValue::Bytes(bytes.clone())),
            ],
        );
        Ok(Self {
            adapter_id,
            schema_id,
            bytes,
            digest,
        })
    }

    /// Adapter identity to which these bytes are bound.
    #[must_use]
    pub fn adapter_id(&self) -> &str {
        &self.adapter_id
    }

    /// Exact configuration schema identity.
    #[must_use]
    pub fn schema_id(&self) -> &str {
        &self.schema_id
    }

    /// Explicit immutable configuration bytes.
    #[must_use]
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// Complete configuration binding identity.
    #[must_use]
    pub const fn digest(&self) -> SemanticDigest {
        self.digest
    }
}

/// Canonical saved-snapshot input available to one shell-side adapter.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SavedSnapshotInput<'a> {
    path: &'a RepositoryPath,
    bytes: &'a [u8],
    configuration: &'a AdapterConfiguration,
    active: ActiveAlgorithm,
    digest: SemanticDigest,
}

impl<'a> SavedSnapshotInput<'a> {
    /// Binds saved bytes to the first available member for compatibility callers.
    ///
    /// # Errors
    ///
    /// Returns an encoding error if the complete input cannot be canonically bound.
    ///
    /// # Panics
    ///
    /// Panics if the build contains no digest adapter.
    #[allow(clippy::expect_used)]
    pub fn new(
        path: &'a RepositoryPath,
        bytes: &'a [u8],
        configuration: &'a AdapterConfiguration,
    ) -> Result<Self, AdapterSdkError> {
        let active = ActiveAlgorithm::build_default()
            .expect("a shell build must contain one digest adapter");
        Self::new_with(active, path, bytes, configuration)
    }

    /// Binds one saved input to the repository-selected digest member.
    ///
    /// # Errors
    ///
    /// The result form reserves error compatibility for future schema versions.
    ///
    /// # Panics
    ///
    /// Panics if the internal closed schema contains duplicate canonical field tags.
    #[allow(clippy::expect_used)]
    pub fn new_with(
        active: ActiveAlgorithm,
        path: &'a RepositoryPath,
        bytes: &'a [u8],
        configuration: &'a AdapterConfiguration,
    ) -> Result<Self, AdapterSdkError> {
        let digest = semantic_digest_with(
            active,
            "shell-adapter-saved-input",
            vec![
                (1, CanonicalValue::Text(ADAPTER_SDK_SCHEMA_ID.to_owned())),
                (
                    2,
                    CanonicalValue::Text(configuration.adapter_id().to_owned()),
                ),
                (3, CanonicalValue::Digest(configuration.digest())),
                (4, CanonicalValue::Text(path.as_str().to_owned())),
                (5, CanonicalValue::Bytes(bytes.to_vec())),
                (
                    6,
                    CanonicalValue::Text(active.algorithm().canonical_name().to_owned()),
                ),
            ],
        )
        .expect("closed saved-input fields have unique canonical tags");
        Ok(Self {
            path,
            bytes,
            configuration,
            active,
            digest,
        })
    }

    /// Canonical repository-relative path; no absolute root is available.
    #[must_use]
    pub const fn path(&self) -> &RepositoryPath {
        self.path
    }

    /// Exact saved bytes; no editor, daemon, or unsaved buffer is available.
    #[must_use]
    pub const fn bytes(&self) -> &[u8] {
        self.bytes
    }

    /// Exact explicit adapter configuration binding.
    #[must_use]
    pub const fn configuration(&self) -> &AdapterConfiguration {
        self.configuration
    }

    /// Repository-selected digest member for all emitted identities.
    #[must_use]
    pub const fn active_algorithm(&self) -> ActiveAlgorithm {
        self.active
    }

    /// Complete saved-input identity.
    #[must_use]
    pub const fn digest(&self) -> SemanticDigest {
        self.digest
    }
}

/// Parser-backed source location for one complete materialized core object.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MaterializedSource {
    object: ObjectRef,
    item: ParsedItemIdentity,
    span: SourceSpan,
    syntax_capability: String,
}

impl MaterializedSource {
    /// Constructs one exact object/source/capability mapping.
    ///
    /// # Errors
    ///
    /// Rejects a malformed versioned syntax capability.
    pub fn new(
        object: ObjectRef,
        item: ParsedItemIdentity,
        span: SourceSpan,
        syntax_capability: &str,
    ) -> Result<Self, AdapterSdkError> {
        if !versioned_schema_id(syntax_capability) {
            return Err(AdapterSdkError::Capability);
        }
        Ok(Self::from_validated(object, item, span, syntax_capability))
    }

    fn from_validated(
        object: ObjectRef,
        item: ParsedItemIdentity,
        span: SourceSpan,
        syntax_capability: &str,
    ) -> Self {
        Self {
            object,
            item,
            span,
            syntax_capability: syntax_capability.to_owned(),
        }
    }

    /// Exact core object reference.
    #[must_use]
    pub const fn object(&self) -> &ObjectRef {
        &self.object
    }

    /// Complete parser-canonical identity of the attached source item.
    #[must_use]
    pub const fn item(&self) -> &ParsedItemIdentity {
        &self.item
    }

    /// Parser-backed original-byte source span.
    #[must_use]
    pub const fn span(&self) -> &SourceSpan {
        &self.span
    }

    /// Declared syntax capability that produced the value.
    #[must_use]
    pub fn syntax_capability(&self) -> &str {
        &self.syntax_capability
    }
}

/// Parser-backed source location for one retained content-addressed source node.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MaterializedSourceNode {
    node: SourceNode,
    span: SourceSpan,
    syntax_capability: String,
}

impl MaterializedSourceNode {
    /// Retains one parsed source node, its exact span, and its capability.
    ///
    /// # Errors
    ///
    /// Rejects a malformed versioned syntax capability.
    pub fn new(
        node: SourceNode,
        span: SourceSpan,
        syntax_capability: &str,
    ) -> Result<Self, AdapterSdkError> {
        if !versioned_schema_id(syntax_capability) {
            return Err(AdapterSdkError::Capability);
        }
        Ok(Self::from_validated(node, span, syntax_capability))
    }

    fn from_validated(node: SourceNode, span: SourceSpan, syntax_capability: &str) -> Self {
        Self {
            node,
            span,
            syntax_capability: syntax_capability.to_owned(),
        }
    }

    /// Retained content-addressed source node.
    #[must_use]
    pub const fn node(&self) -> &SourceNode {
        &self.node
    }

    /// Parser-backed original-byte source span.
    #[must_use]
    pub const fn span(&self) -> &SourceSpan {
        &self.span
    }

    /// Declared syntax capability that retained the node.
    #[must_use]
    pub fn syntax_capability(&self) -> &str {
        &self.syntax_capability
    }
}

/// Closed classification of the selector that retained one declared root.
///
/// Only a standing specification selector may later establish current
/// `OpenSpec` document roots. Every other selected syntax retains navigational
/// roots that carry no specification currency.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum SourceRootClass {
    /// A standing `OpenSpec` specification selector.
    StandingSpecification = 1,
    /// Any other selected syntax; the root is navigational only.
    Navigational = 2,
}

impl SourceRootClass {
    /// Frozen canonical class name.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::StandingSpecification => "standing-specification",
            Self::Navigational => "navigational",
        }
    }
}

/// One declared current root and the selector class that established it.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct DeclaredSourceRoot {
    node: SourceNodeRef,
    class: SourceRootClass,
}

impl DeclaredSourceRoot {
    /// Declares one current root under one selector class.
    #[must_use]
    pub const fn new(node: SourceNodeRef, class: SourceRootClass) -> Self {
        Self { node, class }
    }

    /// Exact declared root node.
    #[must_use]
    pub const fn node(self) -> SourceNodeRef {
        self.node
    }

    /// Selector class that established this root.
    #[must_use]
    pub const fn class(self) -> SourceRootClass {
        self.class
    }
}

/// Complete retained source-reference product of one adapter.
///
/// A retained node needs no semantic object, so an adapter keeps every
/// navigational node it parsed together with the unresolved typed references
/// that another selected adapter owns. Repository-level aggregation, not this
/// product, settles reference closure.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct SourceProduct {
    nodes: Vec<MaterializedSourceNode>,
    edges: Vec<SourceEdge>,
    roots: Vec<DeclaredSourceRoot>,
    authorities: Vec<SourceAuthority>,
    attachments: Vec<BindingAttachment>,
    artifact_attachments: Vec<ArtifactBindingAttachment>,
    bindings: Vec<StoredBinding>,
}

impl SourceProduct {
    /// Collects at most one object-store record and no other source row.
    #[must_use]
    pub fn from_stored_binding(stored: Option<StoredBinding>) -> Self {
        Self {
            bindings: stored.into_iter().collect(),
            ..Self::default()
        }
    }

    /// Collects at most one parsed artifact attachment and no other source row.
    #[must_use]
    pub fn from_artifact_attachment(attachment: Option<ArtifactBindingAttachment>) -> Self {
        Self {
            artifact_attachments: attachment.into_iter().collect(),
            ..Self::default()
        }
    }

    /// Collects retained nodes, declared edges, current roots, authorities,
    /// attachments, and records.
    ///
    /// A declared edge leaves one node this adapter retained and may reach one
    /// node another selected adapter owns, so `implements` and `authority`
    /// navigation exists before repository-level closure settles it. Local
    /// containment stays Merkle-backed: a declared `contains` edge that no
    /// retained node preimage binds fails closed during aggregation.
    ///
    /// # Errors
    ///
    /// Rejects duplicate rows and every root, authority, attachment, or edge
    /// origin that names a node this adapter did not retain.
    pub fn new(
        nodes: Vec<MaterializedSourceNode>,
        edges: Vec<SourceEdge>,
        roots: Vec<DeclaredSourceRoot>,
        authorities: Vec<SourceAuthority>,
        attachments: Vec<BindingAttachment>,
        bindings: Vec<StoredBinding>,
    ) -> Result<Self, AdapterSdkError> {
        let nodes = canonicalize_rows(nodes, source_node_value)?;
        let retained = nodes
            .iter()
            .map(|row| row.node.reference())
            .collect::<BTreeSet<_>>();
        if roots.iter().any(|root| !retained.contains(&root.node()))
            || authorities
                .iter()
                .any(|authority| !retained.contains(&authority.node()))
            || attachments
                .iter()
                .any(|attachment| !retained.contains(&attachment.subject()))
            || edges.iter().any(|edge| !retained.contains(&edge.source()))
        {
            return Err(AdapterSdkError::Materialization);
        }
        Ok(Self {
            nodes,
            edges: canonicalize_rows(edges, source_edge_value)?,
            roots: canonicalize_rows(roots, declared_root_value)?,
            authorities: canonicalize_rows(authorities, source_authority_value)?,
            attachments: canonicalize_rows(attachments, binding_attachment_value)?,
            artifact_attachments: Vec::new(),
            bindings: canonicalize_rows(bindings, stored_binding_value)?,
        })
    }

    /// Retained nodes in canonical encoded order.
    #[must_use]
    pub fn nodes(&self) -> &[MaterializedSourceNode] {
        &self.nodes
    }

    /// Declared typed navigation edges in canonical encoded order.
    #[must_use]
    pub fn edges(&self) -> &[SourceEdge] {
        &self.edges
    }

    /// Declared current roots in canonical encoded order.
    #[must_use]
    pub fn roots(&self) -> &[DeclaredSourceRoot] {
        &self.roots
    }

    /// Materialized requirement-to-atom authorities in canonical encoded order.
    #[must_use]
    pub fn authorities(&self) -> &[SourceAuthority] {
        &self.authorities
    }

    /// Unresolved compact locators in canonical encoded order.
    #[must_use]
    pub fn attachments(&self) -> &[BindingAttachment] {
        &self.attachments
    }

    /// Parsed repository-owned artifact sidecars in canonical encoded order.
    #[must_use]
    pub fn artifact_attachments(&self) -> &[ArtifactBindingAttachment] {
        &self.artifact_attachments
    }

    /// Materialized object-store records in canonical encoded order.
    #[must_use]
    pub fn bindings(&self) -> &[StoredBinding] {
        &self.bindings
    }

    /// Whether this adapter retained no source reference at all.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self == &Self::default()
    }
}

/// Explicit schema-defined nonnormative syntax disposition.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NonnormativeDisposition {
    span: SourceSpan,
    syntax_capability: String,
}

/// Parser-backed source location for one external semantic object.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExternalMaterializedSource {
    object: ExternalObjectRef,
    item: ParsedItemIdentity,
    span: SourceSpan,
    syntax_capability: String,
}

impl ExternalMaterializedSource {
    /// Constructs one exact external object source mapping.
    ///
    /// # Errors
    ///
    /// Rejects a malformed versioned syntax capability.
    pub fn new(
        object: ExternalObjectRef,
        item: ParsedItemIdentity,
        span: SourceSpan,
        syntax_capability: &str,
    ) -> Result<Self, AdapterSdkError> {
        if !versioned_schema_id(syntax_capability) {
            return Err(AdapterSdkError::Capability);
        }
        Ok(Self::from_validated(object, item, span, syntax_capability))
    }

    fn from_validated(
        object: ExternalObjectRef,
        item: ParsedItemIdentity,
        span: SourceSpan,
        syntax_capability: &str,
    ) -> Self {
        Self {
            object,
            item,
            span,
            syntax_capability: syntax_capability.to_owned(),
        }
    }

    /// Returns the exact external object reference.
    #[must_use]
    pub const fn object(&self) -> &ExternalObjectRef {
        &self.object
    }

    /// Returns the complete parser-canonical item identity.
    #[must_use]
    pub const fn item(&self) -> &ParsedItemIdentity {
        &self.item
    }

    /// Returns the exact original-byte source span.
    #[must_use]
    pub const fn span(&self) -> &SourceSpan {
        &self.span
    }

    /// Returns the declared syntax capability.
    #[must_use]
    pub fn syntax_capability(&self) -> &str {
        &self.syntax_capability
    }
}

/// Parser-backed source location for one repository-owned trace link.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MaterializedTraceLink {
    link: ExternalTraceLink,
    item: ParsedItemIdentity,
    span: SourceSpan,
    syntax_capability: String,
}

impl MaterializedTraceLink {
    /// Constructs one exact trace-link source mapping.
    ///
    /// # Errors
    ///
    /// Rejects a malformed versioned syntax capability.
    pub fn new(
        link: ExternalTraceLink,
        item: ParsedItemIdentity,
        span: SourceSpan,
        syntax_capability: &str,
    ) -> Result<Self, AdapterSdkError> {
        if !versioned_schema_id(syntax_capability) {
            return Err(AdapterSdkError::Capability);
        }
        Ok(Self::from_validated(link, item, span, syntax_capability))
    }

    fn from_validated(
        link: ExternalTraceLink,
        item: ParsedItemIdentity,
        span: SourceSpan,
        syntax_capability: &str,
    ) -> Self {
        Self {
            link,
            item,
            span,
            syntax_capability: syntax_capability.to_owned(),
        }
    }

    /// Returns the complete trace link.
    #[must_use]
    pub const fn link(&self) -> &ExternalTraceLink {
        &self.link
    }

    /// Returns the complete parsed-item identity.
    #[must_use]
    pub const fn item(&self) -> &ParsedItemIdentity {
        &self.item
    }

    /// Returns the exact original-byte source span.
    #[must_use]
    pub const fn span(&self) -> &SourceSpan {
        &self.span
    }

    /// Returns the declared syntax capability.
    #[must_use]
    pub fn syntax_capability(&self) -> &str {
        &self.syntax_capability
    }
}

impl NonnormativeDisposition {
    /// Constructs one explicit nonnormative disposition.
    ///
    /// # Errors
    ///
    /// Rejects a malformed versioned syntax capability.
    pub fn new(span: SourceSpan, syntax_capability: &str) -> Result<Self, AdapterSdkError> {
        if !versioned_schema_id(syntax_capability) {
            return Err(AdapterSdkError::Capability);
        }
        Ok(Self::from_validated(span, syntax_capability))
    }

    fn from_validated(span: SourceSpan, syntax_capability: &str) -> Self {
        Self {
            span,
            syntax_capability: syntax_capability.to_owned(),
        }
    }

    /// Parser-backed original-byte source span.
    #[must_use]
    pub const fn span(&self) -> &SourceSpan {
        &self.span
    }

    /// Declared syntax capability owning this nonnormative form.
    #[must_use]
    pub fn syntax_capability(&self) -> &str {
        &self.syntax_capability
    }
}

/// Complete value-only product returned by a shell adapter.
///
/// The graph may contain only validated `buoy-core` values. Every graph object
/// must have at least one exact parser-backed source mapping. All remaining
/// selected syntax must be represented by a nonnormative disposition or stable
/// diagnostic.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AdapterProduct {
    graph: SpecGraph,
    sources: Vec<MaterializedSource>,
    external_sources: Vec<ExternalMaterializedSource>,
    trace_links: Vec<MaterializedTraceLink>,
    source: SourceProduct,
    nonnormative: Vec<NonnormativeDisposition>,
    diagnostics: Vec<AdapterDiagnostic>,
    advisory_declaration_kinds: Vec<ObjectKind>,
}

impl AdapterProduct {
    /// Constructs one complete adapter product.
    ///
    /// # Errors
    ///
    /// Rejects an unencodable graph, missing or unrelated source mapping, or
    /// duplicate output rows.
    pub fn new(
        graph: SpecGraph,
        sources: Vec<MaterializedSource>,
        nonnormative: Vec<NonnormativeDisposition>,
        diagnostics: Vec<AdapterDiagnostic>,
    ) -> Result<Self, AdapterSdkError> {
        Self::new_with_advisory(
            graph,
            sources,
            Vec::new(),
            Vec::new(),
            nonnormative,
            diagnostics,
            Vec::new(),
        )
    }

    /// Constructs a product that retains source nodes and unresolved references.
    ///
    /// # Errors
    ///
    /// Rejects invalid core mappings or duplicate output rows.
    pub fn new_with_source_product(
        graph: SpecGraph,
        sources: Vec<MaterializedSource>,
        source: SourceProduct,
        nonnormative: Vec<NonnormativeDisposition>,
        diagnostics: Vec<AdapterDiagnostic>,
        advisory_declaration_kinds: Vec<ObjectKind>,
    ) -> Result<Self, AdapterSdkError> {
        Self::new_complete(
            graph,
            sources,
            Vec::new(),
            Vec::new(),
            source,
            nonnormative,
            diagnostics,
            advisory_declaration_kinds,
        )
    }

    /// Constructs a product that retains source references and external objects.
    ///
    /// # Errors
    ///
    /// Rejects invalid core mappings or duplicate output rows.
    pub fn new_with_source_and_external(
        graph: SpecGraph,
        sources: Vec<MaterializedSource>,
        external_sources: Vec<ExternalMaterializedSource>,
        source: SourceProduct,
        nonnormative: Vec<NonnormativeDisposition>,
        diagnostics: Vec<AdapterDiagnostic>,
        advisory_declaration_kinds: Vec<ObjectKind>,
    ) -> Result<Self, AdapterSdkError> {
        Self::new_complete(
            graph,
            sources,
            external_sources,
            Vec::new(),
            source,
            nonnormative,
            diagnostics,
            advisory_declaration_kinds,
        )
    }

    /// Constructs a product that contains external semantic objects.
    ///
    /// # Errors
    ///
    /// Rejects invalid core mappings or duplicate output rows.
    pub fn new_with_external(
        graph: SpecGraph,
        sources: Vec<MaterializedSource>,
        external_sources: Vec<ExternalMaterializedSource>,
        nonnormative: Vec<NonnormativeDisposition>,
        diagnostics: Vec<AdapterDiagnostic>,
    ) -> Result<Self, AdapterSdkError> {
        Self::new_with_advisory(
            graph,
            sources,
            external_sources,
            Vec::new(),
            nonnormative,
            diagnostics,
            Vec::new(),
        )
    }

    /// Constructs a product with retained sources, external objects, and traces.
    ///
    /// One host adapter that owns both an authority tree and nonauthority trace
    /// sections returns every retained row through this constructor.
    ///
    /// # Errors
    ///
    /// Rejects invalid core mappings or duplicate output rows.
    #[allow(clippy::too_many_arguments)]
    pub fn new_with_source_external_and_traces(
        graph: SpecGraph,
        sources: Vec<MaterializedSource>,
        external_sources: Vec<ExternalMaterializedSource>,
        trace_links: Vec<MaterializedTraceLink>,
        source: SourceProduct,
        nonnormative: Vec<NonnormativeDisposition>,
        diagnostics: Vec<AdapterDiagnostic>,
        advisory_declaration_kinds: Vec<ObjectKind>,
    ) -> Result<Self, AdapterSdkError> {
        Self::new_complete(
            graph,
            sources,
            external_sources,
            trace_links,
            source,
            nonnormative,
            diagnostics,
            advisory_declaration_kinds,
        )
    }

    /// Constructs a product that contains repository-owned trace links.
    ///
    /// # Errors
    ///
    /// Rejects invalid core mappings or duplicate output rows.
    pub fn new_with_trace_links(
        graph: SpecGraph,
        sources: Vec<MaterializedSource>,
        trace_links: Vec<MaterializedTraceLink>,
        nonnormative: Vec<NonnormativeDisposition>,
        diagnostics: Vec<AdapterDiagnostic>,
        advisory_declaration_kinds: Vec<ObjectKind>,
    ) -> Result<Self, AdapterSdkError> {
        Self::new_with_advisory(
            graph,
            sources,
            Vec::new(),
            trace_links,
            nonnormative,
            diagnostics,
            advisory_declaration_kinds,
        )
    }

    fn new_with_advisory(
        graph: SpecGraph,
        sources: Vec<MaterializedSource>,
        external_sources: Vec<ExternalMaterializedSource>,
        trace_links: Vec<MaterializedTraceLink>,
        nonnormative: Vec<NonnormativeDisposition>,
        diagnostics: Vec<AdapterDiagnostic>,
        advisory_declaration_kinds: Vec<ObjectKind>,
    ) -> Result<Self, AdapterSdkError> {
        Self::new_complete(
            graph,
            sources,
            external_sources,
            trace_links,
            SourceProduct::default(),
            nonnormative,
            diagnostics,
            advisory_declaration_kinds,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn new_complete(
        graph: SpecGraph,
        mut sources: Vec<MaterializedSource>,
        mut external_sources: Vec<ExternalMaterializedSource>,
        mut trace_links: Vec<MaterializedTraceLink>,
        source: SourceProduct,
        mut nonnormative: Vec<NonnormativeDisposition>,
        mut diagnostics: Vec<AdapterDiagnostic>,
        mut advisory_declaration_kinds: Vec<ObjectKind>,
    ) -> Result<Self, AdapterSdkError> {
        semantic_graph_digest(&graph).map_err(|_| AdapterSdkError::Materialization)?;
        let expected = graph
            .object_meta()
            .into_iter()
            .map(|meta| ObjectRef::new(meta.kind(), meta.id().clone(), meta.digest()))
            .collect::<BTreeSet<_>>();
        let actual = sources
            .iter()
            .map(|source| source.object.clone())
            .chain(trace_links.iter().map(|source| {
                ObjectRef::new(
                    source.link().meta().kind(),
                    source.link().meta().id().clone(),
                    source.link().meta().digest(),
                )
            }))
            .collect::<BTreeSet<_>>();
        if expected != actual {
            return Err(AdapterSdkError::Materialization);
        }
        sources = canonicalize_rows(sources, source_value)?;
        external_sources = canonicalize_rows(external_sources, external_source_value)?;
        trace_links = canonicalize_rows(trace_links, trace_link_value)?;
        nonnormative = canonicalize_rows(nonnormative, nonnormative_value)?;
        diagnostics = canonicalize_rows(diagnostics, diagnostic_value)?;
        advisory_declaration_kinds.sort_unstable();
        Ok(Self {
            graph,
            sources,
            external_sources,
            trace_links,
            source,
            nonnormative,
            diagnostics,
            advisory_declaration_kinds,
        })
    }

    /// Complete validated core graph values.
    #[must_use]
    pub const fn graph(&self) -> &SpecGraph {
        &self.graph
    }

    /// Canonically ordered parser-backed object source mappings.
    #[must_use]
    pub fn sources(&self) -> &[MaterializedSource] {
        &self.sources
    }

    /// Canonically ordered external object source mappings.
    #[must_use]
    pub fn external_sources(&self) -> &[ExternalMaterializedSource] {
        &self.external_sources
    }

    /// Canonically ordered external trace links and source mappings.
    #[must_use]
    pub fn trace_links(&self) -> &[MaterializedTraceLink] {
        &self.trace_links
    }

    /// Complete retained source-reference product.
    #[must_use]
    pub const fn source(&self) -> &SourceProduct {
        &self.source
    }

    /// Canonically ordered explicit nonnormative dispositions.
    #[must_use]
    pub fn nonnormative(&self) -> &[NonnormativeDisposition] {
        &self.nonnormative
    }

    /// Canonically ordered deterministic diagnostics.
    #[must_use]
    pub fn diagnostics(&self) -> &[AdapterDiagnostic] {
        &self.diagnostics
    }
}

/// Fully bound and registry-validated adapter materialization.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MaterializedOutput {
    descriptor: AdapterDescriptor,
    configuration_digest: SemanticDigest,
    input_digest: SemanticDigest,
    product: AdapterProduct,
    digest: SemanticDigest,
}

impl MaterializedOutput {
    /// Frozen descriptor used for this materialization.
    #[must_use]
    pub const fn descriptor(&self) -> &AdapterDescriptor {
        &self.descriptor
    }

    /// Exact explicit configuration binding used for this materialization.
    #[must_use]
    pub const fn configuration_digest(&self) -> SemanticDigest {
        self.configuration_digest
    }

    /// Exact saved path/bytes/configuration input binding.
    #[must_use]
    pub const fn input_digest(&self) -> SemanticDigest {
        self.input_digest
    }

    /// Complete validated adapter product.
    #[must_use]
    pub const fn product(&self) -> &AdapterProduct {
        &self.product
    }

    /// Complete canonical materialization identity.
    #[must_use]
    pub const fn digest(&self) -> SemanticDigest {
        self.digest
    }

    pub(crate) fn into_check_parts(
        self,
    ) -> (
        SpecGraph,
        Vec<MaterializedSource>,
        Vec<AdapterDiagnostic>,
        Vec<ObjectKind>,
        SourceProduct,
    ) {
        let AdapterProduct {
            graph,
            mut sources,
            external_sources: _,
            trace_links,
            source,
            nonnormative: _,
            diagnostics,
            advisory_declaration_kinds,
        } = self.product;
        sources.extend(trace_links.into_iter().map(|trace| {
            let MaterializedTraceLink {
                link,
                item,
                span,
                syntax_capability,
            } = trace;
            let object = ObjectRef::new(
                link.meta().kind(),
                link.meta().id().clone(),
                link.meta().digest(),
            );
            MaterializedSource::from_validated(object, item, span, &syntax_capability)
        }));
        (
            graph,
            sources,
            diagnostics,
            advisory_declaration_kinds,
            source,
        )
    }
}

/// Shell-side adapter implementation contract.
///
/// Implementations may call parsers because this trait exists only in the
/// imperative shell. They must not pass callbacks, parser values, or host UX
/// state into `buoy-core`.
pub trait ShellAdapter: Send + Sync {
    /// Returns the adapter's immutable descriptor.
    fn descriptor(&self) -> &AdapterDescriptor;

    /// Materializes one saved-snapshot input into complete value-only output.
    ///
    /// # Errors
    ///
    /// Returns a stable SDK error when explicit configuration or materialized
    /// output cannot satisfy the published contract.
    fn materialize(
        &self,
        input: &SavedSnapshotInput<'_>,
    ) -> Result<AdapterProduct, AdapterSdkError>;
}

struct RegisteredAdapter {
    frozen_descriptor: AdapterDescriptor,
    implementation: Box<dyn ShellAdapter>,
}

/// Deterministic shell-side registry keyed by immutable adapter identity.
#[derive(Default)]
pub struct AdapterRegistry {
    adapters: BTreeMap<String, RegisteredAdapter>,
}

impl AdapterRegistry {
    /// Constructs an empty registry.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            adapters: BTreeMap::new(),
        }
    }

    /// Registers one adapter and freezes its complete descriptor.
    ///
    /// # Errors
    ///
    /// Rejects duplicate adapter identities.
    pub fn register<A>(&mut self, adapter: A) -> Result<(), AdapterSdkError>
    where
        A: ShellAdapter + 'static,
    {
        self.register_boxed(Box::new(adapter))
    }

    fn register_boxed(&mut self, adapter: Box<dyn ShellAdapter>) -> Result<(), AdapterSdkError> {
        let descriptor = adapter.descriptor().clone();
        if self.adapters.contains_key(descriptor.adapter_id()) {
            return Err(AdapterSdkError::DuplicateAdapter);
        }
        self.adapters.insert(
            descriptor.adapter_id().to_owned(),
            RegisteredAdapter {
                frozen_descriptor: descriptor,
                implementation: adapter,
            },
        );
        Ok(())
    }

    /// Reports whether one exact adapter identity is registered.
    #[must_use]
    pub fn contains(&self, adapter_id: &str) -> bool {
        self.adapters.contains_key(adapter_id)
    }

    /// Returns frozen descriptors in canonical adapter-ID order.
    #[must_use]
    pub fn descriptors(&self) -> Vec<&AdapterDescriptor> {
        self.adapters
            .values()
            .map(|registered| &registered.frozen_descriptor)
            .collect()
    }

    /// Materializes and validates one explicitly bound saved-snapshot input.
    ///
    /// # Errors
    ///
    /// Rejects missing or changed descriptors, mismatched configuration,
    /// unsupported capabilities, invalid spans, incomplete disposition, or
    /// canonical output encoding failure.
    pub fn materialize(
        &self,
        input: &SavedSnapshotInput<'_>,
    ) -> Result<MaterializedOutput, AdapterSdkError> {
        let adapter_id = input.configuration().adapter_id();
        let registered = self
            .adapters
            .get(adapter_id)
            .ok_or(AdapterSdkError::UnsupportedAdapter)?;
        if registered.implementation.descriptor() != &registered.frozen_descriptor {
            return Err(AdapterSdkError::Identity);
        }
        if input.configuration().schema_id()
            != registered.frozen_descriptor.configuration_schema_id()
        {
            return Err(AdapterSdkError::Configuration);
        }
        let product = registered.implementation.materialize(input)?;
        validate_product(&registered.frozen_descriptor, input, &product)?;
        let digest = materialized_digest(
            &registered.frozen_descriptor,
            input.configuration().digest(),
            input.digest(),
            &product,
        );
        Ok(MaterializedOutput {
            descriptor: registered.frozen_descriptor.clone(),
            configuration_digest: input.configuration().digest(),
            input_digest: input.digest(),
            product,
            digest,
        })
    }
}

/// Constructs the published registry containing the pinned Markdown and Rust adapters.
///
/// # Errors
///
/// The result shape preserves the registry-construction interface; the pinned
/// built-ins return `Ok` after their frozen declarations are validated.
///
/// # Panics
///
/// Panics if a compiled-in Markdown or Rust declaration violates the frozen
/// descriptor, capability, uniqueness, or encoding contract.
#[allow(clippy::expect_used)]
pub fn built_in_registry() -> Result<AdapterRegistry, AdapterSdkError> {
    let mut registry = AdapterRegistry::new();
    registry
        .register(ArtifactAdapter::new())
        .expect("the built-in artifact identity must be unique");
    registry
        .register(BindingAdapter::new())
        .expect("the built-in binding identity must be unique");
    registry
        .register(MarkdownAdapter::new())
        .expect("the built-in Markdown identity must be unique");
    registry
        .register(RustAdapter::new())
        .expect("the built-in Rust identity must be unique");
    registry
        .register(OpenSpecAdapter::new())
        .expect("the built-in OpenSpec identity must be unique");
    Ok(registry)
}

/// Constructs the exact built-in adapter configuration for one selector expectation.
///
/// # Errors
///
/// Rejects an adapter outside the published built-in registry.
pub fn built_in_configuration(
    adapter_id: &str,
    expectation: AuthorityExpectation,
) -> Result<AdapterConfiguration, AdapterSdkError> {
    if !matches!(adapter_id, "artifact/v1" | "markdown/v1" | "rust/v1") {
        return Err(AdapterSdkError::UnsupportedAdapter);
    }
    AdapterConfiguration::new(
        adapter_id,
        BUILTIN_CONFIGURATION_SCHEMA_ID,
        expectation_name(expectation).as_bytes().to_vec(),
    )
}

/// Binds the repository-selected digest member to the object-store adapter.
///
/// A compact locator names no algorithm, so `binding/v1` has no default member
/// and refuses to run without this explicit repository selection.
///
/// # Errors
///
/// Returns a configuration error when the bound bytes exceed SDK bounds.
pub fn built_in_binding_configuration(
    active: ActiveAlgorithm,
    expectation: AuthorityExpectation,
) -> Result<AdapterConfiguration, AdapterSdkError> {
    AdapterConfiguration::new(
        "binding/v1",
        BUILTIN_CONFIGURATION_SCHEMA_ID,
        format!(
            "{}\n{}",
            expectation_name(expectation),
            active.algorithm().canonical_name()
        )
        .into_bytes(),
    )
}

/// Binds the selected member and one logical mount prefix to `binding/v1`.
///
/// # Errors
///
/// Returns a configuration error when the bound bytes exceed SDK limits.
pub fn built_in_mounted_binding_configuration(
    active: ActiveAlgorithm,
    expectation: AuthorityExpectation,
    mount_prefix: &RepositoryPath,
) -> Result<AdapterConfiguration, AdapterSdkError> {
    AdapterConfiguration::new(
        "binding/v1",
        BUILTIN_CONFIGURATION_SCHEMA_ID,
        format!(
            "{}\n{}\n{}",
            expectation_name(expectation),
            active.algorithm().canonical_name(),
            mount_prefix.as_str(),
        )
        .into_bytes(),
    )
}

/// Validates the versioned identities used to construct a parsed item.
///
/// The adapter descriptor performs the source item kind membership check.
///
/// # Errors
///
/// Returns `Identity` when an adapter or language schema identity is malformed.
pub fn validate_parsed_item_protocol(
    adapter_id: &str,
    language_schema_id: &str,
    _kind: &SourceItemKind,
) -> Result<(), AdapterSdkError> {
    if versioned_schema_id(adapter_id) && versioned_schema_id(language_schema_id) {
        Ok(())
    } else {
        Err(AdapterSdkError::Identity)
    }
}

struct ArtifactAdapter {
    descriptor: AdapterDescriptor,
}

impl ArtifactAdapter {
    #[allow(clippy::expect_used)]
    fn new() -> Self {
        Self {
            descriptor: AdapterDescriptor::new(
                ARTIFACT_ADAPTER_ID,
                "opaque-bytes",
                "1.0.0",
                ARTIFACT_CAPABILITY,
                BUILTIN_CONFIGURATION_SCHEMA_ID,
                vec![ARTIFACT_CAPABILITY.to_owned()],
                vec![
                    SourceItemKind::new(ARTIFACT_FILE_KIND)
                        .expect("the artifact source item kind is valid"),
                ],
            )
            .expect("the reviewed built-in artifact descriptor must remain valid"),
        }
    }
}

impl ShellAdapter for ArtifactAdapter {
    fn descriptor(&self) -> &AdapterDescriptor {
        &self.descriptor
    }

    fn materialize(
        &self,
        input: &SavedSnapshotInput<'_>,
    ) -> Result<AdapterProduct, AdapterSdkError> {
        if parse_expectation(input.configuration().bytes())? != AuthorityExpectation::Forbidden {
            return Err(AdapterSdkError::Configuration);
        }
        let output = extract_artifact(input.active_algorithm(), input.path(), input.bytes());
        let root = output.node.reference();
        let source = SourceProduct {
            nodes: vec![MaterializedSourceNode::from_validated(
                output.node,
                output.span,
                ARTIFACT_CAPABILITY,
            )],
            roots: vec![DeclaredSourceRoot::new(root, SourceRootClass::Navigational)],
            ..SourceProduct::default()
        };
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

struct BindingAdapter {
    descriptor: AdapterDescriptor,
}

impl BindingAdapter {
    #[allow(clippy::expect_used)]
    fn new() -> Self {
        Self {
            descriptor: AdapterDescriptor::new(
                "binding/v1",
                "toml",
                "0.9.5",
                "binding/object-store/v1",
                BUILTIN_CONFIGURATION_SCHEMA_ID,
                vec![BINDING_NONNORMATIVE_CAPABILITY.to_owned()],
                vec![
                    SourceItemKind::new("binding/object/v1")
                        .expect("the binding object item kind is valid"),
                ],
            )
            .expect("the reviewed built-in binding descriptor must remain valid"),
        }
    }
}

impl ShellAdapter for BindingAdapter {
    fn descriptor(&self) -> &AdapterDescriptor {
        &self.descriptor
    }

    fn materialize(
        &self,
        input: &SavedSnapshotInput<'_>,
    ) -> Result<AdapterProduct, AdapterSdkError> {
        let (_, active, mount_prefix) = parse_binding_configuration(input.configuration().bytes())?;
        let output =
            extract_binding_in_mount(active, mount_prefix.as_ref(), input.path(), input.bytes());
        let source = SourceProduct {
            artifact_attachments: output.attachment.into_iter().collect(),
            bindings: output.stored.into_iter().collect(),
            ..SourceProduct::default()
        };
        let mut nonnormative = Vec::new();
        ensure_disposition(
            input,
            &[],
            &output.diagnostics,
            &mut nonnormative,
            BINDING_NONNORMATIVE_CAPABILITY,
        );
        AdapterProduct::new_with_source_product(
            SpecGraph::default(),
            Vec::new(),
            source,
            nonnormative,
            output.diagnostics,
            Vec::new(),
        )
    }
}

struct MarkdownAdapter {
    descriptor: AdapterDescriptor,
}

impl MarkdownAdapter {
    #[allow(clippy::expect_used)]
    fn new() -> Self {
        Self {
            descriptor: AdapterDescriptor::new(
                "markdown/v1",
                "pulldown-cmark",
                "0.13.5",
                "markdown/v1",
                BUILTIN_CONFIGURATION_SCHEMA_ID,
                vec![
                    MARKDOWN_NONNORMATIVE_CAPABILITY.to_owned(),
                    MARKDOWN_SEMANTIC_CAPABILITY.to_owned(),
                ],
                vec![
                    SourceItemKind::new("markdown/external-trace/v1")
                        .expect("the Markdown trace item kind is valid"),
                    SourceItemKind::new("markdown/fence/v1")
                        .expect("the Markdown source item kind is valid"),
                ],
            )
            .expect("the reviewed built-in Markdown descriptor must remain valid"),
        }
    }
}

impl ShellAdapter for MarkdownAdapter {
    fn descriptor(&self) -> &AdapterDescriptor {
        &self.descriptor
    }

    fn materialize(
        &self,
        input: &SavedSnapshotInput<'_>,
    ) -> Result<AdapterProduct, AdapterSdkError> {
        let expectation = parse_expectation(input.configuration().bytes())?;
        let output = extract_markdown(input.path(), input.bytes(), expectation);
        let mut graph = SpecGraph::default();
        let mut sources = Vec::new();
        let trace_links = output
            .external_links
            .into_iter()
            .map(|located| {
                MaterializedTraceLink::from_validated(
                    located.value,
                    located.item,
                    located.span,
                    MARKDOWN_SEMANTIC_CAPABILITY,
                )
            })
            .collect::<Vec<_>>();
        graph
            .external_trace_links
            .extend(trace_links.iter().map(|source| source.link().clone()));
        let diagnostics = output.diagnostics;
        let mut advisory = output
            .semantic
            .iter()
            .map(|located| located.value.meta().kind())
            .collect::<Vec<_>>();
        advisory.extend(trace_links.iter().map(|_| ObjectKind::ExternalTraceLink));
        for located in output.semantic {
            let meta = located.value.meta();
            let object = ObjectRef::new(meta.kind(), meta.id().clone(), meta.digest());
            sources.push(MaterializedSource::from_validated(
                object,
                located.item,
                located.span,
                MARKDOWN_SEMANTIC_CAPABILITY,
            ));
            match located.value {
                MarkdownSemantic::Atom(value) => graph.atoms.push(value),
                MarkdownSemantic::DecisionSurface(value) => graph.decision_surfaces.push(value),
                MarkdownSemantic::CoveragePolicy(value) => graph.coverage_policies.push(value),
                MarkdownSemantic::DebtRecord(value) => graph.debt_records.push(value),
                MarkdownSemantic::Waiver(value) => graph.waivers.push(value),
                MarkdownSemantic::Reaffirmation(value) => graph.reaffirmations.push(value),
                MarkdownSemantic::Derivation(value) => graph.derivations.push(value),
                MarkdownSemantic::Assumption(value) => graph.assumptions.push(value),
                MarkdownSemantic::FiniteModel(value) => graph.finite_models.push(value),
                MarkdownSemantic::FormalProperty(value) => graph.formal_properties.push(value),
                MarkdownSemantic::ProofRecord(value) => graph.proof_records.push(value),
            }
        }
        let mut nonnormative = output
            .nonnormative
            .into_iter()
            .map(|span| {
                NonnormativeDisposition::from_validated(span, MARKDOWN_NONNORMATIVE_CAPABILITY)
            })
            .collect::<Vec<_>>();
        ensure_disposition(
            input,
            &sources,
            &diagnostics,
            &mut nonnormative,
            MARKDOWN_NONNORMATIVE_CAPABILITY,
        );
        AdapterProduct::new_with_trace_links(
            graph,
            sources,
            trace_links,
            nonnormative,
            diagnostics,
            advisory,
        )
    }
}

struct RustAdapter {
    descriptor: AdapterDescriptor,
}

impl RustAdapter {
    #[allow(clippy::expect_used)]
    fn new() -> Self {
        Self {
            descriptor: AdapterDescriptor::new(
                "rust/v1",
                "syn",
                "2.0.106",
                "rust/v1",
                BUILTIN_CONFIGURATION_SCHEMA_ID,
                vec![
                    RUST_SEMANTIC_CAPABILITY.to_owned(),
                    RUST_NONNORMATIVE_CAPABILITY.to_owned(),
                ],
                vec![
                    SourceItemKind::new("rust/item/v1")
                        .expect("the Rust source item kind is valid"),
                ],
            )
            .expect("the reviewed built-in Rust descriptor must remain valid"),
        }
    }
}

impl ShellAdapter for RustAdapter {
    fn descriptor(&self) -> &AdapterDescriptor {
        &self.descriptor
    }

    fn materialize(
        &self,
        input: &SavedSnapshotInput<'_>,
    ) -> Result<AdapterProduct, AdapterSdkError> {
        let _ = parse_expectation(input.configuration().bytes())?;
        let output = extract_rust_with(input.active_algorithm(), input.path(), input.bytes());
        let mut graph = SpecGraph::default();
        let mut sources = Vec::new();
        let diagnostics = output.diagnostics;
        let advisory = output
            .semantic
            .iter()
            .map(|located| located.value.meta().kind())
            .collect();
        for located in output.semantic {
            let meta = located.value.meta();
            let object = ObjectRef::new(meta.kind(), meta.id().clone(), meta.digest());
            sources.push(MaterializedSource::from_validated(
                object,
                located.item,
                located.span,
                RUST_SEMANTIC_CAPABILITY,
            ));
            match located.value {
                RustSemantic::EvidenceCarrier(value) => graph.evidence_carriers.push(value),
                RustSemantic::ImplementationLink(value) => graph.implementation_links.push(value),
                RustSemantic::DecisionSurface(value) => graph.decision_surfaces.push(value),
                RustSemantic::Derivation(value) => graph.derivations.push(value),
            }
        }
        let retained = output
            .source_nodes
            .into_iter()
            .map(|located| {
                MaterializedSourceNode::from_validated(
                    located.node,
                    located.span,
                    RUST_SEMANTIC_CAPABILITY,
                )
            })
            .collect::<Vec<_>>();
        let roots = retained
            .iter()
            .map(|located| {
                DeclaredSourceRoot::new(located.node().reference(), SourceRootClass::Navigational)
            })
            .collect();
        let source = SourceProduct::new(
            retained,
            Vec::new(),
            roots,
            Vec::new(),
            output
                .bindings
                .into_iter()
                .map(|binding| BindingAttachment::new(binding.subject, binding.record))
                .collect(),
            Vec::new(),
        )?;
        let mut nonnormative = output
            .nonnormative
            .into_iter()
            .map(|span| NonnormativeDisposition::from_validated(span, RUST_NONNORMATIVE_CAPABILITY))
            .collect::<Vec<_>>();
        ensure_disposition(
            input,
            &sources,
            &diagnostics,
            &mut nonnormative,
            RUST_NONNORMATIVE_CAPABILITY,
        );
        AdapterProduct::new_with_source_product(
            graph,
            sources,
            source,
            nonnormative,
            diagnostics,
            advisory,
        )
    }
}

struct OpenSpecAdapter {
    descriptor: AdapterDescriptor,
}

impl OpenSpecAdapter {
    #[allow(clippy::expect_used)]
    fn new() -> Self {
        Self {
            descriptor: AdapterDescriptor::new(
                "openspec/v1",
                "pulldown-cmark",
                "0.13.5",
                "openspec/standing-spec/v1",
                BUILTIN_CONFIGURATION_SCHEMA_ID,
                vec![
                    OPENSPEC_SEMANTIC_CAPABILITY.to_owned(),
                    OPENSPEC_NONNORMATIVE_CAPABILITY.to_owned(),
                ],
                vec![
                    SourceItemKind::new("openspec/document/v1")
                        .expect("the OpenSpec document kind is valid"),
                    SourceItemKind::new(OPENSPEC_EXTERNAL_TRACE_KIND)
                        .expect("the OpenSpec trace item kind is valid"),
                    SourceItemKind::new("openspec/requirement/v1")
                        .expect("the OpenSpec requirement kind is valid"),
                    SourceItemKind::new("openspec/scenario/v1")
                        .expect("the OpenSpec scenario kind is valid"),
                    SourceItemKind::new("openspec/step/v1")
                        .expect("the OpenSpec step kind is valid"),
                ],
            )
            .expect("the reviewed built-in OpenSpec descriptor must remain valid"),
        }
    }
}

impl ShellAdapter for OpenSpecAdapter {
    fn descriptor(&self) -> &AdapterDescriptor {
        &self.descriptor
    }

    #[allow(clippy::expect_used)]
    fn materialize(
        &self,
        input: &SavedSnapshotInput<'_>,
    ) -> Result<AdapterProduct, AdapterSdkError> {
        let configuration = OpenSpecConfiguration::parse(input.configuration().bytes())
            .map_err(|_| AdapterSdkError::Configuration)?;
        let output = extract_openspec_configured_with(
            input.active_algorithm(),
            input.path(),
            input.bytes(),
            &configuration,
        );
        let mut graph = SpecGraph::default();
        let mut sources = Vec::new();
        let mut external_sources = Vec::new();
        let mut advisory = Vec::new();
        for located in output.semantic {
            if let Some(atom) = located.value.atom().cloned() {
                let object = ObjectRef::new(
                    atom.meta().kind(),
                    atom.meta().id().clone(),
                    atom.meta().digest(),
                );
                sources.push(MaterializedSource::from_validated(
                    object,
                    located.item,
                    located.span,
                    OPENSPEC_SEMANTIC_CAPABILITY,
                ));
                advisory.push(ObjectKind::Atom);
                graph.atoms.push(atom);
            } else {
                external_sources.push(ExternalMaterializedSource::from_validated(
                    located.value.external().clone(),
                    located.item,
                    located.span,
                    OPENSPEC_SEMANTIC_CAPABILITY,
                ));
            }
        }
        let source = SourceProduct::new(
            output
                .source_nodes
                .into_iter()
                .map(|located| {
                    MaterializedSourceNode::from_validated(
                        located.node,
                        located.span,
                        OPENSPEC_SEMANTIC_CAPABILITY,
                    )
                })
                .collect(),
            output.source_edges,
            output
                .document_root
                .into_iter()
                .map(|root| DeclaredSourceRoot::new(root, SourceRootClass::StandingSpecification))
                .collect(),
            output.authorities,
            Vec::new(),
            Vec::new(),
        )
        .expect("validated OpenSpec extraction produces one closed source product");
        let nonnormative = output
            .nonnormative
            .into_iter()
            .map(|span| {
                NonnormativeDisposition::from_validated(span, OPENSPEC_NONNORMATIVE_CAPABILITY)
            })
            .collect();
        // Version 1 returns no trace link, so this pass is empty for it.
        let trace_links = output
            .external_links
            .into_iter()
            .map(|located| {
                MaterializedTraceLink::from_validated(
                    located.value,
                    located.item,
                    located.span,
                    OPENSPEC_SEMANTIC_CAPABILITY,
                )
            })
            .collect::<Vec<_>>();
        graph
            .external_trace_links
            .extend(trace_links.iter().map(|source| source.link().clone()));
        advisory.extend(trace_links.iter().map(|_| ObjectKind::ExternalTraceLink));
        AdapterProduct::new_with_source_external_and_traces(
            graph,
            sources,
            external_sources,
            trace_links,
            source,
            nonnormative,
            output.diagnostics,
            advisory,
        )
    }
}

fn ensure_disposition(
    input: &SavedSnapshotInput<'_>,
    sources: &[MaterializedSource],
    diagnostics: &[AdapterDiagnostic],
    nonnormative: &mut Vec<NonnormativeDisposition>,
    capability: &str,
) {
    if sources.is_empty() && diagnostics.is_empty() && nonnormative.is_empty() {
        nonnormative.push(NonnormativeDisposition::from_validated(
            SourceSpan {
                path: input.path().clone(),
                start: 0,
                end: input.bytes().len() as u64,
            },
            capability,
        ));
    }
}

#[allow(clippy::too_many_lines)]
fn validate_product(
    descriptor: &AdapterDescriptor,
    input: &SavedSnapshotInput<'_>,
    product: &AdapterProduct,
) -> Result<(), AdapterSdkError> {
    if product.sources.is_empty()
        && product.external_sources.is_empty()
        && product.trace_links.is_empty()
        && product.source.is_empty()
        && product.nonnormative.is_empty()
        && product.diagnostics.is_empty()
    {
        return Err(AdapterSdkError::Materialization);
    }
    // The registry freezes the descriptor identity, so only the standing
    // specification adapter can declare a standing-specification root. No
    // adapter classifies its own selector.
    if product
        .source
        .roots
        .iter()
        .any(|root| root.class() == SourceRootClass::StandingSpecification)
        && descriptor.adapter_id() != STANDING_SPECIFICATION_ADAPTER
    {
        return Err(AdapterSdkError::Materialization);
    }
    for retained in &product.source.nodes {
        validate_span(input, retained.span())?;
        let item = retained.node().item();
        if item.path() != input.path() {
            return Err(AdapterSdkError::Materialization);
        }
        validate_parsed_item_protocol(
            descriptor.adapter_id(),
            item.language_schema_id(),
            item.kind(),
        )?;
        if !descriptor.supports_source_item_kind(item.kind()) {
            return Err(AdapterSdkError::UndeclaredSourceItemKind);
        }
        if !descriptor.supports(retained.syntax_capability()) {
            return Err(AdapterSdkError::Capability);
        }
    }
    for source in &product.sources {
        validate_span(input, source.span())?;
        if source.item().path() != input.path() {
            return Err(AdapterSdkError::Materialization);
        }
        validate_parsed_item_protocol(
            descriptor.adapter_id(),
            source.item().language_schema_id(),
            source.item().kind(),
        )?;
        if !descriptor.supports_source_item_kind(source.item().kind()) {
            return Err(AdapterSdkError::UndeclaredSourceItemKind);
        }
        if !descriptor.supports(source.syntax_capability()) {
            return Err(AdapterSdkError::Capability);
        }
    }
    for source in &product.external_sources {
        validate_span(input, source.span())?;
        if source.item().path() != input.path() {
            return Err(AdapterSdkError::Materialization);
        }
        validate_parsed_item_protocol(
            descriptor.adapter_id(),
            source.item().language_schema_id(),
            source.item().kind(),
        )?;
        if !descriptor.supports_source_item_kind(source.item().kind()) {
            return Err(AdapterSdkError::UndeclaredSourceItemKind);
        }
        if !descriptor.supports(source.syntax_capability()) {
            return Err(AdapterSdkError::Capability);
        }
    }
    for source in &product.trace_links {
        validate_span(input, source.span())?;
        if source.item().path() != input.path() {
            return Err(AdapterSdkError::Materialization);
        }
        validate_parsed_item_protocol(
            descriptor.adapter_id(),
            source.item().language_schema_id(),
            source.item().kind(),
        )?;
        if !descriptor.supports_source_item_kind(source.item().kind()) {
            return Err(AdapterSdkError::UndeclaredSourceItemKind);
        }
        if !descriptor.supports(source.syntax_capability()) {
            return Err(AdapterSdkError::Capability);
        }
    }
    for disposition in &product.nonnormative {
        validate_span(input, disposition.span())?;
        if !descriptor.supports(disposition.syntax_capability()) {
            return Err(AdapterSdkError::Capability);
        }
    }
    for diagnostic in &product.diagnostics {
        validate_span(input, &diagnostic.span)?;
        if !diagnostic_code(diagnostic.code) {
            return Err(AdapterSdkError::Materialization);
        }
    }
    if descriptor.adapter_id() == ARTIFACT_ADAPTER_ID {
        validate_artifact_product(input, product)?;
    }
    Ok(())
}

#[allow(clippy::expect_used)]
fn validate_artifact_product(
    input: &SavedSnapshotInput<'_>,
    product: &AdapterProduct,
) -> Result<(), AdapterSdkError> {
    let [retained] = product.source.nodes.as_slice() else {
        return Err(AdapterSdkError::Materialization);
    };
    let [root] = product.source.roots.as_slice() else {
        return Err(AdapterSdkError::Materialization);
    };
    let expected_semantic = CanonicalValue::Record(vec![
        (1, CanonicalValue::Text(ARTIFACT_MEDIA_TYPE.to_owned())),
        (2, CanonicalValue::Text(ARTIFACT_REPRESENTATION.to_owned())),
        (3, CanonicalValue::Bytes(input.bytes().to_vec())),
    ])
    .encode()
    .expect("fixed artifact semantic fields always encode");
    if retained.span().start != 0
        || retained.span().end != input.bytes().len() as u64
        || retained.node().item().language_schema_id() != ARTIFACT_CAPABILITY
        || retained.node().item().kind().as_str() != ARTIFACT_FILE_KIND
        || retained.node().item().semantic_bytes() != expected_semantic
        || root.node() != retained.node().reference()
        || root.class() != SourceRootClass::Navigational
        || !product.graph.object_meta().is_empty()
        || !product.sources.is_empty()
        || !product.external_sources.is_empty()
        || !product.trace_links.is_empty()
        || !product.source.edges.is_empty()
        || !product.source.authorities.is_empty()
        || !product.source.attachments.is_empty()
        || !product.source.artifact_attachments.is_empty()
        || !product.source.bindings.is_empty()
        || !product.nonnormative.is_empty()
        || !product.diagnostics.is_empty()
    {
        return Err(AdapterSdkError::Materialization);
    }
    Ok(())
}

fn validate_span(input: &SavedSnapshotInput<'_>, span: &SourceSpan) -> Result<(), AdapterSdkError> {
    let length = input.bytes().len() as u64;
    if &span.path != input.path() || span.start > span.end || span.end > length {
        return Err(AdapterSdkError::Materialization);
    }
    Ok(())
}

#[allow(clippy::expect_used)]
fn materialized_digest(
    descriptor: &AdapterDescriptor,
    configuration_digest: SemanticDigest,
    input_digest: SemanticDigest,
    product: &AdapterProduct,
) -> SemanticDigest {
    let graph_digest = semantic_graph_digest(&product.graph)
        .expect("AdapterProduct construction must retain a canonical graph");
    // `AdapterProduct::new_with_advisory` already rejects duplicates and stores
    // these rows in canonical encoded order. Re-sorting here was redundant and
    // left an impossible duplicate branch in the digest path.
    let sources = CanonicalValue::Set(product.sources.iter().map(source_value).collect());
    let external_sources = CanonicalValue::Set(
        product
            .external_sources
            .iter()
            .map(external_source_value)
            .collect(),
    );
    let trace_links =
        CanonicalValue::Set(product.trace_links.iter().map(trace_link_value).collect());
    let nonnormative = CanonicalValue::Set(
        product
            .nonnormative
            .iter()
            .map(nonnormative_value)
            .collect(),
    );
    let diagnostics =
        CanonicalValue::Set(product.diagnostics.iter().map(diagnostic_value).collect());
    let advisory_declarations = CanonicalValue::Sequence(
        product
            .advisory_declaration_kinds
            .iter()
            .map(|kind| CanonicalValue::Enum(*kind as u16, None))
            .collect(),
    );
    validated_semantic_digest(
        "shell-adapter-materialization",
        vec![
            (1, CanonicalValue::Text(ADAPTER_SDK_SCHEMA_ID.to_owned())),
            (2, CanonicalValue::Digest(descriptor.digest())),
            (3, CanonicalValue::Digest(configuration_digest)),
            (4, CanonicalValue::Digest(input_digest)),
            (5, CanonicalValue::Digest(graph_digest)),
            (6, sources),
            (7, external_sources),
            (8, trace_links),
            (9, nonnormative),
            (10, diagnostics),
            (11, advisory_declarations),
            (12, source_product_value(&product.source)),
        ],
    )
}

fn source_product_value(source: &SourceProduct) -> CanonicalValue {
    let mut fields = vec![
        (
            1,
            CanonicalValue::Set(source.nodes.iter().map(source_node_value).collect()),
        ),
        (
            2,
            CanonicalValue::Set(source.roots.iter().map(declared_root_value).collect()),
        ),
        (
            3,
            CanonicalValue::Set(
                source
                    .authorities
                    .iter()
                    .map(source_authority_value)
                    .collect(),
            ),
        ),
        (
            4,
            CanonicalValue::Set(
                source
                    .attachments
                    .iter()
                    .map(binding_attachment_value)
                    .collect(),
            ),
        ),
        (
            5,
            CanonicalValue::Set(source.bindings.iter().map(stored_binding_value).collect()),
        ),
        (
            6,
            CanonicalValue::Set(source.edges.iter().map(source_edge_value).collect()),
        ),
    ];
    if !source.artifact_attachments.is_empty() {
        fields.push((
            7,
            CanonicalValue::Set(
                source
                    .artifact_attachments
                    .iter()
                    .map(ArtifactBindingAttachment::canonical_value)
                    .collect(),
            ),
        ));
    }
    CanonicalValue::Record(fields)
}

fn declared_root_value(root: &DeclaredSourceRoot) -> CanonicalValue {
    CanonicalValue::Record(vec![
        (1, CanonicalValue::Digest(root.node().digest())),
        (2, CanonicalValue::Enum(root.class() as u16, None)),
    ])
}

fn source_edge_value(edge: &SourceEdge) -> CanonicalValue {
    CanonicalValue::Record(vec![
        (1, CanonicalValue::Digest(edge.source().digest())),
        (2, CanonicalValue::Enum(edge.relation() as u16, None)),
        (
            3,
            edge.position().map_or(CanonicalValue::None, |position| {
                CanonicalValue::Some(Box::new(CanonicalValue::U64(u64::from(position))))
            }),
        ),
        (4, CanonicalValue::Digest(edge.target().digest())),
    ])
}

fn source_node_value(retained: &MaterializedSourceNode) -> CanonicalValue {
    CanonicalValue::Record(vec![
        (1, CanonicalValue::Digest(retained.node().digest())),
        (2, parsed_item_value(retained.node().item())),
        (3, span_value(retained.span())),
        (
            4,
            CanonicalValue::Text(retained.syntax_capability().to_owned()),
        ),
    ])
}

fn source_authority_value(authority: &SourceAuthority) -> CanonicalValue {
    CanonicalValue::Record(vec![
        (1, CanonicalValue::Digest(authority.node().digest())),
        (
            2,
            CanonicalValue::Text(authority.atom().id().as_str().to_owned()),
        ),
        (3, CanonicalValue::Digest(authority.atom().revision())),
    ])
}

fn binding_attachment_value(attachment: &BindingAttachment) -> CanonicalValue {
    CanonicalValue::Record(vec![
        (1, CanonicalValue::Digest(attachment.subject().digest())),
        (2, CanonicalValue::Digest(attachment.record().digest())),
    ])
}

fn stored_binding_value(stored: &StoredBinding) -> CanonicalValue {
    CanonicalValue::Record(vec![
        (1, CanonicalValue::Digest(stored.locator().digest())),
        (2, CanonicalValue::Digest(stored.record().meta().digest())),
    ])
}

#[allow(clippy::expect_used)]
fn validated_semantic_digest(domain: &str, fields: Vec<(u16, CanonicalValue)>) -> SemanticDigest {
    semantic_digest(domain, fields)
        .expect("validated adapter fields must retain canonical encoding")
}

fn canonicalize_rows<T, F>(values: Vec<T>, canonical_value: F) -> Result<Vec<T>, AdapterSdkError>
where
    F: Fn(&T) -> CanonicalValue,
{
    let canonical = values.iter().map(canonical_value).collect::<Vec<_>>();
    let validation = validated_row_keys(&canonical);
    let mut encoded = validation.keys.into_iter().zip(values).collect::<Vec<_>>();
    encoded.sort_unstable_by(|left, right| left.0.cmp(&right.0));
    let rows = encoded.into_iter().map(|(_, value)| value).collect();
    validation.error.map_or(Ok(rows), Err)
}

struct RowKeyValidation {
    keys: Vec<Vec<u8>>,
    error: Option<AdapterSdkError>,
}

fn validated_row_keys(values: &[CanonicalValue]) -> RowKeyValidation {
    let Ok(keys) = values
        .iter()
        .map(CanonicalValue::encode)
        .collect::<Result<Vec<_>, _>>()
    else {
        return RowKeyValidation {
            keys: Vec::new(),
            error: Some(AdapterSdkError::Encoding),
        };
    };
    let mut sorted = keys.clone();
    sorted.sort_unstable();
    let error = sorted
        .windows(2)
        .any(|pair| pair[0] == pair[1])
        .then_some(AdapterSdkError::Materialization);
    RowKeyValidation { keys, error }
}

fn source_value(source: &MaterializedSource) -> CanonicalValue {
    CanonicalValue::Record(vec![
        (1, object_ref_value(source.object())),
        (2, parsed_item_value(source.item())),
        (3, span_value(source.span())),
        (
            4,
            CanonicalValue::Text(source.syntax_capability().to_owned()),
        ),
    ])
}

fn parsed_item_value(item: &ParsedItemIdentity) -> CanonicalValue {
    CanonicalValue::Record(vec![
        (
            1,
            CanonicalValue::Text(item.language_schema_id().to_owned()),
        ),
        (2, CanonicalValue::Text(item.path().as_str().to_owned())),
        (3, CanonicalValue::Text(item.kind().as_str().to_owned())),
        (4, CanonicalValue::Bytes(item.semantic_bytes().to_vec())),
        (
            5,
            item.enclosing_item()
                .map_or(CanonicalValue::None, |digest| {
                    CanonicalValue::Some(Box::new(CanonicalValue::Digest(digest)))
                }),
        ),
        (6, CanonicalValue::Digest(item.digest())),
    ])
}

fn external_source_value(source: &ExternalMaterializedSource) -> CanonicalValue {
    CanonicalValue::Record(vec![
        (
            1,
            CanonicalValue::Text(source.object().system_id().as_str().to_owned()),
        ),
        (
            2,
            CanonicalValue::Text(source.object().kind().as_str().to_owned()),
        ),
        (
            3,
            CanonicalValue::Text(source.object().id().as_str().to_owned()),
        ),
        (4, CanonicalValue::Digest(source.object().revision())),
        (5, CanonicalValue::Digest(source.object().digest())),
        (6, parsed_item_value(source.item())),
        (7, span_value(source.span())),
        (
            8,
            CanonicalValue::Text(source.syntax_capability().to_owned()),
        ),
    ])
}

fn trace_link_value(source: &MaterializedTraceLink) -> CanonicalValue {
    CanonicalValue::Record(vec![
        (
            1,
            CanonicalValue::Text(source.link().meta().id().as_str().to_owned()),
        ),
        (
            2,
            CanonicalValue::Set(
                source
                    .link()
                    .atoms()
                    .iter()
                    .map(|atom| {
                        CanonicalValue::Record(vec![
                            (1, CanonicalValue::Text(atom.id().as_str().to_owned())),
                            (2, CanonicalValue::Digest(atom.revision())),
                        ])
                    })
                    .collect(),
            ),
        ),
        (
            3,
            CanonicalValue::Enum(source.link().relation() as u16, None),
        ),
        (
            4,
            CanonicalValue::Text(source.link().target().as_str().to_owned()),
        ),
        (5, CanonicalValue::Digest(source.link().meta().digest())),
        (6, parsed_item_value(source.item())),
        (7, span_value(source.span())),
        (
            8,
            CanonicalValue::Text(source.syntax_capability().to_owned()),
        ),
    ])
}

fn nonnormative_value(disposition: &NonnormativeDisposition) -> CanonicalValue {
    CanonicalValue::Record(vec![
        (1, span_value(disposition.span())),
        (
            2,
            CanonicalValue::Text(disposition.syntax_capability().to_owned()),
        ),
    ])
}

fn diagnostic_value(diagnostic: &AdapterDiagnostic) -> CanonicalValue {
    CanonicalValue::Record(vec![
        (1, CanonicalValue::Text(diagnostic.code.to_owned())),
        (2, span_value(&diagnostic.span)),
        (3, CanonicalValue::Text(diagnostic.context.clone())),
    ])
}

fn object_ref_value(object: &ObjectRef) -> CanonicalValue {
    CanonicalValue::Record(vec![
        (1, CanonicalValue::Enum(object.kind() as u16, None)),
        (2, CanonicalValue::Text(object.id().as_str().to_owned())),
        (3, CanonicalValue::Digest(object.digest())),
    ])
}

fn span_value(span: &SourceSpan) -> CanonicalValue {
    CanonicalValue::Record(vec![
        (1, CanonicalValue::Text(span.path.as_str().to_owned())),
        (2, CanonicalValue::U64(span.start)),
        (3, CanonicalValue::U64(span.end)),
    ])
}

fn expectation_name(expectation: AuthorityExpectation) -> &'static str {
    match expectation {
        AuthorityExpectation::Forbidden => "forbidden",
        AuthorityExpectation::Optional => "optional",
        AuthorityExpectation::Required => "required",
    }
}

fn parse_expectation(bytes: &[u8]) -> Result<AuthorityExpectation, AdapterSdkError> {
    match bytes {
        b"forbidden" => Ok(AuthorityExpectation::Forbidden),
        b"optional" => Ok(AuthorityExpectation::Optional),
        b"required" => Ok(AuthorityExpectation::Required),
        _ => Err(AdapterSdkError::Configuration),
    }
}

/// Parses the authority expectation, selected member, and optional logical prefix.
fn parse_binding_configuration(
    bytes: &[u8],
) -> Result<
    (
        AuthorityExpectation,
        ActiveAlgorithm,
        Option<RepositoryPath>,
    ),
    AdapterSdkError,
> {
    let text = core::str::from_utf8(bytes).map_err(|_| AdapterSdkError::Configuration)?;
    let lines = text.split('\n').collect::<Vec<_>>();
    let (expectation, member, mount_prefix) = match lines.as_slice() {
        [expectation, member] => (*expectation, *member, None),
        [expectation, member, prefix] => (
            *expectation,
            *member,
            Some(RepositoryPath::new(prefix).map_err(|_| AdapterSdkError::Configuration)?),
        ),
        _ => return Err(AdapterSdkError::Configuration),
    };
    let active = DigestAlgorithm::from_canonical_name(member)
        .and_then(|algorithm| ActiveAlgorithm::new(algorithm).ok())
        .ok_or(AdapterSdkError::Configuration)?;
    Ok((
        parse_expectation(expectation.as_bytes())?,
        active,
        mount_prefix,
    ))
}

fn versioned_schema_id(value: &str) -> bool {
    value.contains('/') && SchemaId::new(value).is_ok()
}

fn scope_id(value: &str) -> bool {
    !value.contains('/') && ScopeId::new(value).is_ok()
}

fn exact_version(value: &str) -> bool {
    let bytes = value.as_bytes();
    if bytes.is_empty() {
        return false;
    }
    bytes[0].is_ascii_alphanumeric()
        && bytes[bytes.len() - 1].is_ascii_alphanumeric()
        && bytes.iter().any(u8::is_ascii_digit)
        && bytes
            .iter()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-' | b'+'))
}

fn diagnostic_code(value: &str) -> bool {
    !value.is_empty()
        && value
            .bytes()
            .all(|byte| byte.is_ascii_uppercase() || byte.is_ascii_digit() || byte == b'-')
}

#[cfg(all(test, feature = "coverage-controls"))]
#[allow(clippy::expect_used, clippy::too_many_lines)]
mod coverage_tests {
    use super::*;
    use crate::adapters::{extract_artifact, extract_rust};

    fn control_path() -> RepositoryPath {
        RepositoryPath::new("spec/coverage.md").expect("coverage path")
    }

    fn control_input<'a>(
        path: &'a RepositoryPath,
        bytes: &'a [u8],
        configuration: &'a AdapterConfiguration,
    ) -> SavedSnapshotInput<'a> {
        SavedSnapshotInput::new(path, bytes, configuration).expect("coverage input")
    }

    fn artifact_retained(
        path: &RepositoryPath,
        bytes: &[u8],
        schema: &str,
        kind: &str,
    ) -> MaterializedSourceNode {
        let semantic = CanonicalValue::Record(vec![
            (1, CanonicalValue::Text(ARTIFACT_MEDIA_TYPE.to_owned())),
            (2, CanonicalValue::Text(ARTIFACT_REPRESENTATION.to_owned())),
            (3, CanonicalValue::Bytes(bytes.to_vec())),
        ])
        .encode()
        .expect("artifact semantic bytes");
        let item = ParsedItemIdentity::new(
            SchemaId::new(schema).expect("artifact schema"),
            path.clone(),
            SourceItemKind::new(kind).expect("artifact kind"),
            semantic,
            None,
        )
        .expect("artifact item");
        let node = SourceNode::new(item, Vec::new()).expect("artifact node");
        MaterializedSourceNode::from_validated(
            node,
            SourceSpan {
                path: path.clone(),
                start: 0,
                end: bytes.len() as u64,
            },
            ARTIFACT_CAPABILITY,
        )
    }

    fn control_item(path: &RepositoryPath, schema: &str) -> ParsedItemIdentity {
        let kind = if schema == "rust/v1" {
            "rust/item/v1"
        } else {
            "markdown/fence/v1"
        };
        ParsedItemIdentity::new(
            SchemaId::new(schema).expect("valid schema"),
            path.clone(),
            SourceItemKind::new(kind).expect("source item kind"),
            b"coverage-item".to_vec(),
            None,
        )
        .expect("coverage parsed item")
    }

    struct ChangingDescriptorAdapter {
        frozen: AdapterDescriptor,
        changed: AdapterDescriptor,
        first_read: std::sync::atomic::AtomicBool,
    }

    impl ShellAdapter for ChangingDescriptorAdapter {
        fn descriptor(&self) -> &AdapterDescriptor {
            if self
                .first_read
                .swap(false, std::sync::atomic::Ordering::SeqCst)
            {
                &self.frozen
            } else {
                &self.changed
            }
        }

        fn materialize(
            &self,
            _input: &SavedSnapshotInput<'_>,
        ) -> Result<AdapterProduct, AdapterSdkError> {
            Err(AdapterSdkError::Materialization)
        }
    }

    struct FailingAdapter {
        descriptor: AdapterDescriptor,
    }

    impl ShellAdapter for FailingAdapter {
        fn descriptor(&self) -> &AdapterDescriptor {
            &self.descriptor
        }

        fn materialize(
            &self,
            _input: &SavedSnapshotInput<'_>,
        ) -> Result<AdapterProduct, AdapterSdkError> {
            Err(AdapterSdkError::Materialization)
        }
    }

    struct EmptyProductAdapter {
        descriptor: AdapterDescriptor,
    }

    impl ShellAdapter for EmptyProductAdapter {
        fn descriptor(&self) -> &AdapterDescriptor {
            &self.descriptor
        }

        fn materialize(
            &self,
            _input: &SavedSnapshotInput<'_>,
        ) -> Result<AdapterProduct, AdapterSdkError> {
            AdapterProduct::new(SpecGraph::default(), Vec::new(), Vec::new(), Vec::new())
        }
    }

    #[test]
    fn unit_variant_covers_adapter_invariants_and_validation_propagation() {
        let path = control_path();
        let markdown_bytes = include_bytes!("../fixtures/adapters/markdown/positive.md");
        let markdown_configuration =
            built_in_configuration("markdown/v1", AuthorityExpectation::Required)
                .expect("Markdown configuration");
        let mut registry = built_in_registry().expect("built-in registry");
        assert_eq!(AdapterSdkError::Identity.code(), "SPG-ADAPTER-SDK-IDENTITY");
        assert_eq!(
            AdapterSdkError::Capability.code(),
            "SPG-ADAPTER-SDK-CAPABILITY"
        );
        assert_eq!(
            AdapterSdkError::Configuration.code(),
            "SPG-ADAPTER-SDK-CONFIGURATION"
        );
        assert_eq!(
            AdapterSdkError::DuplicateAdapter.code(),
            "SPG-ADAPTER-SDK-DUPLICATE"
        );
        assert_eq!(
            AdapterSdkError::UnsupportedAdapter.code(),
            "SPG-ADAPTER-UNSUPPORTED"
        );
        assert_eq!(
            AdapterSdkError::Materialization.code(),
            "SPG-ADAPTER-SDK-MATERIALIZATION"
        );
        assert_eq!(
            AdapterSdkError::UndeclaredSourceItemKind.code(),
            "SPG-ADAPTER-UNDECLARED-SOURCE-ITEM-KIND"
        );
        assert_eq!(AdapterSdkError::Encoding.code(), "DET-ENC-ADAPTER-SDK");
        assert!(registry.contains("markdown/v1"));
        assert!(!registry.contains("coverage/v1"));
        assert_eq!(
            registry
                .descriptors()
                .into_iter()
                .map(AdapterDescriptor::adapter_id)
                .collect::<Vec<_>>(),
            vec![
                "artifact/v1",
                "binding/v1",
                "markdown/v1",
                "openspec/v1",
                "rust/v1",
            ]
        );
        assert_eq!(
            registry.register(MarkdownAdapter::new()),
            Err(AdapterSdkError::DuplicateAdapter)
        );
        assert_eq!(
            validate_parsed_item_protocol(
                "markdown/v1",
                "markdown/v1",
                &SourceItemKind::new("markdown/fence/v1").expect("source item kind"),
            ),
            Ok(())
        );
        assert_eq!(
            validate_parsed_item_protocol(
                "rust/v1",
                "rust/v1",
                &SourceItemKind::new("rust/item/v1").expect("source item kind"),
            ),
            Ok(())
        );
        assert_eq!(
            validate_parsed_item_protocol(
                "coverage/v1",
                "coverage/v1",
                &SourceItemKind::new("host/item/v1").expect("source item kind"),
            ),
            Ok(())
        );
        assert_eq!(
            built_in_configuration("coverage/v1", AuthorityExpectation::Forbidden),
            Err(AdapterSdkError::UnsupportedAdapter)
        );
        let forbidden_configuration =
            built_in_configuration("rust/v1", AuthorityExpectation::Forbidden)
                .expect("forbidden Rust configuration");
        assert_eq!(forbidden_configuration.bytes(), b"forbidden");
        assert_eq!(
            parse_expectation(b"forbidden"),
            Ok(AuthorityExpectation::Forbidden)
        );
        let invalid_markdown_configuration = AdapterConfiguration::new(
            "markdown/v1",
            BUILTIN_CONFIGURATION_SCHEMA_ID,
            b"invalid".to_vec(),
        )
        .expect("structurally valid Markdown configuration");
        assert_eq!(
            registry.materialize(&control_input(
                &path,
                markdown_bytes,
                &invalid_markdown_configuration,
            )),
            Err(AdapterSdkError::Configuration)
        );
        let binding_path = RepositoryPath::new(
            "buoy/objects/v1/blake3-256/1111111111111111111111111111111111111111111111111111111111111111.toml",
        )
        .expect("binding object path");
        for bytes in [
            Vec::new(),
            vec![0xff],
            b"forbidden".to_vec(),
            b"forbidden\nblake3-255".to_vec(),
            b"invalid\nblake3-256".to_vec(),
            b"forbidden\nxxh3-128\n/invalid".to_vec(),
        ] {
            let configuration =
                AdapterConfiguration::new("binding/v1", BUILTIN_CONFIGURATION_SCHEMA_ID, bytes)
                    .expect("structurally valid binding configuration");
            assert_eq!(
                registry.materialize(&control_input(&binding_path, b"", &configuration)),
                Err(AdapterSdkError::Configuration)
            );
        }
        assert!(
            SourceProduct::from_stored_binding(None)
                .bindings()
                .is_empty()
        );
        let markdown_input = control_input(&path, markdown_bytes, &markdown_configuration);
        let markdown = registry
            .materialize(&markdown_input)
            .expect("Markdown materialization");
        assert_eq!(markdown.descriptor().adapter_id(), "markdown/v1");
        assert_eq!(
            markdown.configuration_digest(),
            markdown_configuration.digest()
        );
        assert_eq!(markdown.input_digest(), markdown_input.digest());
        assert_eq!(
            markdown.digest(),
            registry
                .materialize(&markdown_input)
                .expect("repeat Markdown materialization")
                .digest()
        );
        assert_eq!(markdown.product().graph().atoms.len(), 1);
        assert_eq!(markdown.product().sources().len(), 1);
        let object_configuration =
            built_in_configuration("markdown/v1", AuthorityExpectation::Optional)
                .expect("Markdown object configuration");
        let markdown_objects = registry
            .materialize(&control_input(
                &path,
                include_bytes!("../fixtures/adapters/markdown/objects.md"),
                &object_configuration,
            ))
            .expect("Markdown object materialization");
        assert!(markdown_objects.product().diagnostics().is_empty());
        assert_eq!(markdown_objects.product().sources().len(), 10);

        let mut duplicate_graph = markdown.product().graph().clone();
        duplicate_graph.atoms.push(duplicate_graph.atoms[0].clone());
        assert_eq!(
            AdapterProduct::new(duplicate_graph, Vec::new(), Vec::new(), Vec::new()),
            Err(AdapterSdkError::Materialization)
        );

        let graph = markdown.product().graph().clone();
        assert_eq!(
            AdapterProduct::new(graph.clone(), Vec::new(), Vec::new(), Vec::new()),
            Err(AdapterSdkError::Materialization)
        );
        let atom = &graph.atoms[0];
        let object = ObjectRef::new(
            ObjectKind::Atom,
            atom.meta().id().clone(),
            atom.meta().digest(),
        );
        let descriptor = AdapterDescriptor::new(
            "markdown/v1",
            "coverage-parser",
            "1.0.0",
            "markdown/v1",
            "coverage.configuration/v1",
            vec![String::from("coverage.syntax/v1")],
            vec![SourceItemKind::new("markdown/fence/v1").expect("source item kind")],
        )
        .expect("coverage descriptor");
        assert_eq!(descriptor.parser_id(), "coverage-parser");
        assert_eq!(descriptor.parser_version(), "1.0.0");
        assert_eq!(descriptor.syntax_schema_id(), "markdown/v1");
        assert_eq!(
            descriptor.syntax_capabilities(),
            &[String::from("coverage.syntax/v1")]
        );
        let configuration =
            AdapterConfiguration::new("markdown/v1", "coverage.configuration/v1", Vec::new())
                .expect("coverage configuration");
        let input = control_input(&path, b"x", &configuration);
        let empty_product =
            AdapterProduct::new(SpecGraph::default(), Vec::new(), Vec::new(), Vec::new())
                .expect("empty adapter product");
        assert_eq!(
            validate_product(&descriptor, &input, &empty_product),
            Err(AdapterSdkError::Materialization)
        );
        let valid_span = SourceSpan {
            path: path.clone(),
            start: 0,
            end: 1,
        };
        let unsupported_source = MaterializedSource::new(
            object.clone(),
            control_item(&path, "markdown/v1"),
            valid_span.clone(),
            "coverage.unsupported/v1",
        )
        .expect("unsupported source capability remains structurally valid");
        let unsupported_source_product = AdapterProduct::new(
            graph.clone(),
            vec![unsupported_source],
            Vec::new(),
            Vec::new(),
        )
        .expect("unsupported source product");
        assert_eq!(
            validate_product(&descriptor, &input, &unsupported_source_product),
            Err(AdapterSdkError::Capability)
        );

        let unsupported_disposition =
            NonnormativeDisposition::new(valid_span.clone(), "coverage.unsupported/v1")
                .expect("unsupported disposition capability remains structurally valid");
        let unsupported_disposition_product = AdapterProduct::new(
            SpecGraph::default(),
            Vec::new(),
            vec![unsupported_disposition],
            Vec::new(),
        )
        .expect("unsupported disposition product");
        assert_eq!(
            validate_product(&descriptor, &input, &unsupported_disposition_product),
            Err(AdapterSdkError::Capability)
        );

        let invalid_diagnostic_product = AdapterProduct::new(
            SpecGraph::default(),
            Vec::new(),
            Vec::new(),
            vec![AdapterDiagnostic {
                code: "invalid",
                span: valid_span,
                context: String::from("unit-variant"),
            }],
        )
        .expect("invalid diagnostic product");
        assert_eq!(
            validate_product(&descriptor, &input, &invalid_diagnostic_product),
            Err(AdapterSdkError::Materialization)
        );

        let mut synthesized_disposition = Vec::new();
        ensure_disposition(
            &input,
            &[],
            &[],
            &mut synthesized_disposition,
            "coverage.syntax/v1",
        );
        assert_eq!(synthesized_disposition.len(), 1);

        let invalid_span = SourceSpan {
            path: path.clone(),
            start: 0,
            end: 2,
        };

        let source = MaterializedSource::new(
            object,
            control_item(&path, "markdown/v1"),
            invalid_span.clone(),
            "coverage.syntax/v1",
        )
        .expect("source mapping");
        assert_eq!(
            AdapterProduct::new(
                graph.clone(),
                vec![source.clone(), source.clone()],
                Vec::new(),
                Vec::new(),
            ),
            Err(AdapterSdkError::Materialization)
        );
        let source_product = AdapterProduct::new(graph, vec![source], Vec::new(), Vec::new())
            .expect("source product");
        assert_eq!(
            validate_product(&descriptor, &input, &source_product),
            Err(AdapterSdkError::Materialization)
        );

        let disposition = NonnormativeDisposition::new(invalid_span.clone(), "coverage.syntax/v1")
            .expect("disposition");
        assert_eq!(
            AdapterProduct::new(
                SpecGraph::default(),
                Vec::new(),
                vec![disposition.clone(), disposition.clone()],
                Vec::new(),
            ),
            Err(AdapterSdkError::Materialization)
        );
        let disposition_product = AdapterProduct::new(
            SpecGraph::default(),
            Vec::new(),
            vec![disposition],
            Vec::new(),
        )
        .expect("disposition product");
        assert_eq!(
            validate_product(&descriptor, &input, &disposition_product),
            Err(AdapterSdkError::Materialization)
        );

        let diagnostic = AdapterDiagnostic {
            code: "COVERAGE-CONTROL",
            span: invalid_span,
            context: String::from("unit-variant"),
        };
        assert_eq!(
            AdapterProduct::new(
                SpecGraph::default(),
                Vec::new(),
                Vec::new(),
                vec![diagnostic.clone(), diagnostic.clone()],
            ),
            Err(AdapterSdkError::Materialization)
        );
        let diagnostic_product = AdapterProduct::new(
            SpecGraph::default(),
            Vec::new(),
            Vec::new(),
            vec![diagnostic],
        )
        .expect("diagnostic product");
        assert_eq!(
            validate_product(&descriptor, &input, &diagnostic_product),
            Err(AdapterSdkError::Materialization)
        );

        let rust_configuration = built_in_configuration("rust/v1", AuthorityExpectation::Optional)
            .expect("Rust configuration");
        let invalid_rust_configuration = AdapterConfiguration::new(
            "rust/v1",
            BUILTIN_CONFIGURATION_SCHEMA_ID,
            b"invalid".to_vec(),
        )
        .expect("structurally valid Rust configuration");
        assert_eq!(
            registry.materialize(&control_input(
                &path,
                b"fn ordinary() {}\n",
                &invalid_rust_configuration,
            )),
            Err(AdapterSdkError::Configuration)
        );
        let rust = registry
            .materialize(&control_input(
                &path,
                b"fn ordinary() {}\n",
                &rust_configuration,
            ))
            .expect("ordinary Rust materialization");
        assert_eq!(rust.product().nonnormative().len(), 1);
        assert_eq!(
            registry.materialize(&control_input(
                &path,
                b"fn duplicate() {}\nfn duplicate() {}\n",
                &rust_configuration,
            )),
            Err(AdapterSdkError::Materialization)
        );

        let carrier = registry
            .materialize(&control_input(
                &path,
                include_bytes!("../fixtures/adapters/rust/positive.rs"),
                &rust_configuration,
            ))
            .expect("carrier Rust materialization");
        assert_eq!(carrier.product().graph().evidence_carriers.len(), 1);
        let rust_objects = registry
            .materialize(&control_input(
                &path,
                include_bytes!("../fixtures/adapters/rust/links.rs"),
                &rust_configuration,
            ))
            .expect("Rust object materialization");
        assert_eq!(rust_objects.product().graph().implementation_links.len(), 1);
        assert_eq!(rust_objects.product().graph().decision_surfaces.len(), 1);
        assert_eq!(rust_objects.product().graph().derivations.len(), 1);

        let declaration = registry
            .materialize(&control_input(
                &path,
                b"#[spec_provenance::governed_surface(syntax = \"v1\", id = \"SURFACE-COVERAGE\")]\nfn governed() {}\n",
                &rust_configuration,
            ))
            .expect("declaration Rust materialization");
        assert_eq!(declaration.product().diagnostics().len(), 1);

        assert_eq!(
            extract_rust(
                &path,
                b"#[spec_provenance::implementation(syntax = \"v1\", id = \"LINK-COVERAGE\")]\nfn incomplete() {}\n",
            )
            .diagnostics[0]
                .code,
            "SPG-ADAPTER-ATTRIBUTE-FIELDS"
        );
        assert_eq!(
            extract_rust(
                &path,
                include_bytes!("../fixtures/adapters/rust/file-level.rs"),
            )
            .diagnostics[0]
                .code,
            "SPG-ADAPTER-UNATTACHED-ATTRIBUTE"
        );
        assert_eq!(
            extract_rust(
                &path,
                b"#[spec_provenance::carrier::extra]\nfn unrelated() {}\n",
            )
            .nonnormative_items,
            1
        );

        for version in ["", "-1", "1-", "release", "1_bad"] {
            assert!(!exact_version(version), "{version}");
        }
        assert!(exact_version("1.0.0+reviewed"));
        assert!(!versioned_schema_id("missing-version"));
        assert!(!versioned_schema_id("Bad/v1"));
        assert!(!scope_id("parser/v1"));
        assert!(!scope_id("Bad"));
        assert!(!diagnostic_code(""));
        assert!(!diagnostic_code("_"));
        assert!(diagnostic_code("A1-"));

        let capabilities = || vec![String::from("coverage.syntax/v1")];
        let kinds =
            || vec![SourceItemKind::new("coverage/item/v1").expect("coverage source item kind")];
        for invalid in [
            AdapterDescriptor::new(
                "invalid",
                "coverage-parser",
                "1.0.0",
                "coverage.syntax/v1",
                "coverage.configuration/v1",
                capabilities(),
                kinds(),
            ),
            AdapterDescriptor::new(
                "coverage/v1",
                "parser/v1",
                "1.0.0",
                "coverage.syntax/v1",
                "coverage.configuration/v1",
                capabilities(),
                kinds(),
            ),
            AdapterDescriptor::new(
                "coverage/v1",
                "coverage-parser",
                "invalid",
                "coverage.syntax/v1",
                "coverage.configuration/v1",
                capabilities(),
                kinds(),
            ),
            AdapterDescriptor::new(
                "coverage/v1",
                "coverage-parser",
                "1.0.0",
                "invalid",
                "coverage.configuration/v1",
                capabilities(),
                kinds(),
            ),
            AdapterDescriptor::new(
                "coverage/v1",
                "coverage-parser",
                "1.0.0",
                "coverage.syntax/v1",
                "invalid",
                capabilities(),
                kinds(),
            ),
        ] {
            assert_eq!(invalid.err(), Some(AdapterSdkError::Identity));
        }
        assert_eq!(
            AdapterDescriptor::new(
                "coverage/v1",
                "coverage-parser",
                "1.0.0",
                "coverage.syntax/v1",
                "coverage.configuration/v1",
                Vec::new(),
                kinds(),
            )
            .err(),
            Some(AdapterSdkError::Capability)
        );
        assert_eq!(
            AdapterDescriptor::new(
                "coverage/v1",
                "coverage-parser",
                "1.0.0",
                "coverage.syntax/v1",
                "coverage.configuration/v1",
                vec![String::from("invalid")],
                kinds(),
            )
            .err(),
            Some(AdapterSdkError::Capability)
        );
        assert_eq!(
            AdapterDescriptor::new(
                "coverage/v1",
                "coverage-parser",
                "1.0.0",
                "coverage.syntax/v1",
                "coverage.configuration/v1",
                vec![
                    String::from("coverage.syntax/v1"),
                    String::from("coverage.syntax/v1"),
                ],
                kinds(),
            )
            .err(),
            Some(AdapterSdkError::Capability)
        );

        for invalid in [
            AdapterConfiguration::new("invalid", "coverage.configuration/v1", Vec::new()),
            AdapterConfiguration::new("coverage/v1", "invalid", Vec::new()),
            AdapterConfiguration::new(
                "coverage/v1",
                "coverage.configuration/v1",
                vec![0; CONFIGURATION_BYTES_MAXIMUM + 1],
            ),
        ] {
            assert_eq!(invalid.err(), Some(AdapterSdkError::Configuration));
        }
        assert_eq!(
            MaterializedSource::new(
                ObjectRef::new(
                    ObjectKind::Atom,
                    buoy_core::StableId::new("COVERAGE-OBJECT").expect("coverage object ID"),
                    SemanticDigest::xxh3_128([1; 16]),
                ),
                control_item(&path, "markdown/v1"),
                SourceSpan {
                    path: path.clone(),
                    start: 0,
                    end: 1,
                },
                "invalid",
            )
            .err(),
            Some(AdapterSdkError::Capability)
        );
        assert_eq!(
            NonnormativeDisposition::new(
                SourceSpan {
                    path: path.clone(),
                    start: 0,
                    end: 1,
                },
                "invalid",
            )
            .err(),
            Some(AdapterSdkError::Capability)
        );

        let missing_configuration = AdapterConfiguration::new(
            "missing/v1",
            BUILTIN_CONFIGURATION_SCHEMA_ID,
            b"optional".to_vec(),
        )
        .expect("missing adapter configuration");
        assert_eq!(
            AdapterRegistry::new()
                .materialize(&control_input(&path, b"x", &missing_configuration,)),
            Err(AdapterSdkError::UnsupportedAdapter)
        );
        let mismatched_configuration = AdapterConfiguration::new(
            "markdown/v1",
            "other.configuration/v1",
            b"optional".to_vec(),
        )
        .expect("mismatched schema configuration");
        assert_eq!(
            registry.materialize(&control_input(
                &path,
                markdown_bytes,
                &mismatched_configuration,
            )),
            Err(AdapterSdkError::Configuration)
        );

        let changing_frozen = AdapterDescriptor::new(
            "changing/v1",
            "changing-parser",
            "1.0.0",
            "changing.syntax/v1",
            "changing.configuration/v1",
            vec![String::from("changing.syntax/v1")],
            vec![SourceItemKind::new("changing/item/v1").expect("changing source item kind")],
        )
        .expect("changing frozen descriptor");
        let changing_changed = AdapterDescriptor::new(
            "changing/v1",
            "changing-parser",
            "2.0.0",
            "changing.syntax/v1",
            "changing.configuration/v1",
            vec![String::from("changing.syntax/v1")],
            vec![SourceItemKind::new("changing/item/v1").expect("changing source item kind")],
        )
        .expect("changing replacement descriptor");
        let changing_configuration =
            AdapterConfiguration::new("changing/v1", "changing.configuration/v1", Vec::new())
                .expect("changing configuration");
        let changing_adapter = ChangingDescriptorAdapter {
            frozen: changing_frozen,
            changed: changing_changed,
            first_read: std::sync::atomic::AtomicBool::new(true),
        };
        assert_eq!(
            changing_adapter.materialize(&control_input(&path, b"x", &changing_configuration,)),
            Err(AdapterSdkError::Materialization)
        );
        let mut changing_registry = AdapterRegistry::new();
        changing_registry
            .register(changing_adapter)
            .expect("register changing adapter");
        assert_eq!(
            changing_registry.materialize(&control_input(&path, b"x", &changing_configuration,)),
            Err(AdapterSdkError::Identity)
        );

        let failing_descriptor = AdapterDescriptor::new(
            "failing/v1",
            "failing-parser",
            "1.0.0",
            "failing.syntax/v1",
            "failing.configuration/v1",
            vec![String::from("failing.syntax/v1")],
            vec![SourceItemKind::new("failing/item/v1").expect("failing source item kind")],
        )
        .expect("failing descriptor");
        let failing_configuration =
            AdapterConfiguration::new("failing/v1", "failing.configuration/v1", Vec::new())
                .expect("failing configuration");
        let mut failing_registry = AdapterRegistry::new();
        failing_registry
            .register(FailingAdapter {
                descriptor: failing_descriptor,
            })
            .expect("register failing adapter");
        assert_eq!(
            failing_registry.materialize(&control_input(&path, b"x", &failing_configuration,)),
            Err(AdapterSdkError::Materialization)
        );

        let empty_descriptor = AdapterDescriptor::new(
            "empty/v1",
            "empty-parser",
            "1.0.0",
            "empty.syntax/v1",
            "empty.configuration/v1",
            vec![String::from("empty.syntax/v1")],
            vec![SourceItemKind::new("empty/item/v1").expect("empty source item kind")],
        )
        .expect("empty-product descriptor");
        let empty_configuration =
            AdapterConfiguration::new("empty/v1", "empty.configuration/v1", Vec::new())
                .expect("empty-product configuration");
        let mut empty_registry = AdapterRegistry::new();
        empty_registry
            .register(EmptyProductAdapter {
                descriptor: empty_descriptor,
            })
            .expect("register empty-product adapter");
        assert_eq!(
            empty_registry.materialize(&control_input(&path, b"x", &empty_configuration)),
            Err(AdapterSdkError::Materialization)
        );

        let validation_graph = markdown.product().graph().clone();
        let validation_atom = &validation_graph.atoms[0];
        let validation_object = ObjectRef::new(
            ObjectKind::Atom,
            validation_atom.meta().id().clone(),
            validation_atom.meta().digest(),
        );
        let alternate_path = RepositoryPath::new("spec/alternate.md").expect("alternate path");
        let input_span = SourceSpan {
            path: path.clone(),
            start: 0,
            end: 1,
        };
        let source_with_other_item = MaterializedSource::new(
            validation_object.clone(),
            control_item(&alternate_path, "markdown/v1"),
            input_span.clone(),
            "coverage.syntax/v1",
        )
        .expect("other-item source");
        let product_with_other_item = AdapterProduct::new(
            validation_graph.clone(),
            vec![source_with_other_item],
            Vec::new(),
            Vec::new(),
        )
        .expect("other-item product");
        assert_eq!(
            validate_product(&descriptor, &input, &product_with_other_item),
            Err(AdapterSdkError::Materialization)
        );
        let incompatible_source = MaterializedSource::new(
            validation_object,
            control_item(&path, "rust/v1"),
            input_span,
            "coverage.syntax/v1",
        )
        .expect("incompatible source");
        let incompatible_product = AdapterProduct::new(
            validation_graph,
            vec![incompatible_source],
            Vec::new(),
            Vec::new(),
        )
        .expect("incompatible product");
        assert_eq!(
            validate_product(&descriptor, &input, &incompatible_product),
            Err(AdapterSdkError::UndeclaredSourceItemKind)
        );

        for span in [
            SourceSpan {
                path: alternate_path,
                start: 0,
                end: 1,
            },
            SourceSpan {
                path: path.clone(),
                start: 1,
                end: 0,
            },
        ] {
            let disposition = NonnormativeDisposition::new(span, "coverage.syntax/v1")
                .expect("invalid-span disposition remains structurally valid");
            let product = AdapterProduct::new(
                SpecGraph::default(),
                Vec::new(),
                vec![disposition],
                Vec::new(),
            )
            .expect("invalid-span product");
            assert_eq!(
                validate_product(&descriptor, &input, &product),
                Err(AdapterSdkError::Materialization)
            );
        }
        let empty_code_product = AdapterProduct::new(
            SpecGraph::default(),
            Vec::new(),
            Vec::new(),
            vec![AdapterDiagnostic {
                code: "",
                span: SourceSpan {
                    path: path.clone(),
                    start: 0,
                    end: 1,
                },
                context: String::from("empty-code"),
            }],
        )
        .expect("empty-code product");
        assert_eq!(
            validate_product(&descriptor, &input, &empty_code_product),
            Err(AdapterSdkError::Materialization)
        );
        assert_eq!(
            AdapterProduct::new(
                SpecGraph::default(),
                Vec::new(),
                Vec::new(),
                vec![AdapterDiagnostic {
                    code: "NONCANONICAL-CONTEXT",
                    span: SourceSpan {
                        path: path.clone(),
                        start: 0,
                        end: 1,
                    },
                    context: String::from("e\u{301}"),
                }],
            )
            .err(),
            Some(AdapterSdkError::Encoding)
        );

        let enclosing_digest = SemanticDigest::xxh3_128([2; 16]);
        let enclosed_item = ParsedItemIdentity::new(
            SchemaId::new("markdown/v1").expect("valid schema"),
            path,
            SourceItemKind::new("markdown/fence/v1").expect("source item kind"),
            b"enclosed".to_vec(),
            Some(enclosing_digest),
        )
        .expect("enclosed item");
        assert_eq!(enclosed_item.enclosing_item(), Some(enclosing_digest));
        let _ = parsed_item_value(&enclosed_item);
    }

    #[test]
    fn artifact_product_controls_cover_every_opaque_invariant() {
        let path = RepositoryPath::new("artifact.bin").expect("artifact path");
        let bytes = b"artifact-bytes";
        let configuration = built_in_configuration("artifact/v1", AuthorityExpectation::Forbidden)
            .expect("artifact configuration");
        let input = control_input(&path, bytes, &configuration);
        let registry = built_in_registry().expect("built-in registry");
        let invalid_configuration = AdapterConfiguration::new(
            "artifact/v1",
            BUILTIN_CONFIGURATION_SCHEMA_ID,
            b"invalid".to_vec(),
        )
        .expect("structural artifact configuration");
        assert_eq!(
            registry.materialize(&control_input(&path, bytes, &invalid_configuration)),
            Err(AdapterSdkError::Configuration)
        );
        let valid = registry
            .materialize(&input)
            .expect("valid artifact materialization")
            .product()
            .clone();
        assert_eq!(validate_artifact_product(&input, &valid), Ok(()));

        let mut invalid = valid.clone();
        invalid.source.nodes.clear();
        assert!(validate_artifact_product(&input, &invalid).is_err());
        let mut invalid = valid.clone();
        invalid.source.roots.clear();
        assert!(validate_artifact_product(&input, &invalid).is_err());
        let mut invalid = valid.clone();
        invalid.source.nodes[0].span.start = 1;
        assert!(validate_artifact_product(&input, &invalid).is_err());
        let mut invalid = valid.clone();
        invalid.source.nodes[0].span.end -= 1;
        assert!(validate_artifact_product(&input, &invalid).is_err());
        let mut invalid = valid.clone();
        invalid.source.nodes[0] =
            artifact_retained(&path, bytes, "artifact.other/v1", ARTIFACT_FILE_KIND);
        assert!(validate_artifact_product(&input, &invalid).is_err());
        let mut invalid = valid.clone();
        invalid.source.nodes[0] =
            artifact_retained(&path, bytes, ARTIFACT_CAPABILITY, "artifact/other/v1");
        assert!(validate_artifact_product(&input, &invalid).is_err());
        let mut invalid = valid.clone();
        invalid.source.nodes[0] =
            artifact_retained(&path, b"different", ARTIFACT_CAPABILITY, ARTIFACT_FILE_KIND);
        invalid.source.nodes[0].span.end = bytes.len() as u64;
        assert!(validate_artifact_product(&input, &invalid).is_err());
        let mut invalid = valid.clone();
        invalid.source.roots[0] = DeclaredSourceRoot::new(
            SourceNodeRef::new(SemanticDigest::xxh3_128([9; 16])),
            SourceRootClass::Navigational,
        );
        assert!(validate_artifact_product(&input, &invalid).is_err());
        let mut invalid = valid.clone();
        invalid.source.roots[0] = DeclaredSourceRoot::new(
            invalid.source.nodes[0].node().reference(),
            SourceRootClass::StandingSpecification,
        );
        assert!(validate_artifact_product(&input, &invalid).is_err());

        let markdown_path = RepositoryPath::new("spec/control.md").expect("Markdown path");
        let markdown_configuration =
            built_in_configuration("markdown/v1", AuthorityExpectation::Optional)
                .expect("Markdown configuration");
        let markdown_input = control_input(
            &markdown_path,
            include_bytes!("../fixtures/adapters/markdown/positive.md"),
            &markdown_configuration,
        );
        let markdown = registry
            .materialize(&markdown_input)
            .expect("Markdown materialization");
        let mut invalid = valid.clone();
        invalid.graph = markdown.product().graph().clone();
        assert!(validate_artifact_product(&input, &invalid).is_err());
        let mut invalid = valid.clone();
        invalid.sources = markdown.product().sources().to_vec();
        assert!(validate_artifact_product(&input, &invalid).is_err());

        let openspec_path =
            RepositoryPath::new("openspec/specs/demo/spec.md").expect("OpenSpec path");
        let openspec_bytes = include_bytes!("../fixtures/openspec/v1/positive/standing-spec.md");
        let openspec_configuration = AdapterConfiguration::new(
            "openspec/v1",
            BUILTIN_CONFIGURATION_SCHEMA_ID,
            include_bytes!("../fixtures/openspec/v1/positive/configuration.toml").to_vec(),
        )
        .expect("OpenSpec configuration");
        let openspec_input = control_input(&openspec_path, openspec_bytes, &openspec_configuration);
        let openspec = registry
            .materialize(&openspec_input)
            .expect("OpenSpec materialization");
        let mut invalid = valid.clone();
        invalid.external_sources = openspec.product().external_sources().to_vec();
        assert!(validate_artifact_product(&input, &invalid).is_err());

        let trace_path = RepositoryPath::new("spec/external.md").expect("trace path");
        let trace_bytes = include_bytes!("../fixtures/openspec/v1/positive/external-trace.md");
        let trace_configuration =
            built_in_configuration("markdown/v1", AuthorityExpectation::Optional)
                .expect("trace configuration");
        let trace_input = control_input(&trace_path, trace_bytes, &trace_configuration);
        let trace = registry
            .materialize(&trace_input)
            .expect("trace materialization");
        let mut invalid = valid.clone();
        invalid.trace_links = trace.product().trace_links().to_vec();
        assert!(validate_artifact_product(&input, &invalid).is_err());

        let node = valid.source.nodes[0].node().reference();
        let edge = SourceEdge::new(node, buoy_core::SourceEdgeKind::Authority, None, node)
            .expect("source edge");
        let mut invalid = valid.clone();
        invalid.source.edges.push(edge);
        assert!(validate_artifact_product(&input, &invalid).is_err());
        let authority = SourceAuthority::new(
            node,
            buoy_core::AtomRevisionRef::new(
                buoy_core::StableId::new("COVERAGE-ATOM").expect("atom ID"),
                node.digest(),
            ),
        );
        let mut invalid = valid.clone();
        invalid.source.authorities.push(authority);
        assert!(validate_artifact_product(&input, &invalid).is_err());
        let record_digest =
            semantic_digest_with(input.active_algorithm(), "coverage-record", Vec::new())
                .expect("record digest");
        let mut invalid = valid.clone();
        invalid.source.attachments.push(BindingAttachment::new(
            node,
            buoy_core::ImplementationBindingRef::new(record_digest),
        ));
        assert!(validate_artifact_product(&input, &invalid).is_err());
        let relation = buoy_core::StableId::new("COVERAGE-ARTIFACT").expect("relation");
        let artifact_attachment = ArtifactBindingAttachment::new_with(
            input.active_algorithm(),
            relation.clone(),
            path.clone(),
            node,
            buoy_core::ImplementationBindingRef::new(record_digest),
        )
        .expect("artifact attachment");
        let mut invalid = valid.clone();
        invalid
            .source
            .artifact_attachments
            .push(artifact_attachment);
        assert!(validate_artifact_product(&input, &invalid).is_err());

        let requirement = SourceNodeRef::new(
            semantic_digest_with(input.active_algorithm(), "coverage-requirement", Vec::new())
                .expect("requirement digest"),
        );
        let record = buoy_core::ImplementationBinding::new_with(
            input.active_algorithm(),
            relation,
            buoy_core::NavigationKind::Implements,
            node,
            requirement,
            buoy_core::AtomRevisionRef::new(
                buoy_core::StableId::new("COVERAGE-REQUIREMENT").expect("requirement ID"),
                requirement.digest(),
            ),
        )
        .expect("binding record");
        let mut invalid = valid.clone();
        invalid
            .source
            .bindings
            .push(StoredBinding::new(record.record(), record));
        assert!(validate_artifact_product(&input, &invalid).is_err());
        let mut invalid = valid.clone();
        invalid.nonnormative.push(
            NonnormativeDisposition::new(
                SourceSpan {
                    path: path.clone(),
                    start: 0,
                    end: bytes.len() as u64,
                },
                ARTIFACT_CAPABILITY,
            )
            .expect("artifact disposition"),
        );
        assert!(validate_artifact_product(&input, &invalid).is_err());
        let mut invalid = valid;
        invalid.diagnostics.push(AdapterDiagnostic {
            code: "COVERAGE-ARTIFACT",
            span: SourceSpan {
                path: path.clone(),
                start: 0,
                end: bytes.len() as u64,
            },
            context: String::from("artifact"),
        });
        assert!(validate_artifact_product(&input, &invalid).is_err());

        assert_eq!(
            parse_binding_configuration(b"forbidden\nxxh3-128\nproject\nextra"),
            Err(AdapterSdkError::Configuration)
        );
        let _ = extract_artifact(input.active_algorithm(), input.path(), input.bytes());
    }

    fn openspec_item(path: &RepositoryPath, kind: &str) -> ParsedItemIdentity {
        ParsedItemIdentity::new(
            SchemaId::new("openspec/standing-spec/v1").expect("OpenSpec schema"),
            path.clone(),
            SourceItemKind::new(kind).expect("OpenSpec item kind"),
            b"coverage-item".to_vec(),
            None,
        )
        .expect("OpenSpec item")
    }

    #[test]
    fn external_and_trace_products_cover_complete_sdk_validation() {
        let kinds = || vec![SourceItemKind::new("coverage/item/v1").expect("coverage item kind")];
        assert_eq!(
            AdapterDescriptor::new(
                "coverage/v1",
                "coverage-parser",
                "1.0.0",
                "coverage.syntax/v1",
                "coverage.configuration/v1",
                vec![String::from("coverage.syntax/v1")],
                Vec::new(),
            ),
            Err(AdapterSdkError::UndeclaredSourceItemKind)
        );
        let kind = SourceItemKind::new("coverage/item/v1").expect("coverage item kind");
        assert_eq!(
            AdapterDescriptor::new(
                "coverage/v1",
                "coverage-parser",
                "1.0.0",
                "coverage.syntax/v1",
                "coverage.configuration/v1",
                vec![String::from("coverage.syntax/v1")],
                vec![kind.clone(), kind],
            ),
            Err(AdapterSdkError::UndeclaredSourceItemKind)
        );
        assert_eq!(
            validate_parsed_item_protocol("invalid", "coverage/v1", &kinds()[0],),
            Err(AdapterSdkError::Identity)
        );
        assert_eq!(
            validate_parsed_item_protocol("coverage/v1", "invalid", &kinds()[0],),
            Err(AdapterSdkError::Identity)
        );

        let path = RepositoryPath::new("openspec/specs/demo/spec.md").expect("OpenSpec path");
        let bytes = include_bytes!("../fixtures/openspec/v1/positive/standing-spec.md");
        let configuration = AdapterConfiguration::new(
            "openspec/v1",
            BUILTIN_CONFIGURATION_SCHEMA_ID,
            include_bytes!("../fixtures/openspec/v1/positive/configuration.toml").to_vec(),
        )
        .expect("OpenSpec configuration");
        let input = control_input(&path, bytes, &configuration);
        let registry = built_in_registry().expect("built-in registry");
        let invalid_configuration = AdapterConfiguration::new(
            "openspec/v1",
            BUILTIN_CONFIGURATION_SCHEMA_ID,
            b"invalid".to_vec(),
        )
        .expect("structural OpenSpec configuration");
        assert_eq!(
            registry.materialize(&control_input(&path, bytes, &invalid_configuration,)),
            Err(AdapterSdkError::Configuration)
        );
        let output = registry.materialize(&input).expect("OpenSpec output");
        let source = output.product().source();
        let retained_nodes = source.nodes().to_vec();
        let subject = retained_nodes[0].node().reference();
        let requirement = SourceNodeRef::new(
            semantic_digest_with(input.active_algorithm(), "coverage-requirement", Vec::new())
                .expect("requirement digest"),
        );
        let relation =
            buoy_core::StableId::new("COVERAGE-DUPLICATE-BINDING").expect("coverage relation");
        let binding = buoy_core::ImplementationBinding::new_with(
            input.active_algorithm(),
            relation,
            buoy_core::NavigationKind::Implements,
            subject,
            requirement,
            buoy_core::AtomRevisionRef::new(
                buoy_core::StableId::new("COVERAGE-DUPLICATE-REQUIREMENT").expect("requirement ID"),
                requirement.digest(),
            ),
        )
        .expect("coverage binding");
        let attachment = BindingAttachment::new(subject, binding.record());
        let stored = StoredBinding::new(binding.record(), binding);
        assert_eq!(
            SourceProduct::new(
                vec![retained_nodes[0].clone(), retained_nodes[0].clone()],
                Vec::new(),
                Vec::new(),
                Vec::new(),
                Vec::new(),
                Vec::new(),
            ),
            Err(AdapterSdkError::Materialization)
        );
        for duplicate_product in [
            SourceProduct::new(
                retained_nodes.clone(),
                vec![source.edges()[0], source.edges()[0]],
                Vec::new(),
                Vec::new(),
                Vec::new(),
                Vec::new(),
            ),
            SourceProduct::new(
                retained_nodes.clone(),
                Vec::new(),
                vec![source.roots()[0], source.roots()[0]],
                Vec::new(),
                Vec::new(),
                Vec::new(),
            ),
            SourceProduct::new(
                retained_nodes.clone(),
                Vec::new(),
                Vec::new(),
                vec![
                    source.authorities()[0].clone(),
                    source.authorities()[0].clone(),
                ],
                Vec::new(),
                Vec::new(),
            ),
            SourceProduct::new(
                retained_nodes.clone(),
                Vec::new(),
                Vec::new(),
                Vec::new(),
                vec![attachment, attachment],
                Vec::new(),
            ),
            SourceProduct::new(
                retained_nodes,
                Vec::new(),
                Vec::new(),
                Vec::new(),
                Vec::new(),
                vec![stored.clone(), stored],
            ),
        ] {
            assert_eq!(duplicate_product, Err(AdapterSdkError::Materialization));
        }
        let descriptor = &registry
            .adapters
            .get("openspec/v1")
            .expect("OpenSpec adapter")
            .frozen_descriptor;
        assert_eq!(
            SourceProduct::new(
                Vec::new(),
                Vec::new(),
                Vec::new(),
                vec![output.product().source().authorities()[0].clone()],
                Vec::new(),
                Vec::new(),
            ),
            Err(AdapterSdkError::Materialization)
        );
        let alternate = RepositoryPath::new("openspec/specs/demo/alternate.md")
            .expect("alternate OpenSpec path");
        let retained_product =
            |item: ParsedItemIdentity, capability: &str, root_class: Option<SourceRootClass>| {
                let source_node = SourceNode::new(item, Vec::new()).expect("source-node control");
                let node = source_node.reference();
                let retained = MaterializedSourceNode::new(
                    source_node,
                    SourceSpan {
                        path: path.clone(),
                        start: 0,
                        end: 1,
                    },
                    capability,
                )
                .expect("retained source-node control");
                assert_eq!(
                    MaterializedSourceNode::new(
                        retained.node().clone(),
                        retained.span().clone(),
                        "invalid",
                    ),
                    Err(AdapterSdkError::Capability)
                );
                let roots = root_class
                    .into_iter()
                    .map(|class| DeclaredSourceRoot::new(node, class))
                    .collect();
                let source = SourceProduct::new(
                    vec![retained],
                    Vec::new(),
                    roots,
                    Vec::new(),
                    Vec::new(),
                    Vec::new(),
                )
                .expect("source product control");
                AdapterProduct::new_with_source_product(
                    SpecGraph::default(),
                    Vec::new(),
                    source,
                    Vec::new(),
                    Vec::new(),
                    Vec::new(),
                )
                .expect("adapter product control")
            };
        for item in [
            openspec_item(&alternate, "openspec/scenario/v1"),
            ParsedItemIdentity::new(
                SchemaId::new("invalid").expect("structural schema"),
                path.clone(),
                SourceItemKind::new("openspec/scenario/v1").expect("scenario kind"),
                b"coverage-item".to_vec(),
                None,
            )
            .expect("protocol-invalid source item"),
        ] {
            let invalid = retained_product(item, OPENSPEC_SEMANTIC_CAPABILITY, None);
            assert!(validate_product(descriptor, &input, &invalid).is_err());
        }
        let coverage_descriptor = AdapterDescriptor::new(
            "coverage/v1",
            "coverage-parser",
            "1.0.0",
            "coverage/v1",
            "coverage.configuration/v1",
            vec![String::from("coverage.syntax/v1")],
            vec![SourceItemKind::new("coverage/item/v1").expect("coverage item kind")],
        )
        .expect("coverage descriptor");
        let coverage_item = || {
            ParsedItemIdentity::new(
                SchemaId::new("coverage/v1").expect("coverage schema"),
                path.clone(),
                SourceItemKind::new("coverage/item/v1").expect("coverage item kind"),
                b"coverage-item".to_vec(),
                None,
            )
            .expect("coverage item")
        };
        assert_eq!(
            validate_product(
                &coverage_descriptor,
                &input,
                &retained_product(
                    coverage_item(),
                    "coverage.syntax/v1",
                    Some(SourceRootClass::StandingSpecification),
                ),
            ),
            Err(AdapterSdkError::Materialization)
        );
        assert_eq!(
            validate_product(
                descriptor,
                &input,
                &retained_product(
                    openspec_item(&path, "openspec/unknown/v1"),
                    OPENSPEC_SEMANTIC_CAPABILITY,
                    None,
                ),
            ),
            Err(AdapterSdkError::UndeclaredSourceItemKind)
        );
        assert_eq!(
            validate_product(
                descriptor,
                &input,
                &retained_product(
                    openspec_item(&path, "openspec/scenario/v1"),
                    "openspec.unsupported/v1",
                    None,
                ),
            ),
            Err(AdapterSdkError::Capability)
        );
        let outside_input =
            SourceNode::new(openspec_item(&path, "openspec/scenario/v1"), Vec::new())
                .expect("outside-input node");
        let outside_input = SourceProduct::new(
            vec![
                MaterializedSourceNode::new(
                    outside_input,
                    SourceSpan {
                        path: path.clone(),
                        start: 0,
                        end: u64::try_from(bytes.len() + 1).expect("bounded fixture length"),
                    },
                    OPENSPEC_SEMANTIC_CAPABILITY,
                )
                .expect("outside-input retained node"),
            ],
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
        )
        .expect("outside-input source product");
        let outside_input = AdapterProduct::new_with_source_product(
            SpecGraph::default(),
            Vec::new(),
            outside_input,
            Vec::new(),
            Vec::new(),
            Vec::new(),
        )
        .expect("outside-input adapter product");
        assert_eq!(
            validate_product(descriptor, &input, &outside_input),
            Err(AdapterSdkError::Materialization)
        );
        let external = output.product().external_sources()[0].clone();
        assert_eq!(
            external.object(),
            output.product().external_sources()[0].object()
        );
        assert_eq!(external.item().path(), &path);
        assert_eq!(external.span().path, path);
        assert_eq!(external.syntax_capability(), OPENSPEC_SEMANTIC_CAPABILITY);
        assert!(
            ExternalMaterializedSource::new(
                external.object().clone(),
                external.item().clone(),
                external.span().clone(),
                OPENSPEC_SEMANTIC_CAPABILITY,
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
        assert!(
            AdapterProduct::new_with_external(
                SpecGraph::default(),
                Vec::new(),
                Vec::new(),
                Vec::new(),
                Vec::new(),
            )
            .is_ok()
        );
        let external_only = AdapterProduct::new_with_external(
            SpecGraph::default(),
            Vec::new(),
            vec![external.clone()],
            Vec::new(),
            Vec::new(),
        )
        .expect("external-only product");
        assert_eq!(validate_product(descriptor, &input, &external_only), Ok(()));
        let source_and_external = AdapterProduct::new_with_source_and_external(
            output.product().graph.clone(),
            output.product().sources.clone(),
            vec![external.clone()],
            output.product().source.clone(),
            output.product().nonnormative.clone(),
            output.product().diagnostics.clone(),
            output.product().advisory_declaration_kinds.clone(),
        )
        .expect("source-and-external product");
        assert_eq!(
            source_and_external.external_sources(),
            core::slice::from_ref(&external)
        );
        assert_eq!(
            AdapterProduct::new_with_external(
                SpecGraph::default(),
                Vec::new(),
                vec![external.clone(), external.clone()],
                Vec::new(),
                Vec::new(),
            ),
            Err(AdapterSdkError::Materialization)
        );

        let mut invalid = output.product().clone();
        invalid.sources[0].item = ParsedItemIdentity::new(
            SchemaId::new("invalid").expect("structural schema"),
            path.clone(),
            SourceItemKind::new("openspec/requirement/v1").expect("requirement kind"),
            b"coverage-item".to_vec(),
            None,
        )
        .expect("protocol-invalid source item");
        assert_eq!(
            validate_product(descriptor, &input, &invalid),
            Err(AdapterSdkError::Identity)
        );
        let mut invalid = output.product().clone();
        invalid.external_sources[0].span.end = bytes.len() as u64 + 1;
        assert_eq!(
            validate_product(descriptor, &input, &invalid),
            Err(AdapterSdkError::Materialization)
        );
        let mut invalid = output.product().clone();
        invalid.external_sources[0].item = openspec_item(&alternate, "openspec/scenario/v1");
        assert_eq!(
            validate_product(descriptor, &input, &invalid),
            Err(AdapterSdkError::Materialization)
        );
        let mut invalid = output.product().clone();
        invalid.external_sources[0].item = ParsedItemIdentity::new(
            SchemaId::new("invalid").expect("structural schema"),
            path.clone(),
            SourceItemKind::new("openspec/scenario/v1").expect("scenario kind"),
            b"coverage-item".to_vec(),
            None,
        )
        .expect("protocol-invalid item");
        assert_eq!(
            validate_product(descriptor, &input, &invalid),
            Err(AdapterSdkError::Identity)
        );
        let mut invalid = output.product().clone();
        invalid.external_sources[0].item = openspec_item(&path, "coverage/item/v1");
        assert_eq!(
            validate_product(descriptor, &input, &invalid),
            Err(AdapterSdkError::UndeclaredSourceItemKind)
        );
        let mut invalid = output.product().clone();
        invalid.external_sources[0].syntax_capability = String::from("coverage.syntax/v1");
        assert_eq!(
            validate_product(descriptor, &input, &invalid),
            Err(AdapterSdkError::Capability)
        );

        let markdown_path = RepositoryPath::new("spec/external.md").expect("Markdown path");
        let markdown_bytes = include_bytes!("../fixtures/openspec/v1/positive/external-trace.md");
        let markdown_configuration =
            built_in_configuration("markdown/v1", AuthorityExpectation::Optional)
                .expect("Markdown configuration");
        let markdown_input = control_input(&markdown_path, markdown_bytes, &markdown_configuration);
        let trace_output = registry
            .materialize(&markdown_input)
            .expect("external trace output");
        let markdown_descriptor = &registry
            .adapters
            .get("markdown/v1")
            .expect("Markdown adapter")
            .frozen_descriptor;
        let trace = trace_output.product().trace_links()[0].clone();
        assert_eq!(trace.item().path(), &markdown_path);
        assert_eq!(trace.span().path, markdown_path);
        assert!(
            MaterializedTraceLink::new(
                trace.link().clone(),
                trace.item().clone(),
                trace.span().clone(),
                MARKDOWN_SEMANTIC_CAPABILITY,
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
        assert_eq!(
            AdapterProduct::new_with_trace_links(
                trace_output.product().graph().clone(),
                Vec::new(),
                vec![trace.clone(), trace],
                Vec::new(),
                Vec::new(),
                Vec::new(),
            ),
            Err(AdapterSdkError::Materialization)
        );

        let mut invalid = trace_output.product().clone();
        invalid.trace_links[0].span.end = markdown_bytes.len() as u64 + 1;
        assert_eq!(
            validate_product(markdown_descriptor, &markdown_input, &invalid),
            Err(AdapterSdkError::Materialization)
        );
        let alternate_markdown =
            RepositoryPath::new("spec/alternate.md").expect("alternate Markdown path");
        let mut invalid = trace_output.product().clone();
        invalid.trace_links[0].item = control_item(&alternate_markdown, "markdown/v1");
        assert_eq!(
            validate_product(markdown_descriptor, &markdown_input, &invalid),
            Err(AdapterSdkError::Materialization)
        );
        let mut invalid = trace_output.product().clone();
        invalid.trace_links[0].item = ParsedItemIdentity::new(
            SchemaId::new("invalid").expect("structural schema"),
            markdown_path.clone(),
            SourceItemKind::new("markdown/external-trace/v1").expect("trace item kind"),
            b"coverage-item".to_vec(),
            None,
        )
        .expect("protocol-invalid trace item");
        assert_eq!(
            validate_product(markdown_descriptor, &markdown_input, &invalid),
            Err(AdapterSdkError::Identity)
        );
        let mut invalid = trace_output.product().clone();
        invalid.trace_links[0].item = control_item(&markdown_path, "rust/v1");
        assert_eq!(
            validate_product(markdown_descriptor, &markdown_input, &invalid),
            Err(AdapterSdkError::UndeclaredSourceItemKind)
        );
        let mut invalid = trace_output.product().clone();
        invalid.trace_links[0].syntax_capability = String::from("coverage.syntax/v1");
        assert_eq!(
            validate_product(markdown_descriptor, &markdown_input, &invalid),
            Err(AdapterSdkError::Capability)
        );
    }
}
