//! Pure values for repository-owned artifact binding attachments.

use crate::{
    ActiveAlgorithm, CanonicalValue, EncodingError, ImplementationBinding,
    ImplementationBindingRef, RepositoryPath, SemanticDigest, SourceNode, SourceNodeRef, StableId,
    semantic_digest_with,
};
use alloc::vec::Vec;

/// One canonical repository-owned attachment sidecar value.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct ArtifactBindingAttachment {
    relation: StableId,
    artifact: RepositoryPath,
    subject: SourceNodeRef,
    record: ImplementationBindingRef,
    digest: SemanticDigest,
}

impl ArtifactBindingAttachment {
    /// Constructs one attachment with the build-default digest member.
    ///
    /// # Errors
    ///
    /// Returns an unavailable-member or mixed-member error.
    #[allow(clippy::expect_used, clippy::missing_panics_doc)]
    pub fn new(
        relation: StableId,
        artifact: RepositoryPath,
        subject: SourceNodeRef,
        record: ImplementationBindingRef,
    ) -> Result<Self, ArtifactAttachmentError> {
        #[cfg(feature = "xxh3-128")]
        let algorithm = crate::DigestAlgorithm::Xxh3_128;
        #[cfg(all(not(feature = "xxh3-128"), feature = "blake3-256"))]
        let algorithm = crate::DigestAlgorithm::Blake3_256;
        #[cfg(all(
            not(feature = "xxh3-128"),
            not(feature = "blake3-256"),
            feature = "sha-256"
        ))]
        let algorithm = crate::DigestAlgorithm::Sha256;
        #[cfg(not(any(feature = "xxh3-128", feature = "blake3-256", feature = "sha-256")))]
        return Err(ArtifactAttachmentError::DigestMember);
        #[cfg(any(feature = "xxh3-128", feature = "blake3-256", feature = "sha-256"))]
        {
            let active =
                ActiveAlgorithm::new(algorithm).expect("the selected build member is available");
            Self::new_with(active, relation, artifact, subject, record)
        }
    }

    /// Constructs one attachment under an explicit active member.
    ///
    /// # Errors
    ///
    /// Returns a mixed-member error when a referenced digest uses another member.
    #[allow(clippy::expect_used, clippy::missing_panics_doc)]
    pub fn new_with(
        active: ActiveAlgorithm,
        relation: StableId,
        artifact: RepositoryPath,
        subject: SourceNodeRef,
        record: ImplementationBindingRef,
    ) -> Result<Self, ArtifactAttachmentError> {
        if subject.digest().algorithm() != active.algorithm() {
            return Err(ArtifactAttachmentError::DigestMember);
        }
        if record.digest().algorithm() != active.algorithm() {
            return Err(ArtifactAttachmentError::DigestMember);
        }
        let fields = attachment_fields(&relation, &artifact, subject, record);
        let digest = semantic_digest_with(active, "buoy/binding-attachment/v1", fields)
            .expect("validated attachment fields always encode");
        Ok(Self {
            relation,
            artifact,
            subject,
            record,
            digest,
        })
    }

    /// Returns the stable relation ID.
    #[must_use]
    pub const fn relation(&self) -> &StableId {
        &self.relation
    }

    /// Returns the complete logical artifact path.
    #[must_use]
    pub const fn artifact(&self) -> &RepositoryPath {
        &self.artifact
    }

    /// Returns the exact current artifact subject.
    #[must_use]
    pub const fn subject(&self) -> SourceNodeRef {
        self.subject
    }

    /// Returns the selected immutable binding record.
    #[must_use]
    pub const fn record(&self) -> ImplementationBindingRef {
        self.record
    }

    /// Returns the attachment value digest.
    #[must_use]
    pub const fn digest(&self) -> SemanticDigest {
        self.digest
    }

    /// Returns the canonical attachment value.
    #[must_use]
    pub fn canonical_value(&self) -> CanonicalValue {
        CanonicalValue::Record(attachment_fields(
            &self.relation,
            &self.artifact,
            self.subject,
            self.record,
        ))
    }

    /// Returns complete canonical attachment bytes.
    ///
    /// # Errors
    ///
    /// Returns an encoding error only if the closed fields stop encoding.
    pub fn canonical_bytes(&self) -> Result<Vec<u8>, EncodingError> {
        self.canonical_value().encode()
    }
}

