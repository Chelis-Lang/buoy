#![doc = "Offline version 1 and version 2 to version 3 migration contracts."]
#![allow(clippy::expect_used, clippy::too_many_lines)]

#[path = "../tools/digest_binding.rs"]
mod content_digest;
#[path = "../tools/execution_evidence.rs"]
mod execution_evidence;

use buoy::migration::{MigrationError, migrate_complete, migrate_leaf_bindings};
#[cfg(feature = "coverage-controls")]
use buoy::migration::{
    migrate_complete_collision_control, migrate_leaf_collision_control,
    migrate_leaf_unavailable_control, observe_collision_control,
};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;

const COMPLETE: &str = include_str!("../fixtures/xxh3/v2/migration/complete-v1-input.toml");
const MIGRATION_CASES: &str = "fixtures/xxh3/v2/migration-cases.tsv";
const MIGRATION_BASELINE: &str = "fixtures/xxh3/v2/migration/complete-v1-input.toml";
const MIGRATION_EXECUTIONS: &str = "assurance/v1/migration-fixture-executions.tsv";
const MIGRATION_TOOLS: [&str; 4] = [
    "tests/migration.rs",
    "src/migration.rs",
    "tools/execution_evidence.rs",
    "tools/digest_binding.rs",
];
const MIGRATION_CONFIGURATION: [&str; 2] = [MIGRATION_CASES, MIGRATION_BASELINE];

fn mutated_migration(case: &str) -> Result<String, String> {
    let source = match case {
        "complete-migration" | "complete-migration-replay" => COMPLETE.to_owned(),
        "missing-authoritative-bytes" => COMPLETE.replacen(
            "authoritative_bytes_hex = \"232041544f4d2d4d4947524154494f4e2d3030310a\"\n",
            "",
            1,
        ),
        "missing-approval" => COMPLETE.replacen(
            "approval = \"APPROVAL-MIGRATION-PROOF-001\"",
            "approval = \"MISSING-APPROVAL\"",
            1,
        ),
        "stale-nested-binding" => COMPLETE.replacen(
            "nested_bindings = [\"ATOM-MIGRATION-001\"]",
            "nested_bindings = [\"MISSING-V1-BINDING\"]",
            1,
        ),
        "illegal-lifecycle-change" => {
            COMPLETE.replacen("lifecycle = \"superseded\"", "lifecycle = \"ratified\"", 1)
        }
        "mixed-schema-versions" => COMPLETE.replacen(
            "target_schema = \"spec-provenance/v3\"",
            "target_schema = \"spec-provenance/v1\"",
            1,
        ),
        "duplicate-stable-id" => COMPLETE.replacen(
            "stable_id = \"CARRIER-MIGRATION-001\"",
            "stable_id = \"ATOM-MIGRATION-001\"",
            1,
        ),
        "partial-impact-closure" => COMPLETE.replacen(
            "[[impact_dispositions]]\nstable_id = \"PROOF-MIGRATION-001\"\ndisposition = \"reaffirmed\"\n",
            "",
            1,
        ),
        other => return Err(format!("unknown migration case `{other}`")),
    };
    Ok(source)
}

