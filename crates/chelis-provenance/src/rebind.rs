//! Sanctioned regeneration of record-carried digests and revisions.
//!
//! The plan recomputes every mechanical binding from the current graph:
//! item digests, atom revisions in references and surface rows, oracle
//! digest references, and declared runner digests. The dry run never
//! mutates; `--write` splices only `--`-prefixed record field lines, so a
//! declaration is structurally unreachable. Statements, IDs, scopes, and
//! every other human-authored field stay untouched: changing a claim is a
//! human act, and this command only propagates its digest consequences.

use crate::command::{CommandError, artifact_digest, snapshot_digests};
use crate::map::{MappedValue, map_record};
use crate::records::{ChelisDialect, ChelisRecord, extract};
use crate::snapshot::{ChelisConfiguration, ChelisSnapshot};
use buoy::core::SemanticDigest;
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

/// The kind of change one planned entry performs.
///
/// Inline records rewrite one field in place. Compact bindings are content
/// addressed and therefore immutable, so their regeneration writes one
/// successor object, updates the in-source address, and removes exactly the
/// superseded object.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum RebindAction {
    /// Rewrite one inline record field value.
    Field,
    /// Write one successor content-addressed object.
    ObjectWrite,
    /// Update one in-source compact binding address.
    Address,
    /// Remove exactly one superseded object.
    ObjectRemove,
}

/// One planned field rewrite or object-store change.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct RebindEntry {
    /// Change this entry performs.
    pub action: RebindAction,
    /// Canonical repository path of the changed file.
    pub path: String,
    /// Stable record ID.
    pub record: String,
    /// Record field name, or the store role for object entries.
    pub field: String,
    /// Currently recorded value.
    pub bound: String,
    /// Recomputed current value.
    pub current: String,
    #[serde(skip)]
    value_start: usize,
    #[serde(skip)]
    value_end: usize,
    #[serde(skip)]
    object_bytes: Option<String>,
}

#[derive(Serialize)]
struct RebindReport<'a> {
    schema: &'static str,
    entries: &'a [RebindEntry],
    written: bool,
}

/// Complete deterministic rebind outcome.
#[derive(Clone, Debug)]
pub struct RebindOutcome {
    /// Canonical machine-report bytes.
    pub report: String,
    /// Planned entries in deterministic order.
    pub entries: usize,
    /// Whether saved sources changed.
    pub written: bool,
}

/// Deterministic rebind failure.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RebindError {
    /// A command-layer failure.
    Command(CommandError),
    /// A selected source fails the pinned grammar; nothing is rewritten.
    ParseRejected(String),
    /// The rewrite did not converge to an empty plan.
    Diverged,
    /// A filesystem write failed.
    Write(String),
    /// A successor object path already holds different bytes.
    ObjectCollision(String),
    /// One compact binding could not derive its successor object.
    Bind(crate::bind::BindError),
}

struct CurrentValues {
    atoms: BTreeMap<String, SemanticDigest>,
    items: BTreeMap<String, SemanticDigest>,
    oracles: BTreeMap<String, SemanticDigest>,
    file_digests: BTreeMap<String, SemanticDigest>,
}

fn runner_paths(record: &ChelisRecord) -> Vec<String> {
    record
        .fields
        .get("runner-config-paths")
        .map(|paths| {
            paths
                .split(';')
                .map(str::trim)
                .filter(|entry| !entry.is_empty())
                .map(str::to_owned)
                .collect()
        })
        .unwrap_or_default()
}

fn current_values(snapshot: &ChelisSnapshot) -> Result<CurrentValues, RebindError> {
    let file_digests = snapshot_digests(snapshot).map_err(RebindError::Command)?;
    let mut atoms = BTreeMap::new();
    let mut items = BTreeMap::new();
    let mut oracles = BTreeMap::new();
    for (path, dialect, bytes) in snapshot.files() {
        let extraction = extract(dialect, bytes)
            .map_err(|_| RebindError::ParseRejected(path.as_str().to_owned()))?;
        for record in &extraction.records {
            let Some(id) = record.fields.get("id").cloned() else {
                continue;
            };
            items.insert(
                id.clone(),
                artifact_digest(record.declaration_text.as_bytes())
                    .map_err(RebindError::Command)?,
            );
            match record.kind.as_str() {
                "authority" => {
                    if let Ok(MappedValue::Atom(atom)) = map_record(record) {
                        atoms.insert(id, atom.revision());
                    }
                }
                "oracle" => {
                    let mut updated = record.clone();
                    let paths = runner_paths(record);
                    for (index, artifact_path) in paths.iter().enumerate() {
                        let field = if index == 0 {
                            "runner-digest"
                        } else {
                            "runner-configuration-digest"
                        };
                        if let Some(current) = file_digests.get(artifact_path) {
                            updated
                                .fields
                                .insert(field.to_owned(), current.canonical_text());
                        }
                    }
                    if let Ok(MappedValue::Oracle(oracle)) = map_record(&updated) {
                        oracles.insert(id, oracle.meta().digest());
                    }
                }
                _ => {}
            }
        }
    }
    Ok(CurrentValues {
        atoms,
        items,
        oracles,
        file_digests,
    })
}

