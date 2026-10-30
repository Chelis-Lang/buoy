//! Rebind and trace integration for compact content-addressed bindings.
//!
//! Objects are immutable, so a stale binding regenerates as a successor
//! object plus one in-source address update, and the same reviewed plan
//! removes the superseded object. Trace expands a binding through the store
//! with every hop recomputed from saved bytes.
//!
//! Content addressing requires a collision-resistant member, so this suite
//! exists only in a build with exactly one collision-resistant member.

#![cfg(all(
    any(feature = "blake3-256", feature = "sha-256"),
    not(feature = "xxh3-128")
))]
#![allow(clippy::expect_used, clippy::panic)]

use chelis_provenance::bind::bind;
use chelis_provenance::command::{EXIT_OK, EXIT_STATIC, binding_digest, static_check};
use chelis_provenance::rebind::{RebindError, rebind};
use chelis_provenance::snapshot::{ChelisConfiguration, ChelisSnapshot};
use chelis_provenance::trace::trace_projection;
use std::fs;
use std::path::Path;

const STORE: &str = "chelis/objects";
const STATEMENT: &str = "The regenerated subject MUST stay addressed by content.";
const REVISED: &str = "The regenerated subject MUST stay addressed by exact content.";
const DECLARATION: &str = "def regen_impl(x: f32) -> f32 = x";

fn configuration() -> ChelisConfiguration {
    ChelisConfiguration::parse(
        format!(
            "schema = \"chelis-adapter-config/v1\"\nbinding-store = \"{STORE}\"\n\n[[selector]]\nroot = \"chelis\"\ndialect = \"surf\"\n"
        )
        .as_bytes(),
    )
    .expect("configuration")
}

fn active() -> buoy::core::ActiveAlgorithm {
    chelis_provenance::command::binding_algorithm()
        .expect("a collision-resistant build addresses content")
}

fn source_for(statement: &str) -> String {
    let atom = buoy::core::Atom::new_with(
        active(),
        buoy::core::StableId::new("REGEN-ATOM").expect("atom id"),
        buoy::core::LifecycleState::Proposed,
        buoy::core::AtomKind::Behavioral,
        vec![buoy::core::ScopeId::new("chelis-core").expect("scope")],
        statement,
        None,
    )
    .expect("atom value");
    let item = chelis_provenance::command::binding_digest(active(), DECLARATION.as_bytes())
        .expect("item digest")
        .canonical_text();
    format!(
        "-- chelis:provenance/v1 authority\n-- id = REGEN-ATOM\n-- kind = behavioral\n-- scopes = chelis-core\n-- statement = {statement}\ndef regen_subject(x: f32) -> f32 = x\n\
         -- chelis:provenance/v1 implementation-link\n-- id = REGEN-LINK\n-- atoms = REGEN-ATOM@{}\n-- item-digest = {item}\n-- navigation = implements\n{DECLARATION}\n",
        atom.revision().canonical_text(),
    )
}

/// Writes the tree and converts its inline link into a compact binding.
fn converted_tree(root: &Path) {
    fs::create_dir_all(root.join("chelis")).expect("selector root");
    fs::write(root.join("chelis/source.ch"), source_for(STATEMENT)).expect("write source");
    bind(root, &configuration(), "REGEN-LINK", true).expect("conversion");
}

fn stored_objects(root: &Path) -> Vec<String> {
    let store = root.join(STORE);
    let mut found = Vec::new();
    if !store.exists() {
        return found;
    }
    let mut stack = vec![store];
    while let Some(directory) = stack.pop() {
        for entry in fs::read_dir(&directory).expect("store directory") {
            let path = entry.expect("store entry").path();
            if path.is_dir() {
                stack.push(path);
            } else if let Ok(relative) = path.strip_prefix(root) {
                found.push(relative.to_string_lossy().replace('\\', "/"));
            }
        }
    }
    found.sort();
    found
}

/// Revises the committed statement, leaving the binding stale by review.
fn revise_statement(root: &Path) {
    let path = root.join("chelis/source.ch");
    let current = fs::read_to_string(&path).expect("source bytes");
    let revised = current.replace(
        &format!("-- statement = {STATEMENT}"),
        &format!("-- statement = {REVISED}"),
    );
    assert_ne!(revised, current, "the revision applies");
    fs::write(&path, revised).expect("write revision");
}

