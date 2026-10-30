//! Sanctioned conversion of one inline link into one compact binding.
//!
//! Conversion is mechanical and reviewable: the dry run mutates nothing, the
//! write touches only the selected record block and the object store, the
//! attached declaration stays byte-identical, and the resulting graph carries
//! the same navigation the inline form carried. Conversion also refuses every
//! link it cannot re-encode exactly, because narrowing a claim to fit the
//! stored schema would launder it.
//!
//! Content addressing requires a collision-resistant member, so this suite
//! exists only in a build with exactly one collision-resistant member.

#![cfg(all(
    any(feature = "blake3-256", feature = "sha-256"),
    not(feature = "xxh3-128")
))]
#![allow(clippy::expect_used, clippy::panic)]

use buoy::core::ActiveAlgorithm;
use chelis_provenance::bind::{BindError, bind};
use chelis_provenance::command::{
    ChangeManifest, EXIT_OK, binding_digest, change_check, static_check,
};
use chelis_provenance::snapshot::{ChelisConfiguration, ChelisSnapshot};
use std::fs;
use std::path::Path;

const STORE: &str = "chelis/objects";
const STATEMENT: &str = "The converted subject MUST keep its navigation.";
const DECLARATION: &str = "def convert_impl(x: f32) -> f32 = x";

fn active() -> ActiveAlgorithm {
    chelis_provenance::command::binding_algorithm()
        .expect("a collision-resistant build addresses content")
}

fn configuration() -> ChelisConfiguration {
    ChelisConfiguration::parse(
        format!(
            "schema = \"chelis-adapter-config/v1\"\nbinding-store = \"{STORE}\"\n\n[[selector]]\nroot = \"chelis\"\ndialect = \"surf\"\n"
        )
        .as_bytes(),
    )
    .expect("configuration")
}

/// Renders one repository whose single link carries the given link fields.
fn source_with(atoms: &str, item: &str, navigation: &str) -> String {
    format!(
        "-- chelis:provenance/v1 authority\n-- id = CONVERT-ATOM\n-- kind = behavioral\n-- scopes = chelis-core\n-- statement = {STATEMENT}\ndef convert_subject(x: f32) -> f32 = x\n\
         -- chelis:provenance/v1 implementation-link\n-- id = CONVERT-LINK\n-- atoms = {atoms}\n-- item-digest = {item}\n-- navigation = {navigation}\n{DECLARATION}\n"
    )
}

fn bound_revision() -> String {
    buoy::core::Atom::new_with(
        active(),
        buoy::core::StableId::new("CONVERT-ATOM").expect("atom id"),
        buoy::core::LifecycleState::Proposed,
        buoy::core::AtomKind::Behavioral,
        vec![buoy::core::ScopeId::new("chelis-core").expect("scope")],
        STATEMENT,
        None,
    )
    .expect("atom value")
    .revision()
    .canonical_text()
}

fn current_item() -> String {
    binding_digest(active(), DECLARATION.as_bytes())
        .expect("item digest")
        .canonical_text()
}

/// Writes one repository whose single link is inline and current.
fn write_tree(root: &Path) {
    let source = source_with(
        &format!("CONVERT-ATOM@{}", bound_revision()),
        &current_item(),
        "implements",
    );
    write_source(root, &source);
}

fn write_source(root: &Path, source: &str) {
    fs::create_dir_all(root.join("chelis")).expect("selector root");
    fs::write(root.join("chelis/source.ch"), source).expect("write source");
}

