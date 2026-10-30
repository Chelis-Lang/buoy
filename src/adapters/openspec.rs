//! `OpenSpec` standing-spec extraction from exact saved Markdown bytes.

use super::external_trace::{EXTERNAL_TRACE_FENCE, parse_external_trace_document};
use super::{AdapterDiagnostic, LocatedExternalTraceLink, SourceSpan};
use crate::traceability::{ExternalObjectId, ExternalObjectRef, ExternalTraceLink};
use buoy_core::{
    ActiveAlgorithm, ApprovalBinding, Atom, AtomKind, AtomRevisionRef, CanonicalValue,
    LifecycleState, ParsedItemIdentity, RepositoryPath, SchemaId, ScopeId, SemanticDigest,
    SourceAuthority, SourceChildRef, SourceEdge, SourceEdgeKind, SourceItemKind, SourceNode,
    StableId, semantic_digest,
};
use pulldown_cmark::{CodeBlockKind, Event, HeadingLevel, Parser, Tag, TagEnd};
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};

/// Versioned source-item kind for one accepted nonauthority trace section.
pub const OPENSPEC_EXTERNAL_TRACE_KIND: &str = "openspec/external-trace/v1";

/// Parsed authority metadata for all selected `OpenSpec` capabilities.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OpenSpecConfiguration {
    capabilities: BTreeMap<String, OpenSpecCapabilityMetadata>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum OpenSpecFormatVersion {
    StandingSpecV1,
    StandingSpecV2,
}

impl OpenSpecFormatVersion {
    fn parse(value: Option<&str>) -> Result<Self, &'static str> {
        match value {
            Some("openspec/standing-spec/v1") => Ok(Self::StandingSpecV1),
            Some("openspec/standing-spec/v2") => Ok(Self::StandingSpecV2),
            _ => Err("SPG-OPENSPEC-CONFIG-FORMAT"),
        }
    }

    /// Whether this format materializes nonauthority trace sections.
    ///
    /// Version 1 keeps every recognized fence nonnormative, so its accepted
    /// bytes, identities, and dispositions stay unchanged.
    const fn parses_trace_sections(self) -> bool {
        matches!(self, Self::StandingSpecV2)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct OpenSpecCapabilityMetadata {
    format: OpenSpecFormatVersion,
    scope: ScopeId,
    atom_kind: AtomKind,
    lifecycle: LifecycleState,
    approval: Option<ApprovalBinding>,
    reserved_ids: BTreeSet<StableId>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawOpenSpecConfiguration {
    schema: String,
    capabilities: Vec<RawOpenSpecCapability>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawOpenSpecCapability {
    name: String,
    format: Option<String>,
    scope: String,
    atom_kind: String,
    lifecycle: String,
    reserved_ids: Vec<String>,
    approval: Option<RawApproval>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawApproval {
    authority: String,
    record: String,
    digest: String,
}

impl OpenSpecConfiguration {
    /// Parses strict capability metadata at the adapter boundary.
    ///
    /// # Errors
    ///
    /// Returns a stable code for malformed, duplicate, or incomplete metadata.
    #[allow(clippy::needless_bitwise_bool)]
    pub fn parse(bytes: &[u8]) -> Result<Self, &'static str> {
        let source = core::str::from_utf8(bytes).map_err(|_| "SPG-OPENSPEC-CONFIG-UTF8")?;
        let raw = toml::from_str::<RawOpenSpecConfiguration>(source)
            .map_err(|_| "SPG-OPENSPEC-CONFIG-SYNTAX")?;
        if (raw.schema != "buoy.openspec-authority/v1") | raw.capabilities.is_empty() {
            return Err("SPG-OPENSPEC-CONFIG-SCHEMA");
        }
        let mut capabilities = BTreeMap::new();
        for capability in raw.capabilities {
            if capability.name.is_empty()
                | !capability
                    .name
                    .bytes()
                    .all(|byte| byte.is_ascii_lowercase() | byte.is_ascii_digit() | (byte == b'-'))
            {
                return Err("SPG-OPENSPEC-CONFIG-CAPABILITY");
            }
            let format = OpenSpecFormatVersion::parse(capability.format.as_deref())?;
            let atom_kind = match capability.atom_kind.as_str() {
                "behavioral" => AtomKind::Behavioral,
                "structural" => AtomKind::Structural,
                "constraint" => AtomKind::Constraint,
                _ => return Err("SPG-OPENSPEC-CONFIG-ATOM-KIND"),
            };
            let lifecycle = match capability.lifecycle.as_str() {
                "proposed" => LifecycleState::Proposed,
                "ratified" => LifecycleState::Ratified,
                _ => return Err("SPG-OPENSPEC-CONFIG-LIFECYCLE"),
            };
            let approval = capability
                .approval
                .map(|approval| {
                    ApprovalBinding::new(
                        ScopeId::new(&approval.authority)
                            .map_err(|_| "SPG-OPENSPEC-CONFIG-APPROVAL")?,
                        RepositoryPath::new(&approval.record)
                            .map_err(|_| "SPG-OPENSPEC-CONFIG-APPROVAL")?,
                        SemanticDigest::parse_canonical(&approval.digest)
                            .map_err(|_| "SPG-OPENSPEC-CONFIG-APPROVAL")?,
                    )
                    .map_err(|_| "SPG-OPENSPEC-CONFIG-APPROVAL")
                })
                .transpose()?;
            if (lifecycle == LifecycleState::Ratified) != approval.is_some() {
                return Err("SPG-OPENSPEC-CONFIG-APPROVAL");
            }
            let reserved_ids = capability
                .reserved_ids
                .iter()
                .map(|id| StableId::new(id).map_err(|_| "SPG-OPENSPEC-CONFIG-RESERVED-ID"))
                .collect::<Result<BTreeSet<_>, _>>()?;
            if (reserved_ids.len() != capability.reserved_ids.len()) | reserved_ids.is_empty() {
                return Err("SPG-OPENSPEC-CONFIG-RESERVED-ID");
            }
            let metadata = OpenSpecCapabilityMetadata {
                format,
                scope: ScopeId::new(&capability.scope).map_err(|_| "SPG-OPENSPEC-CONFIG-SCOPE")?,
                atom_kind,
                lifecycle,
                approval,
                reserved_ids,
            };
            if capabilities.insert(capability.name, metadata).is_some() {
                return Err("SPG-OPENSPEC-CONFIG-CAPABILITY");
            }
        }
        Ok(Self { capabilities })
    }
}

/// One `OpenSpec` semantic object from a standing specification.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OpenSpecObject {
    external: ExternalObjectRef,
    title: String,
    body: String,
    atom: Option<Atom>,
    digest: SemanticDigest,
}

impl OpenSpecObject {
    /// Returns the canonical external object reference.
    #[must_use]
    pub const fn external(&self) -> &ExternalObjectRef {
        &self.external
    }

    /// Returns the exact object title.
    #[must_use]
    pub fn title(&self) -> &str {
        &self.title
    }

    /// Returns the exact object body from the standing spec.
    #[must_use]
    pub fn body(&self) -> &str {
        &self.body
    }

    /// Returns the materialized requirement atom, if this object is a requirement.
    #[must_use]
    pub const fn atom(&self) -> Option<&Atom> {
        self.atom.as_ref()
    }

    /// Returns the exact semantic object digest.
    #[must_use]
    pub const fn digest(&self) -> SemanticDigest {
        self.digest
    }
}

/// One `OpenSpec` object with its parsed identity and byte span.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LocatedOpenSpecObject {
    /// Complete `OpenSpec` semantic object.
    pub value: OpenSpecObject,
    /// Complete parser-canonical item identity.
    pub item: ParsedItemIdentity,
    /// Exact original-byte source span.
    pub span: SourceSpan,
}

/// Total extraction result for one `OpenSpec` standing spec.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OpenSpecOutput {
    /// Complete requirement and scenario objects.
    pub semantic: Vec<LocatedOpenSpecObject>,
    /// Complete document, requirement, scenario, and step source nodes.
    pub source_nodes: Vec<LocatedOpenSpecNode>,
    /// Ordered containment edges for the complete source tree.
    pub source_edges: Vec<SourceEdge>,
    /// Exact requirement-node to atom-revision correspondences.
    pub authorities: Vec<SourceAuthority>,
    /// Exact current standing-spec document root.
    pub document_root: Option<buoy_core::SourceNodeRef>,
    /// Explicit nonnormative source spans.
    pub nonnormative: Vec<SourceSpan>,
    /// Accepted nonauthority trace links with exact secondary source mappings.
    ///
    /// Version 1 never populates this list. A trace mapping nests inside one
    /// nonnormative span and never changes the primary byte partition.
    pub external_links: Vec<LocatedExternalTraceLink>,
    /// Stable malformed-input diagnostics.
    pub diagnostics: Vec<AdapterDiagnostic>,
}