/// One deterministic attachment construction or correspondence failure.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum ArtifactAttachmentError {
    /// A digest differs from the active member.
    DigestMember,
    /// Closed canonical fields failed to encode.
    Encoding,
    /// The immutable record locator or relation differs.
    Record,
    /// The record, sidecar, and source node do not name one subject.
    Subject,
    /// The source node has another logical path.
    Path,
    /// The selected node is not an opaque artifact file.
    Kind,
}

/// One planned effect stage in an artifact binding transaction.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum ArtifactBindingTransactionStage {
    /// Validate every input before a write.
    Validate,
    /// Create the immutable-object directory.
    CreateObjectDirectory,
    /// Create the immutable-object temporary file.
    CreateObjectTemporary,
    /// Write the immutable-object bytes.
    WriteObject,
    /// Synchronize the immutable-object temporary file.
    SyncObject,
    /// Replace the immutable-object destination.
    ReplaceObject,
    /// Create the attachment directory.
    CreateAttachmentDirectory,
    /// Create the attachment temporary file.
    CreateAttachmentTemporary,
    /// Write the attachment bytes.
    WriteAttachment,
    /// Synchronize the attachment temporary file.
    SyncAttachment,
    /// Replace the attachment destination.
    ReplaceAttachment,
    /// Remove an uncommitted object during cleanup.
    RemoveUncommittedObject,
    /// Retry one transient cleanup failure.
    RetryCleanup,
}

/// Observable state from the pure artifact binding transaction plan.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ArtifactBindingTransactionState {
    /// Whether an immutable object is present after the modeled operation.
    pub object_present: bool,
    /// Exact current sidecar bytes.
    pub attachment: Option<Vec<u8>>,
    /// Whether a bounded temporary path remains.
    pub temporary_present: bool,
    /// Whether the plan reached its commit point.
    pub committed: bool,
    /// Number of forward stages completed before success or the injected failure.
    pub completed_steps: u8,
}

/// Evaluates the pure transaction plan for one optional one-shot failure.
#[must_use]
#[allow(clippy::match_same_arms)]
pub fn artifact_binding_transaction_state(
    prior_object_present: bool,
    prior_attachment: Option<&[u8]>,
    next_attachment: &[u8],
    failure: Option<ArtifactBindingTransactionStage>,
) -> ArtifactBindingTransactionState {
    const FORWARD: [ArtifactBindingTransactionStage; 11] = [
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
    ];
    let mut state = ArtifactBindingTransactionState {
        object_present: prior_object_present,
        attachment: prior_attachment.map(<[u8]>::to_vec),
        temporary_present: false,
        committed: false,
        completed_steps: 0,
    };
    for stage in FORWARD {
        if failure == Some(stage) {
            state.object_present = prior_object_present;
            state.attachment = prior_attachment.map(<[u8]>::to_vec);
            state.temporary_present = false;
            return state;
        }
        match stage {
            ArtifactBindingTransactionStage::CreateObjectTemporary
            | ArtifactBindingTransactionStage::CreateAttachmentTemporary => {
                state.temporary_present = true;
            }
            ArtifactBindingTransactionStage::ReplaceObject => {
                state.object_present = true;
                state.temporary_present = false;
            }
            ArtifactBindingTransactionStage::ReplaceAttachment => {
                state.attachment = Some(next_attachment.to_vec());
                state.temporary_present = false;
                state.committed = true;
            }
            ArtifactBindingTransactionStage::Validate
            | ArtifactBindingTransactionStage::CreateObjectDirectory
            | ArtifactBindingTransactionStage::WriteObject
            | ArtifactBindingTransactionStage::SyncObject
            | ArtifactBindingTransactionStage::CreateAttachmentDirectory
            | ArtifactBindingTransactionStage::WriteAttachment
            | ArtifactBindingTransactionStage::SyncAttachment
            | ArtifactBindingTransactionStage::RemoveUncommittedObject
            | ArtifactBindingTransactionStage::RetryCleanup => {}
        }
        state.completed_steps += 1;
    }
    match failure {
        Some(ArtifactBindingTransactionStage::RemoveUncommittedObject) => {
            restore_transaction_state(&mut state, prior_object_present, prior_attachment);
        }
        Some(ArtifactBindingTransactionStage::RetryCleanup) => {
            restore_transaction_state(&mut state, prior_object_present, prior_attachment);
        }
        _ => {}
    }
    state
}

