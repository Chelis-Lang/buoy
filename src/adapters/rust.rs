//! `syn`-backed Rust attachment extraction.

use super::schema::{
    ParseIssue, parse_carrier, parse_derivation, parse_implementation_link, parse_surface,
};
use super::{AdapterDiagnostic, Located, SourceSpan};
use buoy_core::{
    ActiveAlgorithm, Derivation, DigestAssuranceClass, EvidenceCarrier, EvidenceRole,
    ImplementationBindingRef, ImplementationLink, ObjectMeta, ParsedItemIdentity, RepositoryPath,
    SchemaId, SemanticDigest, SourceItemKind, SourceNode,
};
use proc_macro2::LineColumn;
use quote::ToTokens;
use std::collections::BTreeMap;
use syn::spanned::Spanned;
use syn::{Attribute, File, Item, LitStr};

/// Complete semantic values supported by Rust v1 attachments.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RustSemantic {
    /// A carrier, structural checker, or verifier attachment.
    EvidenceCarrier(EvidenceCarrier),
    /// A navigational implementation link.
    ImplementationLink(ImplementationLink),
    /// A complete governed decision surface.
    DecisionSurface(buoy_core::DecisionSurface),
    /// A complete derivation attached to the parsed item that owns it.
    Derivation(Derivation),
}

impl RustSemantic {
    /// Complete object metadata for source mapping and graph insertion.
    #[must_use]
    pub const fn meta(&self) -> &ObjectMeta {
        match self {
            Self::EvidenceCarrier(value) => value.meta(),
            Self::ImplementationLink(value) => value.meta(),
            Self::DecisionSurface(value) => value.meta(),
            Self::Derivation(value) => value.meta(),
        }
    }
}

/// Total result for one selected Rust file.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RustOutput {
    /// Complete supported semantic attachments with exact parsed-item identities.
    pub semantic: Vec<Located<RustSemantic>>,
    /// Retained subject and enclosing nodes for compact bindings.
    pub source_nodes: Vec<LocatedRustNode>,
    /// Valid compact locators attached to exact subject nodes.
    pub bindings: Vec<LocatedRustBinding>,
    /// Parser-backed spans for items with no semantic attachment.
    pub nonnormative: Vec<SourceSpan>,
    /// Parsed items with no semantic attachment.
    pub nonnormative_items: usize,
    /// Deterministic malformed, unsupported, disabled, or unattached diagnostics.
    pub diagnostics: Vec<AdapterDiagnostic>,
}

/// One Rust source node with its exact parser-backed item span.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LocatedRustNode {
    /// Complete content-addressed subject or enclosing node.
    pub node: SourceNode,
    /// Exact original-byte item span.
    pub span: SourceSpan,
}

/// One compact locator with its exact attribute span and subject reference.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LocatedRustBinding {
    /// Exact compact implementation-binding locator.
    pub record: ImplementationBindingRef,
    /// Exact subject node that owns the attribute.
    pub subject: buoy_core::SourceNodeRef,
    /// Exact original-byte attribute span.
    pub span: SourceSpan,
}

/// Extracts every selected Rust item into semantic, nonnormative, or diagnostic output.
///
/// # Panics
///
/// Panics if the build contains no digest adapter.
#[must_use]
#[allow(clippy::expect_used)]
pub fn extract_rust(path: &RepositoryPath, bytes: &[u8]) -> RustOutput {
    let active = ActiveAlgorithm::build_default()
        .expect("a Rust adapter build must contain one digest member");
    extract_rust_with(active, path, bytes)
}

/// Extracts Rust attachments under the repository-selected digest member.
#[must_use]
pub fn extract_rust_with(
    active: ActiveAlgorithm,
    path: &RepositoryPath,
    bytes: &[u8],
) -> RustOutput {
    let Ok(source) = core::str::from_utf8(bytes) else {
        return error_output(path, bytes.len(), "SPG-ADAPTER-INVALID-UTF8", "rust/v1");
    };
    let Ok(file) = syn::parse_file(source) else {
        return error_output(path, bytes.len(), "SPG-ADAPTER-RUST-PARSE", "rust/v1");
    };
    extract_file(active, path, source, &file)
}

