//! `CommonMark` fenced-object extraction with original-byte spans.

use super::external_trace::{EXTERNAL_TRACE_FENCE, parse_external_trace_document};
use super::schema::parse_markdown_semantic;
use super::{AdapterDiagnostic, Located, LocatedExternalTraceLink, MarkdownSemantic, SourceSpan};
use buoy_core::{ParsedItemIdentity, RepositoryPath, SchemaId, SourceItemKind};
use pulldown_cmark::{CodeBlockKind, Event, Parser, Tag, TagEnd};
use std::collections::BTreeSet;
use std::ops::Range;

/// Configured authority expectation for one selected Markdown input.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AuthorityExpectation {
    /// Authorities are invalid in this selection.
    Forbidden,
    /// Zero or more authorities are valid.
    Optional,
    /// At least one valid authority is required.
    Required,
}

/// Total result for one selected Markdown file.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MarkdownOutput {
    /// Complete supported semantic declarations with exact parsed-item identities.
    pub semantic: Vec<Located<MarkdownSemantic>>,
    /// Repository-owned external trace links with exact source identities.
    pub external_links: Vec<LocatedExternalTraceLink>,
    /// Parser-backed spans explicitly treated as nonnormative.
    pub nonnormative: Vec<SourceSpan>,
    /// Count of parser nodes explicitly treated as nonnormative.
    pub nonnormative_nodes: usize,
    /// Deterministic malformed, unsupported, or expectation diagnostics.
    pub diagnostics: Vec<AdapterDiagnostic>,
}

/// Extracts every selected Markdown syntax node into semantic, nonnormative, or diagnostic output.
#[must_use]
pub fn extract_markdown(
    path: &RepositoryPath,
    bytes: &[u8],
    expectation: AuthorityExpectation,
) -> MarkdownOutput {
    let Ok(source) = core::str::from_utf8(bytes) else {
        return error_output(path, bytes.len(), "SPG-ADAPTER-INVALID-UTF8", "markdown/v1");
    };
    if has_lone_carriage_return(source) {
        return error_output(path, bytes.len(), "SPG-ADAPTER-LONE-CR", "markdown/v1");
    }

    let mut semantic = Vec::new();
    let mut diagnostics = Vec::new();
    let mut external_links = Vec::new();
    let mut nonnormative = Vec::new();
    let mut nonnormative_nodes = 0_usize;
    let mut active: Option<(String, usize, String)> = None;
    for (event, range) in Parser::new(source).into_offset_iter() {
        match event {
            Event::Start(Tag::CodeBlock(CodeBlockKind::Fenced(info))) => {
                active = Some((info.into_string(), range.start, String::new()));
            }
            Event::Text(text) | Event::Code(text) => {
                if let Some((_, _, body)) = &mut active {
                    body.push_str(&text);
                } else {
                    nonnormative_nodes += 1;
                    nonnormative.push(source_span(path, range));
                }
            }
            Event::End(TagEnd::CodeBlock) => {
                active.take().into_iter().for_each(|(info, start, body)| {
                    classify_fence(
                        path,
                        &info,
                        &body.replace("\r\n", "\n"),
                        start..range.end,
                        &mut semantic,
                        &mut external_links,
                        &mut diagnostics,
                        &mut nonnormative,
                        &mut nonnormative_nodes,
                    );
                });
            }
            Event::Html(_) | Event::InlineHtml(_) => {
                nonnormative_nodes += 1;
                nonnormative.push(source_span(path, range));
            }
            _ => {}
        }
    }

    let mut link_ids = BTreeSet::new();
    let mut link_digests = BTreeSet::new();
    let mut duplicate = false;
    for located in &external_links {
        let duplicate_id = !link_ids.insert(located.value.meta().id().clone());
        let duplicate_digest = !link_digests.insert(located.value.meta().digest());
        duplicate |= duplicate_id | duplicate_digest;
    }
    if duplicate {
        diagnostics.extend(external_links.drain(..).map(|located| AdapterDiagnostic {
            code: "SPG-EXTERNAL-TRACE-DUPLICATE",
            span: located.span,
            context: located.value.meta().id().as_str().to_owned(),
        }));
    }

    if semantic.is_empty() && diagnostics.is_empty() && nonnormative.is_empty() {
        nonnormative_nodes = 1;
        nonnormative.push(source_span(path, 0..bytes.len()));
    }

    let atom_count = semantic
        .iter()
        .filter(|located| matches!(located.value, MarkdownSemantic::Atom(_)))
        .count();
    match expectation {
        AuthorityExpectation::Forbidden if atom_count != 0 => diagnostics.push(diagnostic(
            "SPG-ADAPTER-AUTHORITY-FORBIDDEN",
            path,
            0..bytes.len(),
            "markdown/v1",
        )),
        AuthorityExpectation::Required if atom_count == 0 => diagnostics.push(diagnostic(
            "SPG-ADAPTER-EXPECTED-AUTHORITY-EMPTY",
            path,
            0..bytes.len(),
            "markdown/v1",
        )),
        _ => {}
    }
    diagnostics.sort_by(|left, right| {
        (left.span.start, left.span.end, left.code, &left.context).cmp(&(
            right.span.start,
            right.span.end,
            right.code,
            &right.context,
        ))
    });
    MarkdownOutput {
        semantic,
        external_links,
        nonnormative,
        nonnormative_nodes,
        diagnostics,
    }
}

