//! Pure read-only trace queries over one materialized source snapshot.
//!
//! A trace projection supplies navigation and impact information only. It
//! establishes no coverage, execution, proof, approval, correctness, or
//! prose-to-code correspondence.

use crate::algorithms::{
    BindingValidationContext, Diagnostic, SourceTraversal, implementation_binding_diagnostics,
    require_current_source_reference,
};
use crate::identifier::{SourceItemKind, StableId};
use crate::model::{
    AtomRevisionRef, BindingAttachment, ImplementationBindingRef, NavigationKind, SourceAuthority,
    SourceGraph, SourceNodeRef, StoredBinding,
};
use crate::value::{RepositoryPath, SemanticDigest};
use alloc::collections::{BTreeMap, BTreeSet};
use alloc::vec::Vec;

/// One typed selector for a read-only trace query.
///
/// Exactly one selector reaches this core. The shell parses untrusted text at
/// its boundary and never forwards an ambiguous or partial value.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum TraceSelector {
    /// Selects every relation whose governing atom carries this stable ID.
    RequirementId(StableId),
    /// Selects every record that carries this stable relation ID.
    RelationId(StableId),
    /// Selects every relation whose implementation node sits in this path.
    ImplementationPath(RepositoryPath),
    /// Selects one exact immutable record.
    Record(ImplementationBindingRef),
}

/// Explicit history selection for one trace query.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum TraceHistory {
    /// Reports current attachments only.
    CurrentOnly,
    /// Also reports retained records that no current attachment selects.
    IncludeHistory,
}

/// Closed classification of one retained implementation record.
///
/// The three states are exclusive. A retained record never becomes current
/// without one current attachment.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum TraceState {
    /// One current attachment selects the record and every bound side is current.
    Current,
    /// One current attachment selects the record and one bound side is not current.
    Stale,
    /// No current attachment selects the record.
    Historical,
}

impl TraceState {
    /// Whether one current source item declares this record locator.
    #[must_use]
    pub const fn attached(self) -> bool {
        matches!(self, Self::Current | Self::Stale)
    }

    /// Stable lower-case state name for deterministic rendering.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Current => "current",
            Self::Stale => "stale",
            Self::Historical => "historical",
        }
    }
}

/// Separate freshness facts for one projected relation.
///
/// The facts are orthogonal. No renderer may replace them with one optimistic
/// summary.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "each freshness fact is orthogonal and must not collapse into one summary"
)]
pub struct TraceFreshness {
    record: bool,
    attachment: bool,
    implementation: bool,
    requirement: bool,
    authority: bool,
}

impl TraceFreshness {
    /// Whether the stored record content matches its claimed locator.
    #[must_use]
    pub const fn record(self) -> bool {
        self.record
    }

    /// Whether one current source item declares this record locator.
    #[must_use]
    pub const fn attachment(self) -> bool {
        self.attachment
    }

    /// Whether the exact implementation subject node is current.
    #[must_use]
    pub const fn implementation(self) -> bool {
        self.implementation
    }

    /// Whether the exact requirement node is current.
    #[must_use]
    pub const fn requirement(self) -> bool {
        self.requirement
    }

    /// Whether exactly one materialized authority governs the bound requirement.
    #[must_use]
    pub const fn authority(self) -> bool {
        self.authority
    }
}

/// One projected trace relation and its complete current-state facts.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TraceRow {
    state: TraceState,
    relation: StableId,
    navigation: NavigationKind,
    record: ImplementationBindingRef,
    atom: AtomRevisionRef,
    requirement: SourceNodeRef,
    implementation: SourceNodeRef,
    requirement_path: Option<RepositoryPath>,
    implementation_path: Option<RepositoryPath>,
    freshness: TraceFreshness,
    diagnostics: Vec<Diagnostic>,
}

impl TraceRow {
    /// Closed classification of this record.
    #[must_use]
    pub const fn state(&self) -> TraceState {
        self.state
    }

    /// Stable relation ID that this record preserves across revisions.
    #[must_use]
    pub const fn relation(&self) -> &StableId {
        &self.relation
    }