fn rebound_atoms(value: &str, atoms: &BTreeMap<String, SemanticDigest>) -> String {
    value
        .split(';')
        .map(str::trim)
        .filter(|entry| !entry.is_empty())
        .map(|entry| match entry.split_once('@') {
            Some((id, _)) => match atoms.get(id) {
                Some(revision) => format!("{id}@{}", revision.canonical_text()),
                None => entry.to_owned(),
            },
            None => entry.to_owned(),
        })
        .collect::<Vec<_>>()
        .join(";")
}

fn rebound_rows(value: &str, atoms: &BTreeMap<String, SemanticDigest>) -> String {
    value
        .split(';')
        .map(str::trim)
        .filter(|entry| !entry.is_empty())
        .map(|entry| {
            let mut parts = entry.splitn(3, ':');
            match (parts.next(), parts.next(), parts.next()) {
                (Some(member), Some(disposition), Some(authority)) => {
                    match authority.split_once('@') {
                        Some((id, _)) => match atoms.get(id) {
                            Some(revision) => {
                                format!("{member}:{disposition}:{id}@{}", revision.canonical_text())
                            }
                            None => entry.to_owned(),
                        },
                        None => entry.to_owned(),
                    }
                }
                _ => entry.to_owned(),
            }
        })
        .collect::<Vec<_>>()
        .join(";")
}

fn field_updates(record: &ChelisRecord, values: &CurrentValues) -> Vec<(String, String)> {
    let mut updates = Vec::new();
    let id = record.fields.get("id").cloned().unwrap_or_default();
    if matches!(record.kind.as_str(), "carrier" | "implementation-link") {
        if let Some(current) = values.items.get(&id) {
            updates.push((String::from("item-digest"), current.canonical_text()));
        }
    }
    if matches!(
        record.kind.as_str(),
        "carrier" | "implementation-link" | "formal-property"
    ) {
        if let Some(bound) = record.fields.get("atoms") {
            updates.push((String::from("atoms"), rebound_atoms(bound, &values.atoms)));
        }
    }
    if record.kind == "surface" {
        if let Some(bound) = record.fields.get("rows") {
            updates.push((String::from("rows"), rebound_rows(bound, &values.atoms)));
        }
    }
    if let (Some(oracle_id), Some(_)) = (
        record.fields.get("oracle-id"),
        record.fields.get("oracle-digest"),
    ) {
        if let Some(current) = values.oracles.get(oracle_id) {
            updates.push((String::from("oracle-digest"), current.canonical_text()));
        }
    }
    if record.kind == "oracle" {
        for (index, artifact_path) in runner_paths(record).iter().enumerate() {
            let field = if index == 0 {
                "runner-digest"
            } else {
                "runner-configuration-digest"
            };
            if let Some(current) = values.file_digests.get(artifact_path) {
                updates.push((field.to_owned(), current.canonical_text()));
            }
        }
    }
    updates
}

fn locate_field_value(
    source: &str,
    record: &ChelisRecord,
    field: &str,
) -> Option<(usize, usize, String)> {
    let start = usize::try_from(record.start).ok()?;
    let end = usize::try_from(record.end).ok()?.min(source.len());
    let block = source.get(start..end)?;
    let mut offset = start;
    for line in block.split_inclusive('\n') {
        let text = line.trim_end_matches(['\n', '\r']);
        let trimmed = text.trim_start();
        if let Some(rest) = trimmed.strip_prefix("--") {
            let rest = rest.trim_start();
            if let Some((key, value)) = rest.split_once('=') {
                if key.trim() == field {
                    let value_text = value.trim();
                    if let Some(relative) = text.rfind(value_text) {
                        if value_text.is_empty() {
                            return None;
                        }
                        let value_start = offset + relative;
                        return Some((
                            value_start,
                            value_start + value_text.len(),
                            value_text.to_owned(),
                        ));
                    }
                }
            }
        }
        offset += line.len();
    }
    None
}