#[allow(clippy::too_many_arguments)]
fn classify_fence(
    path: &RepositoryPath,
    info: &str,
    body: &str,
    range: Range<usize>,
    semantic: &mut Vec<Located<MarkdownSemantic>>,
    external_links: &mut Vec<LocatedExternalTraceLink>,
    diagnostics: &mut Vec<AdapterDiagnostic>,
    nonnormative: &mut Vec<SourceSpan>,
    nonnormative_nodes: &mut usize,
) {
    if !supported_fence(info) {
        if info.starts_with("spec-provenance-") {
            diagnostics.push(diagnostic(
                "SPG-ADAPTER-UNSUPPORTED-SYNTAX-VERSION",
                path,
                range,
                info,
            ));
        } else {
            *nonnormative_nodes += 1;
            nonnormative.push(source_span(path, range));
        }
        return;
    }

    let span = source_span(path, range);
    if info == EXTERNAL_TRACE_FENCE {
        match parse_external_trace(path, body, &span) {
            Ok(link) => external_links.push(link),
            Err(code) => diagnostics.push(AdapterDiagnostic {
                code,
                span,
                context: String::from(EXTERNAL_TRACE_FENCE),
            }),
        }
        return;
    }
    match parse_markdown_semantic(info, body) {
        Ok(value) => {
            let item = parsed_item(path, &value);
            semantic.push(Located { value, item, span });
        }
        Err(issue) => diagnostics.push(AdapterDiagnostic {
            code: issue.code,
            span,
            context: issue.context,
        }),
    }
}

fn supported_fence(info: &str) -> bool {
    matches!(
        info,
        EXTERNAL_TRACE_FENCE
            | "spec-provenance-atom-v1"
            | "spec-provenance-surface-v1"
            | "spec-provenance-policy-v1"
            | "spec-provenance-debt-v1"
            | "spec-provenance-waiver-v1"
            | "spec-provenance-reaffirmation-v1"
            | "spec-provenance-derivation-v1"
            | "spec-provenance-assumption-v1"
            | "spec-provenance-finite-model-v1"
            | "spec-provenance-formal-property-v1"
            | "spec-provenance-proof-record-v1"
    )
}

#[allow(clippy::expect_used)]
fn parsed_item(path: &RepositoryPath, value: &MarkdownSemantic) -> ParsedItemIdentity {
    let meta = value.meta();
    let semantic_bytes = format!(
        "{}:{}:{}",
        meta.kind() as u16,
        meta.id().as_str(),
        meta.digest()
    )
    .into_bytes();
    ParsedItemIdentity::new(
        SchemaId::new("markdown/v1").expect("the fixed Markdown schema is valid"),
        path.clone(),
        SourceItemKind::new("markdown/fence/v1").expect("the fixed Markdown item kind is valid"),
        semantic_bytes,
        None,
    )
    .expect("the fixed Markdown schema and validated repository path must construct")
}

/// Builds one Markdown-owned trace link from the shared value parser.
///
/// The Markdown adapter owns its item identity and span. The link value comes
/// from the one shared parser, so an `OpenSpec` version 2 fence with equal bytes
/// produces an equal value.
fn parse_external_trace(
    path: &RepositoryPath,
    body: &str,
    span: &SourceSpan,
) -> Result<LocatedExternalTraceLink, &'static str> {
    let value = parse_external_trace_document(body)?;
    #[allow(clippy::expect_used)]
    let item = ParsedItemIdentity::new(
        SchemaId::new("markdown/v1").expect("the fixed Markdown schema must be valid"),
        path.clone(),
        SourceItemKind::new("markdown/external-trace/v1")
            .expect("the fixed external trace kind must be valid"),
        body.as_bytes().to_vec(),
        None,
    )
    .expect("validated external trace item fields must construct");
    Ok(LocatedExternalTraceLink {
        value,
        item,
        span: span.clone(),
    })
}