    /// Closed navigation relation the record declares.
    #[must_use]
    pub const fn navigation(&self) -> NavigationKind {
        self.navigation
    }

    /// Exact immutable record locator.
    #[must_use]
    pub const fn record(&self) -> ImplementationBindingRef {
        self.record
    }

    /// Exact governing atom revision the record binds.
    #[must_use]
    pub const fn atom(&self) -> &AtomRevisionRef {
        &self.atom
    }

    /// Exact requirement source node the record binds.
    #[must_use]
    pub const fn requirement(&self) -> SourceNodeRef {
        self.requirement
    }

    /// Exact implementation source node the record binds.
    #[must_use]
    pub const fn implementation(&self) -> SourceNodeRef {
        self.implementation
    }

    /// Logical requirement path when the node is retained in this snapshot.
    #[must_use]
    pub const fn requirement_path(&self) -> Option<&RepositoryPath> {
        self.requirement_path.as_ref()
    }

    /// Logical implementation path when the node is retained in this snapshot.
    #[must_use]
    pub const fn implementation_path(&self) -> Option<&RepositoryPath> {
        self.implementation_path.as_ref()
    }

    /// Separate freshness facts for this relation.
    #[must_use]
    pub const fn freshness(&self) -> TraceFreshness {
        self.freshness
    }

    /// Canonically ordered blocking facts for this relation.
    #[must_use]
    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }

    /// Canonical sort key from the reviewed projection design.
    ///
    /// Current rows precede historical rows, so the key leads with the negated
    /// attachment fact rather than with the classification itself.
    fn order_key(
        &self,
    ) -> (
        bool,
        &StableId,
        &StableId,
        Option<&RepositoryPath>,
        SemanticDigest,
        SemanticDigest,
    ) {
        (
            !self.state.attached(),
            &self.relation,
            self.atom.id(),
            self.implementation_path.as_ref(),
            self.implementation.digest(),
            self.record.digest(),
        )
    }
}

/// One complete trace projection over exact saved inputs.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TraceProjection {
    graph: SemanticDigest,
    selector: TraceSelector,
    history: TraceHistory,
    rows: Vec<TraceRow>,
}

impl TraceProjection {
    /// Complete source-graph digest this projection binds.
    #[must_use]
    pub const fn graph(&self) -> SemanticDigest {
        self.graph
    }

    /// Exact typed selector this projection answered.
    #[must_use]
    pub const fn selector(&self) -> &TraceSelector {
        &self.selector
    }

    /// Explicit history selection this projection applied.
    #[must_use]
    pub const fn history(&self) -> TraceHistory {
        self.history
    }

    /// Projected relations in canonical order.
    #[must_use]
    pub fn rows(&self) -> &[TraceRow] {
        &self.rows
    }

    /// Whether one selected current attachment is stale.
    ///
    /// A historical row creates no current attachment, so it never sets this
    /// fact and never changes a successful exit class.
    #[must_use]
    pub fn has_stale_attachment(&self) -> bool {
        self.rows.iter().any(|row| row.state == TraceState::Stale)
    }
}

/// Deterministic trace-query rejections.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum TraceError {
    /// Two stored entries claim one record locator.
    DuplicateRecord,
    /// One stored record content does not match its claimed locator.
    RecordDigestMismatch,
    /// Two current attachments repeat one subject and record pair.
    DuplicateAttachment,
    /// One current subject declares two different record locators.
    AmbiguousAttachment,
    /// The typed selector matched no retained record.
    SelectorNotFound,
}

impl TraceError {
    /// Stable machine diagnostic code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::DuplicateRecord => "TRC-RECORD-DUPLICATE",
            Self::RecordDigestMismatch => "TRC-RECORD-DIGEST",
            Self::DuplicateAttachment => "TRC-ATTACHMENT-DUPLICATE",
            Self::AmbiguousAttachment => "TRC-ATTACHMENT-AMBIGUOUS",
            Self::SelectorNotFound => "TRC-SELECTOR-NOT-FOUND",
        }
    }
}