fn source_text(root: &Path) -> String {
    fs::read_to_string(root.join("chelis/source.ch")).expect("source bytes")
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

#[test]
fn dry_run_reports_the_plan_without_mutating() {
    let root = tempfile::tempdir().expect("root");
    write_tree(root.path());
    let before = source_text(root.path());
    let outcome = bind(root.path(), &configuration(), "CONVERT-LINK", false).expect("dry run");
    assert!(!outcome.written);
    assert!(
        outcome.report.contains("\"written\":false"),
        "{}",
        outcome.report
    );
    assert!(
        outcome.report.contains("CONVERT-LINK"),
        "the plan names the converted link: {}",
        outcome.report
    );
    assert_eq!(before, source_text(root.path()), "a dry run cannot mutate");
    assert!(
        stored_objects(root.path()).is_empty(),
        "a dry run writes no object"
    );
}

#[test]
fn write_converts_and_the_static_check_accepts() {
    let root = tempfile::tempdir().expect("root");
    write_tree(root.path());
    let configuration = configuration();
    let before = static_check(
        &ChelisSnapshot::from_directory(root.path(), &configuration).expect("before snapshot"),
        &configuration,
    )
    .expect("before outcome");
    assert_eq!(before.graph.implementation_links.len(), 1);

    let outcome = bind(root.path(), &configuration, "CONVERT-LINK", true).expect("write run");
    assert!(outcome.written);
    let objects = stored_objects(root.path());
    assert_eq!(objects.len(), 1, "exactly one object exists: {objects:?}");
    assert!(
        objects[0].starts_with(&format!(
            "{STORE}/buoy/objects/v1/{}/",
            active().algorithm().cargo_feature()
        )),
        "the object follows the frozen upstream layout: {objects:?}"
    );

    let after_source = source_text(root.path());
    assert!(
        after_source.contains("-- chelis:provenance/v1 binding"),
        "the compact marker replaces the inline record: {after_source}"
    );
    assert!(
        !after_source.contains("implementation-link"),
        "no inline link remains: {after_source}"
    );
    assert!(
        after_source.contains(DECLARATION),
        "the declaration survives: {after_source}"
    );

    let after = static_check(
        &ChelisSnapshot::from_directory(root.path(), &configuration).expect("after snapshot"),
        &configuration,
    )
    .expect("after outcome");
    assert_eq!(after.exit, EXIT_OK, "{:?}", after.diagnostics);
    assert_eq!(
        after.graph.implementation_bindings.len(),
        1,
        "the compact form materializes the same navigation"
    );
    assert!(
        after.graph.implementation_links.is_empty(),
        "the inline link is gone"
    );
    let binding = &after.graph.implementation_bindings[0];
    assert_eq!(binding.meta().id().as_str(), "CONVERT-LINK");
    assert_eq!(binding.atom().id().as_str(), "CONVERT-ATOM");
    assert_eq!(
        binding.atom().revision(),
        before.graph.implementation_links[0].atoms()[0].revision(),
        "the converted binding keeps the exact bound atom revision"
    );
}

#[test]
fn inline_and_compact_forms_cannot_share_one_relation_id() {
    let root = tempfile::tempdir().expect("root");
    write_tree(root.path());
    let inline_source = source_text(root.path());
    bind(root.path(), &configuration(), "CONVERT-LINK", true).expect("conversion");
    let compact_source = source_text(root.path());
    let link_start = inline_source
        .find("-- chelis:provenance/v1 implementation-link")
        .expect("inline link");
    let link_block = inline_source.get(link_start..).expect("inline block");
    write_source(root.path(), &format!("{compact_source}{link_block}"));

    let configuration = configuration();
    let outcome = static_check(
        &ChelisSnapshot::from_directory(root.path(), &configuration).expect("snapshot"),
        &configuration,
    )
    .expect("static outcome");
    assert!(outcome.diagnostics.iter().any(|diagnostic| {
        diagnostic.code == "CHELIS-PROV-BINDING-LINK-DUPLICATE"
            && diagnostic.context == "relation:CONVERT-LINK"
    }));
}

#[test]
fn write_keeps_declarations_and_unrelated_bytes_identical() {
    let root = tempfile::tempdir().expect("root");
    write_tree(root.path());
    let before = source_text(root.path());
    bind(root.path(), &configuration(), "CONVERT-LINK", true).expect("write run");
    let after = source_text(root.path());
    let declarations = |text: &str| {
        text.lines()
            .filter(|line| !line.trim_start().starts_with("--"))
            .map(str::to_owned)
            .collect::<Vec<_>>()
    };
    assert_eq!(
        declarations(&before),
        declarations(&after),
        "every declaration byte survives conversion"
    );
    assert!(
        after.contains("-- chelis:provenance/v1 authority"),
        "unrelated records survive: {after}"
    );
    assert!(
        after.contains(&format!("-- statement = {STATEMENT}")),
        "the authority statement survives verbatim: {after}"
    );
}

#[test]
fn conversion_refuses_a_repeat_and_unknown_ids() {
    let root = tempfile::tempdir().expect("root");
    write_tree(root.path());
    bind(root.path(), &configuration(), "CONVERT-LINK", true).expect("write run");
    let after_first = source_text(root.path());

    // The link no longer exists as an inline record, so a second conversion
    // has nothing to select and must refuse rather than duplicate an object.
    let second = bind(root.path(), &configuration(), "CONVERT-LINK", true);
    assert_eq!(
        second,
        Err(BindError::UnknownLink(String::from("CONVERT-LINK")))
    );
    assert_eq!(
        after_first,
        source_text(root.path()),
        "a refusal mutates nothing"
    );
    assert_eq!(stored_objects(root.path()).len(), 1);

    assert_eq!(
        bind(root.path(), &configuration(), "CONVERT-ATOM", false),
        Err(BindError::UnknownLink(String::from("CONVERT-ATOM"))),
        "a non-link ID is not convertible"
    );

    let missing = tempfile::tempdir().expect("root");
    let source = source_with(
        &format!("UNKNOWN-ATOM@{}", bound_revision()),
        &current_item(),
        "implements",
    );
    write_source(missing.path(), &source);
    assert_eq!(
        bind(missing.path(), &configuration(), "CONVERT-LINK", true),
        Err(BindError::UnknownAtom(String::from("UNKNOWN-ATOM")))
    );
    assert_eq!(source_text(missing.path()), source);
    assert!(stored_objects(missing.path()).is_empty());
}

#[test]
fn conversion_requires_a_configured_store() {
    let root = tempfile::tempdir().expect("root");
    write_tree(root.path());
    let configuration = ChelisConfiguration::parse(
        b"schema = \"chelis-adapter-config/v1\"\n\n[[selector]]\nroot = \"chelis\"\ndialect = \"surf\"\n",
    )
    .expect("configuration");
    assert_eq!(
        bind(root.path(), &configuration, "CONVERT-LINK", false),
        Err(BindError::StoreUnconfigured)
    );

    let source = root.path().join("chelis/source.ch");
    let before = fs::read(&source).expect("source bytes");
    fs::write(root.path().join("chelis/.git"), "gitdir: elsewhere\n").expect("repository boundary");
    let failure = bind(root.path(), &self::configuration(), "CONVERT-LINK", true);
    assert!(
        matches!(&failure, Err(BindError::Write(error)) if error.starts_with("snapshot:")),
        "{failure:?}"
    );
    assert_eq!(fs::read(source).expect("source bytes"), before);
    assert!(stored_objects(root.path()).is_empty());
}

/// The stored schema hardcodes `implements`, so any other navigation would be
/// silently rewritten by conversion rather than re-encoded.
#[test]
fn conversion_refuses_navigation_the_compact_form_cannot_carry() {
    for navigation in ["owns", "consumes", "configures"] {
        let root = tempfile::tempdir().expect("root");
        write_source(
            root.path(),
            &source_with(
                &format!("CONVERT-ATOM@{}", bound_revision()),
                &current_item(),
                navigation,
            ),
        );
        let before = source_text(root.path());
        assert_eq!(
            bind(root.path(), &configuration(), "CONVERT-LINK", true),
            Err(BindError::UnsupportedNavigation(navigation.to_owned())),
            "{navigation} is not convertible"
        );
        assert_eq!(
            before,
            source_text(root.path()),
            "a refusal mutates nothing"
        );
        assert!(
            stored_objects(root.path()).is_empty(),
            "a refusal writes no object"
        );
    }

    let root = tempfile::tempdir().expect("root");
    let source = source_with(
        &format!("CONVERT-ATOM@{}", bound_revision()),
        &current_item(),
        "implements",
    )
    .replace("-- navigation = implements\n", "");
    write_source(root.path(), &source);
    assert_eq!(
        bind(root.path(), &configuration(), "CONVERT-LINK", true),
        Err(BindError::IncompleteLink(String::from("CONVERT-LINK")))
    );
    assert_eq!(source_text(root.path()), source);
    assert!(stored_objects(root.path()).is_empty());
}

/// One stored binding carries exactly one atom revision, so a link over two
/// atoms cannot convert without dropping one of its dependencies.
#[test]
fn conversion_refuses_a_link_over_other_than_one_atom() {
    let revision = bound_revision();
    for (atoms, expected) in [
        (format!("CONVERT-ATOM@{revision};OTHER-ATOM@{revision}"), 2),
        (String::new(), 0),
    ] {
        let root = tempfile::tempdir().expect("root");
        write_source(
            root.path(),
            &source_with(&atoms, &current_item(), "implements"),
        );
        assert_eq!(
            bind(root.path(), &configuration(), "CONVERT-LINK", true),
            Err(BindError::NotExactlyOneAtom(format!(
                "CONVERT-LINK:{expected}"
            ))),
            "{atoms:?} is not convertible"
        );
        assert!(stored_objects(root.path()).is_empty());
    }

    for atoms in ["CONVERT-ATOM", "CONVERT-ATOM@invalid"] {
        let root = tempfile::tempdir().expect("root");
        let source = source_with(atoms, &current_item(), "implements");
        write_source(root.path(), &source);
        assert_eq!(
            bind(root.path(), &configuration(), "CONVERT-LINK", true),
            Err(BindError::IncompleteLink(String::from("CONVERT-LINK")))
        );
        assert_eq!(source_text(root.path()), source);
        assert!(stored_objects(root.path()).is_empty());
    }
}

/// The compact subject is the item digest the inline record already enforced.
/// Recomputing it from current bytes would convert a rejected link into an
/// accepted binding, so a stale inline digest must refuse.
#[test]
fn conversion_refuses_a_stale_item_digest() {
    let root = tempfile::tempdir().expect("root");
    let stale = binding_digest(active(), b"def convert_impl(x: f32) -> f32 = neg(x)")
        .expect("stale item digest")
        .canonical_text();
    write_source(
        root.path(),
        &source_with(
            &format!("CONVERT-ATOM@{}", bound_revision()),
            &stale,
            "implements",
        ),
    );
    assert_eq!(
        bind(root.path(), &configuration(), "CONVERT-LINK", true),
        Err(BindError::StaleItem {
            bound: stale,
            current: current_item(),
        })
    );
    assert!(
        stored_objects(root.path()).is_empty(),
        "a stale link writes no object"
    );

    let foreign = if active().algorithm().cargo_feature() == "blake3-256" {
        "sha-256"
    } else {
        "blake3-256"
    };
    let foreign_digest = format!("{foreign}:{}", "11".repeat(32));
    for (atoms, item) in [
        (format!("CONVERT-ATOM@{foreign_digest}"), current_item()),
        (
            format!("CONVERT-ATOM@{}", bound_revision()),
            foreign_digest.clone(),
        ),
    ] {
        let root = tempfile::tempdir().expect("root");
        let source = source_with(&atoms, &item, "implements");
        write_source(root.path(), &source);
        assert_eq!(
            bind(root.path(), &configuration(), "CONVERT-LINK", true),
            Err(BindError::NotCollisionResistant(format!(
                "digest-member-mismatch;bound={foreign};current={}",
                active().algorithm().cargo_feature()
            )))
        );
        assert_eq!(source_text(root.path()), source);
        assert!(stored_objects(root.path()).is_empty());
    }
}

/// A content address is a hash of the record, so different bytes at the
/// destination mean the store is not content addressed and must not be
/// overwritten. The refusal happens while planning, so a dry run reports it.
#[test]
fn conversion_refuses_a_destination_holding_different_bytes() {
    let root = tempfile::tempdir().expect("root");
    write_tree(root.path());
    let plan = bind(root.path(), &configuration(), "CONVERT-LINK", false).expect("dry run");
    let object_path = serde_json::from_str::<serde_json::Value>(&plan.report)
        .expect("plan JSON")["plan"]["object_path"]
        .as_str()
        .expect("object path")
        .to_owned();
    let object = root.path().join(&object_path);
    fs::create_dir_all(object.parent().expect("object parent")).expect("store root");
    fs::write(&object, "schema = \"buoy/implementation-binding/v1\"\n").expect("squat object");

    assert_eq!(
        bind(root.path(), &configuration(), "CONVERT-LINK", false),
        Err(BindError::ObjectCollision(object_path)),
        "planning refuses before any write"
    );
    assert_eq!(
        fs::read_to_string(&object).expect("object bytes"),
        "schema = \"buoy/implementation-binding/v1\"\n",
        "the squatting bytes survive"
    );

    for object_is_directory in [false, true] {
        let root = tempfile::tempdir().expect("root");
        write_tree(root.path());
        let before = source_text(root.path());
        let plan =
            bind(root.path(), &configuration(), "CONVERT-LINK", false).expect("conversion plan");
        let object_path = serde_json::from_str::<serde_json::Value>(&plan.report)
            .expect("plan JSON")["plan"]["object_path"]
            .as_str()
            .expect("object path")
            .to_owned();
        if object_is_directory {
            fs::create_dir_all(root.path().join(&object_path)).expect("object directory");
        } else {
            fs::write(root.path().join(STORE), b"occupied store").expect("store file");
        }
        let failure = bind(root.path(), &configuration(), "CONVERT-LINK", true);
        assert!(
            matches!(&failure, Err(BindError::Write(error)) if error.starts_with(&object_path)),
            "{failure:?}"
        );
        assert_eq!(source_text(root.path()), before);
        if object_is_directory {
            assert!(root.path().join(&object_path).is_dir());
        } else {
            assert_eq!(
                fs::read(root.path().join(STORE)).expect("store bytes"),
                b"occupied store"
            );
        }
    }
}

/// Conversion changes how a relation is encoded, not what it depends on. The
/// change closure must still name the relation, so a revised atom keeps
/// requiring an explicit disposition after conversion.
#[test]
fn conversion_preserves_the_impact_requirement() {
    let configuration = configuration();
    let root = tempfile::tempdir().expect("root");
    write_tree(root.path());
    bind(root.path(), &configuration, "CONVERT-LINK", true).expect("conversion");
    let outcome_now = || {
        let snapshot =
            ChelisSnapshot::from_directory(root.path(), &configuration).expect("snapshot");
        static_check(&snapshot, &configuration).expect("static outcome")
    };
    let base = outcome_now();
    assert_eq!(base.graph.implementation_bindings.len(), 1);

    // Revise the statement so the atom revision moves. The object stays valid
    // and still materializes, so the closure reads the compact relation.
    let converted = source_text(root.path());
    write_source(
        root.path(),
        &converted.replace(
            &format!("-- statement = {STATEMENT}"),
            "-- statement = The converted subject MUST keep its revised navigation.",
        ),
    );
    let revised = outcome_now();
    assert_eq!(revised.graph.implementation_bindings.len(), 1);
    let manifest = ChangeManifest::parse(
        b"schema = \"chelis-change-manifest/v1\"\n\n[[disposition]]\nobject-kind = \"atom\"\nobject-id = \"CONVERT-ATOM\"\nkind = \"revised\"\n",
    )
    .expect("manifest");
    let change = change_check(&base, &revised, &manifest).expect("change outcome");
    assert!(
        change
            .report
            .contains("implementation-binding:CONVERT-LINK"),
        "the compact relation stays in the impact closure: {}",
        change.report
    );
    assert!(
        change.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "CHELIS-PROV-IMPACT-DISPOSITION-MISSING"
                && diagnostic.context == "implementation-binding:CONVERT-LINK"
        }),
        "the relation still requires its own disposition: {:?}",
        change.diagnostics
    );
}