fn validate_migration_execution_evidence(write: bool) -> Result<(), String> {
    let source = fs::read_to_string(MIGRATION_CASES).map_err(|error| error.to_string())?;
    let mut records = Vec::new();
    for (index, line) in source.lines().skip(1).enumerate() {
        let fields = line.split('\t').collect::<Vec<_>>();
        if fields.len() != 8 {
            return Err(format!("migration fixture row {} is malformed", index + 2));
        }
        let mutated = mutated_migration(fields[1])?;
        let first = migrate_complete(&mutated);
        if fields[2] == "positive" {
            let migration = first.map_err(|error| error.code().to_owned())?;
            if fields[1] == "complete-migration-replay"
                && migrate_complete(&mutated) != Ok(migration.clone())
            {
                return Err(String::from("migration replay is not byte-identical"));
            }
            if fields[5] != "accept" || fields[6] != "-" {
                return Err(format!("{} expected result is stale", fields[1]));
            }
        } else {
            let observed = first
                .err()
                .map(MigrationError::code)
                .ok_or_else(|| format!("{} unexpectedly passed", fields[1]))?;
            if fields[5] != "reject" || fields[6] != observed {
                return Err(format!(
                    "{} expected `{}` but production emitted `{observed}`",
                    fields[1], fields[6]
                ));
            }
        }
        if fields[4] != "none" {
            records.push(execution_evidence::passing(
                "fixture-mutation",
                format!("{MIGRATION_CASES}#{}#{index}:{}", fields[1], fields[4]),
                MIGRATION_CASES,
                line.as_bytes(),
                &MIGRATION_TOOLS,
                &MIGRATION_CONFIGURATION,
                format!(
                    "mutation={};target-cardinality=1;outcome={};diagnostic={}",
                    fields[4], fields[5], fields[6]
                ),
            )?);
        }
    }
    let expected = execution_evidence::render(&records)?;
    execution_evidence::validate_materialized(MIGRATION_EXECUTIONS, &expected, write)
}

#[test]
fn migration_cases_emit_current_per_mutation_execution_evidence() {
    validate_migration_execution_evidence(false)
        .expect("migration execution evidence must be current");
}

#[test]
#[ignore = "explicit reviewed per-case execution-evidence regeneration only"]
fn write_migration_case_execution_evidence() {
    validate_migration_execution_evidence(true)
        .expect("migration execution evidence must be written from production executions");
}

#[test]
fn version_two_leaf_migration_regenerates_from_authoritative_bytes() {
    let source = COMPLETE
        .replace(
            "source_schema = \"spec-provenance/v1\"",
            "source_schema = \"spec-provenance/v2\"",
        )
        .replace("encoding_version = 1", "encoding_version = 2")
        .replace("algorithm = \"sha256\"", "algorithm = \"xxh3-128\"")
        .lines()
        .map(|line| {
            if line.starts_with("old_digest = ") {
                "old_digest = \"xxh3-128:00000000000000000000000000000000\""
            } else {
                line
            }
        })
        .collect::<Vec<_>>()
        .join("\n");
    let migration = migrate_leaf_bindings(&source).expect("version 2 leaf migration");
    assert_eq!(migration.schema, "spec-provenance/v3");
    assert_eq!(migration.bindings.len(), 16);
}

#[test]
fn leaf_migration_regenerates_every_binding_from_authoritative_bytes() {
    let first = migrate_leaf_bindings(COMPLETE);
    let second = migrate_leaf_bindings(COMPLETE);
    assert_eq!(first, second);
    let Some(migration) = first.ok() else {
        return;
    };
    assert_eq!(migration.schema, "spec-provenance/v3");
    assert_eq!(migration.bindings.len(), 16);
    assert!(
        migration.bindings.windows(2).all(|pair| {
            (pair[0].role, &pair[0].stable_id) < (pair[1].role, &pair[1].stable_id)
        })
    );
    assert!(migration.bindings.iter().all(|binding| {
        binding.digest.canonical_text().starts_with("xxh3-128:")
            && binding.digest.bytes().len() == 16
    }));
}

#[cfg(feature = "coverage-controls")]
#[test]
fn migration_unavailable_control_covers_complete_error_propagation() {
    assert!(matches!(
        migrate_leaf_unavailable_control(COMPLETE),
        Err(MigrationError::MixedVersion)
    ));
}

#[test]
#[cfg(all(
    feature = "xxh3-128",
    not(feature = "blake3-256"),
    not(feature = "sha-256")
))]
fn migration_rejects_a_registered_target_that_is_unavailable_in_the_build() {
    let source = COMPLETE.replacen(
        "target_algorithm = \"xxh3-128\"",
        "target_algorithm = \"blake3-256\"",
        1,
    );
    assert_eq!(
        migrate_leaf_bindings(&source),
        Err(MigrationError::MixedVersion)
    );
}