/// One retained `OpenSpec` source node with its parser-backed span.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LocatedOpenSpecNode {
    /// Complete content-addressed source node.
    pub node: SourceNode,
    /// Exact original-byte source span.
    pub span: SourceSpan,
}

#[derive(Clone, Debug)]
struct Heading {
    level: u8,
    title: String,
    start: usize,
    line_end: usize,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct ByteInterval {
    start: usize,
    end: usize,
}

#[derive(Clone, Debug)]
struct ScenarioSection {
    title: String,
    slug: String,
    span: ByteInterval,
    steps: Vec<StepSection>,
}

#[derive(Clone, Debug)]
struct StepSection {
    keyword: String,
    text: String,
    span: ByteInterval,
}

#[derive(Clone, Debug)]
struct RequirementSection {
    stable_id: StableId,
    title: String,
    span: ByteInterval,
    scenarios: Vec<ScenarioSection>,
}

/// One accepted nonauthority trace section and its complete link value.
#[derive(Clone, Debug)]
struct TraceSection {
    span: ByteInterval,
    body: String,
    value: ExternalTraceLink,
}

#[derive(Clone, Debug)]
struct OpenSpecDocument {
    requirements: Vec<RequirementSection>,
    nonnormative: Vec<ByteInterval>,
    traces: Vec<TraceSection>,
}

/// Extracts a standing spec with explicit authority metadata.
///
/// # Panics
///
/// Panics if the build contains no digest adapter.
#[must_use]
#[allow(clippy::too_many_lines)]
#[allow(clippy::expect_used)]
pub fn extract_openspec_configured(
    path: &RepositoryPath,
    bytes: &[u8],
    configuration: &OpenSpecConfiguration,
) -> OpenSpecOutput {
    let active = ActiveAlgorithm::build_default()
        .expect("an OpenSpec adapter build must contain one digest member");
    extract_openspec_configured_with(active, path, bytes, configuration)
}

/// Extracts a standing spec under the repository-selected digest member.
///
/// # Panics
///
/// Panics if validated parser output violates a fixed source-tree invariant.
#[must_use]
#[allow(
    clippy::cast_possible_truncation,
    clippy::expect_used,
    clippy::too_many_lines
)]
pub fn extract_openspec_configured_with(
    active: ActiveAlgorithm,
    path: &RepositoryPath,
    bytes: &[u8],
    configuration: &OpenSpecConfiguration,
) -> OpenSpecOutput {
    let Some(capability) = standing_spec_capability(path.as_str()) else {
        return diagnostic_output(
            path,
            bytes.len(),
            "SPG-OPENSPEC-STANDING-PATH",
            "standing-spec/v1",
        );
    };
    let Some(metadata) = configuration.capabilities.get(capability) else {
        return diagnostic_output(
            path,
            bytes.len(),
            "SPG-OPENSPEC-CONFIG-CAPABILITY",
            capability,
        );
    };
    let Ok(source) = core::str::from_utf8(bytes) else {
        return diagnostic_output(path, bytes.len(), "SPG-ADAPTER-INVALID-UTF8", "openspec/v1");
    };
    if source.as_bytes().contains(&b'\r') {
        return diagnostic_output(
            path,
            bytes.len(),
            "SPG-OPENSPEC-NONCANONICAL-NEWLINE",
            "openspec/v1",
        );
    }
    let document = match parse_document(path, source, metadata.format, capability) {
        Ok(document) => document,
        Err(diagnostics) => {
            return OpenSpecOutput {
                semantic: Vec::new(),
                source_nodes: Vec::new(),
                source_edges: Vec::new(),
                authorities: Vec::new(),
                document_root: None,
                nonnormative: Vec::new(),
                external_links: Vec::new(),
                diagnostics,
            };
        }
    };

    let mut diagnostics = Vec::new();
    let mut atoms = Vec::with_capacity(document.requirements.len());
    for requirement in &document.requirements {
        let requirement_id = format!(
            "{capability}/{}",
            requirement.stable_id.as_str().to_ascii_lowercase()
        );
        if !metadata.reserved_ids.contains(&requirement.stable_id) {
            diagnostics.push(diagnostic(
                path,
                requirement.span.start,
                requirement.span.end,
                "SPG-OPENSPEC-REQUIREMENT-UNRESERVED",
                requirement.stable_id.as_str(),
            ));
            continue;
        }
        let body = &source[requirement.span.start..requirement.span.end];
        let Some(atom) = make_atom(active, requirement.stable_id.clone(), body, metadata) else {
            diagnostics.push(diagnostic(
                path,
                requirement.span.start,
                requirement.span.end,
                "SPG-OPENSPEC-ATOM-CONSTRUCTION",
                &requirement_id,
            ));
            continue;
        };
        atoms.push(atom);
    }
    if !diagnostics.is_empty() {
        sort_diagnostics(&mut diagnostics);
        return OpenSpecOutput {
            semantic: Vec::new(),
            source_nodes: Vec::new(),
            source_edges: Vec::new(),
            authorities: Vec::new(),
            document_root: None,
            nonnormative: Vec::new(),
            external_links: Vec::new(),
            diagnostics,
        };
    }

    let document_item = make_item(
        active,
        path,
        "openspec/document/v1",
        capability.as_bytes().to_vec(),
        None,
    );
    let mut semantic = Vec::new();
    let mut source_nodes = Vec::new();
    let mut source_edges = Vec::new();
    let mut authorities = Vec::new();
    let mut requirement_children = Vec::new();
    for (requirement_position, (requirement, atom)) in
        document.requirements.iter().zip(atoms).enumerate()
    {
        let requirement_id = format!(
            "{capability}/{}",
            requirement.stable_id.as_str().to_ascii_lowercase()
        );
        let requirement_body = &source[requirement.span.start..requirement.span.end];
        let requirement_item = make_item(
            active,
            path,
            "openspec/requirement/v1",
            requirement_body.as_bytes().to_vec(),
            Some(document_item.digest()),
        );
        let mut scenario_children = Vec::new();
        for (scenario_position, scenario) in requirement.scenarios.iter().enumerate() {
            let scenario_id = format!("{requirement_id}/scenario/{}", scenario.slug);
            let scenario_body = &source[scenario.span.start..scenario.span.end];
            let scenario_item = make_item(
                active,
                path,
                "openspec/scenario/v1",
                scenario_body.as_bytes().to_vec(),
                Some(requirement_item.digest()),
            );
            let mut step_children = Vec::new();
            for (step_position, step) in scenario.steps.iter().enumerate() {
                let step_item = make_item(
                    active,
                    path,
                    "openspec/step/v1",
                    format!("{step_position}\n{}\n{}", step.keyword, step.text).into_bytes(),
                    Some(scenario_item.digest()),
                );
                let step_node = SourceNode::new_with(active, step_item, Vec::new())
                    .expect("parsed OpenSpec step nodes have no children");
                let step_reference = step_node.reference();
                step_children.push(SourceChildRef::new(
                    SourceEdgeKind::Contains,
                    step_position as u32,
                    step_reference,
                ));
                source_nodes.push(LocatedOpenSpecNode {
                    node: step_node,
                    span: source_span(path, step.span.start, step.span.end),
                });
            }
            let scenario_node = SourceNode::new_with(active, scenario_item.clone(), step_children)
                .expect("source-order OpenSpec steps have contiguous positions");
            let scenario_reference = scenario_node.reference();
            for child in scenario_node.children() {
                source_edges.push(
                    SourceEdge::new(
                        scenario_reference,
                        SourceEdgeKind::Contains,
                        Some(child.position()),
                        child.target(),
                    )
                    .expect("one OpenSpec containment edge has one source position"),
                );
            }
            scenario_children.push(SourceChildRef::new(
                SourceEdgeKind::Contains,
                scenario_position as u32,
                scenario_reference,
            ));
            source_nodes.push(LocatedOpenSpecNode {
                node: scenario_node,
                span: source_span(path, scenario.span.start, scenario.span.end),
            });
            semantic.push(LocatedOpenSpecObject {
                value: make_object(
                    "openspec/scenario/v1",
                    &scenario_id,
                    &scenario.title,
                    scenario_body,
                    None,
                ),
                item: scenario_item,
                span: source_span(path, scenario.span.start, scenario.span.end),
            });
        }

        let requirement_node =
            SourceNode::new_with(active, requirement_item.clone(), scenario_children)
                .expect("source-order OpenSpec scenarios have contiguous positions");
        let requirement_reference = requirement_node.reference();
        for child in requirement_node.children() {
            source_edges.push(
                SourceEdge::new(
                    requirement_reference,
                    SourceEdgeKind::Contains,
                    Some(child.position()),
                    child.target(),
                )
                .expect("one OpenSpec containment edge has one source position"),
            );
        }
        authorities.push(SourceAuthority::new(
            requirement_reference,
            AtomRevisionRef::new(atom.meta().id().clone(), atom.revision()),
        ));
        requirement_children.push(SourceChildRef::new(
            SourceEdgeKind::Contains,
            requirement_position as u32,
            requirement_reference,
        ));
        source_nodes.push(LocatedOpenSpecNode {
            node: requirement_node,
            span: source_span(path, requirement.span.start, requirement.span.end),
        });
        semantic.push(LocatedOpenSpecObject {
            value: make_object(
                "openspec/requirement/v1",
                &requirement_id,
                &requirement.title,
                requirement_body,
                Some(atom),
            ),
            item: requirement_item,
            span: source_span(path, requirement.span.start, requirement.span.end),
        });
    }

    let document_node = SourceNode::new_with(active, document_item, requirement_children)
        .expect("source-order OpenSpec requirements have contiguous positions");
    let document_root = document_node.reference();
    for child in document_node.children() {
        source_edges.push(
            SourceEdge::new(
                document_root,
                SourceEdgeKind::Contains,
                Some(child.position()),
                child.target(),
            )
            .expect("one OpenSpec containment edge has one source position"),
        );
    }
    source_nodes.push(LocatedOpenSpecNode {
        node: document_node,
        span: source_span(path, 0, source.len()),
    });

    // Each accepted trace section keeps its own parser-backed item identity and
    // exact secondary source mapping. The link value came from the one shared
    // external trace parser, so an equal Markdown fence produces an equal value.
    let mut external_links = document
        .traces
        .into_iter()
        .map(|trace| LocatedExternalTraceLink {
            value: trace.value,
            item: make_item(
                active,
                path,
                OPENSPEC_EXTERNAL_TRACE_KIND,
                trace.body.into_bytes(),
                None,
            ),
            span: source_span(path, trace.span.start, trace.span.end),
        })
        .collect::<Vec<_>>();

    semantic.sort_by_key(|located| located.item.digest());
    source_nodes.sort_by_key(|located| located.node.digest());
    source_edges.sort_unstable();
    authorities.sort_unstable();
    external_links.sort_by_key(|located| located.item.digest());
    let mut link_ids = BTreeSet::new();
    let mut link_digests = BTreeSet::new();
    if let Some(duplicate) = external_links.iter().find(|located| {
        !link_ids.insert(located.value.meta().id().clone())
            || !link_digests.insert(located.value.meta().digest())
    }) {
        return OpenSpecOutput {
            semantic: Vec::new(),
            source_nodes: Vec::new(),
            source_edges: Vec::new(),
            authorities: Vec::new(),
            document_root: None,
            nonnormative: Vec::new(),
            external_links: Vec::new(),
            diagnostics: vec![AdapterDiagnostic {
                code: "SPG-EXTERNAL-TRACE-DUPLICATE",
                span: source_span(path, 0, source.len()),
                context: duplicate.value.meta().id().as_str().to_owned(),
            }],
        };
    }
    OpenSpecOutput {
        semantic,
        source_nodes,
        source_edges,
        authorities,
        document_root: Some(document_root),
        nonnormative: document
            .nonnormative
            .into_iter()
            .map(|span| source_span(path, span.start, span.end))
            .collect(),
        external_links,
        diagnostics: Vec::new(),
    }
}

