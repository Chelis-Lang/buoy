#![doc = "Structural controls for the project-local Buoy `OpenSpec` workflow."]
#![allow(clippy::expect_used, clippy::panic)]

#[path = "support/traceability_control.rs"]
mod traceability_control;
use traceability_control::{
    TraceabilityFault, validate_active_traceability, validate_traceability,
};

/// Reads one reviewed workflow fixture.
fn workflow_fixture(relative: &str) -> String {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("fixtures/readable-trace-projections/v1")
        .join(relative);
    std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("workflow fixture {}: {error}", path.display()))
}

fn faults(text: &str) -> Vec<&'static str> {
    validate_traceability(text)
        .into_iter()
        .map(TraceabilityFault::name)
        .collect()
}

#[test]
fn the_workflow_control_accepts_complete_planned_and_completed_rows() {
    assert!(faults(&workflow_fixture("workflow/planned-relations.md")).is_empty());
    assert!(faults(&workflow_fixture("workflow/completion-snapshot.md")).is_empty());
}

#[test]
fn the_workflow_control_accepts_empty_planned_and_completion_tables() {
    let empty = "## Planned Implementation Relations\n\n\
| Requirement ID | Relation ID | Consumer | Logical target selector | Action | Owner | Focused Buoy command |\n\
|---|---|---|---|---|---|---|\n\n\
## Completion Snapshot\n\n\
| Requirement ID | Canonical record locator | Consumer commit | Final static result |\n\
|---|---|---|---|\n";
    assert!(
        faults(empty).is_empty(),
        "an empty table records no intent and carries no fault"
    );
}

#[test]
fn the_workflow_control_rejects_every_reviewed_fault() {
    for (relative, expected) in [
        (
            "workflow/negative-fabricated-completion.md",
            "premature-completion",
        ),
        (
            "workflow/negative-placeholder-snapshot.md",
            "placeholder-identity",
        ),
        (
            "workflow/negative-copied-record-body.md",
            "copied-record-body",
        ),
        ("workflow/negative-provider-status.md", "provider-status"),
        ("workflow/negative-incomplete-row.md", "incomplete-row"),
        (
            "workflow/negative-missing-link-review.md",
            "missing-link-review",
        ),
        (
            "workflow/negative-record-locator-in-plan.md",
            "premature-completion",
        ),
    ] {
        let reported = faults(&workflow_fixture(relative));
        assert!(
            reported.contains(&expected),
            "{relative}: the control must report {expected}, found {reported:?}"
        );
    }
}

#[test]
fn generated_evidence_completion_block_is_accepted_alongside_a_human_block() {
    assert!(
        faults(&workflow_fixture(
            "workflow/generated-evidence-completion.md"
        ))
        .is_empty()
    );
}

#[test]
fn generated_evidence_completion_block_rejects_drift_and_authority_claims() {
    let conflict = std::fs::read_to_string("fixtures/trace-publication/v1/completion/conflict.md")
        .expect("conflict completion fixture");
    assert!(faults(&conflict).contains(&"generated-evidence-drift"));

    let false_authority = std::fs::read_to_string(
        "fixtures/trace-publication/v1/completion/false-current-authority.md",
    )
    .expect("false-authority completion fixture");
    assert!(faults(&false_authority).contains(&"current-authority-claim"));

    let empty = std::fs::read_to_string("fixtures/trace-publication/v1/completion/empty-state.md")
        .expect("empty completion fixture");
    assert!(faults(&empty).is_empty());
}

#[test]
fn the_project_template_declares_every_reviewed_section() {
    let template = include_str!("../openspec/schemas/buoy-bridge/templates/traceability.md");
    for required in [
        "## Requirement Actions",
        "## Planned Implementation Relations",
        "## Link Review",
        "## Completion Snapshot",
        "## External Targets",
        "Relation ID",
        "Consumer",
        "Logical target selector",
        "Focused Buoy command",
        "Consumer commit",
        "Review decision",
        "<!-- BEGIN buoy-trace-publication-evidence-v1 -->",
        "<!-- END buoy-trace-publication-evidence-v1 -->",
        "buoy publication evidence",
        "Buoy remains the current-state authority",
        "keep the table empty",
        "Do not copy the immutable record body",
    ] {
        assert!(
            template.contains(required),
            "the project template must declare {required}"
        );
    }
    // The template ships empty planned and completion tables, so copying it
    // cannot carry one placeholder identity into a change artifact.
    assert!(faults(template).is_empty());
    // A change artifact starts as a copy of this template and is ACTIVE from
    // the moment it exists, so the template must satisfy the strict lifecycle
    // too: it must declare BOTH the generated block and the separate human
    // completion block, or every copy would start out already faulted.
    let active = validate_active_traceability(template)
        .into_iter()
        .map(TraceabilityFault::name)
        .collect::<Vec<_>>();
    assert!(
        active.is_empty(),
        "the project template must pass the active control: {active:?}"
    );
    // Both blocks must be structurally present, not merely mentioned in prose.
    let completion = template
        .split_once("## Completion Snapshot")
        .expect("completion section")
        .1;
    assert!(completion.contains("<!-- BEGIN buoy-trace-publication-evidence-v1 -->"));
    assert!(completion.contains("<!-- END buoy-trace-publication-evidence-v1 -->"));
    assert!(completion.contains("| Owner | Consumer commit | Review decision |"));
}