#[test]
fn conversion_selects_the_requested_link_across_mixed_saved_inputs() {
    let root = tempfile::tempdir().expect("root");
    let source = source_with(
        &format!("CONVERT-ATOM@{}", bound_revision()),
        &current_item(),
        "implements",
    );
    let split = source
        .find("-- chelis:provenance/v1 implementation-link")
        .expect("link block");
    let authority = &source[..split];
    let link = &source[split..];
    fs::create_dir(root.path().join("chelis")).expect("selector root");
    let links = format!("{}{link}", link.replace("CONVERT-LINK", "OTHER-LINK"));
    fs::write(root.path().join("chelis/a.ch"), &links).expect("links");
    let unsupported = b"-- chelis:provenance/v2 authority\ndef bad(x: f32) -> f32 = x\n";
    fs::write(root.path().join("chelis/b.ch"), unsupported).expect("unsupported source");
    fs::write(root.path().join("chelis/c.ch"), [0xff]).expect("invalid UTF-8");
    fs::create_dir(root.path().join("a-deep")).expect("Deep root");
    fs::write(root.path().join("a-deep/a.dp"), unsupported).expect("Deep source");
    fs::write(root.path().join("chelis/z.ch"), authority).expect("authority");
    let config = ChelisConfiguration::parse(
        format!("schema = \"chelis-adapter-config/v1\"\nbinding-store = \"{STORE}\"\n[[selector]]\nroot = \"a-deep\"\ndialect = \"deep\"\n[[selector]]\nroot = \"chelis\"\ndialect = \"surf\"\n").as_bytes(),
    ).expect("mixed configuration");
    bind(root.path(), &config, "CONVERT-LINK", true).expect("selected conversion");
    let after = fs::read_to_string(root.path().join("chelis/a.ch")).expect("converted source");
    assert!(after.starts_with(&link.replace("CONVERT-LINK", "OTHER-LINK")));
    assert_eq!(
        after.matches("-- chelis:provenance/v1 binding\n").count(),
        1
    );
    assert_eq!(
        fs::read(root.path().join("chelis/b.ch")).expect("unsupported bytes"),
        unsupported
    );
    assert_eq!(
        fs::read(root.path().join("chelis/c.ch")).expect("invalid bytes"),
        [0xff]
    );
    assert_eq!(
        fs::read(root.path().join("a-deep/a.dp")).expect("Deep bytes"),
        unsupported
    );
    assert_eq!(
        fs::read_to_string(root.path().join("chelis/z.ch")).expect("authority bytes"),
        authority
    );
    assert_eq!(stored_objects(root.path()).len(), 1);
}