/// Reads the trace hops of the single traced binding.
fn binding_hops(report: &str) -> Vec<(String, String, String, bool)> {
    let value = serde_json::from_str::<serde_json::Value>(report).expect("trace JSON");
    let bindings = value["bindings"].as_array().expect("bindings array");
    assert_eq!(bindings.len(), 1, "exactly one traced binding: {report}");
    bindings[0]["hops"]
        .as_array()
        .expect("hops array")
        .iter()
        .map(|hop| {
            (
                hop["hop"].as_str().expect("hop name").to_owned(),
                hop["bound"].as_str().expect("bound value").to_owned(),
                hop["current"].as_str().expect("current value").to_owned(),
                hop["matches"].as_bool().expect("hop verdict"),
            )
        })
        .collect()
}

fn hop<'a>(
    hops: &'a [(String, String, String, bool)],
    name: &str,
) -> &'a (String, String, String, bool) {
    hops.iter()
        .find(|entry| entry.0 == name)
        .unwrap_or_else(|| panic!("hop {name} is projected: {hops:?}"))
}

/// The address the marker carries for the single converted binding.
fn marker_address(root: &Path) -> String {
    fs::read_to_string(root.join("chelis/source.ch"))
        .expect("source bytes")
        .lines()
        .find_map(|line| line.strip_prefix("-- record = ").map(str::to_owned))
        .expect("compact marker address")
}

#[test]
fn converted_binding_starts_current_and_traces() {
    let root = tempfile::tempdir().expect("root");
    converted_tree(root.path());
    let configuration = configuration();
    let snapshot = ChelisSnapshot::from_directory(root.path(), &configuration).expect("snapshot");
    let outcome = static_check(&snapshot, &configuration).expect("static outcome");
    assert_eq!(outcome.exit, EXIT_OK, "{:?}", outcome.diagnostics);

    let trace = trace_projection(&snapshot, &configuration, "REGEN-ATOM").expect("trace outcome");
    assert_eq!(trace.exit, EXIT_OK, "{}", trace.report);
    assert!(
        trace.report.contains("REGEN-LINK"),
        "the trace expands the stored binding: {}",
        trace.report
    );

    // Every hop reports a value recomputed from saved bytes, not a copy of the
    // value it is checking, so the exact expected values are asserted here.
    assert!(
        trace.report.contains("\"verified\":true"),
        "a resolved object is explicitly verified: {}",
        trace.report
    );
    let hops = binding_hops(&trace.report);
    let address = marker_address(root.path());
    let object = hop(&hops, "binding-object");
    assert_eq!(
        (object.1.as_str(), object.2.as_str(), object.3),
        (address.as_str(), address.as_str(), true),
        "the object hop compares the marker address with the parsed record address"
    );

    let subject = binding_digest(active(), DECLARATION.as_bytes())
        .expect("subject digest")
        .canonical_text();
    let subject_hop = hop(&hops, "binding-subject");
    assert_eq!(
        (
            subject_hop.1.as_str(),
            subject_hop.2.as_str(),
            subject_hop.3
        ),
        (subject.as_str(), subject.as_str(), true)
    );

    let authority = format!(
        "-- chelis:provenance/v1 authority\n-- id = REGEN-ATOM\n-- kind = behavioral\n-- scopes = chelis-core\n-- statement = {STATEMENT}\n"
    );
    let requirement = binding_digest(active(), authority.as_bytes())
        .expect("requirement digest")
        .canonical_text();
    let requirement_hop = hop(&hops, "binding-requirement");
    assert_eq!(
        (
            requirement_hop.1.as_str(),
            requirement_hop.2.as_str(),
            requirement_hop.3
        ),
        (requirement.as_str(), requirement.as_str(), true),
        "the requirement hop recomputes the exact authority record block"
    );
}