#[allow(clippy::too_many_lines)]
fn parse_document(
    path: &RepositoryPath,
    source: &str,
    format: OpenSpecFormatVersion,
    capability: &str,
) -> Result<OpenSpecDocument, Vec<AdapterDiagnostic>> {
    // Both formats share every authority rule. Only the later trace-section
    // pass differs, and that pass runs after requirement boundaries exist.
    let headings = headings(source);
    let authority_headings = headings
        .iter()
        .filter(|heading| heading.level == 2 && heading.title == "Requirements")
        .collect::<Vec<_>>();
    let Some(requirements_heading) = authority_headings.first().copied() else {
        return Err(vec![diagnostic(
            path,
            0,
            source.len(),
            "SPG-OPENSPEC-REQUIREMENTS-MISSING",
            capability,
        )]);
    };
    if authority_headings.len() != 1 {
        let mut diagnostics = authority_headings
            .iter()
            .skip(1)
            .map(|heading| {
                diagnostic(
                    path,
                    heading.start,
                    heading.line_end,
                    "SPG-OPENSPEC-REQUIREMENTS-DUPLICATE",
                    "Requirements",
                )
            })
            .collect::<Vec<_>>();
        sort_diagnostics(&mut diagnostics);
        return Err(diagnostics);
    }
    let authority_end = headings
        .iter()
        .find(|heading| heading.start > requirements_heading.start && heading.level <= 2)
        .map_or(source.len(), |heading| heading.start);
    let section_headings = headings
        .iter()
        .filter(|heading| {
            heading.start > requirements_heading.start && heading.start < authority_end
        })
        .collect::<Vec<_>>();
    let requirement_headings = section_headings
        .iter()
        .copied()
        .filter(|heading| heading.level == 3 && heading.title.starts_with("Requirement: "))
        .collect::<Vec<_>>();
    if requirement_headings.is_empty() {
        return Err(vec![diagnostic(
            path,
            requirements_heading.start,
            authority_end,
            "SPG-OPENSPEC-REQUIREMENT-EMPTY",
            capability,
        )]);
    }

    let mut diagnostics = Vec::new();
    for heading in &section_headings {
        let authority_like =
            heading.title.starts_with("Requirement:") || heading.title.starts_with("Scenario:");
        if heading.level == 3 && !heading.title.starts_with("Requirement: ") {
            let code = if heading.title.starts_with("Scenario: ") {
                "SPG-OPENSPEC-SCENARIO-HEADING"
            } else {
                "SPG-OPENSPEC-UNSUPPORTED-SYNTAX"
            };
            diagnostics.push(diagnostic(
                path,
                heading.start,
                heading.line_end,
                code,
                &heading.title,
            ));
        } else if authority_like
            && !(heading.level == 3
                || (heading.level == 4 && heading.title.starts_with("Scenario: ")))
        {
            diagnostics.push(diagnostic(
                path,
                heading.start,
                heading.line_end,
                "SPG-OPENSPEC-UNSUPPORTED-SYNTAX",
                &heading.title,
            ));
        }
    }

    let mut requirements = Vec::new();
    let mut object_ids = BTreeSet::new();
    for requirement_heading in requirement_headings {
        let requirement_end = section_headings
            .iter()
            .copied()
            .find(|heading| heading.start > requirement_heading.start && heading.level <= 3)
            .map_or(authority_end, |heading| heading.start);
        let heading_value = requirement_heading
            .title
            .strip_prefix("Requirement: ")
            .unwrap_or_default();
        let Some(stable_id) = parse_requirement_heading(heading_value) else {
            let code = if heading_value.contains('[') && heading_value.ends_with(']') {
                "SPG-OPENSPEC-UNSUPPORTED-SYNTAX"
            } else {
                "SPG-OPENSPEC-REQUIREMENT-ID"
            };
            diagnostics.push(diagnostic(
                path,
                requirement_heading.start,
                requirement_heading.line_end,
                code,
                heading_value,
            ));
            continue;
        };
        let requirement_id = format!("{capability}/{}", stable_id.as_str().to_ascii_lowercase());
        if !object_ids.insert(requirement_id.clone()) {
            diagnostics.push(diagnostic(
                path,
                requirement_heading.start,
                requirement_heading.line_end,
                "SPG-OPENSPEC-DUPLICATE-OBJECT",
                &requirement_id,
            ));
            continue;
        }
        let mut scenarios = Vec::new();
        for scenario_heading in section_headings.iter().copied().filter(|heading| {
            heading.level == 4
                && heading.title.starts_with("Scenario: ")
                && heading.start > requirement_heading.start
                && heading.start < requirement_end
        }) {
            let scenario_title = scenario_heading
                .title
                .strip_prefix("Scenario: ")
                .unwrap_or_default();
            let Some(scenario_slug) = slug(scenario_title) else {
                diagnostics.push(diagnostic(
                    path,
                    scenario_heading.start,
                    scenario_heading.line_end,
                    "SPG-OPENSPEC-SCENARIO-ID",
                    scenario_title,
                ));
                continue;
            };
            let scenario_id = format!("{requirement_id}/scenario/{scenario_slug}");
            if !object_ids.insert(scenario_id.clone()) {
                diagnostics.push(diagnostic(
                    path,
                    scenario_heading.start,
                    scenario_heading.line_end,
                    "SPG-OPENSPEC-DUPLICATE-OBJECT",
                    &scenario_id,
                ));
                continue;
            }
            let scenario_end = section_headings
                .iter()
                .copied()
                .find(|heading| {
                    heading.start > scenario_heading.start
                        && heading.start < requirement_end
                        && heading.level <= 4
                })
                .map_or(requirement_end, |heading| heading.start);
            let span = ByteInterval {
                start: scenario_heading.start,
                end: scenario_end,
            };
            scenarios.push(ScenarioSection {
                title: scenario_title.to_owned(),
                slug: scenario_slug,
                span,
                steps: scenario_steps(source, span),
            });
        }
        requirements.push(RequirementSection {
            stable_id,
            title: heading_value.to_owned(),
            span: ByteInterval {
                start: requirement_heading.start,
                end: requirement_end,
            },
            scenarios,
        });
    }
    for heading in section_headings.iter().copied().filter(|heading| {
        heading.level == 4
            && heading.title.starts_with("Scenario: ")
            && !requirements.iter().any(|requirement| {
                heading.start > requirement.span.start && heading.start < requirement.span.end
            })
    }) {
        diagnostics.push(diagnostic(
            path,
            heading.start,
            heading.line_end,
            "SPG-OPENSPEC-UNSUPPORTED-SYNTAX",
            &heading.title,
        ));
    }
    let traces = collect_trace_sections(
        &TracePass {
            path,
            source,
            format,
            requirements: &requirements,
            headings: &headings,
            authority_end,
            capability,
        },
        &mut diagnostics,
    );

    if !diagnostics.is_empty() {
        sort_diagnostics(&mut diagnostics);
        return Err(diagnostics);
    }

    requirements.sort_by_key(|requirement| requirement.span);
    let nonnormative = complement_intervals(
        source.len(),
        &requirements
            .iter()
            .map(|requirement| requirement.span)
            .collect::<Vec<_>>(),
    );
    finish_document(
        path,
        source.len(),
        capability,
        requirements,
        nonnormative,
        traces,
    )
}