fn extract_file(
    active: ActiveAlgorithm,
    path: &RepositoryPath,
    source: &str,
    file: &File,
) -> RustOutput {
    let line_starts = line_starts(source);
    let mut output = RustOutput {
        semantic: Vec::new(),
        source_nodes: Vec::new(),
        bindings: Vec::new(),
        nonnormative: Vec::new(),
        nonnormative_items: 0,
        diagnostics: Vec::new(),
    };
    for attribute in &file.attrs {
        if provenance_attribute(attribute).is_some() || is_buoy_implementation(attribute) {
            output.diagnostics.push(attribute_diagnostic(
                "SPG-ADAPTER-UNATTACHED-ATTRIBUTE",
                path,
                attribute,
                &line_starts,
                "file-level",
            ));
        }
    }
    extract_items(
        active,
        path,
        &file.items,
        &line_starts,
        None,
        None,
        &mut output,
    );
    // Every parsed item retains one source node. Thus, an empty node set also
    // implies empty semantic, binding, and nonnormative item sets.
    if output.source_nodes.is_empty() && output.diagnostics.is_empty() {
        output.nonnormative_items = 1;
        output.nonnormative.push(SourceSpan {
            path: path.clone(),
            start: 0,
            end: source.len() as u64,
        });
    }
    output.diagnostics.sort_by(|left, right| {
        (left.span.start, left.span.end, left.code, &left.context).cmp(&(
            right.span.start,
            right.span.end,
            right.code,
            &right.context,
        ))
    });
    output
}

#[allow(clippy::expect_used, clippy::too_many_arguments)]
fn extract_items(
    active: ActiveAlgorithm,
    path: &RepositoryPath,
    items: &[Item],
    starts: &[usize],
    enclosing: Option<buoy_core::SemanticDigest>,
    source_enclosing: Option<buoy_core::SemanticDigest>,
    output: &mut RustOutput,
) {
    for item in items {
        let parsed_item = parsed_item(active, path, item, enclosing);
        let source_item = parsed_subject_item(active, path, item, source_enclosing);
        let source_item_digest = source_item.digest();
        let source_node = SourceNode::new_with(active, source_item, Vec::new())
            .expect("a childless node over an active-member item constructs");
        let subject = source_node.reference();
        let compact_span = item_attributes(item)
            .iter()
            .find(|attribute| is_buoy_implementation(attribute))
            .map_or_else(
                || span_from(path, item.span(), starts),
                |attribute| span_from(path, attribute.span(), starts),
            );
        output.source_nodes.push(LocatedRustNode {
            node: source_node,
            span: compact_span,
        });
        extract_item(active, path, item, starts, &parsed_item, subject, output);
        if let Item::Mod(module) = item {
            if let Some((_, nested)) = &module.content {
                extract_items(
                    active,
                    path,
                    nested,
                    starts,
                    Some(parsed_item.digest()),
                    Some(source_item_digest),
                    output,
                );
            }
        }
    }
}