/// A reviewed statement edit moves the requirement digest and the atom
/// revision together, so the trace must report those hops as mismatched
/// instead of echoing the stored value back.
#[test]
fn a_revised_statement_shows_mismatched_hops() {
    let root = tempfile::tempdir().expect("root");
    converted_tree(root.path());
    revise_statement(root.path());
    let configuration = configuration();
    let snapshot = ChelisSnapshot::from_directory(root.path(), &configuration).expect("snapshot");
    let trace = trace_projection(&snapshot, &configuration, "REGEN-ATOM").expect("trace outcome");
    assert_eq!(trace.exit, EXIT_STATIC, "{}", trace.report);

    let hops = binding_hops(&trace.report);
    let requirement_hop = hop(&hops, "binding-requirement");
    assert!(
        !requirement_hop.3,
        "the requirement hop must not match a revised statement: {requirement_hop:?}"
    );
    assert_ne!(requirement_hop.1, requirement_hop.2);
    let revised_authority = format!(
        "-- chelis:provenance/v1 authority\n-- id = REGEN-ATOM\n-- kind = behavioral\n-- scopes = chelis-core\n-- statement = {REVISED}\n"
    );
    assert_eq!(
        requirement_hop.2,
        binding_digest(active(), revised_authority.as_bytes())
            .expect("revised requirement digest")
            .canonical_text()
    );
    // The declaration did not change, so the subject hop still matches: the
    // hops are independent, not one verdict copied across three rows.
    assert!(hop(&hops, "binding-subject").3);
}

/// A tampered object materializes nothing, so the projection reads its bytes
/// directly and reports the address they now hash to.
#[test]
fn a_tampered_object_projects_a_failed_object_hop() {
    let root = tempfile::tempdir().expect("root");
    converted_tree(root.path());
    let objects = stored_objects(root.path());
    let object = root.path().join(&objects[0]);
    let bytes = fs::read_to_string(&object).expect("object bytes");
    fs::write(&object, bytes.replace("REGEN-ATOM", "OTHER-ATOM")).expect("tamper object");

    let configuration = configuration();
    let snapshot = ChelisSnapshot::from_directory(root.path(), &configuration).expect("snapshot");
    let outcome = static_check(&snapshot, &configuration).expect("static outcome");
    let address = marker_address(root.path());
    let diagnostic = outcome
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == "CHELIS-PROV-BINDING-ADDRESS")
        .expect("a tampered object must dangle");
    assert!(
        diagnostic.context.contains(&format!(
            "bound={address};current={}:",
            active().algorithm().cargo_feature()
        )),
        "the diagnostic names the address the tampered bytes now hash to: {}",
        diagnostic.context
    );

    // The atom the tampered object names still exists in no graph, so the
    // trace of that atom projects the failed hop rather than an empty array.
    let trace = trace_projection(&snapshot, &configuration, "OTHER-ATOM").expect("trace outcome");
    assert_eq!(trace.exit, EXIT_STATIC, "{}", trace.report);
    assert!(
        trace.report.contains("\"verified\":false"),
        "a refused object's fields stay explicitly unverified: {}",
        trace.report
    );
    let hops = binding_hops(&trace.report);
    let object_hop = hop(&hops, "binding-object");
    assert!(
        !object_hop.3,
        "the object hop must report the mismatch: {object_hop:?}"
    );
    assert_eq!(object_hop.1, address, "bound is the in-source address");
    assert_ne!(
        object_hop.2, address,
        "current is the address the bytes recompute to"
    );
}

/// Resolution consults the one path an address derives under the configured
/// store. An object planted anywhere else answers for nothing.
#[test]
fn an_object_outside_the_configured_store_resolves_nothing() {
    let root = tempfile::tempdir().expect("root");
    converted_tree(root.path());
    let objects = stored_objects(root.path());
    let bytes = fs::read_to_string(root.path().join(&objects[0])).expect("object bytes");
    let relocated = root
        .path()
        .join(objects[0].replace("chelis/objects/", "chelis/elsewhere/"));
    fs::create_dir_all(relocated.parent().expect("relocated parent")).expect("relocated root");
    fs::write(&relocated, &bytes).expect("relocate object");
    fs::remove_file(root.path().join(&objects[0])).expect("remove configured object");

    let configuration = configuration();
    let snapshot = ChelisSnapshot::from_directory(root.path(), &configuration).expect("snapshot");
    let outcome = static_check(&snapshot, &configuration).expect("static outcome");
    assert_eq!(outcome.exit, EXIT_STATIC);
    assert!(
        outcome.diagnostics.iter().any(|diagnostic| diagnostic.code
            == "CHELIS-PROV-BINDING-ADDRESS"
            && diagnostic.context.contains("missing-object")),
        "{:?}",
        outcome.diagnostics
    );
    assert!(outcome.graph.implementation_bindings.is_empty());

    // Regeneration reads the same one path, so it derives no successor from a
    // relocated object either and plans nothing.
    let plan = rebind(root.path(), &configuration, false).expect("dry run");
    assert_eq!(plan.entries, 0, "{}", plan.report);
}

