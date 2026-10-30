//! Sanctioned conversion of one inline link into one compact binding.
//!
//! Conversion is mechanical: it reads one committed `implementation-link`
//! record, derives the immutable object the Rust flow would have written, and
//! replaces the inline record block with the two-line compact marker. The
//! declaration and every unrelated byte stay untouched, and no reverse command
//! exists, so reverting a conversion is a reviewed revert like any other edit.

use crate::command::{
    CommandError, binding_algorithm, binding_digest, binding_object_repository_path,
};
use crate::records::{
    BINDING_ADDRESS_FIELD, BINDING_RECORD_KIND, ChelisDialect, ChelisRecord, extract,
};
use crate::snapshot::{ChelisConfiguration, ChelisSnapshot};
use buoy::adapters::{canonical_binding_record, extract_binding_in_mount};
use buoy::core::{
    ActiveAlgorithm, AtomRevisionRef, ImplementationBinding, NavigationKind, RepositoryPath,
    SemanticDigest, SourceNodeRef, StableId,
};
use serde::Serialize;
use std::path::Path;

/// One planned conversion.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct BindPlan {
    /// Canonical path of the source file carrying the inline record.
    pub path: String,
    /// Stable relation ID of the converted link.
    pub record: String,
    /// Store-relative path of the object the write creates.
    pub object_path: String,
    /// Content address the compact marker will carry.
    pub address: String,
    /// Exact replacement text of the record block.
    pub marker: String,
    #[serde(skip)]
    block_start: usize,
    #[serde(skip)]
    block_end: usize,
    #[serde(skip)]
    object_bytes: String,
    #[serde(skip)]
    block_bytes: Vec<u8>,
}

#[derive(Serialize)]
struct BindReport<'a> {
    schema: &'static str,
    plan: &'a BindPlan,
    written: bool,
}

/// Complete deterministic conversion outcome.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BindOutcome {
    /// Canonical machine-report bytes.
    pub report: String,
    /// Whether saved bytes changed.
    pub written: bool,
}

/// Deterministic conversion failure.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BindError {
    /// A command-layer failure.
    Command(CommandError),
    /// The configuration declares no binding store.
    StoreUnconfigured,
    /// No inline implementation link carries the selected ID.
    UnknownLink(String),
    /// The selected link cannot supply one complete binding.
    IncompleteLink(String),
    /// The selected link carries navigation the compact form cannot express.
    UnsupportedNavigation(String),
    /// The selected link binds a number of atoms other than one.
    NotExactlyOneAtom(String),
    /// The selected link's recorded item digest is not current.
    StaleItem {
        /// Digest the inline link recorded.
        bound: String,
        /// Digest the attached declaration currently produces.
        current: String,
    },
    /// The bound atom is absent from the snapshot.
    UnknownAtom(String),
    /// A selected source fails the pinned grammar; nothing is written.
    ParseRejected(String),
    /// The active build carries no collision-resistant member.
    NotCollisionResistant(String),
    /// The planned object does not read back as the record it claims.
    ObjectRoundTrip(String),
    /// The destination object exists with different bytes.
    ObjectCollision(String),
    /// Saved source bytes changed after the plan was built.
    Diverged(String),
    /// A filesystem write failed.
    Write(String),
}

/// Renders the two-line compact marker for one address.
fn compact_marker(address: SemanticDigest) -> String {
    format!(
        "-- chelis:provenance/v1 {BINDING_RECORD_KIND}\n-- {BINDING_ADDRESS_FIELD} = {}\n",
        address.canonical_text()
    )
}

/// One exact authority record block and its saved source location.
pub(crate) struct AuthorityBlock {
    pub(crate) path: String,
    pub(crate) start: u64,
    pub(crate) end: u64,
    pub(crate) bytes: String,
}

/// Returns the exact authority record block for one atom ID.
///
/// The block is the requirement's source node. Its bytes are the requirement
/// digest preimage, so formatting changes also require a reviewed rebind.
pub(crate) fn authority_block(snapshot: &ChelisSnapshot, atom_id: &str) -> Option<AuthorityBlock> {
    for (path, dialect, bytes) in snapshot.files() {
        let Ok(extraction) = extract(dialect, bytes) else {
            continue;
        };
        let Ok(source) = core::str::from_utf8(bytes) else {
            continue;
        };
        for record in &extraction.records {
            if record.kind != "authority" {
                continue;
            }
            if record.fields.get("id").map(String::as_str) != Some(atom_id) {
                continue;
            }
            let start = usize::try_from(record.start).ok()?;
            let end = usize::try_from(record.end).ok()?.min(source.len());
            return source.get(start..end).map(|bytes| AuthorityBlock {
                path: path.as_str().to_owned(),
                start: record.start,
                end: record.end,
                bytes: bytes.to_owned(),
            });
        }
    }
    None
}