/// One recognized external trace fence and its exact original-byte extent.
#[derive(Clone, Debug)]
struct TraceFence {
    start: usize,
    end: usize,
    body: String,
}

/// Collects every recognized trace fence with its exact source range.
fn trace_fences(source: &str) -> Vec<TraceFence> {
    let mut fences = Vec::new();
    let mut active: Option<(String, usize, String)> = None;
    for (event, range) in Parser::new(source).into_offset_iter() {
        match event {
            Event::Start(Tag::CodeBlock(CodeBlockKind::Fenced(info))) => {
                active = Some((info.into_string(), range.start, String::new()));
            }
            Event::Text(text) | Event::Code(text) => {
                if let Some((_, _, body)) = &mut active {
                    body.push_str(&text);
                }
            }
            Event::End(TagEnd::CodeBlock) => {
                let (info, start, body) = active.take().unwrap_or_default();
                if info == EXTERNAL_TRACE_FENCE {
                    fences.push(TraceFence {
                        start,
                        end: range.end,
                        body: body.replace("\r\n", "\n"),
                    });
                }
            }
            _ => {}
        }
    }
    fences
}

/// Complete inputs for one version 2 trace-section pass.
///
/// Requirement and scenario boundaries already exist, so this pass reads them
/// and never changes one authority identity.
#[derive(Clone, Copy)]
struct TracePass<'a> {
    path: &'a RepositoryPath,
    source: &'a str,
    format: OpenSpecFormatVersion,
    requirements: &'a [RequirementSection],
    headings: &'a [Heading],
    authority_end: usize,
    capability: &'a str,
}

/// Accepts nonauthority trace sections and rejects every authority-span fence.
///
/// Requirement boundaries exist before this function runs, so trace bytes can
/// never enter one requirement source-node identity or atom revision.
///
/// Version 1 collects nothing, so the same later fence stays nonnormative.
fn collect_trace_sections(
    pass: &TracePass<'_>,
    diagnostics: &mut Vec<AdapterDiagnostic>,
) -> Vec<TraceSection> {
    let TracePass {
        path,
        source,
        format,
        requirements,
        headings,
        authority_end,
        capability,
    } = *pass;
    if !format.parses_trace_sections() {
        return Vec::new();
    }
    let mut traces = Vec::new();
    for fence in trace_fences(source) {
        let intersects_authority = requirements.iter().any(|requirement| {
            overlaps(fence.start, fence.end, requirement.span)
                || requirement
                    .scenarios
                    .iter()
                    .any(|scenario| overlaps(fence.start, fence.end, scenario.span))
        });
        if intersects_authority {
            diagnostics.push(diagnostic(
                path,
                fence.start,
                fence.end,
                "SPG-OPENSPEC-TRACE-IN-AUTHORITY",
                capability,
            ));
            continue;
        }
        let host = headings
            .iter()
            .rev()
            .find(|heading| heading.start < fence.start && heading.level <= 2);
        let in_later_level_two_section = host.is_some_and(|heading| {
            heading.level == 2 && heading.title != "Requirements" && heading.start >= authority_end
        });
        if fence.start < authority_end || !in_later_level_two_section {
            diagnostics.push(diagnostic(
                path,
                fence.start,
                fence.end,
                "SPG-OPENSPEC-TRACE-SECTION",
                capability,
            ));
            continue;
        }
        match parse_external_trace_document(&fence.body) {
            Ok(value) => traces.push(TraceSection {
                span: ByteInterval {
                    start: fence.start,
                    end: fence.end,
                },
                body: fence.body,
                value,
            }),
            Err(code) => diagnostics.push(diagnostic(
                path,
                fence.start,
                fence.end,
                code,
                EXTERNAL_TRACE_FENCE,
            )),
        }
    }
    traces
}

const fn overlaps(start: usize, end: usize, span: ByteInterval) -> bool {
    start < span.end && span.start < end
}

fn finish_document(
    path: &RepositoryPath,
    length: usize,
    capability: &str,
    requirements: Vec<RequirementSection>,
    nonnormative: Vec<ByteInterval>,
    traces: Vec<TraceSection>,
) -> Result<OpenSpecDocument, Vec<AdapterDiagnostic>> {
    if validate_partition(length, &requirements, &nonnormative).is_err()
        || validate_trace_nesting(&traces, &nonnormative).is_err()
    {
        return Err(vec![diagnostic(
            path,
            0,
            length,
            "SPG-OPENSPEC-PARTITION",
            capability,
        )]);
    }
    Ok(OpenSpecDocument {
        requirements,
        nonnormative,
        traces,
    })
}

/// Requires each secondary trace mapping to nest inside one nonnormative span.
///
/// A trace mapping never replaces, splits, or overlaps one primary partition
/// entry, so the primary requirement and nonnormative spans still partition
/// every selected byte.
fn validate_trace_nesting(
    traces: &[TraceSection],
    nonnormative: &[ByteInterval],
) -> Result<(), ()> {
    for trace in traces {
        if trace.span.start >= trace.span.end {
            return Err(());
        }
        let hosts = nonnormative
            .iter()
            .filter(|span| span.start <= trace.span.start && trace.span.end <= span.end)
            .count();
        if hosts != 1 {
            return Err(());
        }
    }
    Ok(())
}

fn scenario_steps(source: &str, scenario: ByteInterval) -> Vec<StepSection> {
    let mut steps = Vec::new();
    let mut cursor = scenario.start;
    for line in source[scenario.start..scenario.end].split_inclusive('\n') {
        let line_end = cursor + line.trim_end_matches('\n').len();
        if let Some(content) = line.strip_prefix("- **")
            && let Some((keyword, text)) = content.trim_end_matches('\n').split_once("** ")
            && !keyword.is_empty()
            && !text.is_empty()
        {
            steps.push(StepSection {
                keyword: keyword.to_owned(),
                text: text.to_owned(),
                span: ByteInterval {
                    start: cursor,
                    end: line_end,
                },
            });
        }
        cursor += line.len();
    }
    steps
}

fn complement_intervals(length: usize, requirements: &[ByteInterval]) -> Vec<ByteInterval> {
    let mut result = Vec::new();
    let mut cursor = 0;
    for requirement in requirements {
        if cursor < requirement.start {
            result.push(ByteInterval {
                start: cursor,
                end: requirement.start,
            });
        }
        cursor = requirement.end;
    }
    if cursor < length {
        result.push(ByteInterval {
            start: cursor,
            end: length,
        });
    }
    result
}