/// A successor address is a hash of the successor record, so different bytes
/// at that path mean the store is not content addressed. No plan may write.
#[test]
fn rebind_refuses_a_successor_path_holding_different_bytes() {
    let root = tempfile::tempdir().expect("root");
    converted_tree(root.path());
    revise_statement(root.path());
    let configuration = configuration();
    let plan = rebind(root.path(), &configuration, false).expect("dry run");
    let successor =
        serde_json::from_str::<serde_json::Value>(&plan.report).expect("plan JSON")["entries"]
            .as_array()
            .expect("entries")
            .iter()
            .find(|entry| entry["action"] == "object-write")
            .expect("successor entry")["path"]
            .as_str()
            .expect("successor path")
            .to_owned();

    let squat = root.path().join(&successor);
    fs::create_dir_all(squat.parent().expect("successor parent")).expect("store root");
    fs::write(&squat, "schema = \"buoy/implementation-binding/v1\"\n").expect("squat successor");
    assert_eq!(
        rebind(root.path(), &configuration, false).err(),
        Some(RebindError::ObjectCollision(successor.clone())),
        "planning refuses before any write"
    );
    assert_eq!(
        fs::read_to_string(root.path().join(&successor)).expect("squatting bytes"),
        "schema = \"buoy/implementation-binding/v1\"\n",
        "the squatting bytes survive"
    );
}

#[test]
fn rebind_keeps_an_object_that_another_marker_still_names() {
    let root = tempfile::tempdir().expect("root");
    converted_tree(root.path());
    let path = root.path().join("chelis/source.ch");
    let source = fs::read_to_string(&path).expect("source bytes");
    let marker_start = source
        .find("-- chelis:provenance/v1 binding")
        .expect("compact marker");
    let duplicate = source.get(marker_start..).expect("marker tail");
    let duplicated = format!("{source}{duplicate}");
    let stale_declaration = "def regen_impl(x: f32) -> f32 = neg(x)";
    let partially_stale = duplicated.replacen(DECLARATION, stale_declaration, 1);
    fs::write(&path, partially_stale).expect("write partial staleness");

    let configuration = configuration();
    let plan = rebind(root.path(), &configuration, false).expect("dry run");
    assert!(plan.report.contains("object-write"), "{}", plan.report);
    assert!(
        plan.report.contains("\"action\":\"address\""),
        "{}",
        plan.report
    );
    assert!(
        !plan.report.contains("object-remove"),
        "the current second marker retains the old object: {}",
        plan.report
    );

    rebind(root.path(), &configuration, true).expect("write run");
    assert_eq!(
        stored_objects(root.path()).len(),
        2,
        "the old and successor objects each have one current marker"
    );
}

/// A compact binding keeps the same atom-revision enforcement as an inline link.
#[test]
fn stale_binding_atom_revision_is_rejected() {
    let root = tempfile::tempdir().expect("root");
    converted_tree(root.path());
    let path = root.path().join("chelis/source.ch");
    let current = fs::read_to_string(&path).expect("source bytes");
    let revised = current.replace(
        &format!("-- statement = {STATEMENT}"),
        &format!("-- statement = {REVISED}"),
    );
    assert_ne!(revised, current, "the revision applies");
    fs::write(&path, revised).expect("write revision");
    let configuration = configuration();
    let outcome = static_check(
        &ChelisSnapshot::from_directory(root.path(), &configuration).expect("snapshot"),
        &configuration,
    )
    .expect("static outcome");

    assert_eq!(outcome.exit, EXIT_STATIC, "{:?}", outcome.diagnostics);
    assert!(outcome.diagnostics.iter().any(|diagnostic| {
        diagnostic.code == "SPG-REF-DANGLING-OR-STALE"
            && diagnostic
                .context
                .starts_with("REGEN-LINK;atom=REGEN-ATOM;bound=")
            && diagnostic.context.contains(";current=")
    }));
    assert!(outcome.diagnostics.iter().any(|diagnostic| {
        diagnostic.code == "CHELIS-PROV-BINDING-REQUIREMENT-STALE"
            && diagnostic.context.starts_with("binding:REGEN-LINK;bound=")
            && diagnostic.context.contains(";current=")
    }));
}