/// Derives the immutable object one inline link converts into.
fn plan_for(
    snapshot: &ChelisSnapshot,
    store: &RepositoryPath,
    link_id: &str,
) -> Result<BindPlan, BindError> {
    for (path, dialect, bytes) in snapshot.files() {
        if dialect != ChelisDialect::Surf {
            continue;
        }
        let extraction = extract(dialect, bytes)
            .map_err(|_| BindError::ParseRejected(path.as_str().to_owned()))?;
        let Ok(source) = core::str::from_utf8(bytes) else {
            continue;
        };
        for record in &extraction.records {
            if record.kind != "implementation-link"
                || record.fields.get("id").map(String::as_str) != Some(link_id)
            {
                continue;
            }
            return build_plan(snapshot, store, path.as_str(), source, record);
        }
    }
    Err(BindError::UnknownLink(link_id.to_owned()))
}

/// Derives the object one inline link converts into, or refuses.
///
/// Conversion is only mechanical while the compact form can carry everything
/// the inline record carried. The stored schema expresses exactly one
/// `implements` relation over exactly one atom, and its subject is the item
/// digest the inline record already enforced, so any link outside that shape
/// is refused rather than silently narrowed into a weaker claim.
#[allow(clippy::too_many_lines)]
fn build_plan(
    snapshot: &ChelisSnapshot,
    store: &RepositoryPath,
    path: &str,
    source: &str,
    record: &ChelisRecord,
) -> Result<BindPlan, BindError> {
    let link_id = record.fields.get("id").cloned().unwrap_or_default();
    let incomplete = || BindError::IncompleteLink(link_id.clone());
    let active = binding_algorithm().map_err(BindError::NotCollisionResistant)?;
    // The stored schema hardcodes the `implements` relation, so converting any
    // other navigation would rewrite the claim instead of re-encoding it.
    match record.fields.get("navigation").map(String::as_str) {
        Some("implements") => {}
        Some(other) => {
            return Err(BindError::UnsupportedNavigation(other.to_owned()));
        }
        None => return Err(incomplete()),
    }
    let atoms = record.fields.get("atoms").ok_or_else(incomplete)?;
    let entries = atoms
        .split(';')
        .map(str::trim)
        .filter(|entry| !entry.is_empty())
        .collect::<Vec<_>>();
    let [entry] = entries.as_slice() else {
        return Err(BindError::NotExactlyOneAtom(format!(
            "{}:{}",
            link_id,
            entries.len()
        )));
    };
    let (atom_id, revision_text) = entry.split_once('@').ok_or_else(incomplete)?;
    let revision = SemanticDigest::parse_canonical(revision_text).map_err(|_| incomplete())?;
    let member_mismatch = |digest: SemanticDigest| {
        BindError::NotCollisionResistant(format!(
            "digest-member-mismatch;bound={};current={}",
            digest.algorithm().cargo_feature(),
            active.algorithm().cargo_feature()
        ))
    };
    if revision.algorithm() != active.algorithm() {
        return Err(member_mismatch(revision));
    }
    // The compact subject is the item digest the inline record enforced. A
    // stale inline digest must fail here: converting it would recompute the
    // subject from current bytes and launder a rejected link into a fresh one.
    let bound_item = record
        .fields
        .get("item-digest")
        .and_then(|value| SemanticDigest::parse_canonical(value).ok())
        .ok_or_else(incomplete)?;
    if bound_item.algorithm() != active.algorithm() {
        return Err(member_mismatch(bound_item));
    }
    let subject =
        binding_digest(active, record.declaration_text.as_bytes()).map_err(BindError::Command)?;
    if bound_item != subject {
        return Err(BindError::StaleItem {
            bound: bound_item.canonical_text(),
            current: subject.canonical_text(),
        });
    }
    let authority = authority_block(snapshot, atom_id)
        .ok_or_else(|| BindError::UnknownAtom(atom_id.to_owned()))?;
    let requirement =
        binding_digest(active, authority.bytes.as_bytes()).map_err(BindError::Command)?;
    let relation = StableId::new(&link_id).map_err(|_| incomplete())?;
    let atom = StableId::new(atom_id).map_err(|_| incomplete())?;
    let binding = ImplementationBinding::new_with(
        active,
        relation,
        NavigationKind::Implements,
        SourceNodeRef::new(subject),
        SourceNodeRef::new(requirement),
        AtomRevisionRef::new(atom, revision),
    )
    .map_err(|_| incomplete())?;
    let address = binding.record().digest();
    let object_path = binding_object_repository_path(store.as_str(), address);
    let object_bytes = canonical_binding_record(&binding);
    round_trip(active, store, &object_path, &object_bytes, address)?;
    if let Some(existing) = snapshot.artifact(&object_path) {
        if existing != object_bytes.as_bytes() {
            return Err(BindError::ObjectCollision(object_path));
        }
    }
    let block_start = usize::try_from(record.start).map_err(|_| incomplete())?;
    let block_end = usize::try_from(record.end)
        .map_err(|_| incomplete())?
        .min(source.len());
    let block_bytes = source
        .as_bytes()
        .get(block_start..block_end)
        .ok_or_else(incomplete)?
        .to_vec();
    Ok(BindPlan {
        path: path.to_owned(),
        record: link_id,
        object_path,
        address: address.canonical_text(),
        marker: compact_marker(address),
        block_start,
        block_end,
        object_bytes,
        block_bytes,
    })
}