fn validate_partition(
    length: usize,
    requirements: &[RequirementSection],
    nonnormative: &[ByteInterval],
) -> Result<(), ()> {
    let mut primary = requirements
        .iter()
        .map(|requirement| requirement.span)
        .chain(nonnormative.iter().copied())
        .collect::<Vec<_>>();
    primary.sort_unstable();
    if primary.is_empty()
        || primary[0].start != 0
        || primary.last().is_none_or(|span| span.end != length)
        || primary
            .iter()
            .any(|span| span.start >= span.end || span.end > length)
        || primary.windows(2).any(|pair| pair[0].end != pair[1].start)
    {
        return Err(());
    }
    for requirement in requirements {
        if requirement.scenarios.iter().any(|scenario| {
            scenario.span.start < requirement.span.start
                || scenario.span.end > requirement.span.end
                || scenario.span.start >= scenario.span.end
        }) {
            return Err(());
        }
    }
    Ok(())
}

/// Runs malformed parser-state controls through the production validators.
#[cfg(feature = "coverage-controls")]
#[doc(hidden)]
#[must_use]
#[allow(
    clippy::expect_used,
    clippy::needless_bitwise_bool,
    clippy::missing_panics_doc,
    clippy::too_many_lines
)]
pub fn openspec_partition_control() -> bool {
    let path = RepositoryPath::new("openspec/specs/demo/spec.md").expect("control path");
    let stable_id = StableId::new("DEMO-001").expect("control ID");
    let requirement = |scenarios: Vec<ByteInterval>| RequirementSection {
        stable_id: stable_id.clone(),
        title: String::from("DEMO-001 Control"),
        span: ByteInterval { start: 2, end: 8 },
        scenarios: scenarios
            .into_iter()
            .map(|span| ScenarioSection {
                title: String::from("Control"),
                slug: String::from("control"),
                span,
                steps: Vec::new(),
            })
            .collect(),
    };
    let base = requirement(Vec::new());
    let empty_capability_failed = standing_spec_capability("openspec/specs//spec.md").is_none();
    let invalid_capability_failed =
        standing_spec_capability("openspec/specs/Bad/spec.md").is_none();
    let missing_title_failed = parse_requirement_heading("DEMO-001").is_none();
    let empty_title_failed = parse_requirement_heading("DEMO-001 ").is_none();
    let finish_failed = finish_document(
        &path,
        10,
        "demo",
        vec![base.clone()],
        vec![
            ByteInterval { start: 0, end: 1 },
            ByteInterval { start: 8, end: 10 },
        ],
        Vec::new(),
    )
    .is_err();
    let empty_failed = validate_partition(0, &[], &[]).is_err();
    let start_failed = validate_partition(10, &[], &[ByteInterval { start: 2, end: 10 }]).is_err();
    let end_failed = validate_partition(10, &[], &[ByteInterval { start: 0, end: 9 }]).is_err();
    let empty_span_failed = validate_partition(
        10,
        &[],
        &[
            ByteInterval { start: 0, end: 0 },
            ByteInterval { start: 0, end: 10 },
        ],
    )
    .is_err();
    let long_span_failed = validate_partition(
        10,
        &[],
        &[
            ByteInterval { start: 0, end: 11 },
            ByteInterval { start: 1, end: 10 },
        ],
    )
    .is_err();
    let gap_failed = validate_partition(
        10,
        &[base],
        &[
            ByteInterval { start: 0, end: 1 },
            ByteInterval { start: 8, end: 10 },
        ],
    )
    .is_err();
    let before_failed = validate_partition(
        10,
        &[requirement(vec![ByteInterval { start: 1, end: 4 }])],
        &[
            ByteInterval { start: 0, end: 2 },
            ByteInterval { start: 8, end: 10 },
        ],
    )
    .is_err();
    let after_failed = validate_partition(
        10,
        &[requirement(vec![ByteInterval { start: 3, end: 9 }])],
        &[
            ByteInterval { start: 0, end: 2 },
            ByteInterval { start: 8, end: 10 },
        ],
    )
    .is_err();
    let scenario_empty_failed = validate_partition(
        10,
        &[requirement(vec![ByteInterval { start: 3, end: 3 }])],
        &[
            ByteInterval { start: 0, end: 2 },
            ByteInterval { start: 8, end: 10 },
        ],
    )
    .is_err();
    empty_capability_failed
        & invalid_capability_failed
        & missing_title_failed
        & empty_title_failed
        & finish_failed
        & empty_failed
        & start_failed
        & end_failed
        & empty_span_failed
        & long_span_failed
        & gap_failed
        & before_failed
        & after_failed
        & scenario_empty_failed
}

fn sort_diagnostics(diagnostics: &mut [AdapterDiagnostic]) {
    diagnostics.sort_by_key(|diagnostic| {
        (
            diagnostic.span.start,
            diagnostic.span.end,
            diagnostic.code,
            diagnostic.context.clone(),
        )
    });
}

#[allow(clippy::needless_bitwise_bool)]
fn standing_spec_capability(path: &str) -> Option<&str> {
    let parts = path.split('/').collect::<Vec<_>>();
    match parts.as_slice() {
        [.., "openspec", "specs", capability, "spec.md"]
            if !capability.is_empty()
                && capability.bytes().all(|byte| {
                    byte.is_ascii_lowercase() | byte.is_ascii_digit() | (byte == b'-')
                }) =>
        {
            Some(capability)
        }
        _ => None,
    }
}

fn headings(source: &str) -> Vec<Heading> {
    let mut result = Vec::new();
    let mut active: Option<(u8, usize, String)> = None;
    for (event, range) in Parser::new(source).into_offset_iter() {
        match event {
            Event::Start(Tag::Heading { level, .. }) => {
                active = Some((heading_level(level), range.start, String::new()));
            }
            Event::Text(text) | Event::Code(text) => {
                if let Some((_, _, title)) = &mut active {
                    title.push_str(&text);
                }
            }
            Event::End(TagEnd::Heading(_)) => {
                #[allow(clippy::expect_used)]
                let (level, start, title) = active
                    .take()
                    .expect("a Markdown heading end must follow its start");
                result.push(Heading {
                    level,
                    title,
                    start,
                    line_end: range.end,
                });
            }
            _ => {}
        }
    }
    result
}

const fn heading_level(level: HeadingLevel) -> u8 {
    match level {
        HeadingLevel::H1 => 1,
        HeadingLevel::H2 => 2,
        HeadingLevel::H3 => 3,
        HeadingLevel::H4 => 4,
        HeadingLevel::H5 => 5,
        HeadingLevel::H6 => 6,
    }
}

#[allow(clippy::needless_bitwise_bool)]
fn requirement_key(title: &str) -> Option<String> {
    let key = title.split_whitespace().next()?;
    let valid = key.len() >= 3
        && key.contains('-')
        && key
            .bytes()
            .all(|byte| byte.is_ascii_uppercase() | byte.is_ascii_digit() | (byte == b'-'));
    valid.then(|| key.to_owned())
}

fn parse_requirement_heading(value: &str) -> Option<StableId> {
    let (key, title) = value.split_once(char::is_whitespace)?;
    if title.trim().is_empty() || requirement_key(key).as_deref() != Some(key) {
        return None;
    }
    StableId::new(key).ok()
}

fn slug(title: &str) -> Option<String> {
    let mut result = String::new();
    let mut separator = false;
    for byte in title.bytes() {
        if byte.is_ascii_alphanumeric() {
            if separator && !result.is_empty() {
                result.push('-');
            }
            result.push(char::from(byte.to_ascii_lowercase()));
            separator = false;
        } else {
            separator = true;
        }
    }
    (!result.is_empty()).then_some(result)
}

fn make_atom(
    active: ActiveAlgorithm,
    id: StableId,
    body: &str,
    metadata: &OpenSpecCapabilityMetadata,
) -> Option<Atom> {
    let statement = body
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join("\n");
    Atom::new_with(
        active,
        id,
        metadata.lifecycle,
        metadata.atom_kind,
        vec![metadata.scope.clone()],
        statement,
        metadata.approval.clone(),
    )
    .ok()
}