fn has_lone_carriage_return(source: &str) -> bool {
    let bytes = source.as_bytes();
    bytes
        .iter()
        .enumerate()
        .any(|(index, byte)| *byte == b'\r' && bytes.get(index + 1) != Some(&b'\n'))
}

fn diagnostic(
    code: &'static str,
    path: &RepositoryPath,
    range: Range<usize>,
    context: &str,
) -> AdapterDiagnostic {
    AdapterDiagnostic {
        code,
        span: source_span(path, range),
        context: context.to_owned(),
    }
}

fn source_span(path: &RepositoryPath, range: Range<usize>) -> SourceSpan {
    SourceSpan {
        path: path.clone(),
        start: range.start as u64,
        end: range.end as u64,
    }
}

fn error_output(
    path: &RepositoryPath,
    length: usize,
    code: &'static str,
    context: &str,
) -> MarkdownOutput {
    MarkdownOutput {
        semantic: Vec::new(),
        external_links: Vec::new(),
        nonnormative: Vec::new(),
        nonnormative_nodes: 0,
        diagnostics: vec![diagnostic(code, path, 0..length, context)],
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;

    fn external_trace(id: &str, atoms: &str, relation: &str, target: &str) -> String {
        format!(
            "id = \"{id}\"\natoms = {atoms}\nrelation = \"{relation}\"\ntarget = \"{target}\"\n"
        )
    }

    #[test]
    fn duplicate_external_trace_ids_and_digests_fail_closed() {
        let path = RepositoryPath::new("spec/control.md").expect("path");
        let digest = format!("xxh3-128:{}", "1".repeat(32));
        let block = |id: &str, target: &str| {
            format!(
                "```{EXTERNAL_TRACE_FENCE}\n{}\n```\n",
                external_trace(id, &format!("[\"ATOM-001@{digest}\"]"), "tracks", target,)
            )
        };
        for source in [
            format!(
                "{}{}",
                block("TRACE-DUPLICATE", "https://example.com/1"),
                block("TRACE-DUPLICATE", "https://example.com/2")
            ),
            format!(
                "{}{}",
                block("TRACE-IDENTICAL", "https://example.com/1"),
                block("TRACE-IDENTICAL", "https://example.com/1")
            ),
        ] {
            let output = extract_markdown(&path, source.as_bytes(), AuthorityExpectation::Optional);
            assert!(output.external_links.is_empty());
            assert_eq!(
                output
                    .diagnostics
                    .iter()
                    .filter(|diagnostic| diagnostic.code == "SPG-EXTERNAL-TRACE-DUPLICATE")
                    .count(),
                2
            );
        }
    }

    #[test]
    fn external_trace_parser_rejects_each_malformed_field() {
        let path = RepositoryPath::new("spec/control.md").expect("path");
        let span = SourceSpan {
            path: path.clone(),
            start: 0,
            end: 1,
        };
        let valid_digest = format!("xxh3-128:{}", "1".repeat(32));
        let valid_atoms = format!("[\"ATOM-001@{valid_digest}\"]");
        let invalid_id_atoms = format!("[\"invalid id@{valid_digest}\"]");
        for body in [
            String::from("not = [toml"),
            external_trace(
                "invalid id",
                &valid_atoms,
                "tracks",
                "https://example.com/1",
            ),
            external_trace(
                "TRACE-1",
                "[\"missing-at\"]",
                "tracks",
                "https://example.com/1",
            ),
            external_trace(
                "TRACE-1",
                &invalid_id_atoms,
                "tracks",
                "https://example.com/1",
            ),
            external_trace(
                "TRACE-1",
                "[\"ATOM-001@invalid\"]",
                "tracks",
                "https://example.com/1",
            ),
            external_trace("TRACE-1", &valid_atoms, "invalid", "https://example.com/1"),
            external_trace("TRACE-1", &valid_atoms, "tracks", "invalid target"),
            external_trace("TRACE-1", "[]", "tracks", "https://example.com/1"),
        ] {
            assert!(parse_external_trace(&path, &body, &span).is_err(), "{body}");
        }
        assert!(
            parse_external_trace(
                &path,
                &external_trace("TRACE-1", &valid_atoms, "tracks", "https://example.com/1",),
                &span,
            )
            .is_ok()
        );
    }
}