#[allow(clippy::too_many_lines)]
fn extract_item(
    active: ActiveAlgorithm,
    path: &RepositoryPath,
    item: &Item,
    starts: &[usize],
    parsed_item: &ParsedItemIdentity,
    subject: buoy_core::SourceNodeRef,
    output: &mut RustOutput,
) {
    let attributes = item_attributes(item);
    let provenance = attributes
        .iter()
        .filter_map(|attribute| provenance_attribute(attribute).map(|kind| (attribute, kind)))
        .collect::<Vec<_>>();
    let compact = attributes
        .iter()
        .filter(|attribute| is_buoy_implementation(attribute))
        .collect::<Vec<_>>();
    let verbose_implementation = provenance
        .iter()
        .filter(|(_, kind)| kind == "implementation")
        .map(|(attribute, _)| *attribute)
        .collect::<Vec<_>>();
    if !compact.is_empty() && !verbose_implementation.is_empty() {
        for attribute in compact
            .iter()
            .copied()
            .chain(verbose_implementation.iter().copied())
        {
            output.diagnostics.push(attribute_diagnostic(
                "CAB-MIXED-BINDING-SYNTAX",
                path,
                attribute,
                starts,
                "implementation-binding",
            ));
        }
        return;
    }
    if compact.len() > 1 {
        for attribute in &compact {
            output.diagnostics.push(attribute_diagnostic(
                "CAB-COMPACT-ATTRIBUTE-COUNT",
                path,
                attribute,
                starts,
                "buoy::implements",
            ));
        }
        return;
    }
    if let Some(attribute) = compact.first().copied() {
        let span = span_from(path, attribute.span(), starts);
        match compact_binding(active, attribute) {
            Ok(record) => output.bindings.push(LocatedRustBinding {
                record,
                subject,
                span,
            }),
            Err(code) => output.diagnostics.push(AdapterDiagnostic {
                code,
                span,
                context: String::from("buoy::implements"),
            }),
        }
    }
    if provenance.is_empty() && compact.is_empty() {
        output.nonnormative_items += 1;
        output
            .nonnormative
            .push(span_from(path, item.span(), starts));
        return;
    }
    for (attribute, kind) in provenance {
        let span = span_from(path, attribute.span(), starts);
        if has_disabled_cfg(attributes) {
            output.diagnostics.push(AdapterDiagnostic {
                code: "SPG-ADAPTER-DISABLED-ITEM",
                span,
                context: kind,
            });
            continue;
        }
        let Ok(metadata) = parse_metadata(attribute) else {
            output.diagnostics.push(AdapterDiagnostic {
                code: "SPG-ADAPTER-MALFORMED-ATTRIBUTE",
                span,
                context: kind,
            });
            continue;
        };
        if metadata.keys().map(String::as_str).collect::<Vec<_>>() != ["body", "syntax"] {
            output.diagnostics.push(AdapterDiagnostic {
                code: "SPG-ADAPTER-ATTRIBUTE-FIELDS",
                span,
                context: kind,
            });
            continue;
        }
        if metadata.get("syntax").map(String::as_str) != Some("v1") {
            output.diagnostics.push(AdapterDiagnostic {
                code: "SPG-ADAPTER-UNSUPPORTED-SYNTAX-VERSION",
                span,
                context: kind,
            });
            continue;
        }
        let parsed = materialize_attachment(&kind, &metadata["body"], item, parsed_item.digest());
        match parsed {
            Ok(value) => output.semantic.push(Located {
                value,
                item: parsed_item.clone(),
                span,
            }),
            Err(issue) => output.diagnostics.push(AdapterDiagnostic {
                code: issue.code,
                span,
                context: issue.context,
            }),
        }
    }
}

fn materialize_attachment(
    kind: &str,
    body: &str,
    item: &Item,
    parsed_item_digest: SemanticDigest,
) -> Result<RustSemantic, ParseIssue> {
    match kind {
        "carrier" => materialize_carrier(body, item, parsed_item_digest, None),
        "structural" => materialize_carrier(
            body,
            item,
            parsed_item_digest,
            Some(EvidenceRole::Structural),
        ),
        "verifier" => materialize_carrier(
            body,
            item,
            parsed_item_digest,
            Some(EvidenceRole::FormalProperty),
        ),
        "implementation" => parse_implementation_link(body, parsed_item_digest)
            .map(RustSemantic::ImplementationLink),
        "governed_surface" => parse_surface(body).map(RustSemantic::DecisionSurface),
        "derivation" => parse_derivation(body).map(RustSemantic::Derivation),
        _ => Err(ParseIssue {
            code: "SPG-ADAPTER-UNSUPPORTED-RUST-DECLARATION",
            context: kind.to_owned(),
        }),
    }
}

fn materialize_carrier(
    body: &str,
    item: &Item,
    parsed_item_digest: SemanticDigest,
    required_role: Option<EvidenceRole>,
) -> Result<RustSemantic, ParseIssue> {
    if eligible_function(item) {
        parse_carrier(body, parsed_item_digest, required_role).map(RustSemantic::EvidenceCarrier)
    } else {
        Err(ineligible())
    }
}

fn ineligible() -> ParseIssue {
    ParseIssue {
        code: "SPG-ADAPTER-INELIGIBLE-CARRIER",
        context: String::from("carrier"),
    }
}