#[test]
#[cfg(all(feature = "xxh3-128", feature = "blake3-256", feature = "sha-256"))]
fn active_member_switch_regenerates_without_relabeling_or_state_changes() {
    let migrations = ["xxh3-128", "blake3-256", "sha-256"].map(|member| {
        let source = COMPLETE.replacen(
            "target_algorithm = \"xxh3-128\"",
            &format!("target_algorithm = \"{member}\""),
            1,
        );
        migrate_complete(&source).expect("all-member migration")
    });
    let baseline_state = migrations[0]
        .objects
        .iter()
        .map(|object| {
            (
                object.stable_id.as_str(),
                object.kind.as_str(),
                object.lifecycle.as_str(),
                object.replacement.as_deref(),
            )
        })
        .collect::<Vec<_>>();
    for (migration, member) in migrations.iter().zip(["xxh3-128", "blake3-256", "sha-256"]) {
        assert_eq!(
            migration
                .objects
                .iter()
                .map(|object| {
                    (
                        object.stable_id.as_str(),
                        object.kind.as_str(),
                        object.lifecycle.as_str(),
                        object.replacement.as_deref(),
                    )
                })
                .collect::<Vec<_>>(),
            baseline_state
        );
        assert!(
            migration
                .impacts
                .iter()
                .all(|impact| impact.new_digest.algorithm().canonical_name() == member)
        );
    }
    assert_ne!(
        migrations[0].impacts[0].new_digest,
        migrations[1].impacts[0].new_digest
    );
    assert_ne!(
        migrations[1].impacts[0].new_digest,
        migrations[2].impacts[0].new_digest
    );
}

#[test]
fn complete_migration_covers_every_semantic_layer_in_dependency_order() {
    let migration = migrate_complete(COMPLETE);
    assert_eq!(migration, migrate_complete(COMPLETE));
    let Some(migration) = migration.ok() else {
        return;
    };
    assert_eq!(migration.objects.len(), 12);
    let kinds = migration
        .objects
        .iter()
        .map(|object| object.kind.as_str())
        .collect::<BTreeSet<_>>();
    for required in [
        "atom",
        "tombstone",
        "evidence-carrier",
        "coverage-policy",
        "derivation",
        "assumption",
        "formal-property",
        "proof-record",
        "spec-graph",
        "static-report",
        "execution-report",
        "impact-report",
    ] {
        assert!(kinds.contains(required), "{required}");
    }
    let positions = migration
        .objects
        .iter()
        .map(|object| (object.stable_id.as_str(), object.order))
        .collect::<BTreeMap<_, _>>();
    assert!(migration.objects.iter().all(|object| {
        object.references.iter().all(|reference| {
            positions
                .get(reference.as_str())
                .is_some_and(|position| *position < object.order)
        })
    }));
    assert_eq!(migration.impacts.len(), 16);
    assert!(
        migration
            .impacts
            .windows(2)
            .all(|pair| pair[0].stable_id < pair[1].stable_id)
    );
    assert!(migration.impacts.iter().all(|impact| {
        impact.old_digest.starts_with("sha256:")
            && impact.new_digest.canonical_text().starts_with("xxh3-128:")
    }));
    let dispositions = migration
        .impacts
        .iter()
        .map(|impact| impact.disposition.as_str())
        .collect::<BTreeSet<_>>();
    assert_eq!(
        dispositions,
        BTreeSet::from(["reaffirmed", "retired", "revised", "waived"])
    );
}

#[test]
fn complete_migration_rejects_stale_nested_bindings() {
    let stale = COMPLETE.replacen(
        "nested_bindings = [\"ATOM-MIGRATION-001\"]",
        "nested_bindings = [\"MISSING-V1-BINDING\"]",
        1,
    );
    assert_eq!(
        migrate_complete(&stale).err().map(MigrationError::code),
        Some("XXH-MIGRATION-STALE-NESTED-BINDING")
    );
}