fn restore_transaction_state(
    state: &mut ArtifactBindingTransactionState,
    prior_object_present: bool,
    prior_attachment: Option<&[u8]>,
) {
    state.object_present = prior_object_present;
    state.attachment = prior_attachment.map(<[u8]>::to_vec);
    state.temporary_present = false;
    state.committed = false;
}

impl ArtifactAttachmentError {
    /// Returns the stable machine diagnostic code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::DigestMember => "CAB-DIGEST-MEMBER",
            Self::Encoding => "CAB-ATTACHMENT-ENCODING",
            Self::Record => "CAB-ATTACHMENT-RECORD",
            Self::Subject => "CAB-SUBJECT-ATTACHMENT",
            Self::Path => "CAB-ATTACHMENT-PATH",
            Self::Kind => "CAB-SUBJECT-KIND",
        }
    }
}

/// Derives the relation-key digest for one attachment sidecar.
///
/// # Errors
///
/// Returns an encoding error only if the validated relation stops encoding.
pub fn binding_attachment_key(
    active: ActiveAlgorithm,
    relation: &StableId,
) -> Result<SemanticDigest, EncodingError> {
    semantic_digest_with(
        active,
        "buoy/binding-attachment-key/v1",
        alloc::vec![(1, CanonicalValue::Text(relation.as_str().into()))],
    )
}

/// Validates exact sidecar, record, and current artifact correspondence.
///
/// # Errors
///
/// Returns the first stable correspondence failure in record, subject, path, and kind order.
pub fn validate_artifact_attachment(
    attachment: &ArtifactBindingAttachment,
    record: &ImplementationBinding,
    node: &SourceNode,
) -> Result<(), ArtifactAttachmentError> {
    if attachment.record != record.record() || &attachment.relation != record.meta().id() {
        return Err(ArtifactAttachmentError::Record);
    }
    if attachment.subject != record.subject() {
        return Err(ArtifactAttachmentError::Subject);
    }
    if attachment.subject != node.reference() {
        return Err(ArtifactAttachmentError::Subject);
    }
    if node.item().path() != &attachment.artifact {
        return Err(ArtifactAttachmentError::Path);
    }
    if node.item().kind().as_str() != "artifact/file/v1" {
        return Err(ArtifactAttachmentError::Kind);
    }
    Ok(())
}

#[cfg(all(test, feature = "xxh3-128", feature = "sha-256"))]
mod tests {
    use super::{ArtifactAttachmentError, ArtifactBindingAttachment};
    use crate::{
        ActiveAlgorithm, AtomRevisionRef, DigestAlgorithm, ImplementationBinding,
        ImplementationBindingRef, NavigationKind, ParsedItemIdentity, RepositoryPath, SchemaId,
        SemanticDigest, SourceItemKind, SourceNode, SourceNodeRef, StableId,
    };
    use alloc::vec::Vec;

    #[test]
    #[allow(clippy::expect_used)]
    fn explicit_member_rejects_each_mixed_reference() {
        let active = ActiveAlgorithm::new(DigestAlgorithm::Xxh3_128).expect("active member");
        let relation = StableId::new("ATTACHMENT-COVERAGE").expect("relation");
        let artifact = RepositoryPath::new("artifact.bin").expect("artifact path");
        let xxh = SemanticDigest::xxh3_128([1; 16]);
        let sha = SemanticDigest::sha_256([2; 32]);
        assert_eq!(
            ArtifactBindingAttachment::new_with(
                active,
                relation.clone(),
                artifact.clone(),
                SourceNodeRef::new(sha),
                ImplementationBindingRef::new(xxh),
            ),
            Err(ArtifactAttachmentError::DigestMember)
        );
        assert_eq!(
            ArtifactBindingAttachment::new_with(
                active,
                relation,
                artifact,
                SourceNodeRef::new(xxh),
                ImplementationBindingRef::new(sha),
            ),
            Err(ArtifactAttachmentError::DigestMember)
        );

        let artifact = RepositoryPath::new("valid.bin").expect("valid artifact path");
        let item = ParsedItemIdentity::new(
            SchemaId::new("artifact.opaque-bytes/v1").expect("artifact schema"),
            artifact.clone(),
            SourceItemKind::new("artifact/file/v1").expect("artifact kind"),
            b"valid artifact".to_vec(),
            None,
        )
        .expect("artifact item");
        let node = SourceNode::new(item, Vec::new()).expect("artifact node");
        let requirement = SourceNodeRef::new(SemanticDigest::xxh3_128([3; 16]));
        let relation = StableId::new("ATTACHMENT-VALID-COVERAGE").expect("valid relation");
        let binding = ImplementationBinding::new_with(
            active,
            relation.clone(),
            NavigationKind::Implements,
            node.reference(),
            requirement,
            AtomRevisionRef::new(
                StableId::new("ATTACHMENT-VALID-REQUIREMENT").expect("requirement ID"),
                requirement.digest(),
            ),
        )
        .expect("binding");
        let attachment = ArtifactBindingAttachment::new_with(
            active,
            relation,
            artifact,
            node.reference(),
            binding.record(),
        )
        .expect("valid attachment");
        assert!(super::validate_artifact_attachment(&attachment, &binding, &node).is_ok());
    }
}