#[allow(clippy::expect_used)]
fn make_object(
    kind: &str,
    id: &str,
    title: &str,
    body: &str,
    atom: Option<Atom>,
) -> OpenSpecObject {
    let source_kind = SourceItemKind::new(kind).expect("the fixed object kind must be valid");
    let external_id = ExternalObjectId::new(id).expect("a validated object ID must be valid");
    let revision = semantic_digest(
        "openspec-standing-object",
        vec![
            (1, CanonicalValue::Text(kind.to_owned())),
            (2, CanonicalValue::Text(id.to_owned())),
            (3, CanonicalValue::Text(title.to_owned())),
            (4, CanonicalValue::Text(body.to_owned())),
        ],
    )
    .expect("validated object fields must encode");
    let external = ExternalObjectRef::new(
        ScopeId::new("openspec").expect("the fixed OpenSpec scope must be valid"),
        source_kind,
        external_id,
        revision,
    )
    .expect("validated object fields must construct an external reference");
    OpenSpecObject {
        external,
        title: title.to_owned(),
        body: body.to_owned(),
        atom,
        digest: revision,
    }
}

#[allow(clippy::expect_used)]
fn make_item(
    active: ActiveAlgorithm,
    path: &RepositoryPath,
    kind: &str,
    semantic_bytes: Vec<u8>,
    enclosing_item: Option<SemanticDigest>,
) -> ParsedItemIdentity {
    ParsedItemIdentity::new_with(
        active,
        SchemaId::new("openspec/standing-spec/v1").expect("the fixed schema must be valid"),
        path.clone(),
        SourceItemKind::new(kind).expect("the fixed item kind must be valid"),
        semantic_bytes,
        enclosing_item,
    )
    .expect("validated OpenSpec item fields must construct")
}

fn source_span(path: &RepositoryPath, start: usize, end: usize) -> SourceSpan {
    SourceSpan {
        path: path.clone(),
        start: start as u64,
        end: end as u64,
    }
}

fn diagnostic(
    path: &RepositoryPath,
    start: usize,
    end: usize,
    code: &'static str,
    context: &str,
) -> AdapterDiagnostic {
    AdapterDiagnostic {
        code,
        span: source_span(path, start, end),
        context: context.to_owned(),
    }
}

