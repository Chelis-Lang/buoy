//! Seeded Hegel properties for logical mounts and artifact sidecars.

#![allow(clippy::expect_used)]

use buoy::adapters::extract_artifact;
use buoy::core::{
    ActiveAlgorithm, ArtifactBindingTransactionStage, CanonicalValue, LogicalMount, LogicalMountId,
    RepositoryPath, StableId, artifact_binding_transaction_state, binding_attachment_key,
    canonical_logical_mounts, project_logical_path, semantic_digest_observation_with,
};
use buoy_reference::{
    BindingTransactionStage as ReferenceStage, attachment_key_preimage, binding_transaction_state,
    logical_mount_set, logical_projection,
};
use hegel::generators as gs;
use hegel::{Hegel, TestCase};

#[path = "support/hegel_settings.rs"]
mod hegel_settings;
use hegel_settings::settings;

#[test]
fn independent_reference_controls_cover_path_casefold_and_cleanup_edges() {
    for (prefix, relative) in [
        ("/absolute", "file"),
        ("project", "/absolute"),
        ("project", "two//segments"),
        ("project", "reserved/*"),
    ] {
        assert!(logical_projection(prefix, relative).is_err());
    }
    assert!(logical_mount_set(&[("", "project")]).is_err());
    assert!(logical_mount_set(&[("good.UPPER", "project")]).is_err());
    assert!(logical_mount_set(&[("same", "project"), ("same", "authority")]).is_err());
    assert!(logical_mount_set(&[("first", "source/specs"), ("second", "source")]).is_err());
    for (left, right) in [
        ("Straße", "STRASSE"),
        ("ος", "οσ"),
        ("ﬀ", "ff"),
        ("İ", "i\u{307}"),
    ] {
        assert!(logical_mount_set(&[("first", left), ("second", right)]).is_err());
    }
    for stage in [
        ReferenceStage::Validate,
        ReferenceStage::CreateObjectDirectory,
        ReferenceStage::CreateObjectTemporary,
        ReferenceStage::WriteObject,
        ReferenceStage::SyncObject,
        ReferenceStage::ReplaceObject,
        ReferenceStage::CreateAttachmentDirectory,
        ReferenceStage::CreateAttachmentTemporary,
        ReferenceStage::WriteAttachment,
        ReferenceStage::SyncAttachment,
        ReferenceStage::ReplaceAttachment,
        ReferenceStage::RemoveUncommittedObject,
        ReferenceStage::RetryCleanup,
    ] {
        let state = binding_transaction_state(true, Some(b"prior"), b"next", Some(stage));
        assert!(state.object_present);
        assert_eq!(state.attachment.as_deref(), Some(b"prior".as_slice()));
    }
}

fn mount(id: &str, prefix: &str) -> LogicalMount {
    LogicalMount::new(
        LogicalMountId::new(id).expect("property mount ID"),
        RepositoryPath::new(prefix).expect("property mount prefix"),
    )
}

#[test]
fn hegel_mount_permutations_match_the_independent_model() {
    Hegel::new(|test_case: TestCase| {
        let reverse = test_case.draw(gs::booleans());
        let mut production = vec![mount("authority", "authority"), mount("project", "project")];
        let mut reference = vec![("authority", "authority"), ("project", "project")];
        if reverse {
            production.reverse();
            reference.reverse();
        }
        let production = canonical_logical_mounts(production).expect("disjoint production mounts");
        let reference = logical_mount_set(&reference).expect("disjoint reference mounts");
        assert_eq!(
            production
                .iter()
                .map(|mount| (mount.id().as_str(), mount.prefix().as_str()))
                .collect::<Vec<_>>(),
            reference
                .iter()
                .map(|(id, prefix)| (id.as_str(), prefix.as_str()))
                .collect::<Vec<_>>()
        );
    })
    .settings(settings(31))
    .run();
}

#[test]
fn hegel_logical_projection_is_injective_for_disjoint_prefixes() {
    Hegel::new(|test_case: TestCase| {
        let value = test_case.draw(gs::integers::<u8>());
        let relative = format!("src/file-{value}.bin");
        let relative_path = RepositoryPath::new(&relative).expect("generated relative path");
        let project = mount("project", "project");
        let authority = mount("authority", "authority");
        let project_path =
            project_logical_path(&project, &relative_path).expect("projected project path");
        let authority_path =
            project_logical_path(&authority, &relative_path).expect("projected authority path");
        assert_ne!(project_path, authority_path);
        assert_eq!(
            project_path.as_str(),
            logical_projection("project", &relative).expect("reference project path")
        );
        assert_eq!(
            authority_path.as_str(),
            logical_projection("authority", &relative).expect("reference authority path")
        );
    })
    .settings(settings(32))
    .run();
}