/// Every template mutation must be rejected, so the template assertions above
/// cannot pass because the control is inert.
#[test]
fn the_project_template_fails_the_active_control_once_either_block_is_removed() {
    let template = include_str!("../openspec/schemas/buoy-bridge/templates/traceability.md");
    for (case, mutated, expected) in [
        (
            "heading-deleted",
            template.replace("## Completion Snapshot\n", ""),
            "missing-completion-section",
        ),
        (
            "generated-block-deleted",
            template
                .replace("<!-- BEGIN buoy-trace-publication-evidence-v1 -->\n", "")
                .replace("<!-- END buoy-trace-publication-evidence-v1 -->\n", ""),
            "missing-generated-block",
        ),
        (
            "end-marker-truncated",
            template.replace("<!-- END buoy-trace-publication-evidence-v1 -->\n", ""),
            "unterminated-generated-block",
        ),
        (
            "human-block-deleted",
            template.replace(
                "| Owner | Consumer commit | Review decision |\n|---|---|---|\n",
                "",
            ),
            "missing-human-completion-block",
        ),
    ] {
        assert_ne!(mutated, template, "{case}: the mutation must change bytes");
        let reported = validate_active_traceability(&mutated)
            .into_iter()
            .map(TraceabilityFault::name)
            .collect::<Vec<_>>();
        assert!(
            reported.contains(&expected),
            "{case}: the template control must report {expected}, found {reported:?}"
        );
    }
}

#[test]
fn buoy_bridge_schema_requires_traceability_before_tasks() {
    let schema = include_str!("../openspec/schemas/buoy-bridge/schema.yaml");
    assert!(schema.contains("name: buoy-bridge\nversion: 1"));
    assert!(schema.contains("- id: traceability"));
    assert!(schema.contains("generates: traceability.md"));
    assert!(schema.contains("- id: tasks"));
    assert!(schema.contains("      - traceability"));
    assert!(schema.find("- id: traceability") < schema.find("- id: tasks"));
}

#[test]
fn every_active_traceability_artifact_passes_the_workflow_control() {
    let changes = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("openspec/changes");
    let mut visited = 0_usize;
    for entry in std::fs::read_dir(changes).expect("active changes directory") {
        let entry = entry.expect("change entry");
        if !entry.file_type().expect("change entry type").is_dir() || entry.file_name() == "archive"
        {
            continue;
        }
        let traceability = entry.path().join("traceability.md");
        if !traceability.exists() {
            continue;
        }
        let text = std::fs::read_to_string(&traceability).expect("traceability artifact");
        // An unarchived change is an active workflow artifact. The strict
        // lifecycle requires the marker-delimited generated evidence block.
        let active_faults = validate_active_traceability(&text)
            .into_iter()
            .map(TraceabilityFault::name)
            .collect::<Vec<_>>();
        assert!(
            active_faults.is_empty(),
            "{} failed the active workflow control: {active_faults:?}",
            traceability.display()
        );
        visited += 1;
    }
    if visited == 0 {
        // An empty active directory is valid after archive. Exercise the same
        // strict path with a real artifact that passed it before the move.
        let former_active = real_archived_artifact();
        assert!(
            validate_active_traceability(&former_active).is_empty(),
            "the archived former-active control must pass the strict lifecycle"
        );
        visited = 1;
    }
    assert!(visited > 0, "the workflow control must inspect real bytes");
}

/// Reads this change's archived traceability artifact.
fn real_archived_artifact() -> String {
    let traceability = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(
        "openspec/changes/archive/2026-08-25-add-deterministic-trace-publication-tooling/traceability.md",
    );
    std::fs::read_to_string(&traceability)
        .unwrap_or_else(|error| panic!("{}: {error}", traceability.display()))
}

/// Writes one candidate document to a temporary copy and validates the bytes
/// read back through the REAL active entry point.
///
/// The copy keeps the committed artifact untouched while still proving the
/// control runs over on-disk document bytes rather than over an in-memory
/// convenience value.
fn active_faults_of_temporary_copy(case: &str, text: &str) -> Vec<&'static str> {
    let directory = std::env::temp_dir().join(format!(
        "buoy-active-traceability-{}-{case}",
        std::process::id()
    ));
    std::fs::create_dir_all(&directory).expect("temporary mutation directory");
    let path = directory.join("traceability.md");
    std::fs::write(&path, text).expect("temporary mutated artifact");
    let read_back = std::fs::read_to_string(&path).expect("temporary mutated artifact");
    let faults = validate_active_traceability(&read_back)
        .into_iter()
        .map(TraceabilityFault::name)
        .collect::<Vec<_>>();
    std::fs::remove_dir_all(&directory).expect("temporary mutation cleanup");
    faults
}