/// Rejects any planned object the store would not resolve back to its address.
///
/// A plan that cannot be read back is a plan that writes an unresolvable
/// store, so the round trip runs against the same upstream parser resolution
/// uses, at the exact path the plan names, before any byte is written.
fn round_trip(
    active: ActiveAlgorithm,
    store: &RepositoryPath,
    object_path: &str,
    object_bytes: &str,
    address: SemanticDigest,
) -> Result<(), BindError> {
    let mismatch = || BindError::ObjectRoundTrip(object_path.to_owned());
    let canonical = RepositoryPath::new(object_path).map_err(|_| mismatch())?;
    let stored = extract_binding_in_mount(active, Some(store), &canonical, object_bytes.as_bytes())
        .stored
        .ok_or_else(mismatch)?;
    if stored.locator().digest() != address || stored.record().record().digest() != address {
        return Err(mismatch());
    }
    Ok(())
}

/// One successor object derived from a compact marker's current graph state.
pub struct SuccessorBinding {
    /// Stable relation ID the successor keeps.
    pub record: String,
    /// Content address of the successor object.
    pub digest: SemanticDigest,
    /// Canonical address text of the successor object.
    pub address: String,
    /// Canonical object bytes.
    pub object_bytes: String,
}

/// Recomputes the object one compact binding marker should address.
///
/// The successor keeps the relation, navigation, and atom identity the
/// committed object recorded, and takes its subject and requirement from the
/// current saved bytes. A marker whose object cannot be read yields no
/// successor, because regeneration must never invent a record the store
/// never held.
///
/// # Errors
///
/// Returns a conversion failure when digest computation fails.
pub fn successor_binding(
    snapshot: &ChelisSnapshot,
    store: &RepositoryPath,
    record: &ChelisRecord,
) -> Result<Option<SuccessorBinding>, BindError> {
    let Some(address) = record
        .fields
        .get(BINDING_ADDRESS_FIELD)
        .and_then(|value| SemanticDigest::parse_canonical(value).ok())
    else {
        return Ok(None);
    };
    let Ok(active) = binding_algorithm() else {
        return Ok(None);
    };
    if address.algorithm() != active.algorithm() {
        return Ok(None);
    }
    let Some(committed) = committed_object(snapshot, store, address, active) else {
        return Ok(None);
    };
    let atom_id = committed.atom().id().clone();
    let Some(revision) = current_revision(snapshot, atom_id.as_str(), active) else {
        return Ok(None);
    };
    let Some(authority) = authority_block(snapshot, atom_id.as_str()) else {
        return Ok(None);
    };
    let requirement =
        binding_digest(active, authority.bytes.as_bytes()).map_err(BindError::Command)?;
    let subject =
        binding_digest(active, record.declaration_text.as_bytes()).map_err(BindError::Command)?;
    let successor = ImplementationBinding::new_with(
        active,
        committed.meta().id().clone(),
        committed.navigation(),
        SourceNodeRef::new(subject),
        SourceNodeRef::new(requirement),
        AtomRevisionRef::new(atom_id, revision),
    )
    .map_err(|_| BindError::IncompleteLink(committed.meta().id().as_str().to_owned()))?;
    Ok(Some(SuccessorBinding {
        record: successor.meta().id().as_str().to_owned(),
        digest: successor.record().digest(),
        address: successor.record().canonical_text(),
        object_bytes: canonical_binding_record(&successor),
    }))
}

