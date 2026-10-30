//! Total saved-snapshot adapters for the Chelis Surf and Deep dialects.

use crate::map::{MappedValue, map_record};
use crate::records::{ChelisDialect, Extraction, extract};
use buoy::adapter_sdk::{
    AdapterDescriptor, AdapterProduct, AdapterSdkError, NonnormativeDisposition,
    SavedSnapshotInput, ShellAdapter,
};
use buoy::adapters::{AdapterDiagnostic, SourceSpan};
use buoy::core::{ObjectMeta, ObjectRef, ParsedItemIdentity, SchemaId, SourceItemKind, SpecGraph};

/// Configuration schema identity shared by both Chelis adapters.
pub const CONFIGURATION_SCHEMA_ID: &str = "chelis-adapter-config/v1";

/// One registered saved-snapshot Chelis adapter.
pub struct ChelisAdapter {
    dialect: ChelisDialect,
    descriptor: AdapterDescriptor,
}

impl ChelisAdapter {
    /// Constructs the pinned adapter for one Chelis dialect.
    ///
    /// # Errors
    ///
    /// Returns an SDK identity error when a frozen identity is rejected.
    pub fn new(dialect: ChelisDialect) -> Result<Self, AdapterSdkError> {
        let kind = SourceItemKind::new(dialect.source_item_kind())
            .map_err(|_| AdapterSdkError::Identity)?;
        let descriptor = AdapterDescriptor::new(
            dialect.adapter_id(),
            dialect.parser_id(),
            dialect.parser_version(),
            dialect.language_schema_id(),
            CONFIGURATION_SCHEMA_ID,
            vec![
                dialect.comment_capability().to_owned(),
                dialect.declaration_capability().to_owned(),
            ],
            vec![kind],
        )?;
        Ok(Self {
            dialect,
            descriptor,
        })
    }

    /// The adapter's dialect.
    #[must_use]
    pub const fn dialect(&self) -> ChelisDialect {
        self.dialect
    }
}

fn mapped_meta(value: &MappedValue) -> &ObjectMeta {
    match value {
        MappedValue::Atom(value) => value.meta(),
        MappedValue::Carrier(value) => value.meta(),
        MappedValue::Link(value) => value.meta(),
        MappedValue::Surface(value) => value.meta(),
        MappedValue::Policy(value) => value.meta(),
        MappedValue::Debt(value) => value.meta(),
        MappedValue::Waiver(value) => value.meta(),
        MappedValue::Reaffirmation(value) => value.meta(),
        MappedValue::FormalProperty(value) => value.meta(),
        MappedValue::Derivation(value) => value.meta(),
        MappedValue::Oracle(value) => value.meta(),
    }
}

fn push_value(graph: &mut SpecGraph, value: MappedValue) {
    match value {
        MappedValue::Atom(value) => graph.atoms.push(value),
        MappedValue::Carrier(value) => graph.evidence_carriers.push(value),
        MappedValue::Link(value) => graph.implementation_links.push(value),
        MappedValue::Surface(value) => graph.decision_surfaces.push(value),
        MappedValue::Policy(value) => graph.coverage_policies.push(value),
        MappedValue::Debt(value) => graph.debt_records.push(value),
        MappedValue::Waiver(value) => graph.waivers.push(value),
        MappedValue::Reaffirmation(value) => graph.reaffirmations.push(value),
        MappedValue::FormalProperty(value) => graph.formal_properties.push(value),
        MappedValue::Derivation(value) => graph.derivations.push(value),
        MappedValue::Oracle(value) => graph.oracles.push(value),
    }
}