/// Complete explicit inputs for one read-only trace query.
///
/// The context accepts validated values only. It performs no filesystem,
/// parser, clock, network, or process access.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TraceContext<'a> {
    graph: &'a SourceGraph,
    closure: &'a SourceTraversal,
    requirement_kind: SourceItemKind,
    authorities: &'a [SourceAuthority],
    attachments: &'a [BindingAttachment],
    store: &'a [StoredBinding],
    attached: BTreeMap<ImplementationBindingRef, BindingAttachment>,
    governing: BTreeMap<SourceNodeRef, BTreeSet<AtomRevisionRef>>,
}

fn authority_set_matches(
    atoms: Option<&BTreeSet<AtomRevisionRef>>,
    atom: &AtomRevisionRef,
) -> bool {
    match atoms {
        Some(atoms) if atoms.len() == 1 => atoms.contains(atom),
        _ => false,
    }
}

impl<'a> TraceContext<'a> {
    /// Validates one complete snapshot of trace inputs.
    ///
    /// Construction rejects a duplicate record locator, a record whose content
    /// contradicts its locator, a repeated attachment, and one subject that
    /// claims two records. Each rejection is total, so no partial projection
    /// reaches a caller.
    ///
    /// # Errors
    ///
    /// Returns the exact deterministic rejection for the first fault in the
    /// fixed record-then-attachment order.
    pub fn new(
        graph: &'a SourceGraph,
        closure: &'a SourceTraversal,
        requirement_kind: SourceItemKind,
        authorities: &'a [SourceAuthority],
        attachments: &'a [BindingAttachment],
        store: &'a [StoredBinding],
    ) -> Result<Self, TraceError> {
        let mut claimed = BTreeSet::new();
        for member in store {
            if member.record().record() != member.locator() {
                return Err(TraceError::RecordDigestMismatch);
            }
            if !claimed.insert(member.locator()) {
                return Err(TraceError::DuplicateRecord);
            }
        }

        let mut attached = BTreeMap::new();
        let mut subjects = BTreeMap::<SourceNodeRef, ImplementationBindingRef>::new();
        let mut seen = BTreeSet::new();
        for attachment in attachments {
            if !seen.insert((attachment.subject(), attachment.record())) {
                return Err(TraceError::DuplicateAttachment);
            }
            if subjects
                .insert(attachment.subject(), attachment.record())
                .is_some_and(|existing| existing != attachment.record())
                || attached
                    .insert(attachment.record(), *attachment)
                    .is_some_and(|existing| existing.subject() != attachment.subject())
            {
                return Err(TraceError::AmbiguousAttachment);
            }
        }

        let mut governing = BTreeMap::<SourceNodeRef, BTreeSet<AtomRevisionRef>>::new();
        for authority in authorities {
            governing
                .entry(authority.node())
                .or_default()
                .insert(authority.atom().clone());
        }

        Ok(Self {
            graph,
            closure,
            requirement_kind,
            authorities,
            attachments,
            store,
            attached,
            governing,
        })
    }