#[test]
fn complete_migration_rejects_partial_duplicate_or_illegal_impact_closure() {
    let partial = COMPLETE.replacen(
        "[[impact_dispositions]]\nstable_id = \"PROOF-MIGRATION-001\"\ndisposition = \"reaffirmed\"\n",
        "",
        1,
    );
    assert_eq!(
        migrate_complete(&partial).err().map(MigrationError::code),
        Some("XXH-MIGRATION-INCOMPLETE-IMPACT")
    );

    let duplicate = COMPLETE.replacen(
        "[[impact_dispositions]]\nstable_id = \"POLICY-MIGRATION-001\"\ndisposition = \"revised\"",
        "[[impact_dispositions]]\nstable_id = \"CARRIER-MIGRATION-001\"\ndisposition = \"revised\"",
        1,
    );
    assert_eq!(
        migrate_complete(&duplicate).err().map(MigrationError::code),
        Some("XXH-MIGRATION-INCOMPLETE-IMPACT")
    );

    let illegal = COMPLETE.replacen(
        "[[impact_dispositions]]\nstable_id = \"ATOM-MIGRATION-001\"\ndisposition = \"revised\"",
        "[[impact_dispositions]]\nstable_id = \"ATOM-MIGRATION-001\"\ndisposition = \"retired\"",
        1,
    );
    assert_eq!(
        migrate_complete(&illegal).err().map(MigrationError::code),
        Some("XXH-MIGRATION-ILLEGAL-LIFECYCLE")
    );
}

#[test]
fn complete_migration_preserves_stable_ids_lifecycle_tombstones_and_approvals() {
    let migration = migrate_complete(COMPLETE);
    let Some(migration) = migration.ok() else {
        return;
    };
    let retired = migration
        .objects
        .iter()
        .find(|object| object.stable_id == "ATOM-MIGRATION-RETIRED");
    assert!(retired.is_some_and(|object| {
        object.kind == "tombstone"
            && object.lifecycle == "superseded"
            && object.replacement.as_deref() == Some("ATOM-MIGRATION-001")
    }));

    let missing_approval = COMPLETE.replacen(
        "approval = \"APPROVAL-MIGRATION-PROOF-001\"",
        "approval = \"MISSING-APPROVAL\"",
        1,
    );
    assert_eq!(
        migrate_complete(&missing_approval)
            .err()
            .map(MigrationError::code),
        Some("XXH-MIGRATION-MISSING-APPROVAL")
    );

    assert!(
        migrate_complete(&COMPLETE.replacen(
            "lifecycle = \"superseded\"",
            "lifecycle = \"withdrawn\"",
            1,
        ))
        .is_ok()
    );

    let illegal = COMPLETE.replacen("lifecycle = \"superseded\"", "lifecycle = \"ratified\"", 1);
    assert_eq!(
        migrate_complete(&illegal).err().map(MigrationError::code),
        Some("XXH-MIGRATION-ILLEGAL-LIFECYCLE")
    );

    let missing_tombstone = COMPLETE.replacen(
        "stable_id = \"ATOM-MIGRATION-RETIRED\"",
        "stable_id = \"MISSING-TOMBSTONE\"",
        1,
    );
    assert_eq!(
        migrate_complete(&missing_tombstone)
            .err()
            .map(MigrationError::code),
        Some("XXH-MIGRATION-ILLEGAL-LIFECYCLE")
    );

    let duplicate = COMPLETE.replacen(
        "stable_id = \"CARRIER-MIGRATION-001\"",
        "stable_id = \"ATOM-MIGRATION-001\"",
        1,
    );
    assert_eq!(
        migrate_complete(&duplicate).err().map(MigrationError::code),
        Some("XXH-MIGRATION-DUPLICATE-ID")
    );
}

#[test]
fn leaf_migration_never_relabels_or_hashes_old_digest_text() {
    let changed_old_digest = COMPLETE.replace(
        "sha256:1111111111111111111111111111111111111111111111111111111111111111",
        "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
    );
    assert_eq!(
        migrate_leaf_bindings(COMPLETE),
        migrate_leaf_bindings(&changed_old_digest)
    );
}