fn plan(
    snapshot: &ChelisSnapshot,
    configuration: &ChelisConfiguration,
) -> Result<Vec<RebindEntry>, RebindError> {
    let values = current_values(snapshot)?;
    let mut entries = Vec::new();
    for (path, dialect, bytes) in snapshot.files() {
        if dialect != ChelisDialect::Surf {
            continue;
        }
        let Ok(source) = core::str::from_utf8(bytes) else {
            continue;
        };
        let extraction = extract(dialect, bytes)
            .map_err(|_| RebindError::ParseRejected(path.as_str().to_owned()))?;
        for record in &extraction.records {
            let id = record.fields.get("id").cloned().unwrap_or_default();
            for (field, current) in field_updates(record, &values) {
                let Some((value_start, value_end, bound)) =
                    locate_field_value(source, record, &field)
                else {
                    continue;
                };
                if bound == current {
                    continue;
                }
                entries.push(RebindEntry {
                    action: RebindAction::Field,
                    path: path.as_str().to_owned(),
                    record: id.clone(),
                    field,
                    bound,
                    current,
                    value_start,
                    value_end,
                    object_bytes: None,
                });
            }
        }
    }
    entries.extend(binding_entries(snapshot, configuration)?);
    entries.sort_by(|left, right| {
        (&left.path, left.value_start, &left.field).cmp(&(
            &right.path,
            right.value_start,
            &right.field,
        ))
    });
    Ok(entries)
}

/// Plans the successor object, address update, and removal for each stale
/// compact binding.
///
/// A content-addressed object can never be edited in place, so regeneration
/// is expressed as three reviewable entries whose combined effect leaves the
/// store holding exactly one current object per binding.
fn binding_entries(
    snapshot: &ChelisSnapshot,
    configuration: &ChelisConfiguration,
) -> Result<Vec<RebindEntry>, RebindError> {
    let Some(store) = configuration.binding_store.as_ref() else {
        return Ok(Vec::new());
    };
    let mut entries = Vec::new();
    let mut removals = Vec::new();
    let mut current_references = BTreeMap::<String, usize>::new();
    let mut updated_references = BTreeMap::<String, usize>::new();
    for (path, dialect, bytes) in snapshot.files() {
        if dialect != ChelisDialect::Surf {
            continue;
        }
        let Ok(source) = core::str::from_utf8(bytes) else {
            continue;
        };
        let extraction = extract(dialect, bytes)
            .map_err(|_| RebindError::ParseRejected(path.as_str().to_owned()))?;
        for record in &extraction.records {
            if record.kind != crate::records::BINDING_RECORD_KIND {
                continue;
            }
            let Some((value_start, value_end, bound)) =
                locate_field_value(source, record, crate::records::BINDING_ADDRESS_FIELD)
            else {
                continue;
            };
            let superseded = buoy::core::SemanticDigest::parse_canonical(&bound)
                .ok()
                .map(|digest| {
                    crate::command::binding_object_repository_path(store.as_str(), digest)
                });
            if let Some(path) = &superseded {
                *current_references.entry(path.clone()).or_default() += 1;
            }
            let Some(successor) = crate::bind::successor_binding(snapshot, store, record)
                .map_err(RebindError::Bind)?
            else {
                continue;
            };
            let current = successor.address.clone();
            if bound == current {
                continue;
            }
            if let Some(path) = &superseded {
                *updated_references.entry(path.clone()).or_default() += 1;
            }
            let successor_path =
                crate::command::binding_object_repository_path(store.as_str(), successor.digest);
            // A successor address is a hash of the successor record, so any
            // object already at that path must be that same record. Different
            // bytes there mean the store is not content addressed, and no plan
            // may overwrite them.
            if let Some(existing) = snapshot.artifact(&successor_path) {
                if existing != successor.object_bytes.as_bytes() {
                    return Err(RebindError::ObjectCollision(successor_path));
                }
            }
            entries.push(RebindEntry {
                action: RebindAction::ObjectWrite,
                path: successor_path.clone(),
                record: successor.record.clone(),
                field: String::from("object"),
                bound: bound.clone(),
                current: current.clone(),
                value_start,
                value_end: value_end.min(value_start),
                object_bytes: Some(successor.object_bytes.clone()),
            });
            entries.push(RebindEntry {
                action: RebindAction::Address,
                path: path.as_str().to_owned(),
                record: successor.record.clone(),
                field: String::from(crate::records::BINDING_ADDRESS_FIELD),
                bound: bound.clone(),
                current: current.clone(),
                value_start,
                value_end,
                object_bytes: None,
            });
            if let Some(superseded) = superseded {
                removals.push(RebindEntry {
                    action: RebindAction::ObjectRemove,
                    path: superseded,
                    record: successor.record,
                    field: String::from("object"),
                    bound,
                    current,
                    value_start,
                    value_end: value_start,
                    object_bytes: None,
                });
            }
        }
    }
    // One marker's superseded object can be another marker's successor, so a
    // removal is safe only against every successor the plan writes, not
    // against the single successor that displaced it.
    let successors = entries
        .iter()
        .filter(|entry| entry.action == RebindAction::ObjectWrite)
        .map(|entry| entry.path.clone())
        .collect::<BTreeSet<_>>();
    entries.extend(removals.into_iter().filter(|entry| {
        !successors.contains(&entry.path)
            && current_references.get(&entry.path) == updated_references.get(&entry.path)
    }));
    Ok(entries)
}