/// The active control must bite on real committed artifact bytes, and each
/// completion-structure mutation must be detected under its own identity.
///
/// The unmutated copy is validated in this same test, so every failure below
/// is attributable to its own mutation rather than to a pre-existing fault.
/// The regression this pins: deleting the `## Completion Snapshot` heading
/// used to skip every completion rule -- including the generated-block
/// requirement -- because those rules were scoped to the extracted section.
#[test]
fn each_completion_structure_mutation_of_a_real_active_change_is_detected() {
    let original = real_archived_artifact();
    assert!(
        active_faults_of_temporary_copy("unmutated", &original).is_empty(),
        "the control: this change's own artifact must pass unmutated"
    );

    let heading_deleted = original.replace("## Completion Snapshot\n", "");
    assert!(!heading_deleted.contains("## Completion Snapshot"));
    let generated_deleted = original
        .replace("<!-- BEGIN buoy-trace-publication-evidence-v1 -->\n", "")
        .replace("<!-- END buoy-trace-publication-evidence-v1 -->\n", "");
    let end_truncated = original.replace("<!-- END buoy-trace-publication-evidence-v1 -->\n", "");
    let human_deleted = original.replace(
        "| Owner | Consumer commit | Review decision |\n|---|---|---|\n",
        "",
    );

    for (case, mutated, expected, absent) in [
        (
            "heading-deleted",
            &heading_deleted,
            "missing-completion-section",
            "unterminated-generated-block",
        ),
        (
            "generated-block-deleted",
            &generated_deleted,
            "missing-generated-block",
            "missing-completion-section",
        ),
        (
            "end-marker-truncated",
            &end_truncated,
            "unterminated-generated-block",
            "missing-generated-block",
        ),
        (
            "human-block-deleted",
            &human_deleted,
            "missing-human-completion-block",
            "missing-completion-section",
        ),
    ] {
        assert_ne!(mutated, &original, "{case}: the mutation must change bytes");
        let reported = active_faults_of_temporary_copy(case, mutated);
        assert!(
            reported.contains(&expected),
            "{case}: the active control must report {expected}, found {reported:?}"
        );
        assert!(
            !reported.contains(&absent),
            "{case}: {expected} must stay distinguishable from {absent}, found {reported:?}"
        );
        // An ARCHIVED document legitimately predates this structure, so the
        // tolerant path must keep the same bytes readable. Without this the
        // strict rules could silently be applied to the whole archive.
        let archived = validate_traceability(mutated)
            .into_iter()
            .map(TraceabilityFault::name)
            .collect::<Vec<_>>();
        assert!(
            !archived.contains(&expected),
            "{case}: an archived artifact must stay readable, found {archived:?}"
        );
    }
}

/// Archived legacy four-column completion tables must keep validating through
/// the tolerant path even though the active path now rejects that shape.
#[test]
fn the_archived_path_still_accepts_legacy_four_column_completion_tables() {
    let legacy = workflow_fixture("workflow/completion-snapshot.md");
    assert!(
        !legacy.contains("<!-- BEGIN buoy-trace-publication-evidence-v1 -->"),
        "this fixture must be a genuine pre-marker document"
    );
    assert!(
        faults(&legacy).is_empty(),
        "an archived legacy completion table carries no fault"
    );
    let active = validate_active_traceability(&legacy)
        .into_iter()
        .map(TraceabilityFault::name)
        .collect::<Vec<_>>();
    assert!(
        active.contains(&"missing-generated-block")
            && active.contains(&"missing-human-completion-block"),
        "the two lifecycles must really differ on the same bytes: {active:?}"
    );
}

#[test]
fn traceability_template_requires_complete_link_actions() {
    let template = include_str!("../openspec/schemas/buoy-bridge/templates/traceability.md");
    for required in [
        "Requirement ID",
        "Target class",
        "Action",
        "Owner",
        "Buoy command",
        "Link Review",
        "External Targets",
    ] {
        assert!(
            template.contains(required),
            "missing template field: {required}"
        );
    }
    assert!(template.contains("Do not record provider status"));
}

#[test]
fn project_configuration_selects_local_schema_and_keeps_enforcement_separate() {
    let configuration = include_str!("../openspec/config.yaml");
    assert!(configuration.starts_with("schema: buoy-bridge\n"));
    assert!(configuration.contains("positive and negative Given/When/Then scenarios"));
    assert!(configuration.contains("Do not record provider status"));

    let metadata = include_str!(
        "../openspec/changes/archive/2026-08-10-add-openspec-traceability-integration/.openspec.yaml"
    );
    assert!(metadata.starts_with("schema: buoy-bridge\n"));
    let traceability = include_str!(
        "../openspec/changes/archive/2026-08-10-add-openspec-traceability-integration/traceability.md"
    );
    assert!(traceability.contains("| `OBW-001` |"));
    assert!(traceability.contains("## Link Review"));
    assert!(traceability.contains("## External Targets"));

    let checker = include_str!("../src/check.rs");
    assert!(!checker.contains("openspec/schemas/buoy-bridge"));
    assert!(!checker.contains("traceability.md"));
}
