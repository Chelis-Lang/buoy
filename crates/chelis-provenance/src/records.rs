//! Frozen Chelis provenance record grammar over saved source bytes.
//!
//! The grammar is total. Every selected byte region receives exactly one of
//! three treatments: a semantic provenance record, an explicit nonnormative
//! disposition, or a deterministic diagnostic. Unsupported record schema
//! versions fail extraction loudly instead of producing an empty graph.

use chelis_surf_sitter::SurfDeclaration;
use std::collections::BTreeMap;

/// Stable diagnostic for a structurally invalid provenance record.
pub const MALFORMED_RECORD: &str = "CHELIS-PROV-MALFORMED-RECORD";
/// Stable diagnostic for provenance metadata without a following declaration.
pub const UNATTACHED_METADATA: &str = "CHELIS-PROV-UNATTACHED-METADATA";
/// Stable diagnostic for an unsupported provenance schema version.
pub const UNSUPPORTED_SCHEMA: &str = "CHELIS-PROV-UNSUPPORTED-SCHEMA";
/// Stable diagnostic for a Surf source the pinned grammar rejects.
pub const PARSE_ERROR: &str = "CHELIS-PROV-PARSE-ERROR";

/// Frozen record kind of one compact content-addressed binding marker.
pub const BINDING_RECORD_KIND: &str = "binding";
/// Frozen single field of one compact binding marker.
pub const BINDING_ADDRESS_FIELD: &str = "record";

const SURF_ANCHOR_KINDS: [&str; 4] = [
    "function_definition",
    "opaque_type_declaration",
    "type_alias",
    "type_definition",
];

const MARKER_PREFIX: &str = "chelis:provenance/";
const SUPPORTED_MARKER: &str = "chelis:provenance/v1";

/// Closed set of Chelis source dialects with pinned parser identities.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ChelisDialect {
    /// Surface language in `.ch` files with `--` line comments.
    Surf,
    /// Deep IR in `.dp` files with `//` line comments.
    Deep,
}

impl ChelisDialect {
    /// Line-comment marker for this dialect.
    #[must_use]
    pub const fn comment_marker(self) -> &'static str {
        match self {
            Self::Surf => "--",
            Self::Deep => "//",
        }
    }

    /// Immutable adapter identity from the frozen integration boundary.
    #[must_use]
    pub const fn adapter_id(self) -> &'static str {
        match self {
            Self::Surf => "chelis-surf/v1",
            Self::Deep => "chelis-deep/v1",
        }
    }

    /// Pinned parser identity.
    #[must_use]
    pub const fn parser_id(self) -> &'static str {
        match self {
            Self::Surf => "chelis-surf",
            Self::Deep => "chelis-deep",
        }
    }

    /// Pinned language/syntax schema identity.
    #[must_use]
    pub const fn language_schema_id(self) -> &'static str {
        match self {
            Self::Surf => "chelis-surf-syntax/v2",
            Self::Deep => "chelis-deep-syntax/v1",
        }
    }

    /// Pinned parser version from the frozen integration boundary.
    ///
    /// The Surf version binds the vendored tree-sitter grammar to its exact
    /// Chelis source revision. The Deep dialect keeps the lexical scheme.
    #[must_use]
    pub const fn parser_version(self) -> &'static str {
        match self {
            Self::Surf => "0.18.5+grammar.fc3232ff",
            Self::Deep => "0.16.1",
        }
    }

    /// Descriptor-declared schema-v3 source-item kind.
    #[must_use]
    pub const fn source_item_kind(self) -> &'static str {
        match self {
            Self::Surf => "chelis/surf-declaration/v1",
            Self::Deep => "chelis/deep-node/v1",
        }
    }

    /// Syntax capability for declarations and record blocks.
    #[must_use]
    pub const fn declaration_capability(self) -> &'static str {
        match self {
            Self::Surf => "chelis-surf-declaration/v1",
            Self::Deep => "chelis-deep-node/v1",
        }
    }

    /// Syntax capability for ordinary comments.
    #[must_use]
    pub const fn comment_capability(self) -> &'static str {
        match self {
            Self::Surf => "chelis-surf-comment/v1",
            Self::Deep => "chelis-deep-comment/v1",
        }
    }

    /// Selected file extension.
    #[must_use]
    pub const fn extension(self) -> &'static str {
        match self {
            Self::Surf => "ch",
            Self::Deep => "dp",
        }
    }

    const fn declaration_start(self, line: &str) -> bool {
        match self {
            Self::Surf => {
                let bytes = line.as_bytes();
                (bytes.len() >= 4
                    && bytes[0] == b'd'
                    && bytes[1] == b'e'
                    && bytes[2] == b'f'
                    && bytes[3] == b' ')
                    || (bytes.len() >= 5
                        && bytes[0] == b't'
                        && bytes[1] == b'y'
                        && bytes[2] == b'p'
                        && bytes[3] == b'e'
                        && bytes[4] == b' ')
            }
            Self::Deep => {
                let bytes = line.as_bytes();
                !bytes.is_empty() && bytes[0] == b'('
            }
        }
    }
}