fn parse_metadata(attribute: &Attribute) -> syn::Result<BTreeMap<String, String>> {
    let mut values = BTreeMap::new();
    attribute.parse_nested_meta(|meta| {
        let Some(identifier) = meta.path.get_ident() else {
            return Err(meta.error("metadata key must be one identifier"));
        };
        let value = meta.value()?.parse::<LitStr>()?.value();
        if values.insert(identifier.to_string(), value).is_some() {
            return Err(meta.error("duplicate metadata key"));
        }
        Ok(())
    })?;
    Ok(values)
}

#[allow(clippy::expect_used)]
fn provenance_attribute(attribute: &Attribute) -> Option<String> {
    let mut segments = attribute.path().segments.iter();
    let first = segments.next().expect("syn attribute paths are nonempty");
    let second = segments.next()?;
    if segments.next().is_none() && first.ident == "spec_provenance" {
        Some(second.ident.to_string())
    } else {
        None
    }
}

fn is_buoy_implementation(attribute: &Attribute) -> bool {
    let mut segments = attribute.path().segments.iter();
    matches!(
        (segments.next(), segments.next(), segments.next()),
        (Some(first), Some(second), None)
            if first.ident == "buoy" && second.ident == "implements"
    )
}

fn compact_binding(
    active: ActiveAlgorithm,
    attribute: &Attribute,
) -> Result<ImplementationBindingRef, &'static str> {
    if active.algorithm().assurance_class() != DigestAssuranceClass::CollisionResistant {
        return Err("CAB-DIGEST-CLASS");
    }
    let literal = attribute
        .parse_args::<LitStr>()
        .map_err(|_| "CAB-COMPACT-ATTRIBUTE-SYNTAX")?;
    let record = ImplementationBindingRef::parse(&literal.value())
        .map_err(|_| "CAB-COMPACT-ATTRIBUTE-DIGEST")?;
    if record.digest().algorithm() != active.algorithm() {
        return Err("CAB-COMPACT-DIGEST-MEMBER");
    }
    Ok(record)
}

fn item_attributes(item: &Item) -> &[Attribute] {
    match item {
        Item::Const(item) => &item.attrs,
        Item::Enum(item) => &item.attrs,
        Item::ExternCrate(item) => &item.attrs,
        Item::Fn(item) => &item.attrs,
        Item::ForeignMod(item) => &item.attrs,
        Item::Impl(item) => &item.attrs,
        Item::Macro(item) => &item.attrs,
        Item::Mod(item) => &item.attrs,
        Item::Static(item) => &item.attrs,
        Item::Struct(item) => &item.attrs,
        Item::Trait(item) => &item.attrs,
        Item::TraitAlias(item) => &item.attrs,
        Item::Type(item) => &item.attrs,
        Item::Union(item) => &item.attrs,
        Item::Use(item) => &item.attrs,
        _ => &[],
    }
}

fn eligible_function(item: &Item) -> bool {
    let Item::Fn(function) = item else {
        return false;
    };
    function.attrs.iter().any(|attribute| {
        attribute.path().is_ident("test")
            || provenance_attribute(attribute).as_deref() == Some("verifier")
            || provenance_attribute(attribute).as_deref() == Some("structural")
    })
}

fn has_disabled_cfg(attributes: &[Attribute]) -> bool {
    attributes.iter().any(|attribute| {
        attribute.path().is_ident("cfg")
            && attribute
                .meta
                .to_token_stream()
                .to_string()
                .contains("any ()")
    })
}

#[allow(clippy::expect_used)]
fn parsed_item(
    active: ActiveAlgorithm,
    path: &RepositoryPath,
    item: &Item,
    enclosing: Option<buoy_core::SemanticDigest>,
) -> ParsedItemIdentity {
    ParsedItemIdentity::new_with(
        active,
        SchemaId::new("rust/v1").expect("the fixed Rust schema is valid"),
        path.clone(),
        SourceItemKind::new("rust/item/v1").expect("the fixed Rust item kind is valid"),
        item.to_token_stream().to_string().into_bytes(),
        enclosing,
    )
    .expect("the fixed Rust schema and validated repository path must construct")
}