#[test]
fn conversion_refuses_missing_commitments_and_invalid_stable_ids() {
    let original = source_with(
        &format!("CONVERT-ATOM@{}", bound_revision()),
        &current_item(),
        "implements",
    );
    for (source, selected) in [
        (
            original.replace(
                &format!("-- atoms = CONVERT-ATOM@{}\n", bound_revision()),
                "",
            ),
            "CONVERT-LINK",
        ),
        (original.replace(&current_item(), "garbage"), "CONVERT-LINK"),
        (
            original.replace("CONVERT-LINK", "convert-link"),
            "convert-link",
        ),
        (
            original.replace("CONVERT-ATOM", "convert-atom"),
            "CONVERT-LINK",
        ),
    ] {
        let root = tempfile::tempdir().expect("root");
        write_source(root.path(), &source);
        assert_eq!(
            bind(root.path(), &configuration(), selected, true),
            Err(BindError::IncompleteLink(selected.to_owned())),
        );
        assert_eq!(source_text(root.path()), source);
        assert!(stored_objects(root.path()).is_empty());
    }
}

#[test]
fn invalid_utf8_before_a_link_does_not_hide_the_selected_conversion() {
    let root = tempfile::tempdir().expect("root");
    write_tree(root.path());
    fs::write(root.path().join("chelis/a.ch"), [0xff]).expect("invalid source");
    bind(root.path(), &configuration(), "CONVERT-LINK", true).expect("conversion");
    assert_eq!(
        fs::read(root.path().join("chelis/a.ch")).expect("invalid bytes"),
        [0xff]
    );
    assert!(source_text(root.path()).contains("-- chelis:provenance/v1 binding\n"));
    assert_eq!(stored_objects(root.path()).len(), 1);
}