/// One extracted provenance record and its attached declaration.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ChelisRecord {
    /// Declared record kind.
    pub kind: String,
    /// Exact `key = value` record fields.
    pub fields: BTreeMap<String, String>,
    /// Inclusive byte start of the record block.
    pub start: u64,
    /// Exclusive byte end of the record block.
    pub end: u64,
    /// Inclusive byte start of the attached declaration line.
    pub declaration_start: u64,
    /// Exclusive byte end of the attached declaration line.
    pub declaration_end: u64,
    /// Exact attached declaration line text.
    pub declaration_text: String,
}

/// One deterministic extraction diagnostic.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RecordDiagnostic {
    /// Stable diagnostic code.
    pub code: &'static str,
    /// Inclusive byte start.
    pub start: u64,
    /// Exclusive byte end.
    pub end: u64,
    /// Schema-owned deterministic context.
    pub context: String,
}

/// Complete total classification of one saved source file.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Extraction {
    /// Extracted semantic records in source order.
    pub records: Vec<ChelisRecord>,
    /// Ordinary comment spans in source order.
    pub comment_spans: Vec<(u64, u64)>,
    /// Ordinary code, declaration, and blank spans in source order.
    pub code_spans: Vec<(u64, u64)>,
    /// Deterministic diagnostics in source order.
    pub diagnostics: Vec<RecordDiagnostic>,
}

/// Loud extraction failure that must not produce an empty successful graph.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ExtractionError {
    /// A provenance marker names an unsupported schema version.
    UnsupportedSchema {
        /// Inclusive byte start of the marker line.
        start: u64,
        /// Exclusive byte end of the marker line.
        end: u64,
        /// Exact unsupported marker text.
        marker: String,
    },
    /// The pinned Surf grammar rejects the selected source.
    ParseError {
        /// Inclusive byte start of the first defective region.
        start: u64,
        /// Exclusive byte end of the first defective region.
        end: u64,
        /// Deterministic defect context.
        context: String,
    },
}

impl ExtractionError {
    /// Stable diagnostic code for this failure.
    #[must_use]
    pub const fn code(&self) -> &'static str {
        match self {
            Self::UnsupportedSchema { .. } => UNSUPPORTED_SCHEMA,
            Self::ParseError { .. } => PARSE_ERROR,
        }
    }
}

struct Line<'a> {
    text: &'a str,
    start: u64,
    end: u64,
}

fn lines(source: &str) -> Vec<Line<'_>> {
    let mut out = Vec::new();
    let mut start = 0_u64;
    for segment in source.split_inclusive('\n') {
        let end = start + segment.len() as u64;
        out.push(Line {
            text: segment.trim_end_matches(['\n', '\r']),
            start,
            end,
        });
        start = end;
    }
    out
}