#[allow(clippy::expect_used)]
fn parsed_subject_item(
    active: ActiveAlgorithm,
    path: &RepositoryPath,
    item: &Item,
    enclosing: Option<buoy_core::SemanticDigest>,
) -> ParsedItemIdentity {
    let semantic = subject_tokens(item);
    ParsedItemIdentity::new_with(
        active,
        SchemaId::new("rust/v1").expect("the fixed Rust schema is valid"),
        path.clone(),
        SourceItemKind::new("rust/item/v1").expect("the fixed Rust item kind is valid"),
        semantic,
        enclosing,
    )
    .expect("the fixed Rust schema and validated repository path must construct")
}

fn subject_tokens(item: &Item) -> Vec<u8> {
    let mut subject = item.clone();
    strip_binding_attributes(&mut subject);
    subject.to_token_stream().to_string().into_bytes()
}

fn strip_binding_attributes(item: &mut Item) {
    if let Some(attributes) = item_attributes_mut(item) {
        attributes.retain(|attribute| !is_binding_attribute(attribute));
    }
    if let Item::Mod(module) = item
        && let Some((_, nested)) = &mut module.content
    {
        for child in nested {
            strip_binding_attributes(child);
        }
    }
}

fn is_binding_attribute(attribute: &Attribute) -> bool {
    is_buoy_implementation(attribute)
        || provenance_attribute(attribute).as_deref() == Some("implementation")
}

fn item_attributes_mut(item: &mut Item) -> Option<&mut Vec<Attribute>> {
    match item {
        Item::Const(item) => Some(&mut item.attrs),
        Item::Enum(item) => Some(&mut item.attrs),
        Item::ExternCrate(item) => Some(&mut item.attrs),
        Item::Fn(item) => Some(&mut item.attrs),
        Item::ForeignMod(item) => Some(&mut item.attrs),
        Item::Impl(item) => Some(&mut item.attrs),
        Item::Macro(item) => Some(&mut item.attrs),
        Item::Mod(item) => Some(&mut item.attrs),
        Item::Static(item) => Some(&mut item.attrs),
        Item::Struct(item) => Some(&mut item.attrs),
        Item::Trait(item) => Some(&mut item.attrs),
        Item::TraitAlias(item) => Some(&mut item.attrs),
        Item::Type(item) => Some(&mut item.attrs),
        Item::Union(item) => Some(&mut item.attrs),
        Item::Use(item) => Some(&mut item.attrs),
        _ => None,
    }
}

fn line_starts(source: &str) -> Vec<usize> {
    let mut starts = vec![0];
    for (index, byte) in source.bytes().enumerate() {
        if byte == b'\n' {
            starts.push(index + 1);
        }
    }
    starts
}

fn byte_offset(position: LineColumn, starts: &[usize]) -> usize {
    position
        .line
        .checked_sub(1)
        .and_then(|line| starts.get(line))
        .copied()
        .unwrap_or(0)
        .saturating_add(position.column)
}

fn span_from(path: &RepositoryPath, span: proc_macro2::Span, starts: &[usize]) -> SourceSpan {
    SourceSpan {
        path: path.clone(),
        start: byte_offset(span.start(), starts) as u64,
        end: byte_offset(span.end(), starts) as u64,
    }
}

fn attribute_diagnostic(
    code: &'static str,
    path: &RepositoryPath,
    attribute: &Attribute,
    starts: &[usize],
    context: &str,
) -> AdapterDiagnostic {
    AdapterDiagnostic {
        code,
        span: span_from(path, attribute.span(), starts),
        context: context.to_owned(),
    }
}

fn error_output(
    path: &RepositoryPath,
    length: usize,
    code: &'static str,
    context: &str,
) -> RustOutput {
    RustOutput {
        semantic: Vec::new(),
        source_nodes: Vec::new(),
        bindings: Vec::new(),
        nonnormative: Vec::new(),
        nonnormative_items: 0,
        diagnostics: vec![AdapterDiagnostic {
            code,
            span: SourceSpan {
                path: path.clone(),
                start: 0,
                end: length as u64,
            },
            context: context.to_owned(),
        }],
    }
}

#[cfg(all(test, feature = "coverage-controls"))]
mod coverage_tests {
    use super::*;

    #[test]
    fn verbatim_parser_items_have_no_attachable_attributes() {
        let item = Item::Verbatim(proc_macro2::TokenStream::new());
        assert!(item_attributes(&item).is_empty());
    }
}
