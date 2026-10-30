//! Total schema-owned syntax adapters.

mod artifact;
mod binding;
mod external_trace;
mod markdown;
mod openspec;
mod rust;
mod schema;

pub use artifact::{
    ARTIFACT_ADAPTER_ID, ARTIFACT_CAPABILITY, ARTIFACT_FILE_KIND, ARTIFACT_MEDIA_TYPE,
    ARTIFACT_REPRESENTATION, ArtifactOutput, extract_artifact,
};
pub use binding::{
    BINDING_ATTACHMENT_PREFIX, BINDING_ATTACHMENT_SCHEMA, BINDING_OBJECT_PREFIX,
    BINDING_RECORD_SCHEMA, BindingOutput, binding_attachment_key_locator, binding_attachment_path,
    binding_object_locator, binding_object_locator_in_mount, binding_object_path,
    canonical_binding_attachment, canonical_binding_record, extract_binding_in_mount,
    parse_binding_record,
};
pub use external_trace::{EXTERNAL_TRACE_FENCE, parse_external_trace_document};
pub use markdown::{AuthorityExpectation, MarkdownOutput, extract_markdown};
#[cfg(feature = "coverage-controls")]
#[doc(hidden)]
pub use openspec::openspec_partition_control;
pub use openspec::{
    LocatedOpenSpecNode, LocatedOpenSpecObject, OPENSPEC_EXTERNAL_TRACE_KIND,
    OpenSpecConfiguration, OpenSpecObject, OpenSpecOutput, extract_openspec_configured,
    extract_openspec_configured_with,
};
pub use rust::{
    LocatedRustBinding, LocatedRustNode, RustOutput, RustSemantic, extract_rust, extract_rust_with,
};

use crate::traceability::ExternalTraceLink;
use buoy_core::{
    Assumption, Atom, CoveragePolicy, DebtRecord, DecisionSurface, Derivation, FiniteModel,
    FormalProperty, ObjectMeta, ParsedItemIdentity, ProofRecord, Reaffirmation, RepositoryPath,
    Waiver,
};

/// Closed result class for one selected syntax class.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum SyntaxDispositionKind {
    /// A complete validated semantic value is emitted.
    Semantic,
    /// The syntax is explicitly outside normative meaning.
    Nonnormative,
    /// A stable malformed or unsupported diagnostic is emitted.
    Diagnostic,
}

/// Closed syntax classes understood by the built-in v1 adapters.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum AdapterSyntaxClass {
    /// A canonical implementation-binding record at its digest-derived path.
    BindingObjectRecord,
    /// An object-store file with a wrong path, schema, or noncanonical content.
    BindingMalformedObjectRecord,
    /// A supported, complete Markdown semantic fence.
    MarkdownSemanticFence,
    /// A supported Markdown fence with malformed or incomplete object fields.
    MarkdownMalformedSemanticFence,
    /// An authority-like Markdown fence outside the frozen version/kind matrix.
    MarkdownUnsupportedAuthorityFence,
    /// Ordinary `CommonMark` syntax, including ordinary code fences.
    MarkdownCommonMark,
    /// Markdown bytes that are invalid UTF-8 or contain a lone carriage return.
    MarkdownInvalidText,
    /// A supported, complete Rust v1 semantic attribute on an eligible parsed item.
    RustSemanticAttribute,
    /// A supported Rust v1 attribute with malformed or incomplete metadata.
    RustMalformedSemanticAttribute,
    /// A provenance-like Rust attribute outside the supported kind/version/item matrix.
    RustUnsupportedAttribute,
    /// A provenance attribute at file level.
    RustUnattachedAttribute,
    /// A supported semantic attribute on disabled code.
    RustDisabledItem,
    /// An ordinary parsed Rust item with no semantic attachment.
    RustOrdinaryItem,
    /// A macro invocation that is not an explicitly supported attachment.
    RustMacroInvocation,
    /// Rust bytes or syntax that the pinned parser cannot decode.
    RustInvalidText,
}

/// One immutable row in the closed built-in adapter support matrix.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AdapterSupportRow {
    /// Selected syntax class.
    pub syntax_class: AdapterSyntaxClass,
    /// Required total disposition.
    pub disposition: SyntaxDispositionKind,
}