#[test]
fn rebind_dry_run_does_not_mutate_saved_bytes() {
    let root = tempfile::tempdir().expect("root");
    converted_tree(root.path());
    revise_statement(root.path());
    let source_path = root.path().join("chelis/source.ch");
    let source_before = fs::read(&source_path).expect("source bytes");
    let objects_before = stored_objects(root.path())
        .into_iter()
        .map(|path| {
            let bytes = fs::read(root.path().join(&path)).expect("object bytes");
            (path, bytes)
        })
        .collect::<Vec<_>>();

    let plan = rebind(root.path(), &configuration(), false).expect("dry run");
    assert!(!plan.written);
    assert!(plan.entries >= 3, "{}", plan.report);
    assert_eq!(
        fs::read(&source_path).expect("source bytes after dry run"),
        source_before
    );
    let objects_after = stored_objects(root.path())
        .into_iter()
        .map(|path| {
            let bytes = fs::read(root.path().join(&path)).expect("object bytes");
            (path, bytes)
        })
        .collect::<Vec<_>>();
    assert_eq!(objects_after, objects_before);
}

#[test]
fn rebind_supersedes_the_object_and_converges() {
    let root = tempfile::tempdir().expect("root");
    converted_tree(root.path());
    let original = stored_objects(root.path());
    assert_eq!(original.len(), 1);
    revise_statement(root.path());

    let configuration = configuration();
    let stale_trace = trace_projection(
        &ChelisSnapshot::from_directory(root.path(), &configuration).expect("stale snapshot"),
        &configuration,
        "REGEN-ATOM",
    )
    .expect("stale trace");
    assert_eq!(
        stale_trace.exit, EXIT_STATIC,
        "a revised statement leaves the binding stale: {}",
        stale_trace.report
    );

    let dry = rebind(root.path(), &configuration, false).expect("dry run");
    assert!(
        dry.entries >= 3,
        "plan covers the object, address, and removal: {}",
        dry.report
    );
    for action in ["object-write", "address", "object-remove"] {
        assert!(
            dry.report.contains(action),
            "the plan names {action}: {}",
            dry.report
        );
    }
    assert_eq!(
        original,
        stored_objects(root.path()),
        "a dry run leaves the store untouched"
    );

    let written = rebind(root.path(), &configuration, true).expect("write run");
    assert!(written.written);
    let after = stored_objects(root.path());
    assert_eq!(
        after.len(),
        1,
        "exactly one current object remains: {after:?}"
    );
    assert_ne!(
        after, original,
        "the successor object replaced its predecessor"
    );

    let empty = rebind(root.path(), &configuration, true).expect("second run");
    assert_eq!(empty.entries, 0, "a second run reports an empty plan");

    let current = static_check(
        &ChelisSnapshot::from_directory(root.path(), &configuration).expect("current snapshot"),
        &configuration,
    )
    .expect("current outcome");
    assert_eq!(current.exit, EXIT_OK, "{:?}", current.diagnostics);
}

#[test]
fn tampered_object_fails_the_trace() {
    let root = tempfile::tempdir().expect("root");
    converted_tree(root.path());
    let objects = stored_objects(root.path());
    let object = root.path().join(&objects[0]);
    let bytes = fs::read_to_string(&object).expect("object bytes");
    fs::write(
        &object,
        bytes.replace("relation = \"implements\"", "relation = \"owns\""),
    )
    .expect("tamper object");

    let configuration = configuration();
    let snapshot = ChelisSnapshot::from_directory(root.path(), &configuration).expect("snapshot");
    let outcome = static_check(&snapshot, &configuration).expect("static outcome");
    assert_eq!(outcome.exit, EXIT_STATIC);
    assert!(
        outcome
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "CHELIS-PROV-BINDING-ADDRESS"),
        "{:?}",
        outcome.diagnostics
    );
    let trace = trace_projection(&snapshot, &configuration, "REGEN-ATOM").expect("trace outcome");
    assert_eq!(trace.exit, EXIT_STATIC, "{}", trace.report);
}