fn slice(source: &str, start: u64, end: u64) -> &str {
    let start = usize::try_from(start).unwrap_or(source.len());
    let end = usize::try_from(end)
        .unwrap_or(source.len())
        .min(source.len());
    source.get(start..end).unwrap_or("")
}

fn surf_declarations(source: &str) -> Result<Vec<SurfDeclaration>, ExtractionError> {
    match chelis_surf_sitter::parse(source) {
        Ok(outcome) => match outcome.defect {
            Some(defect) => Err(ExtractionError::ParseError {
                start: defect.start,
                end: defect.end,
                context: String::from("grammar-reject"),
            }),
            None => Ok(outcome.declarations),
        },
        Err(_) => Err(ExtractionError::ParseError {
            start: 0,
            end: source.len() as u64,
            context: String::from("grammar-unavailable"),
        }),
    }
}

fn field_line(rest: &str) -> Option<(String, String)> {
    let (key, value) = rest.split_once('=')?;
    let key = key.trim();
    let value = value.trim();
    if key.is_empty()
        || !key
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
    {
        return None;
    }
    Some((key.to_owned(), value.to_owned()))
}

/// Returns the field-set defect of one compact binding marker, when present.
///
/// The compact form exists so that no in-source field can drift. That holds
/// only while the marker's field set is closed: a second field would be an
/// unenforced claim sitting beside the address, so the grammar accepts exactly
/// `record` and nothing else. Duplicate fields are already rejected upstream,
/// which keeps that reason visible instead of being masked here.
fn binding_field_defect(kind: &str, fields: &BTreeMap<String, String>) -> Option<String> {
    if kind != BINDING_RECORD_KIND {
        return None;
    }
    if let Some(unknown) = fields
        .keys()
        .find(|key| key.as_str() != BINDING_ADDRESS_FIELD)
    {
        return Some(format!("unknown-field:{unknown}"));
    }
    (!fields.contains_key(BINDING_ADDRESS_FIELD))
        .then(|| format!("missing-field:{BINDING_ADDRESS_FIELD}"))
}