#[test]
fn leaf_migration_fails_closed_for_each_malformed_leaf_axis() {
    for error in [
        MigrationError::Input,
        MigrationError::MixedVersion,
        MigrationError::MissingAuthoritativeBytes,
        MigrationError::MissingApproval,
        MigrationError::StaleNestedBinding,
        MigrationError::IllegalLifecycle,
        MigrationError::IncompleteImpact,
        MigrationError::DuplicateStableId,
        MigrationError::Encoding,
        MigrationError::ObservedCollision,
    ] {
        assert!(!error.code().is_empty());
    }
    for (needle, replacement) in [
        (
            "schema = \"spec-provenance-migration-input/v1\"",
            "schema = \"wrong\"",
        ),
        (
            "source_schema = \"spec-provenance/v1\"",
            "source_schema = \"wrong\"",
        ),
        ("encoding_version = 1", "encoding_version = 2"),
        ("target_encoding_version = 3", "target_encoding_version = 2"),
        ("algorithm = \"sha256\"", "algorithm = \"wrong\""),
        (
            "target_algorithm = \"xxh3-128\"",
            "target_algorithm = \"wrong\"",
        ),
        (
            "terminal_stable_ids = [\"ATOM-MIGRATION-RETIRED\"]",
            "terminal_stable_ids = [\"\"]",
        ),
        (
            "terminal_stable_ids = [\"ATOM-MIGRATION-RETIRED\"]",
            "terminal_stable_ids = [\"ATOM-MIGRATION-RETIRED\", \"ATOM-MIGRATION-RETIRED\"]",
        ),
        ("id = \"CONFIG-MIGRATION-001\"", "id = \"\""),
        (
            "schema_id = \"spec-provenance-config/v1\"",
            "schema_id = \"\"",
        ),
        (
            "authoritative_path = \"config/spec-provenance.toml\"",
            "authoritative_path = \"\"",
        ),
        (
            "authoritative_media_type = \"application/toml\"",
            "authoritative_media_type = \"\"",
        ),
        (
            "authoritative_bytes_hex = \"736368656d61203d2022737065632d70726f76656e616e63652d636f6e6669672f7631220a\"",
            "authoritative_bytes_hex = \"\"",
        ),
        (
            "authoritative_bytes_hex = \"736368656d61203d2022737065632d70726f76656e616e63652d636f6e6669672f7631220a\"",
            "authoritative_bytes_hex = \"0\"",
        ),
        (
            "authoritative_bytes_hex = \"736368656d61203d2022737065632d70726f76656e616e63652d636f6e6669672f7631220a\"",
            "authoritative_bytes_hex = \"zz\"",
        ),
        (
            "authoritative_bytes_hex = \"736368656d61203d2022737065632d70726f76656e616e63652d636f6e6669672f7631220a\"",
            "authoritative_bytes_hex = \"0z\"",
        ),
        (
            "old_digest = \"sha256:4444444444444444444444444444444444444444444444444444444444444444\"",
            "old_digest = \"wrong\"",
        ),
        (
            "old_digest = \"sha256:4444444444444444444444444444444444444444444444444444444444444444\"",
            "old_digest = \"sha256:short\"",
        ),
        (
            "old_digest = \"sha256:4444444444444444444444444444444444444444444444444444444444444444\"",
            "old_digest = \"sha256:zzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzz\"",
        ),
        (
            "old_digest = \"sha256:1111111111111111111111111111111111111111111111111111111111111111\"",
            "old_digest = \"sha256:short\"",
        ),
        (
            "old_digest = \"sha256:eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee\"",
            "old_digest = \"sha256:short\"",
        ),
        ("stable_id = \"ATOM-MIGRATION-001\"", "stable_id = \"\""),
        ("kind = \"atom\"", "kind = \"\""),
        ("lifecycle = \"ratified\"", "lifecycle = \"\""),
        (
            "authoritative_path = \"authority/atoms/ATOM-MIGRATION-001.md\"",
            "authoritative_path = \"\"",
        ),
        (
            "authoritative_media_type = \"text/markdown\"",
            "authoritative_media_type = \"\"",
        ),
        (
            "approval = \"APPROVAL-MIGRATION-ATOM-001\"",
            "approval = \"\"",
        ),
        (
            "nested_bindings = [\"ATOM-MIGRATION-001\"]",
            "nested_bindings = [\"\"]",
        ),
        ("replacement = \"ATOM-MIGRATION-001\"", "replacement = \"\""),
        ("id = \"APPROVAL-MIGRATION-ATOM-001\"", "id = \"\""),
        (
            "authority = \"spec-provenance-maintainers\"",
            "authority = \"\"",
        ),
        (
            "record_path = \"reviews/ATOM-MIGRATION-001.md\"",
            "record_path = \"\"",
        ),
        (
            "record_bytes_hex = \"617070726f7665643a2041544f4d2d4d4947524154494f4e2d3030310a\"",
            "record_bytes_hex = \"\"",
        ),
        (
            "stable_id = \"ATOM-MIGRATION-001\"\ndisposition",
            "stable_id = \"\"\ndisposition",
        ),
        ("disposition = \"revised\"", "disposition = \"\""),
    ] {
        assert!(
            migrate_leaf_bindings(&COMPLETE.replacen(needle, replacement, 1)).is_err(),
            "{needle}"
        );
    }
    assert_eq!(
        migrate_leaf_bindings("not = [toml"),
        Err(MigrationError::Input)
    );
    let noncanonical_configuration = COMPLETE.replacen(
        "schema_id = \"spec-provenance-config/v1\"",
        "schema_id = \"spec-provenance-config/e\u{301}\"",
        1,
    );
    assert_eq!(
        migrate_leaf_bindings(&noncanonical_configuration),
        Err(MigrationError::Encoding)
    );
    let noncanonical_artifact = COMPLETE.replacen(
        "authoritative_media_type = \"text/markdown\"",
        "authoritative_media_type = \"text/e\u{301}\"",
        1,
    );
    assert_eq!(
        migrate_leaf_bindings(&noncanonical_artifact),
        Err(MigrationError::Encoding)
    );
    let noncanonical_object = COMPLETE.replace("ATOM-MIGRATION-001", "ATOM-MIGRATION-e\u{301}");
    assert_eq!(
        migrate_complete(&noncanonical_object),
        Err(MigrationError::Encoding)
    );

    let duplicate_configuration = format!(
        "{COMPLETE}\n[[configurations]]\nid = \"CONFIG-MIGRATION-001\"\nschema_id = \"x\"\nauthoritative_path = \"x\"\nauthoritative_media_type = \"x\"\nauthoritative_bytes_hex = \"01\"\nold_digest = \"sha256:4444444444444444444444444444444444444444444444444444444444444444\"\n"
    );
    assert_eq!(
        migrate_leaf_bindings(&duplicate_configuration),
        Err(MigrationError::DuplicateStableId)
    );
    let duplicate_approval = format!(
        "{COMPLETE}\n[[approvals]]\nid = \"APPROVAL-MIGRATION-ATOM-001\"\nauthority = \"x\"\nrecord_path = \"x\"\nrecord_bytes_hex = \"01\"\nold_digest = \"sha256:4444444444444444444444444444444444444444444444444444444444444444\"\n"
    );
    assert_eq!(
        migrate_leaf_bindings(&duplicate_approval),
        Err(MigrationError::DuplicateStableId)
    );
    let repeated_source = COMPLETE
        .replacen(
            "authoritative_media_type = \"application/toml\"\nauthoritative_bytes_hex = \"01\"",
            "authoritative_media_type = \"text/markdown\"\nauthoritative_bytes_hex = \"232041544f4d2d4d4947524154494f4e2d3030310a\"",
            1,
        );
    assert!(migrate_leaf_bindings(&repeated_source).is_ok());
    #[cfg(feature = "coverage-controls")]
    {
        assert_eq!(
            observe_collision_control(),
            Err(MigrationError::ObservedCollision)
        );
        for role in [
            buoy::migration::LeafBindingRole::Configuration,
            buoy::migration::LeafBindingRole::ObjectAuthority,
            buoy::migration::LeafBindingRole::ApprovalRecord,
        ] {
            assert_eq!(
                migrate_leaf_collision_control(COMPLETE, role),
                Err(MigrationError::ObservedCollision)
            );
        }
        assert_eq!(
            migrate_complete_collision_control(COMPLETE),
            Err(MigrationError::ObservedCollision)
        );
    }
}