/// Reads the committed object one address names, when it is valid.
///
/// The lookup is exact: one address derives exactly one path under the
/// configured store, and no other saved byte can answer for it. Searching the
/// snapshot for any file that parses would let an object planted outside the
/// configured store supply a record.
fn committed_object(
    snapshot: &ChelisSnapshot,
    store: &RepositoryPath,
    address: SemanticDigest,
    active: ActiveAlgorithm,
) -> Option<ImplementationBinding> {
    let object_path = binding_object_repository_path(store.as_str(), address);
    let bytes = snapshot.artifact(&object_path)?;
    let canonical = RepositoryPath::new(&object_path).ok()?;
    let stored = extract_binding_in_mount(active, Some(store), &canonical, bytes).stored?;
    Some(stored.record().clone())
}

/// Returns the current revision of one atom under an explicit member.
fn current_revision(
    snapshot: &ChelisSnapshot,
    atom_id: &str,
    active: ActiveAlgorithm,
) -> Option<SemanticDigest> {
    for (_, dialect, bytes) in snapshot.files() {
        let Ok(extraction) = extract(dialect, bytes) else {
            continue;
        };
        for record in &extraction.records {
            if record.kind != "authority"
                || record.fields.get("id").map(String::as_str) != Some(atom_id)
            {
                continue;
            }
            if let Ok(crate::map::MappedValue::Atom(atom)) =
                crate::map::map_atom_with(active, record)
            {
                return Some(atom.revision());
            }
        }
    }
    None
}

fn apply(root: &Path, plan: &BindPlan) -> Result<(), BindError> {
    let source_path = root.join(&plan.path);
    let mut bytes = std::fs::read(&source_path)
        .map_err(|error| BindError::Write(format!("{}:{error}", plan.path)))?;
    if bytes.get(plan.block_start..plan.block_end) != Some(plan.block_bytes.as_slice()) {
        return Err(BindError::Diverged(plan.path.clone()));
    }
    let object = root.join(&plan.object_path);
    if let Ok(existing) = std::fs::read(&object) {
        if existing != plan.object_bytes.as_bytes() {
            return Err(BindError::ObjectCollision(plan.object_path.clone()));
        }
    }
    if let Some(parent) = object.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|error| BindError::Write(format!("{}:{error}", plan.object_path)))?;
    }
    std::fs::write(&object, plan.object_bytes.as_bytes())
        .map_err(|error| BindError::Write(format!("{}:{error}", plan.object_path)))?;
    bytes.splice(plan.block_start..plan.block_end, plan.marker.bytes());
    std::fs::write(&source_path, bytes)
        .map_err(|error| BindError::Write(format!("{}:{error}", plan.path)))?;
    Ok(())
}

/// Plans and optionally applies one sanctioned conversion.
///
/// # Errors
///
/// Refuses unknown or incomplete links, unconfigured stores, parse rejections,
/// non-collision-resistant members, destination collisions, and write failures.
pub fn bind(
    root: &Path,
    configuration: &ChelisConfiguration,
    link_id: &str,
    write: bool,
) -> Result<BindOutcome, BindError> {
    let store = configuration
        .binding_store
        .as_ref()
        .ok_or(BindError::StoreUnconfigured)?;
    let snapshot = ChelisSnapshot::from_directory(root, configuration)
        .map_err(|error| BindError::Write(format!("snapshot:{error:?}")))?;
    let plan = plan_for(&snapshot, store, link_id)?;
    if write {
        apply(root, &plan)?;
    }
    let report = serde_json::to_string(&BindReport {
        schema: "chelis-bind-plan/v1",
        plan: &plan,
        written: write,
    })
    .map_err(|_| BindError::Command(CommandError::Encoding))?;
    Ok(BindOutcome {
        report,
        written: write,
    })
}