/// Extracts every provenance record from one saved source file.
///
/// # Errors
///
/// Fails loudly when a provenance marker names an unsupported schema version.
#[allow(clippy::too_many_lines)]
pub fn extract(dialect: ChelisDialect, bytes: &[u8]) -> Result<Extraction, ExtractionError> {
    let mut output = Extraction::default();
    let Ok(source) = core::str::from_utf8(bytes) else {
        output.diagnostics.push(RecordDiagnostic {
            code: MALFORMED_RECORD,
            start: 0,
            end: bytes.len() as u64,
            context: String::from("invalid-utf8"),
        });
        return Ok(output);
    };
    let parsed_declarations = match dialect {
        ChelisDialect::Surf => surf_declarations(source)?,
        ChelisDialect::Deep => Vec::new(),
    };
    let marker = dialect.comment_marker();
    let all = lines(source);
    let mut index = 0_usize;
    while index < all.len() {
        let line = &all[index];
        let trimmed = line.text.trim_start();
        let Some(comment_rest) = trimmed.strip_prefix(marker) else {
            output.code_spans.push((line.start, line.end));
            index += 1;
            continue;
        };
        let comment_rest = comment_rest.trim_start();
        if !comment_rest.starts_with(MARKER_PREFIX) {
            output.comment_spans.push((line.start, line.end));
            index += 1;
            continue;
        }
        let Some(marker_rest) = comment_rest.strip_prefix(SUPPORTED_MARKER) else {
            return Err(ExtractionError::UnsupportedSchema {
                start: line.start,
                end: line.end,
                marker: comment_rest.to_owned(),
            });
        };
        let kind = marker_rest.trim();
        let record_start = line.start;
        let mut record_end = line.end;
        if kind.is_empty() {
            output.diagnostics.push(RecordDiagnostic {
                code: MALFORMED_RECORD,
                start: record_start,
                end: record_end,
                context: String::from("missing-record-kind"),
            });
            index += 1;
            continue;
        }
        let mut fields = BTreeMap::new();
        let mut malformed: Option<String> = None;
        index += 1;
        while index < all.len() {
            let body = &all[index];
            let body_trimmed = body.text.trim_start();
            let Some(rest) = body_trimmed.strip_prefix(marker) else {
                break;
            };
            let rest = rest.trim_start();
            if rest.starts_with(MARKER_PREFIX) {
                break;
            }
            record_end = body.end;
            index += 1;
            match field_line(rest) {
                Some((key, value)) => {
                    if fields.insert(key.clone(), value).is_some() && malformed.is_none() {
                        malformed = Some(format!("duplicate-field:{key}"));
                    }
                }
                None => {
                    if malformed.is_none() {
                        malformed = Some(String::from("invalid-field-line"));
                    }
                }
            }
        }
        let malformed = malformed.or_else(|| binding_field_defect(kind, &fields));
        if let Some(context) = malformed {
            output.diagnostics.push(RecordDiagnostic {
                code: MALFORMED_RECORD,
                start: record_start,
                end: record_end,
                context,
            });
            continue;
        }
        while index < all.len() && all[index].text.trim().is_empty() {
            output.code_spans.push((all[index].start, all[index].end));
            index += 1;
        }
        match dialect {
            ChelisDialect::Deep => {
                let attached = index < all.len() && dialect.declaration_start(all[index].text);
                if !attached {
                    output.diagnostics.push(RecordDiagnostic {
                        code: UNATTACHED_METADATA,
                        start: record_start,
                        end: record_end,
                        context: format!("record-kind:{kind}"),
                    });
                    continue;
                }
                let declaration = &all[index];
                output.records.push(ChelisRecord {
                    kind: kind.to_owned(),
                    fields,
                    start: record_start,
                    end: record_end,
                    declaration_start: declaration.start,
                    declaration_end: declaration.end,
                    declaration_text: declaration.text.to_owned(),
                });
                output.code_spans.push((declaration.start, declaration.end));
                index += 1;
            }
            ChelisDialect::Surf => {
                let anchor = if index < all.len() {
                    let line = &all[index];
                    parsed_declarations
                        .iter()
                        .find(|node| node.start >= line.start && node.start < line.end)
                        .filter(|node| {
                            SURF_ANCHOR_KINDS.contains(&node.kind.as_str())
                                && slice(source, line.start, node.start).trim().is_empty()
                        })
                } else {
                    None
                };
                let Some(node) = anchor else {
                    output.diagnostics.push(RecordDiagnostic {
                        code: UNATTACHED_METADATA,
                        start: record_start,
                        end: record_end,
                        context: format!("record-kind:{kind}"),
                    });
                    continue;
                };
                output.records.push(ChelisRecord {
                    kind: kind.to_owned(),
                    fields,
                    start: record_start,
                    end: record_end,
                    declaration_start: node.start,
                    declaration_end: node.end,
                    declaration_text: slice(source, node.start, node.end).to_owned(),
                });
                output.code_spans.push((node.start, node.end));
                let mut tail_end = node.end;
                while index < all.len() && all[index].start < node.end {
                    let inner = &all[index];
                    if inner.start > node.start {
                        let inner_trimmed = inner.text.trim_start();
                        if let Some(rest) = inner_trimmed.strip_prefix(marker) {
                            let rest = rest.trim_start();
                            if rest.starts_with(MARKER_PREFIX) {
                                if !rest.starts_with(SUPPORTED_MARKER) {
                                    return Err(ExtractionError::UnsupportedSchema {
                                        start: inner.start,
                                        end: inner.end,
                                        marker: rest.to_owned(),
                                    });
                                }
                                output.diagnostics.push(RecordDiagnostic {
                                    code: UNATTACHED_METADATA,
                                    start: inner.start,
                                    end: inner.end,
                                    context: String::from("inside-declaration"),
                                });
                            }
                        }
                    }
                    tail_end = inner.end;
                    index += 1;
                }
                if tail_end > node.end {
                    output.code_spans.push((node.end, tail_end));
                }
            }
        }
    }
    Ok(output)
}