fn apply(root: &Path, entries: &[RebindEntry]) -> Result<(), RebindError> {
    // Successor objects land before any address points at them, and a
    // superseded object leaves only after its address stops naming it, so the
    // store never holds a dangling address at any intermediate step.
    for entry in entries
        .iter()
        .filter(|entry| entry.action == RebindAction::ObjectWrite)
    {
        let Some(bytes) = entry.object_bytes.as_deref() else {
            continue;
        };
        let object = root.join(&entry.path);
        if let Ok(existing) = std::fs::read(&object) {
            if existing != bytes.as_bytes() {
                return Err(RebindError::ObjectCollision(entry.path.clone()));
            }
        }
        if let Some(parent) = object.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|error| RebindError::Write(format!("{}:{error}", entry.path)))?;
        }
        std::fs::write(&object, bytes.as_bytes())
            .map_err(|error| RebindError::Write(format!("{}:{error}", entry.path)))?;
    }
    let mut by_path: BTreeMap<&str, Vec<&RebindEntry>> = BTreeMap::new();
    for entry in entries {
        if matches!(
            entry.action,
            RebindAction::ObjectWrite | RebindAction::ObjectRemove
        ) {
            continue;
        }
        by_path.entry(entry.path.as_str()).or_default().push(entry);
    }
    for (path, mut planned) in by_path {
        let absolute = root.join(path);
        let mut bytes = std::fs::read(&absolute)
            .map_err(|error| RebindError::Write(format!("{path}:{error}")))?;
        planned.sort_by_key(|entry| core::cmp::Reverse(entry.value_start));
        for entry in planned {
            if entry.value_end > bytes.len()
                || bytes.get(entry.value_start..entry.value_end) != Some(entry.bound.as_bytes())
            {
                return Err(RebindError::Diverged);
            }
            bytes.splice(entry.value_start..entry.value_end, entry.current.bytes());
        }
        std::fs::write(&absolute, bytes)
            .map_err(|error| RebindError::Write(format!("{path}:{error}")))?;
    }
    for entry in entries
        .iter()
        .filter(|entry| entry.action == RebindAction::ObjectRemove)
    {
        let object = root.join(&entry.path);
        if object.exists() {
            std::fs::remove_file(&object)
                .map_err(|error| RebindError::Write(format!("{}:{error}", entry.path)))?;
        }
    }
    Ok(())
}

/// Plans and optionally applies the sanctioned rebind.
///
/// # Errors
///
/// Refuses on parse rejection, reports write failures, and fails when a
/// write does not converge to an empty plan.
pub fn rebind(
    root: &Path,
    configuration: &ChelisConfiguration,
    write: bool,
) -> Result<RebindOutcome, RebindError> {
    let snapshot = ChelisSnapshot::from_directory(root, configuration)
        .map_err(|error| RebindError::Write(format!("snapshot:{error:?}")))?;
    let entries = plan(&snapshot, configuration)?;
    let written = write && !entries.is_empty();
    if written {
        apply(root, &entries)?;
        let reread = ChelisSnapshot::from_directory(root, configuration)
            .map_err(|error| RebindError::Write(format!("snapshot:{error:?}")))?;
        if !plan(&reread, configuration)?.is_empty() {
            return Err(RebindError::Diverged);
        }
    }
    let report = serde_json::to_string(&RebindReport {
        schema: "chelis-rebind-plan/v2",
        entries: &entries,
        written,
    })
    .map_err(|_| RebindError::Command(CommandError::Encoding))?;
    Ok(RebindOutcome {
        report,
        entries: entries.len(),
        written,
    })
}