#[test]
fn hegel_mount_collisions_match_the_independent_model() {
    Hegel::new(|test_case: TestCase| {
        let choice = test_case.draw(gs::integers::<u8>()) % 3;
        let (left, right) = match choice {
            0 => ("source", "source"),
            1 => ("source", "source/specs"),
            _ => ("Docs", "docs"),
        };
        let production =
            canonical_logical_mounts(vec![mount("first", left), mount("second", right)]);
        let reference = logical_mount_set(&[("first", left), ("second", right)]);
        assert_eq!(production.is_err(), reference.is_err());
    })
    .settings(settings(33))
    .run();
}

#[test]
fn hegel_opaque_byte_identity_is_deterministic_and_sensitive() {
    Hegel::new(|test_case: TestCase| {
        let bytes = test_case.draw(gs::vecs(gs::integers::<u8>()).max_size(128));
        let active = ActiveAlgorithm::build_default().expect("active property member");
        let path = RepositoryPath::new("project/artifact.bin").expect("artifact path");
        let first = extract_artifact(active, &path, &bytes).node.digest();
        let second = extract_artifact(active, &path, &bytes).node.digest();
        assert_eq!(first, second);
        let mut changed = bytes.clone();
        changed.push(0xff);
        assert_ne!(
            first,
            extract_artifact(active, &path, &changed).node.digest()
        );
    })
    .settings(settings(34))
    .run();
}

#[test]
fn hegel_attachment_keys_match_the_independent_preimage() {
    Hegel::new(|test_case: TestCase| {
        let suffix = test_case.draw(gs::integers::<u8>());
        let relation =
            StableId::new(&format!("ATTACHMENT-PROPERTY-{suffix}")).expect("generated relation ID");
        let active = ActiveAlgorithm::build_default().expect("active property member");
        let key = binding_attachment_key(active, &relation).expect("attachment key");
        let observation = semantic_digest_observation_with(
            active,
            "buoy/binding-attachment-key/v1",
            vec![(1, CanonicalValue::Text(relation.as_str().to_owned()))],
        )
        .expect("attachment key observation");
        assert_eq!(key, observation.digest());
        assert_eq!(
            observation.canonical_preimage(),
            attachment_key_preimage(active.algorithm().tag(), relation.as_str())
                .expect("independent attachment preimage")
        );
    })
    .settings(settings(35))
    .run();
}

#[test]
fn hegel_transaction_plan_matches_the_independent_model() {
    const PRODUCTION: [ArtifactBindingTransactionStage; 13] = [
        ArtifactBindingTransactionStage::Validate,
        ArtifactBindingTransactionStage::CreateObjectDirectory,
        ArtifactBindingTransactionStage::CreateObjectTemporary,
        ArtifactBindingTransactionStage::WriteObject,
        ArtifactBindingTransactionStage::SyncObject,
        ArtifactBindingTransactionStage::ReplaceObject,
        ArtifactBindingTransactionStage::CreateAttachmentDirectory,
        ArtifactBindingTransactionStage::CreateAttachmentTemporary,
        ArtifactBindingTransactionStage::WriteAttachment,
        ArtifactBindingTransactionStage::SyncAttachment,
        ArtifactBindingTransactionStage::ReplaceAttachment,
        ArtifactBindingTransactionStage::RemoveUncommittedObject,
        ArtifactBindingTransactionStage::RetryCleanup,
    ];
    const REFERENCE: [ReferenceStage; 13] = [
        ReferenceStage::Validate,
        ReferenceStage::CreateObjectDirectory,
        ReferenceStage::CreateObjectTemporary,
        ReferenceStage::WriteObject,
        ReferenceStage::SyncObject,
        ReferenceStage::ReplaceObject,
        ReferenceStage::CreateAttachmentDirectory,
        ReferenceStage::CreateAttachmentTemporary,
        ReferenceStage::WriteAttachment,
        ReferenceStage::SyncAttachment,
        ReferenceStage::ReplaceAttachment,
        ReferenceStage::RemoveUncommittedObject,
        ReferenceStage::RetryCleanup,
    ];
    Hegel::new(|test_case: TestCase| {
        let selector = usize::from(test_case.draw(gs::integers::<u8>())) % 14;
        let prior_object_present = test_case.draw(gs::booleans());
        let prior_attachment_present = test_case.draw(gs::booleans());
        let prior = prior_attachment_present.then_some(b"prior".as_slice());
        let production_failure = PRODUCTION.get(selector).copied();
        let reference_failure = REFERENCE.get(selector).copied();
        let production = artifact_binding_transaction_state(
            prior_object_present,
            prior,
            b"next",
            production_failure,
        );
        let reference =
            binding_transaction_state(prior_object_present, prior, b"next", reference_failure);
        assert_eq!(production.object_present, reference.object_present);
        assert_eq!(production.attachment, reference.attachment);
        assert_eq!(production.temporary_present, reference.temporary_present);
        assert_eq!(production.committed, reference.committed);
        assert_eq!(production.completed_steps, reference.completed_steps);
    })
    .settings(settings(36))
    .run();
}