#[test]
fn binding_plans_are_root_independent() {
    let root_a = tempfile::tempdir().expect("root a");
    let root_b = tempfile::tempdir().expect("root b");
    for root in [&root_a, &root_b] {
        converted_tree(root.path());
        revise_statement(root.path());
    }
    let configuration = configuration();
    let plan_a = rebind(root_a.path(), &configuration, false).expect("plan a");
    let plan_b = rebind(root_b.path(), &configuration, false).expect("plan b");
    assert_eq!(
        plan_a.report, plan_b.report,
        "distinct absolute roots produce identical plan bytes"
    );

    for root in [&root_a, &root_b] {
        rebind(root.path(), &configuration, true).expect("write run");
    }
    let trace_a = trace_projection(
        &ChelisSnapshot::from_directory(root_a.path(), &configuration).expect("snapshot a"),
        &configuration,
        "REGEN-ATOM",
    )
    .expect("trace a");
    let trace_b = trace_projection(
        &ChelisSnapshot::from_directory(root_b.path(), &configuration).expect("snapshot b"),
        &configuration,
        "REGEN-ATOM",
    )
    .expect("trace b");
    assert_eq!(trace_a.report, trace_b.report);
    assert_eq!(trace_a.exit, EXIT_OK, "{}", trace_a.report);
}

#[test]
fn unresolved_markers_do_not_create_replacement_objects() {
    for damage in [
        "invalid-address",
        "empty-address",
        "renamed-authority",
        "invalid-object",
    ] {
        let root = tempfile::tempdir().expect("root");
        converted_tree(root.path());
        let source_path = root.path().join("chelis/source.ch");
        let source = fs::read_to_string(&source_path).expect("source");
        let address = marker_address(root.path());
        let object_paths = stored_objects(root.path());
        let object_path = root.path().join(&object_paths[0]);
        let changed = match damage {
            "invalid-address" => source.replace(&address, "garbage"),
            "empty-address" => source.replace(&address, ""),
            "renamed-authority" => source.replace("-- id = REGEN-ATOM", "-- id = OTHER-ATOM"),
            "invalid-object" => {
                fs::write(&object_path, b"not a binding record\n").expect("invalid object");
                source.clone()
            }
            _ => unreachable!("case"),
        };
        fs::write(&source_path, &changed).expect("damaged source");
        let object = fs::read(&object_path).expect("object before");
        let outcome = rebind(root.path(), &configuration(), true).expect("unresolved plan");
        assert_eq!(outcome.entries, 0, "{damage}: {}", outcome.report);
        assert!(!outcome.written);
        assert_eq!(
            fs::read_to_string(&source_path).expect("source after"),
            changed
        );
        assert_eq!(stored_objects(root.path()), object_paths);
        assert_eq!(fs::read(&object_path).expect("object after"), object);
        let checked = static_check(
            &ChelisSnapshot::from_directory(root.path(), &configuration()).expect("snapshot"),
            &configuration(),
        )
        .expect("static result");
        assert_eq!(checked.exit, EXIT_STATIC, "{damage}");
    }
}