    /// Retained records in the order the caller supplied.
    #[must_use]
    pub const fn store(&self) -> &'a [StoredBinding] {
        self.store
    }

    /// Current attachments in the order the caller supplied.
    #[must_use]
    pub const fn attachments(&self) -> &'a [BindingAttachment] {
        self.attachments
    }

    /// Logical path of one retained node, when this snapshot retains it.
    fn node_path(&self, node: SourceNodeRef) -> Option<RepositoryPath> {
        self.graph
            .node(node)
            .map(|retained| retained.item().path().clone())
    }

    /// Whether exactly one materialized authority governs one requirement node.
    fn authority_matches(&self, requirement: SourceNodeRef, atom: &AtomRevisionRef) -> bool {
        authority_set_matches(self.governing.get(&requirement), atom)
    }

    /// Builds one row for one retained record.
    fn row(&self, member: &StoredBinding) -> TraceRow {
        let record = member.record();
        let attachment = self.attached.get(&member.locator()).copied();
        let implementation_current =
            require_current_source_reference(self.closure, record.subject()).is_ok();
        let requirement_current =
            require_current_source_reference(self.closure, record.requirement()).is_ok();
        let freshness = TraceFreshness {
            record: record.record() == member.locator(),
            attachment: attachment.is_some(),
            implementation: implementation_current,
            requirement: requirement_current,
            authority: self.authority_matches(record.requirement(), record.atom()),
        };

        // One shared rule decides staleness, so the projection and the binding
        // report never disagree about the same attachment.
        let diagnostics = attachment.map_or_else(Vec::new, |attachment| {
            let context = BindingValidationContext::new(
                self.graph,
                self.closure,
                self.requirement_kind.clone(),
                self.authorities,
            );
            implementation_binding_diagnostics(
                core::slice::from_ref(&attachment),
                core::slice::from_ref(member),
                &context,
            )
        });

        let state = if attachment.is_none() {
            TraceState::Historical
        } else if diagnostics.is_empty() {
            TraceState::Current
        } else {
            TraceState::Stale
        };

        TraceRow {
            state,
            relation: record.meta().id().clone(),
            navigation: record.navigation(),
            record: member.locator(),
            atom: record.atom().clone(),
            requirement: record.requirement(),
            implementation: record.subject(),
            requirement_path: self.node_path(record.requirement()),
            implementation_path: self.node_path(record.subject()),
            freshness,
            diagnostics,
        }
    }
}

/// Whether one row satisfies one typed selector.
fn selected(row: &TraceRow, selector: &TraceSelector) -> bool {
    match selector {
        TraceSelector::RequirementId(id) => row.atom.id() == id,
        TraceSelector::RelationId(id) => &row.relation == id,
        TraceSelector::ImplementationPath(path) => row.implementation_path.as_ref() == Some(path),
        TraceSelector::Record(reference) => row.record == *reference,
    }
}

/// Answers one read-only trace query over exact saved inputs.
///
/// The query never promotes history: a retained record stays historical while
/// no current attachment declares its locator. An exact record selector still
/// reports its historical row, because that selector names one immutable
/// object rather than one current state.
///
/// The returned rows are canonically ordered and carry no duplicate record.
///
/// A fresh row supplies navigation and impact information only. It establishes
/// no coverage, execution, proof, approval, or correctness.
///
/// # Errors
///
/// Returns [`TraceError::SelectorNotFound`] when the selector matches no
/// retained record after history selection.
pub fn trace_query(
    context: &TraceContext<'_>,
    selector: &TraceSelector,
    history: TraceHistory,
) -> Result<TraceProjection, TraceError> {
    let exact_record = matches!(selector, TraceSelector::Record(_));
    let mut rows = Vec::new();
    for member in context.store {
        let row = context.row(member);
        if !selected(&row, selector) {
            continue;
        }
        // An exact record selector names one immutable object, so it keeps its
        // historical row under either history selection.
        if row.state == TraceState::Historical
            && history == TraceHistory::CurrentOnly
            && !exact_record
        {
            continue;
        }
        rows.push(row);
    }

    if rows.is_empty() {
        return Err(TraceError::SelectorNotFound);
    }
    rows.sort_by(|left, right| left.order_key().cmp(&right.order_key()));

    Ok(TraceProjection {
        graph: context.graph.digest(),
        selector: selector.clone(),
        history,
        rows,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[allow(clippy::expect_used)]
    fn one_authority_must_match_the_selected_atom() {
        let first = AtomRevisionRef::new(
            StableId::new("ATOM-1").expect("atom ID"),
            SemanticDigest::xxh3_128([1; 16]),
        );
        let second = AtomRevisionRef::new(
            StableId::new("ATOM-2").expect("atom ID"),
            SemanticDigest::xxh3_128([2; 16]),
        );
        let mut atoms = BTreeSet::new();
        assert!(!authority_set_matches(None, &first));
        atoms.insert(first.clone());
        assert!(authority_set_matches(Some(&atoms), &first));
        assert!(!authority_set_matches(Some(&atoms), &second));
        atoms.insert(second.clone());
        assert!(!authority_set_matches(Some(&atoms), &second));
    }
}
