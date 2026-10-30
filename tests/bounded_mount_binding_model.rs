#![doc = "Bounded exhaustive model for three mounts, four files, two relations, and every transaction failure."]
#![allow(clippy::expect_used, clippy::too_many_lines)]

use buoy::core::{
    ArtifactBindingTransactionStage, LogicalMount, LogicalMountId, RepositoryPath,
    artifact_binding_transaction_state, canonical_logical_mounts, project_logical_path,
};
use buoy_reference::{
    BindingTransactionStage as ReferenceStage, binding_transaction_state, logical_projection,
};
use std::collections::BTreeSet;

const MAX_MOUNTS: usize = 3;
const FILES: usize = 4;
const RELATIONS: usize = 2;
const FAILURE_CHOICES: usize = 14;
const EXPECTED_STATES: usize = 43_904;

const PRODUCTION_STAGES: [ArtifactBindingTransactionStage; 13] = [
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

const REFERENCE_STAGES: [ReferenceStage; 13] = [
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

fn mounts(count: usize) -> Vec<LogicalMount> {
    canonical_logical_mounts(
        (0..count)
            .map(|index| {
                LogicalMount::new(
                    LogicalMountId::new(&format!("mount{}", index + 1)).expect("bounded mount ID"),
                    RepositoryPath::new(&format!("m{}", index + 1)).expect("bounded mount prefix"),
                )
            })
            .collect(),
    )
    .expect("bounded mount declarations")
}

fn assignment(mut value: usize, radix: usize) -> [usize; FILES] {
    let mut result = [0; FILES];
    for mount in &mut result {
        *mount = value % radix;
        value /= radix;
    }
    result
}

#[test]
fn exhaustive_three_mount_four_file_two_relation_transaction_model() {
    assert_eq!(RELATIONS, 2);
    let mut states = 0_usize;
    for mount_count in 1..=MAX_MOUNTS {
        let mounts = mounts(mount_count);
        let assignments = mount_count.pow(4);
        for assignment_value in 0..assignments {
            let assigned = assignment(assignment_value, mount_count);
            for duplicate_relative_path in [false, true] {
                let mut production_paths = BTreeSet::new();
                let mut reference_paths = BTreeSet::new();
                for file in 0..FILES {
                    let relative = if duplicate_relative_path && file == FILES - 1 {
                        String::from("files/file-0.bin")
                    } else {
                        format!("files/file-{file}.bin")
                    };
                    let relative_path =
                        RepositoryPath::new(&relative).expect("bounded relative path");
                    production_paths.insert(
                        project_logical_path(&mounts[assigned[file]], &relative_path)
                            .expect("bounded production projection")
                            .as_str()
                            .to_owned(),
                    );
                    reference_paths.insert(
                        logical_projection(mounts[assigned[file]].prefix().as_str(), &relative)
                            .expect("bounded reference projection"),
                    );
                }
                assert_eq!(production_paths, reference_paths);
                assert_eq!(
                    production_paths.len() != FILES,
                    duplicate_relative_path && assigned[0] == assigned[FILES - 1]
                );

                for relation_state in 0..(1 << RELATIONS) {
                    for object_state in 0..(1 << RELATIONS) {
                        for failure in 0..FAILURE_CHOICES {
                            for relation in 0..RELATIONS {
                                let prior = (relation_state & (1 << relation) != 0)
                                    .then_some(b"prior".as_slice());
                                let prior_object_present = object_state & (1 << relation) != 0;
                                let production = artifact_binding_transaction_state(
                                    prior_object_present,
                                    prior,
                                    b"next",
                                    PRODUCTION_STAGES.get(failure).copied(),
                                );
                                let reference = binding_transaction_state(
                                    prior_object_present,
                                    prior,
                                    b"next",
                                    REFERENCE_STAGES.get(failure).copied(),
                                );
                                assert_eq!(production.object_present, reference.object_present);
                                assert_eq!(production.attachment, reference.attachment);
                                assert_eq!(
                                    production.temporary_present,
                                    reference.temporary_present
                                );
                                assert_eq!(production.committed, reference.committed);
                                assert_eq!(production.completed_steps, reference.completed_steps);
                            }
                            states += 1;
                        }
                    }
                }
            }
        }
    }
    assert_eq!(states, EXPECTED_STATES);
}