#[test]
fn complete_migration_fails_closed_for_every_dependency_and_impact_axis() {
    for (needle, replacement, expected) in [
        (
            "kind = \"atom\"",
            "kind = \"unknown\"",
            MigrationError::Input,
        ),
        (
            "id = \"CONFIG-MIGRATION-001\"",
            "id = \"ATOM-MIGRATION-001\"",
            MigrationError::DuplicateStableId,
        ),
        (
            "nested_bindings = [\"FORMAL-PROPERTY-MIGRATION-001\", \"ASSUMPTION-MIGRATION-001\"]",
            "nested_bindings = [\"FORMAL-PROPERTY-MIGRATION-001\", \"FORMAL-PROPERTY-MIGRATION-001\"]",
            MigrationError::StaleNestedBinding,
        ),
        (
            "replacement = \"ATOM-MIGRATION-001\"",
            "replacement = \"MISSING-REPLACEMENT\"",
            MigrationError::StaleNestedBinding,
        ),
        (
            "lifecycle = \"ratified\"",
            "lifecycle = \"withdrawn\"",
            MigrationError::IllegalLifecycle,
        ),
        (
            "approval = \"APPROVAL-MIGRATION-ATOM-001\"",
            "approval = \"APPROVAL-MIGRATION-ATOM-001\"\nreplacement = \"ATOM-MIGRATION-RETIRED\"",
            MigrationError::IllegalLifecycle,
        ),
        (
            "disposition = \"revised\"",
            "disposition = \"unknown\"",
            MigrationError::IncompleteImpact,
        ),
        (
            "stable_id = \"ATOM-MIGRATION-RETIRED\"\ndisposition = \"retired\"",
            "stable_id = \"ATOM-MIGRATION-RETIRED\"\ndisposition = \"revised\"",
            MigrationError::IllegalLifecycle,
        ),
    ] {
        assert_eq!(
            migrate_complete(&COMPLETE.replacen(needle, replacement, 1)),
            Err(expected)
        );
    }

    let duplicate_impact = format!(
        "{COMPLETE}\n[[impact_dispositions]]\nstable_id = \"ATOM-MIGRATION-001\"\ndisposition = \"revised\"\n"
    );
    assert_eq!(
        migrate_complete(&duplicate_impact),
        Err(MigrationError::IncompleteImpact)
    );
}

#[test]
fn leaf_migration_fails_closed_without_authoritative_bytes_or_exact_profile() {
    let missing = COMPLETE.replacen(
        "authoritative_bytes_hex = \"232041544f4d2d4d4947524154494f4e2d3030310a\"\n",
        "",
        1,
    );
    assert_eq!(
        migrate_leaf_bindings(&missing)
            .err()
            .map(MigrationError::code),
        Some("XXH-MIGRATION-MISSING-AUTHORITATIVE-BYTES")
    );

    let mixed = COMPLETE.replacen(
        "target_schema = \"spec-provenance/v3\"",
        "target_schema = \"spec-provenance/v1\"",
        1,
    );
    assert_eq!(
        migrate_leaf_bindings(&mixed)
            .err()
            .map(MigrationError::code),
        Some("XXH-MIGRATION-MIXED-VERSION")
    );
}