#[allow(clippy::too_many_lines)]
fn materialize_extraction(
    dialect: ChelisDialect,
    input: &SavedSnapshotInput<'_>,
    extraction: &Extraction,
) -> Result<AdapterProduct, AdapterSdkError> {
    let language =
        SchemaId::new(dialect.language_schema_id()).map_err(|_| AdapterSdkError::Identity)?;
    let kind =
        SourceItemKind::new(dialect.source_item_kind()).map_err(|_| AdapterSdkError::Identity)?;
    let mut graph = SpecGraph::default();
    let mut sources = Vec::new();
    let mut diagnostics = Vec::new();
    let mut nonnormative = Vec::new();
    for record in &extraction.records {
        let span = SourceSpan {
            path: input.path().clone(),
            start: record.start,
            end: record.declaration_end,
        };
        // A compact binding carries only a content address. Its record lives
        // in the object store, which one saved-snapshot input cannot reach,
        // so the command layer resolves it against the whole snapshot. The
        // marker stays explicitly dispositioned here to keep the grammar
        // total over every selected byte. The disposition covers the marker
        // alone: extraction already classified the attached declaration as
        // code, and claiming its bytes here would disposition them twice.
        if dialect == ChelisDialect::Surf && record.kind == crate::records::BINDING_RECORD_KIND {
            nonnormative.push(NonnormativeDisposition::new(
                SourceSpan {
                    path: input.path().clone(),
                    start: record.start,
                    end: record.end,
                },
                dialect.declaration_capability(),
            )?);
            continue;
        }
        match map_record(record) {
            Ok(value) => {
                let meta = mapped_meta(&value);
                let object = ObjectRef::new(meta.kind(), meta.id().clone(), meta.digest());
                let item = ParsedItemIdentity::new(
                    language.clone(),
                    input.path().clone(),
                    kind.clone(),
                    record.declaration_text.as_bytes().to_vec(),
                    None,
                )
                .map_err(|_| AdapterSdkError::Materialization)?;
                sources.push(buoy::adapter_sdk::MaterializedSource::new(
                    object,
                    item,
                    span,
                    dialect.declaration_capability(),
                )?);
                push_value(&mut graph, value);
            }
            Err(error) => diagnostics.push(AdapterDiagnostic {
                code: crate::records::MALFORMED_RECORD,
                span,
                context: error.context,
            }),
        }
    }
    for diagnostic in &extraction.diagnostics {
        diagnostics.push(AdapterDiagnostic {
            code: diagnostic.code,
            span: SourceSpan {
                path: input.path().clone(),
                start: diagnostic.start,
                end: diagnostic.end,
            },
            context: diagnostic.context.clone(),
        });
    }
    for (start, end) in &extraction.comment_spans {
        nonnormative.push(NonnormativeDisposition::new(
            SourceSpan {
                path: input.path().clone(),
                start: *start,
                end: *end,
            },
            dialect.comment_capability(),
        )?);
    }
    for (start, end) in &extraction.code_spans {
        nonnormative.push(NonnormativeDisposition::new(
            SourceSpan {
                path: input.path().clone(),
                start: *start,
                end: *end,
            },
            dialect.declaration_capability(),
        )?);
    }
    if sources.is_empty() && diagnostics.is_empty() && nonnormative.is_empty() {
        nonnormative.push(NonnormativeDisposition::new(
            SourceSpan {
                path: input.path().clone(),
                start: 0,
                end: input.bytes().len() as u64,
            },
            dialect.declaration_capability(),
        )?);
    }
    AdapterProduct::new(graph, sources, nonnormative, diagnostics)
}

impl ShellAdapter for ChelisAdapter {
    fn descriptor(&self) -> &AdapterDescriptor {
        &self.descriptor
    }

    fn materialize(
        &self,
        input: &SavedSnapshotInput<'_>,
    ) -> Result<AdapterProduct, AdapterSdkError> {
        match extract(self.dialect, input.bytes()) {
            Ok(extraction) => materialize_extraction(self.dialect, input, &extraction),
            Err(error) => {
                let code = error.code();
                let (start, end) = match &error {
                    crate::records::ExtractionError::UnsupportedSchema { start, end, .. }
                    | crate::records::ExtractionError::ParseError { start, end, .. } => {
                        (*start, *end)
                    }
                };
                let context = match error {
                    crate::records::ExtractionError::UnsupportedSchema { marker, .. } => {
                        format!("unsupported-marker:{marker}")
                    }
                    crate::records::ExtractionError::ParseError { context, .. } => {
                        format!("parse-defect:{context}")
                    }
                };
                let diagnostics = vec![AdapterDiagnostic {
                    code,
                    span: SourceSpan {
                        path: input.path().clone(),
                        start,
                        end,
                    },
                    context,
                }];
                AdapterProduct::new(SpecGraph::default(), Vec::new(), Vec::new(), diagnostics)
            }
        }
    }
}