/// Complete closed support matrix for built-in v1 Markdown and Rust syntax.
pub const ADAPTER_SUPPORT_MATRIX: &[AdapterSupportRow] = &[
    AdapterSupportRow {
        syntax_class: AdapterSyntaxClass::BindingObjectRecord,
        disposition: SyntaxDispositionKind::Nonnormative,
    },
    AdapterSupportRow {
        syntax_class: AdapterSyntaxClass::BindingMalformedObjectRecord,
        disposition: SyntaxDispositionKind::Diagnostic,
    },
    AdapterSupportRow {
        syntax_class: AdapterSyntaxClass::MarkdownSemanticFence,
        disposition: SyntaxDispositionKind::Semantic,
    },
    AdapterSupportRow {
        syntax_class: AdapterSyntaxClass::MarkdownMalformedSemanticFence,
        disposition: SyntaxDispositionKind::Diagnostic,
    },
    AdapterSupportRow {
        syntax_class: AdapterSyntaxClass::MarkdownUnsupportedAuthorityFence,
        disposition: SyntaxDispositionKind::Diagnostic,
    },
    AdapterSupportRow {
        syntax_class: AdapterSyntaxClass::MarkdownCommonMark,
        disposition: SyntaxDispositionKind::Nonnormative,
    },
    AdapterSupportRow {
        syntax_class: AdapterSyntaxClass::MarkdownInvalidText,
        disposition: SyntaxDispositionKind::Diagnostic,
    },
    AdapterSupportRow {
        syntax_class: AdapterSyntaxClass::RustSemanticAttribute,
        disposition: SyntaxDispositionKind::Semantic,
    },
    AdapterSupportRow {
        syntax_class: AdapterSyntaxClass::RustMalformedSemanticAttribute,
        disposition: SyntaxDispositionKind::Diagnostic,
    },
    AdapterSupportRow {
        syntax_class: AdapterSyntaxClass::RustUnsupportedAttribute,
        disposition: SyntaxDispositionKind::Diagnostic,
    },
    AdapterSupportRow {
        syntax_class: AdapterSyntaxClass::RustUnattachedAttribute,
        disposition: SyntaxDispositionKind::Diagnostic,
    },
    AdapterSupportRow {
        syntax_class: AdapterSyntaxClass::RustDisabledItem,
        disposition: SyntaxDispositionKind::Diagnostic,
    },
    AdapterSupportRow {
        syntax_class: AdapterSyntaxClass::RustOrdinaryItem,
        disposition: SyntaxDispositionKind::Nonnormative,
    },
    AdapterSupportRow {
        syntax_class: AdapterSyntaxClass::RustMacroInvocation,
        disposition: SyntaxDispositionKind::Nonnormative,
    },
    AdapterSupportRow {
        syntax_class: AdapterSyntaxClass::RustInvalidText,
        disposition: SyntaxDispositionKind::Diagnostic,
    },
];

/// Original-byte source span under one canonical repository path.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct SourceSpan {
    /// Canonical repository-relative path.
    pub path: RepositoryPath,
    /// Inclusive byte start.
    pub start: u64,
    /// Exclusive byte end.
    pub end: u64,
}

/// One extracted semantic value, exact parsed-item identity, and attachment span.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Located<T> {
    /// Semantic value.
    pub value: T,
    /// Complete parser-canonical item identity.
    pub item: ParsedItemIdentity,
    /// Parser-backed original-byte extent.
    pub span: SourceSpan,
}

/// One external trace link with its parsed identity and byte span.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LocatedExternalTraceLink {
    /// Complete repository-owned trace link.
    pub value: ExternalTraceLink,
    /// Complete parser-canonical item identity.
    pub item: ParsedItemIdentity,
    /// Exact original-byte source span.
    pub span: SourceSpan,
}

/// Stable adapter diagnostic without raw parser or host text.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AdapterDiagnostic {
    /// Stable machine code.
    pub code: &'static str,
    /// Canonical source span.
    pub span: SourceSpan,
    /// Schema-owned deterministic context.
    pub context: String,
}

/// Complete Markdown semantic extraction values.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MarkdownSemantic {
    /// One authoritative atom.
    Atom(Atom),
    /// One complete finite decision surface.
    DecisionSurface(DecisionSurface),
    /// One complete coverage policy.
    CoveragePolicy(CoveragePolicy),
    /// One complete debt record.
    DebtRecord(DebtRecord),
    /// One complete waiver.
    Waiver(Waiver),
    /// One exact reaffirmation.
    Reaffirmation(Reaffirmation),
    /// One complete derivation.
    Derivation(Derivation),
    /// One reviewed assumption.
    Assumption(Assumption),
    /// One finite model.
    FiniteModel(FiniteModel),
    /// One formal property.
    FormalProperty(FormalProperty),
    /// One complete proof record.
    ProofRecord(ProofRecord),
}

impl MarkdownSemantic {
    /// Complete object metadata for source mapping and graph insertion.
    #[must_use]
    pub const fn meta(&self) -> &ObjectMeta {
        match self {
            Self::Atom(value) => value.meta(),
            Self::DecisionSurface(value) => value.meta(),
            Self::CoveragePolicy(value) => value.meta(),
            Self::DebtRecord(value) => value.meta(),
            Self::Waiver(value) => value.meta(),
            Self::Reaffirmation(value) => value.meta(),
            Self::Derivation(value) => value.meta(),
            Self::Assumption(value) => value.meta(),
            Self::FiniteModel(value) => value.meta(),
            Self::FormalProperty(value) => value.meta(),
            Self::ProofRecord(value) => value.meta(),
        }
    }
}