fn diagnostic_output(
    path: &RepositoryPath,
    length: usize,
    code: &'static str,
    context: &str,
) -> OpenSpecOutput {
    OpenSpecOutput {
        semantic: Vec::new(),
        source_nodes: Vec::new(),
        source_edges: Vec::new(),
        authorities: Vec::new(),
        document_root: None,
        nonnormative: Vec::new(),
        external_links: Vec::new(),
        diagnostics: vec![diagnostic(path, 0, length, code, context)],
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]
    use super::*;

    fn extract_openspec(path: &RepositoryPath, bytes: &[u8]) -> OpenSpecOutput {
        let configuration = OpenSpecConfiguration::parse(
            configuration(
                "demo",
                "openspec.demo",
                "behavioral",
                "proposed",
                "[\"DEMO-001\", \"DEMO-002\"]",
                "",
            )
            .as_bytes(),
        )
        .expect("explicit test configuration");
        extract_openspec_configured(path, bytes, &configuration)
    }

    #[test]
    #[allow(clippy::expect_used)]
    fn standing_spec_materializes_requirements_and_scenarios() {
        let path = RepositoryPath::new("openspec/specs/demo/spec.md").expect("path");
        let source = b"# Demo\n\n## Purpose\nA demo.\n\n## Requirements\n\n### Requirement: DEMO-001 Typed kinds\nThe adapter MUST emit a typed kind.\n\n#### Scenario: Positive - valid kind\n- **WHEN** extraction runs\n- **THEN** one object exists\n";
        let output = extract_openspec(&path, source);
        assert!(output.diagnostics.is_empty());
        assert_eq!(output.semantic.len(), 2);
        assert_eq!(
            output.semantic[0].item.language_schema_id(),
            "openspec/standing-spec/v1"
        );
        assert!(
            output
                .semantic
                .iter()
                .any(|located| { located.item.kind().as_str() == "openspec/requirement/v1" })
        );
        assert!(
            output
                .semantic
                .iter()
                .any(|located| { located.item.kind().as_str() == "openspec/scenario/v1" })
        );
    }

    #[test]
    fn trace_mapping_partition_control_rejects_cross_boundary_spans() {
        let value = parse_external_trace_document(
            "id = \"TRACE-PARTITION\"\n\
             atoms = [\"DEMO-001@xxh3-128:11111111111111111111111111111111\"]\n\
             relation = \"documents\"\n\
             target = \"https://example.com/trace\"\n",
        )
        .expect("trace value");
        let nonnormative = [
            ByteInterval { start: 0, end: 5 },
            ByteInterval { start: 10, end: 20 },
        ];
        let nested = TraceSection {
            span: ByteInterval { start: 12, end: 18 },
            body: String::from("nested"),
            value: value.clone(),
        };
        assert!(validate_trace_nesting(&[nested], &nonnormative).is_ok());
        let empty = TraceSection {
            span: ByteInterval { start: 12, end: 12 },
            body: String::from("empty"),
            value: value.clone(),
        };
        assert!(validate_trace_nesting(&[empty], &nonnormative).is_err());
        let crossing = TraceSection {
            span: ByteInterval { start: 4, end: 12 },
            body: String::from("crossing"),
            value,
        };
        assert!(validate_trace_nesting(&[crossing], &nonnormative).is_err());
    }

    #[test]
    #[allow(clippy::expect_used)]
    fn malformed_standing_spec_fails_with_stable_diagnostics() {
        let path = RepositoryPath::new("openspec/changes/demo/spec.md").expect("path");
        let output = extract_openspec(&path, b"# Demo\n");
        assert_eq!(output.diagnostics[0].code, "SPG-OPENSPEC-STANDING-PATH");

        let path = RepositoryPath::new("openspec/specs/demo/spec.md").expect("path");
        let output = extract_openspec(&path, b"# Demo\n\n## Purpose\nNo requirements.\n");
        assert_eq!(
            output.diagnostics[0].code,
            "SPG-OPENSPEC-REQUIREMENTS-MISSING"
        );
    }

    fn configuration(
        name: &str,
        scope: &str,
        kind: &str,
        lifecycle: &str,
        reserved_ids: &str,
        approval: &str,
    ) -> String {
        format!(
            "schema = \"buoy.openspec-authority/v1\"\n\n[[capabilities]]\nname = \"{name}\"\nformat = \"openspec/standing-spec/v1\"\nscope = \"{scope}\"\natom_kind = \"{kind}\"\nlifecycle = \"{lifecycle}\"\nreserved_ids = {reserved_ids}\n{approval}"
        )
    }

    #[test]
    #[allow(clippy::too_many_lines)]
    fn coverage_controls_close_reachable_configuration_and_extraction_decisions() {
        assert_eq!(
            OpenSpecConfiguration::parse(&[0xff]),
            Err("SPG-OPENSPEC-CONFIG-UTF8")
        );
        let missing_format = configuration(
            "demo",
            "openspec.demo",
            "behavioral",
            "proposed",
            "[\"DEMO-001\"]",
            "",
        )
        .replace("format = \"openspec/standing-spec/v1\"\n", "");
        assert_eq!(
            OpenSpecConfiguration::parse(missing_format.as_bytes()),
            Err("SPG-OPENSPEC-CONFIG-FORMAT")
        );
        let unsupported_format = configuration(
            "demo",
            "openspec.demo",
            "behavioral",
            "proposed",
            "[\"DEMO-001\"]",
            "",
        )
        .replace("openspec/standing-spec/v1", "openspec/standing-spec/v3");
        assert_eq!(
            OpenSpecConfiguration::parse(unsupported_format.as_bytes()),
            Err("SPG-OPENSPEC-CONFIG-FORMAT")
        );
        // Version 2 is the one other accepted format, so it must parse.
        let version_two = configuration(
            "demo",
            "openspec.demo",
            "behavioral",
            "proposed",
            "[\"DEMO-001\"]",
            "",
        )
        .replace("openspec/standing-spec/v1", "openspec/standing-spec/v2");
        assert!(OpenSpecConfiguration::parse(version_two.as_bytes()).is_ok());
        assert!(OpenSpecConfiguration::parse(b"schema = \"wrong\"\ncapabilities = []\n").is_err());
        assert!(
            OpenSpecConfiguration::parse(
                b"schema = \"buoy.openspec-authority/v1\"\ncapabilities = []\n"
            )
            .is_err()
        );
        for name in ["", "Bad"] {
            assert!(
                OpenSpecConfiguration::parse(
                    configuration(
                        name,
                        "openspec.demo",
                        "behavioral",
                        "proposed",
                        "[\"DEMO-001\"]",
                        "",
                    )
                    .as_bytes(),
                )
                .is_err()
            );
        }
        for name in ["de-mo", "demo1"] {
            assert!(
                OpenSpecConfiguration::parse(
                    configuration(
                        name,
                        "openspec.demo",
                        "behavioral",
                        "proposed",
                        "[\"DEMO-001\"]",
                        "",
                    )
                    .as_bytes(),
                )
                .is_ok()
            );
        }
        assert!(
            OpenSpecConfiguration::parse(
                configuration(
                    "demo",
                    "openspec.demo",
                    "invalid",
                    "proposed",
                    "[\"DEMO-001\"]",
                    "",
                )
                .as_bytes(),
            )
            .is_err()
        );
        for kind in ["behavioral", "structural", "constraint"] {
            assert!(
                OpenSpecConfiguration::parse(
                    configuration(
                        "demo",
                        "openspec.demo",
                        kind,
                        "proposed",
                        "[\"DEMO-001\"]",
                        "",
                    )
                    .as_bytes(),
                )
                .is_ok()
            );
        }
        let valid_approval = concat!(
            "approval = { authority = \"maintainers\", record = \"reviews/demo.md\", digest = \"xxh3-128:",
            "11111111111111111111111111111111\" }\n",
        );
        assert!(
            OpenSpecConfiguration::parse(
                configuration(
                    "demo",
                    "openspec.demo",
                    "behavioral",
                    "ratified",
                    "[\"DEMO-001\"]",
                    valid_approval,
                )
                .as_bytes(),
            )
            .is_ok()
        );
        for approval in [
            concat!(
                "approval = { authority = \"invalid scope\", record = \"reviews/demo.md\", digest = \"xxh3-128:",
                "11111111111111111111111111111111\" }\n",
            ),
            concat!(
                "approval = { authority = \"maintainers\", record = \"../invalid\", digest = \"xxh3-128:",
                "11111111111111111111111111111111\" }\n",
            ),
            "approval = { authority = \"maintainers\", record = \"reviews/demo.md\", digest = \"invalid\" }\n",
        ] {
            assert!(
                OpenSpecConfiguration::parse(
                    configuration(
                        "demo",
                        "openspec.demo",
                        "behavioral",
                        "ratified",
                        "[\"DEMO-001\"]",
                        approval,
                    )
                    .as_bytes(),
                )
                .is_err()
            );
        }
        for (lifecycle, approval) in [
            ("invalid", ""),
            ("ratified", ""),
            ("proposed", valid_approval),
        ] {
            assert!(
                OpenSpecConfiguration::parse(
                    configuration(
                        "demo",
                        "openspec.demo",
                        "behavioral",
                        lifecycle,
                        "[\"DEMO-001\"]",
                        approval,
                    )
                    .as_bytes(),
                )
                .is_err()
            );
        }
        for (scope, reserved_ids) in [
            ("openspec.demo", "[]"),
            ("openspec.demo", "[\"invalid id\"]"),
            ("openspec.demo", "[\"DEMO-001\", \"DEMO-001\"]"),
            ("invalid scope", "[\"DEMO-001\"]"),
        ] {
            assert!(
                OpenSpecConfiguration::parse(
                    configuration("demo", scope, "behavioral", "proposed", reserved_ids, "",)
                        .as_bytes(),
                )
                .is_err()
            );
        }
        let duplicate = format!(
            "{}\n{}",
            configuration(
                "demo",
                "openspec.demo",
                "behavioral",
                "proposed",
                "[\"DEMO-001\"]",
                "",
            ),
            configuration(
                "demo",
                "openspec.demo",
                "behavioral",
                "proposed",
                "[\"DEMO-002\"]",
                "",
            )
            .replacen("schema = \"buoy.openspec-authority/v1\"\n\n", "", 1),
        );
        assert!(OpenSpecConfiguration::parse(duplicate.as_bytes()).is_err());

        let path = RepositoryPath::new("openspec/specs/demo/spec.md").expect("path");
        let valid = b"# Demo\n\n## Requirements\n\n### Requirement: DEMO-001 Valid\nThe adapter MUST parse this requirement.\n";
        let configuration = OpenSpecConfiguration::parse(
            configuration(
                "demo",
                "openspec.demo",
                "behavioral",
                "proposed",
                "[\"OTHER-001\"]",
                "",
            )
            .as_bytes(),
        )
        .expect("configuration");
        assert_eq!(
            extract_openspec_configured(&path, valid, &configuration).diagnostics[0].code,
            "SPG-OPENSPEC-REQUIREMENT-UNRESERVED"
        );
        let other_path = RepositoryPath::new("openspec/specs/other/spec.md").expect("other path");
        assert_eq!(
            extract_openspec_configured(&other_path, valid, &configuration).diagnostics[0].code,
            "SPG-OPENSPEC-CONFIG-CAPABILITY"
        );
        assert_eq!(
            extract_openspec(&path, &[0xff]).diagnostics[0].code,
            "SPG-ADAPTER-INVALID-UTF8"
        );
        assert_eq!(
            extract_openspec(&path, b"# Demo\r\n## Requirements\r\n").diagnostics[0].code,
            "SPG-OPENSPEC-NONCANONICAL-NEWLINE"
        );
        assert_eq!(
            extract_openspec(&path, b"# Demo\n\n## Requirements\n").diagnostics[0].code,
            "SPG-OPENSPEC-REQUIREMENT-EMPTY"
        );
        let invalid_id = b"# Demo\n\n## Requirements\n\n### Requirement: BAD- Invalid\nBody.\n";
        assert_eq!(
            extract_openspec(&path, invalid_id).diagnostics[0].code,
            "SPG-OPENSPEC-REQUIREMENT-ID"
        );
        let title_first_id =
            b"# Demo\n\n## Requirements\n\n### Requirement: [DEMO-001] Title first\nBody.\n";
        assert_eq!(
            extract_openspec(&path, title_first_id).diagnostics[0].code,
            "SPG-OPENSPEC-REQUIREMENT-ID"
        );
        let bracketed_id = b"# Demo\n\n## Requirements\n\n### Requirement: [DEMO-001]\nBody.\n";
        assert_eq!(
            extract_openspec(&path, bracketed_id).diagnostics[0].code,
            "SPG-OPENSPEC-UNSUPPORTED-SYNTAX"
        );
        let ordinary_h3 = b"# Demo\n\n## Requirements\n\n### Notes\nBody.\n\n### Requirement: DEMO-001 Valid\nThe adapter MUST parse this.\n";
        assert_eq!(
            extract_openspec(&path, ordinary_h3).diagnostics[0].code,
            "SPG-OPENSPEC-UNSUPPORTED-SYNTAX"
        );
        let invalid_key = b"# Demo\n\n## Requirements\n\n### Requirement: bad Invalid\nBody.\n";
        assert_eq!(
            extract_openspec(&path, invalid_key).diagnostics[0].code,
            "SPG-OPENSPEC-REQUIREMENT-ID"
        );
        let invalid_atom =
            b"# Demo\n\n## Requirements\n\n### Requirement: DEMO-001 Invalid\nBody.\n";
        assert_eq!(
            extract_openspec(&path, invalid_atom).diagnostics[0].code,
            "SPG-OPENSPEC-ATOM-CONSTRUCTION"
        );
        let pre_requirement_prose = b"# Demo\n\n### Requirement: EARLY-001 Ignored\nBody.\n\n## Requirements\n\nOrdinary prose remains nonnormative.\n\n### Requirement: DEMO-001 Valid\nThe adapter MUST parse this.\n";
        assert!(
            extract_openspec(&path, pre_requirement_prose)
                .diagnostics
                .is_empty()
        );
        let duplicate_authority = b"# Demo\n\n## Requirements\n\n### Requirement: DEMO-001 Valid\nThe adapter MUST parse this.\n\n## Requirements\n";
        assert_eq!(
            extract_openspec(&path, duplicate_authority).diagnostics[0].code,
            "SPG-OPENSPEC-REQUIREMENTS-DUPLICATE"
        );
        let requirement_boundaries = b"# Demo\n\n## Requirements\n\n### Requirement: DEMO-001 First\nThe adapter MUST parse this.\n\n#### Scenario: First\nBody.\n\n### Requirement: DEMO-002 Second\nThe adapter MUST parse this.\n\n#### Scenario: Second\nBody.\n";
        assert!(
            extract_openspec(&path, requirement_boundaries)
                .diagnostics
                .is_empty()
        );
        let duplicate_requirement = b"# Demo\n\n## Requirements\n\n### Requirement: DEMO-001 First\nThe adapter MUST parse this.\n\n### Requirement: DEMO-001 Second\nThe adapter MUST parse this.\n";
        assert!(
            extract_openspec(&path, duplicate_requirement)
                .diagnostics
                .iter()
                .any(|value| value.code == "SPG-OPENSPEC-DUPLICATE-OBJECT")
        );
        let invalid_scenario = b"# Demo\n\n## Requirements\n\n### Requirement: DEMO-001 Valid\nThe adapter MUST parse this.\n\n#### Scenario: !!!\nBody.\n";
        assert_eq!(
            extract_openspec(&path, invalid_scenario).diagnostics[0].code,
            "SPG-OPENSPEC-SCENARIO-ID"
        );
        let duplicate_scenario = b"# Demo\n\n## Requirements\n\n### Requirement: DEMO-001 Valid\nThe adapter MUST parse this.\n\n#### Scenario: Same\nBody.\n\n#### Scenario: Same\nBody.\n";
        assert!(
            extract_openspec(&path, duplicate_scenario)
                .diagnostics
                .iter()
                .any(|value| value.code == "SPG-OPENSPEC-DUPLICATE-OBJECT")
        );
        let multiple_diagnostics = b"# Demo\n\n## Requirements\n\n### Scenario: Wrong level\n\n### Requirement: DEMO-001 Valid\nThe adapter MUST parse this.\n\n#### Scenario: !!!\nBody.\n";
        assert_eq!(
            extract_openspec(&path, multiple_diagnostics)
                .diagnostics
                .len(),
            2
        );
        let orphan_scenario = b"# Demo\n\n## Requirements\n\n#### Scenario: Orphan\nBody.\n\n### Requirement: DEMO-001 Valid\nThe adapter MUST parse this.\n";
        assert_eq!(
            extract_openspec(&path, orphan_scenario).diagnostics[0].code,
            "SPG-OPENSPEC-UNSUPPORTED-SYNTAX"
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
        let step_lines = "- **WHEN** extraction runs\n- **unterminated keyword\n- **** empty keyword\n- **THEN** \n";
        assert_eq!(
            scenario_steps(
                step_lines,
                ByteInterval {
                    start: 0,
                    end: step_lines.len(),
                },
            )
            .iter()
            .map(|step| step.keyword.as_str())
            .collect::<Vec<_>>(),
            ["WHEN"]
        );
        let heading_rows = headings("##### Five\n###### Six with `code`\n");
        assert_eq!(
            heading_rows
                .iter()
                .map(|value| value.level)
                .collect::<Vec<_>>(),
            [5, 6]
        );
        assert!(standing_spec_capability("openspec/specs//spec.md").is_none());
        assert!(standing_spec_capability("openspec/specs/de-mo/spec.md").is_some());
        assert!(standing_spec_capability("openspec/specs/demo1/spec.md").is_some());
        assert!(standing_spec_capability("openspec/specs/Bad/spec.md").is_none());
        assert!(requirement_key("").is_none());
        assert!(requirement_key("AB short").is_none());
        assert_eq!(slug("-A"), Some(String::from("a")));

        let output = extract_openspec(&path, valid);
        let object = &output.semantic[0].value;
        assert_eq!(object.title(), "DEMO-001 Valid");
        assert!(object.body().contains("MUST"));
        assert_eq!(object.digest(), object.external().revision());
    }

    fn model_requirement(
        start: usize,
        end: usize,
        scenarios: Vec<ByteInterval>,
    ) -> RequirementSection {
        RequirementSection {
            stable_id: StableId::new("DEMO-001").expect("model stable ID"),
            title: String::from("DEMO-001 Model"),
            span: ByteInterval { start, end },
            scenarios: scenarios
                .into_iter()
                .map(|span| ScenarioSection {
                    title: String::from("Model"),
                    slug: String::from("model"),
                    span,
                    steps: Vec::new(),
                })
                .collect(),
        }
    }

    #[test]
    fn finite_partition_model_covers_valid_intervals() {
        for length in 1..=6 {
            for start in 0..length {
                for end in (start + 1)..=length {
                    let scenarios = (start..end)
                        .flat_map(|scenario_start| {
                            ((scenario_start + 1)..=end).map(move |scenario_end| ByteInterval {
                                start: scenario_start,
                                end: scenario_end,
                            })
                        })
                        .collect::<Vec<_>>();
                    let requirement = model_requirement(start, end, scenarios);
                    let nonnormative = complement_intervals(length, &[requirement.span]);
                    assert!(validate_partition(length, &[requirement], &nonnormative).is_ok());
                }
            }
        }
    }

    #[test]
    fn finite_partition_model_rejects_invalid_intervals() {
        let requirement = model_requirement(2, 8, Vec::new());
        for nonnormative in [
            vec![
                ByteInterval { start: 0, end: 1 },
                ByteInterval { start: 2, end: 10 },
            ],
            vec![
                ByteInterval { start: 0, end: 3 },
                ByteInterval { start: 8, end: 10 },
            ],
        ] {
            assert!(
                validate_partition(10, core::slice::from_ref(&requirement), &nonnormative,)
                    .is_err()
            );
        }
        assert!(validate_partition(0, &[], &[]).is_err());
        assert!(validate_partition(10, &[], &[ByteInterval { start: 2, end: 10 }]).is_err());
        for nonnormative in [
            vec![
                ByteInterval { start: 0, end: 0 },
                ByteInterval { start: 0, end: 10 },
            ],
            vec![
                ByteInterval { start: 0, end: 11 },
                ByteInterval { start: 1, end: 10 },
            ],
        ] {
            assert!(validate_partition(10, &[], &nonnormative).is_err());
        }
        for scenario in [
            ByteInterval { start: 1, end: 4 },
            ByteInterval { start: 3, end: 9 },
            ByteInterval { start: 3, end: 3 },
        ] {
            let invalid_scenario = model_requirement(2, 8, vec![scenario]);
            assert!(
                validate_partition(
                    10,
                    &[invalid_scenario],
                    &[
                        ByteInterval { start: 0, end: 2 },
                        ByteInterval { start: 8, end: 10 }
                    ],
                )
                .is_err()
            );
        }
        let path = RepositoryPath::new("openspec/specs/demo/spec.md").expect("path");
        let invalid_partition = finish_document(
            &path,
            10,
            "demo",
            vec![requirement],
            vec![
                ByteInterval { start: 0, end: 1 },
                ByteInterval { start: 8, end: 10 },
            ],
            Vec::new(),
        )
        .expect_err("partition diagnostic");
        assert_eq!(invalid_partition[0].code, "SPG-OPENSPEC-PARTITION");
        #[cfg(feature = "coverage-controls")]
        assert!(openspec_partition_control());
    }

    #[test]
    fn trace_collection_checks_nested_hosts_and_trace_partition_membership() {
        let source = "## Trace\n\n### Nested host\n\n```buoy-external-trace-v1\nid = \"LINK-1\"\natoms = [\"DEMO-001@xxh3-128:11111111111111111111111111111111\"]\nrelation = \"documents\"\ntarget = \"https://example.com/trace\"\n```\n\n```text\nunrelated\n```\n\n## Later section\n";
        let path = RepositoryPath::new("openspec/specs/demo/spec.md").expect("trace path");
        let heading_rows = headings(source);
        let pass = TracePass {
            path: &path,
            source,
            format: OpenSpecFormatVersion::StandingSpecV2,
            requirements: &[],
            headings: &heading_rows,
            authority_end: 0,
            capability: "demo",
        };
        let mut diagnostics = Vec::new();
        let traces = collect_trace_sections(&pass, &mut diagnostics);
        assert!(diagnostics.is_empty());
        assert_eq!(traces.len(), 1);

        let authority_pass = TracePass {
            authority_end: source.len(),
            ..pass
        };
        let mut authority_diagnostics = Vec::new();
        assert!(collect_trace_sections(&authority_pass, &mut authority_diagnostics).is_empty());
        assert!(!authority_diagnostics.is_empty());

        let requirements_source = source.replacen("## Trace", "## Requirements", 1);
        let requirements_headings = headings(&requirements_source);
        let requirements_pass = TracePass {
            path: &path,
            source: &requirements_source,
            format: OpenSpecFormatVersion::StandingSpecV2,
            requirements: &[],
            headings: &requirements_headings,
            authority_end: 0,
            capability: "demo",
        };
        let mut requirements_diagnostics = Vec::new();
        assert!(
            collect_trace_sections(&requirements_pass, &mut requirements_diagnostics).is_empty()
        );
        assert!(!requirements_diagnostics.is_empty());

        let trace_span = traces[0].span;
        let requirement = model_requirement(trace_span.start, trace_span.end, Vec::new());
        let nonnormative = complement_intervals(source.len(), &[requirement.span]);
        let error = finish_document(
            &path,
            source.len(),
            "demo",
            vec![requirement],
            nonnormative,
            traces,
        )
        .expect_err("trace outside nonnormative sections");
        assert_eq!(error[0].code, "SPG-OPENSPEC-PARTITION");
    }
}