fn attachment_fields(
    relation: &StableId,
    artifact: &RepositoryPath,
    subject: SourceNodeRef,
    record: ImplementationBindingRef,
) -> Vec<(u16, CanonicalValue)> {
    alloc::vec![
        (1, CanonicalValue::Text(relation.as_str().into())),
        (2, CanonicalValue::Text(artifact.as_str().into())),
        (3, CanonicalValue::Digest(subject.digest())),
        (4, CanonicalValue::Digest(record.digest())),
    ]
}

#[cfg(all(test, feature = "coverage-controls"))]
#[allow(clippy::expect_used)]
mod coverage_tests {
    use super::*;
    use crate::{
        AtomRevisionRef, DigestAlgorithm, NavigationKind, ParsedItemIdentity, SchemaId,
        SourceItemKind,
    };

    fn artifact_node(path: &str, semantic: &[u8]) -> SourceNode {
        let item = ParsedItemIdentity::new(
            SchemaId::new("artifact.opaque-bytes/v1").expect("artifact schema"),
            RepositoryPath::new(path).expect("artifact path"),
            SourceItemKind::new("artifact/file/v1").expect("artifact kind"),
            semantic.to_vec(),
            None,
        )
        .expect("artifact item");
        SourceNode::new(item, Vec::new()).expect("artifact node")
    }

    #[test]
    fn artifact_attachment_controls_cover_second_member_and_subject_checks() {
        let active = ActiveAlgorithm::new(DigestAlgorithm::Xxh3_128).expect("XXH3 member");
        let subject = SourceNodeRef::new(SemanticDigest::xxh3_128([1; 16]));
        let foreign_record = ImplementationBindingRef::new(SemanticDigest::sha_256([2; 32]));
        assert_eq!(
            ArtifactBindingAttachment::new_with(
                active,
                StableId::new("COVERAGE-RELATION").expect("relation"),
                RepositoryPath::new("artifact.bin").expect("artifact path"),
                subject,
                foreign_record,
            ),
            Err(ArtifactAttachmentError::DigestMember)
        );

        let node = artifact_node("artifact.bin", b"first");
        let other = artifact_node("artifact.bin", b"second");
        let requirement = SourceNodeRef::new(SemanticDigest::xxh3_128([3; 16]));
        let relation = StableId::new("COVERAGE-ATTACHMENT").expect("relation");
        let record = ImplementationBinding::new_with(
            active,
            relation.clone(),
            NavigationKind::Implements,
            node.reference(),
            requirement,
            AtomRevisionRef::new(
                StableId::new("COVERAGE-REQUIREMENT").expect("requirement ID"),
                requirement.digest(),
            ),
        )
        .expect("binding record");
        let attachment = ArtifactBindingAttachment::new_with(
            active,
            relation,
            RepositoryPath::new("artifact.bin").expect("artifact path"),
            node.reference(),
            record.record(),
        )
        .expect("attachment");
        assert_eq!(
            validate_artifact_attachment(&attachment, &record, &other),
            Err(ArtifactAttachmentError::Subject)
        );
        for failure in [
            ArtifactBindingTransactionStage::RemoveUncommittedObject,
            ArtifactBindingTransactionStage::RetryCleanup,
        ] {
            let state =
                artifact_binding_transaction_state(true, Some(b"prior"), b"next", Some(failure));
            assert!(state.object_present);
            assert_eq!(state.attachment.as_deref(), Some(b"prior".as_slice()));
            assert!(!state.committed);
        }
    }
}