#[test]
fn a_successor_used_by_another_marker_survives_the_removal_plan() {
    let root = tempfile::tempdir().expect("root");
    converted_tree(root.path());
    let source_path = root.path().join("chelis/source.ch");
    let first_source = fs::read_to_string(&source_path).expect("first source");
    let first_address = marker_address(root.path());
    let first_objects = stored_objects(root.path());
    let first_object = fs::read(root.path().join(&first_objects[0])).expect("first object");
    let second_declaration = "def regen_impl(x: f32) -> f32 = neg(x)";
    let third_declaration = "def regen_impl(x: f32) -> f32 = add(x, x)";
    fs::write(
        &source_path,
        first_source.replace(DECLARATION, second_declaration),
    )
    .expect("second declaration");
    rebind(root.path(), &configuration(), true).expect("second binding");
    let second_address = marker_address(root.path());
    let second_objects = stored_objects(root.path());
    fs::write(root.path().join(&first_objects[0]), first_object).expect("restore first object");
    let authority = first_source
        .split("-- chelis:provenance/v1 binding")
        .next()
        .expect("authority");
    let source = format!(
        "{authority}-- chelis:provenance/v1 binding\n-- record = {first_address}\n{second_declaration}\n-- chelis:provenance/v1 binding\n-- record = {second_address}\n{third_declaration}\n",
    );
    fs::write(&source_path, &source).expect("two stale markers");
    let plan = rebind(root.path(), &configuration(), false).expect("shared successor plan");
    let report: serde_json::Value = serde_json::from_str(&plan.report).expect("plan JSON");
    let removals = report["entries"]
        .as_array()
        .expect("entries")
        .iter()
        .filter(|entry| entry["action"] == "object-remove")
        .collect::<Vec<_>>();
    assert_eq!(removals.len(), 1, "{}", plan.report);
    assert_eq!(removals[0]["path"], first_objects[0]);
    rebind(root.path(), &configuration(), true).expect("shared successor write");
    let objects = stored_objects(root.path());
    assert_eq!(objects.len(), 2);
    assert!(objects.contains(&second_objects[0]));
    assert!(!objects.contains(&first_objects[0]));
    assert_eq!(
        rebind(root.path(), &configuration(), false)
            .expect("current plan")
            .entries,
        0
    );
    let saved = fs::read_to_string(source_path).expect("current source");
    assert!(saved.contains(&format!(
        "-- record = {second_address}\n{second_declaration}"
    )));
    assert!(saved.ends_with(&format!("{third_declaration}\n")));
}

#[test]
fn library_successor_uses_valid_authority_after_unreadable_selected_sources() {
    let root = tempfile::tempdir().expect("root");
    converted_tree(root.path());
    revise_statement(root.path());
    let source_path = root.path().join("chelis/source.ch");
    let source = fs::read(&source_path).expect("revised source");
    let unsupported = b"-- chelis:provenance/v2 authority\ndef bad(x: f32) -> f32 = x\n";
    fs::write(root.path().join("chelis/a.ch"), unsupported).expect("unsupported source");
    fs::write(root.path().join("chelis/b.ch"), [0xff]).expect("invalid source");
    let objects = stored_objects(root.path());
    let object_before = fs::read(root.path().join(&objects[0])).expect("committed object");
    let configuration = configuration();
    let snapshot = ChelisSnapshot::from_directory(root.path(), &configuration).expect("snapshot");
    let extraction = chelis_provenance::records::extract(
        chelis_provenance::records::ChelisDialect::Surf,
        &source,
    )
    .expect("marker extraction");
    let marker = extraction
        .records
        .iter()
        .find(|record| record.kind == "binding")
        .expect("marker");
    let successor = chelis_provenance::bind::successor_binding(
        &snapshot,
        configuration.binding_store.as_ref().expect("store"),
        marker,
    )
    .expect("successor result")
    .expect("resolved successor");
    let atom = buoy::core::Atom::new_with(
        active(),
        buoy::core::StableId::new("REGEN-ATOM").expect("atom id"),
        buoy::core::LifecycleState::Proposed,
        buoy::core::AtomKind::Behavioral,
        vec![buoy::core::ScopeId::new("chelis-core").expect("scope")],
        REVISED,
        None,
    )
    .expect("revised atom");
    let authority = format!(
        "-- chelis:provenance/v1 authority\n-- id = REGEN-ATOM\n-- kind = behavioral\n-- scopes = chelis-core\n-- statement = {REVISED}\n"
    );
    let expected = buoy::core::ImplementationBinding::new_with(
        active(),
        buoy::core::StableId::new("REGEN-LINK").expect("relation id"),
        buoy::core::NavigationKind::Implements,
        buoy::core::SourceNodeRef::new(
            binding_digest(active(), DECLARATION.as_bytes()).expect("subject"),
        ),
        buoy::core::SourceNodeRef::new(
            binding_digest(active(), authority.as_bytes()).expect("requirement"),
        ),
        buoy::core::AtomRevisionRef::new(atom.meta().id().clone(), atom.revision()),
    )
    .expect("expected binding");
    assert_eq!(successor.address, expected.record().canonical_text());
    assert_eq!(
        successor.object_bytes,
        buoy::adapters::canonical_binding_record(&expected)
    );
    assert_eq!(fs::read(source_path).expect("source after"), source);
    assert_eq!(
        fs::read(root.path().join(&objects[0])).expect("object after"),
        object_before
    );
    assert_eq!(stored_objects(root.path()), objects);
}
