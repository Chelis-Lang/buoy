//! Total repository object-store adapter for content-addressed binding records.
//!
//! The store owns repository paths, so this adapter reconstructs one claimed
//! locator from the path, recomputes the record digest from the parsed fields,
//! and compares the two. A record that fails any check reaches no aggregate.

use super::{AdapterDiagnostic, SourceSpan};
use buoy_core::{
    ActiveAlgorithm, ArtifactBindingAttachment, AtomRevisionRef, DigestAssuranceClass,
    ImplementationBinding, ImplementationBindingRef, NavigationKind, RepositoryPath,
    SemanticDigest, SourceNodeRef, StableId, StoredBinding, binding_attachment_key,
};
use serde::Deserialize;

/// Frozen object-store prefix for version 1 binding records.
pub const BINDING_OBJECT_PREFIX: &str = "buoy/objects/v1/";
/// Frozen record schema ID inside one object-store file.
pub const BINDING_RECORD_SCHEMA: &str = "buoy/implementation-binding/v1";
/// Frozen sidecar prefix for version 1 artifact attachments.
pub const BINDING_ATTACHMENT_PREFIX: &str = "buoy/attachments/v1/";
/// Frozen schema ID inside one artifact attachment sidecar.
pub const BINDING_ATTACHMENT_SCHEMA: &str = "buoy/binding-attachment/v1";
const BINDING_RECORD_RELATION: &str = "implements";

/// Complete object-store extraction for one selected repository file.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BindingOutput {
    /// The one record this file materializes, absent when this is a sidecar or any check failed.
    pub stored: Option<StoredBinding>,
    /// The one sidecar this file materializes, absent when this is an object or any check failed.
    pub attachment: Option<ArtifactBindingAttachment>,
    /// Stable deterministic diagnostics.
    pub diagnostics: Vec<AdapterDiagnostic>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RecordInput {
    schema: String,
    id: String,
    relation: String,
    subject: String,
    requirement: String,
    atom: AtomInput,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct AtomInput {
    id: String,
    revision: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct AttachmentInput {
    schema: String,
    relation: String,
    artifact: String,
    subject: String,
    record: String,
}

/// Renders the canonical object-store text of one immutable binding record.
#[must_use]
pub fn canonical_binding_record(record: &ImplementationBinding) -> String {
    format!(
        "schema = \"{BINDING_RECORD_SCHEMA}\"\nid = \"{}\"\nrelation = \"{BINDING_RECORD_RELATION}\"\nsubject = \"{}\"\nrequirement = \"{}\"\n\n[atom]\nid = \"{}\"\nrevision = \"{}\"\n",
        record.meta().id().as_str(),
        record.subject().canonical_text(),
        record.requirement().canonical_text(),
        record.atom().id().as_str(),
        record.atom().revision().canonical_text(),
    )
}

/// Renders the canonical text of one mutable artifact attachment sidecar.
#[must_use]
pub fn canonical_binding_attachment(attachment: &ArtifactBindingAttachment) -> String {
    format!(
        "schema = \"{BINDING_ATTACHMENT_SCHEMA}\"\nrelation = \"{}\"\nartifact = \"{}\"\nsubject = \"{}\"\nrecord = \"{}\"\n",
        attachment.relation().as_str(),
        attachment.artifact().as_str(),
        attachment.subject().canonical_text(),
        attachment.record().canonical_text(),
    )
}

/// Returns the relation-keyed mount-relative path of one attachment sidecar.
///
/// # Errors
///
/// Returns an encoding error only if the validated relation stops encoding.
pub fn binding_attachment_path(
    active: ActiveAlgorithm,
    relation: &StableId,
) -> Result<String, buoy_core::EncodingError> {
    binding_attachment_key(active, relation).map(|key| {
        format!(
            "{BINDING_ATTACHMENT_PREFIX}{}.toml",
            key.canonical_text().replace(':', "/")
        )
    })
}

/// Returns the digest-derived object-store path of one record locator.
///
/// A canonical digest carries exactly one separator and no algorithm name
/// contains one, so the algorithm becomes one directory and the payload becomes
/// one colon-free filename.
#[must_use]
pub fn binding_object_path(locator: ImplementationBindingRef) -> String {
    format!(
        "{BINDING_OBJECT_PREFIX}{}.toml",
        locator.canonical_text().replace(':', "/")
    )
}

/// Parses one strict object-store path into the locator it claims.
///
/// A payload outside lowercase hexadecimal, an extra path component, and a
/// filename outside the `.toml` suffix all fail closed, so no mutable alias
/// reaches the store.
#[must_use]
pub fn binding_object_locator(path: &RepositoryPath) -> Option<ImplementationBindingRef> {
    binding_object_locator_in_mount(path, None)
}

/// Parses one object-store path relative to an optional logical mount prefix.
#[must_use]
pub fn binding_object_locator_in_mount(
    path: &RepositoryPath,
    mount_prefix: Option<&RepositoryPath>,
) -> Option<ImplementationBindingRef> {
    let relative = mount_relative_text(path, mount_prefix)?;
    let remainder = relative.strip_prefix(BINDING_OBJECT_PREFIX)?;
    let digest = path_digest(remainder)?;
    Some(ImplementationBindingRef::new(digest))
}

/// Parses one attachment path relative to an optional logical mount prefix.
#[must_use]
pub fn binding_attachment_key_locator(
    path: &RepositoryPath,
    mount_prefix: Option<&RepositoryPath>,
) -> Option<SemanticDigest> {
    let relative = mount_relative_text(path, mount_prefix)?;
    path_digest(relative.strip_prefix(BINDING_ATTACHMENT_PREFIX)?)
}

/// Materializes one object or sidecar below an optional logical mount prefix.
#[must_use]
pub fn extract_binding_in_mount(
    active: ActiveAlgorithm,
    mount_prefix: Option<&RepositoryPath>,
    path: &RepositoryPath,
    bytes: &[u8],
) -> BindingOutput {
    let span = SourceSpan {
        path: path.clone(),
        start: 0,
        end: bytes.len() as u64,
    };
    let relative = mount_relative_text(path, mount_prefix).unwrap_or_default();
    if relative.starts_with(BINDING_ATTACHMENT_PREFIX) {
        return match parse_artifact_attachment(active, mount_prefix, path, bytes) {
            Ok(attachment) => BindingOutput {
                stored: None,
                attachment: Some(attachment),
                diagnostics: Vec::new(),
            },
            Err(code) => BindingOutput {
                stored: None,
                attachment: None,
                diagnostics: vec![AdapterDiagnostic {
                    code,
                    span,
                    context: String::from("buoy/attachments/v1"),
                }],
            },
        };
    }
    match parse_stored_binding(active, mount_prefix, path, bytes) {
        Ok(stored) => BindingOutput {
            stored: Some(stored),
            attachment: None,
            diagnostics: Vec::new(),
        },
        Err(code) => BindingOutput {
            stored: None,
            attachment: None,
            diagnostics: vec![AdapterDiagnostic {
                code,
                span,
                context: String::from("buoy/objects/v1"),
            }],
        },
    }
}

/// Parses one canonical binding-object payload without consulting any path.
///
/// The store adapter derives a claimed locator from the file path and refuses
/// any disagreement, so it can never report the record that tampered bytes
/// actually describe. A host diagnosing a substituted object needs exactly
/// that record to name the address the bytes now hash to, so this entry point
/// applies every byte-level check the store applies and none of the path
/// checks. It grants no authority: a record parsed here still reaches no
/// aggregate until the store resolves it at its own address.
#[must_use]
pub fn parse_binding_record(
    active: ActiveAlgorithm,
    bytes: &[u8],
) -> Option<ImplementationBinding> {
    parse_binding_payload(active, None, bytes).ok()
}

fn parse_stored_binding(
    active: ActiveAlgorithm,
    mount_prefix: Option<&RepositoryPath>,
    path: &RepositoryPath,
    bytes: &[u8],
) -> Result<StoredBinding, &'static str> {
    let locator = binding_object_locator_in_mount(path, mount_prefix).ok_or("CAB-OBJECT-PATH")?;
    let record = parse_binding_payload(active, Some(locator), bytes)?;
    if record.record() != locator {
        return Err("CAB-OBJECT-PATH-DIGEST");
    }
    Ok(StoredBinding::new(locator, record))
}

fn parse_binding_payload(
    active: ActiveAlgorithm,
    locator: Option<ImplementationBindingRef>,
    bytes: &[u8],
) -> Result<ImplementationBinding, &'static str> {
    let source = core::str::from_utf8(bytes).map_err(|_| "CAB-OBJECT-SYNTAX")?;
    let input = toml::from_str::<RecordInput>(source).map_err(|_| "CAB-OBJECT-SYNTAX")?;
    if input.schema != BINDING_RECORD_SCHEMA {
        return Err("CAB-OBJECT-SCHEMA");
    }
    if input.relation != BINDING_RECORD_RELATION {
        return Err("CAB-OBJECT-RELATION");
    }
    let relation = StableId::new(&input.id).map_err(|_| "CAB-OBJECT-FIELD")?;
    let atom_id = StableId::new(&input.atom.id).map_err(|_| "CAB-OBJECT-FIELD")?;
    let subject = parse_digest(&input.subject)?;
    let requirement = parse_digest(&input.requirement)?;
    let revision = parse_digest(&input.atom.revision)?;

    let member = active.algorithm();
    let mut addressed = vec![subject, requirement, revision];
    if let Some(locator) = locator {
        addressed.insert(0, locator.digest());
    }
    if !all_collision_resistant(&addressed) {
        return Err("CAB-DIGEST-CLASS");
    }
    if locator.is_some_and(|locator| locator.digest().algorithm() != member) {
        return Err("CAB-DIGEST-MEMBER");
    }

    let record = ImplementationBinding::new_with(
        active,
        relation,
        NavigationKind::Implements,
        SourceNodeRef::new(subject),
        SourceNodeRef::new(requirement),
        AtomRevisionRef::new(atom_id, revision),
    )
    .map_err(|_| "CAB-DIGEST-MEMBER")?;
    if canonical_binding_record(&record) != source {
        return Err("CAB-OBJECT-CANONICAL");
    }
    Ok(record)
}

#[allow(clippy::expect_used)]
fn parse_artifact_attachment(
    active: ActiveAlgorithm,
    mount_prefix: Option<&RepositoryPath>,
    path: &RepositoryPath,
    bytes: &[u8],
) -> Result<ArtifactBindingAttachment, &'static str> {
    let path_key =
        binding_attachment_key_locator(path, mount_prefix).ok_or("CAB-ATTACHMENT-PATH")?;
    let source = core::str::from_utf8(bytes).map_err(|_| "CAB-ATTACHMENT-SYNTAX")?;
    let input = toml::from_str::<AttachmentInput>(source).map_err(|_| "CAB-ATTACHMENT-SYNTAX")?;
    if input.schema != BINDING_ATTACHMENT_SCHEMA {
        return Err("CAB-ATTACHMENT-SCHEMA");
    }
    let relation = StableId::new(&input.relation).map_err(|_| "CAB-ATTACHMENT-FIELD")?;
    let artifact = RepositoryPath::new(&input.artifact).map_err(|_| "CAB-ATTACHMENT-FIELD")?;
    let subject =
        SemanticDigest::parse_canonical(&input.subject).map_err(|_| "CAB-ATTACHMENT-FIELD")?;
    let record =
        SemanticDigest::parse_canonical(&input.record).map_err(|_| "CAB-ATTACHMENT-FIELD")?;
    let addressed = [path_key, subject, record];
    if !all_collision_resistant(&addressed) {
        return Err("CAB-DIGEST-CLASS");
    }
    for digest in addressed {
        if digest.algorithm() != active.algorithm() {
            return Err("CAB-DIGEST-MEMBER");
        }
    }
    let attachment = ArtifactBindingAttachment::new_with(
        active,
        relation.clone(),
        artifact,
        SourceNodeRef::new(subject),
        ImplementationBindingRef::new(record),
    )
    .expect("validated attachment members always construct");
    if canonical_binding_attachment(&attachment) != source {
        return Err("CAB-ATTACHMENT-CANONICAL");
    }
    let expected_key = binding_attachment_key(active, &relation)
        .expect("validated attachment relation always encodes");
    if expected_key != path_key {
        return Err("CAB-ATTACHMENT-PATH");
    }
    Ok(attachment)
}

#[cfg(test)]
mod tests {
    use super::{binding_attachment_key_locator, binding_object_locator_in_mount};
    use buoy_core::RepositoryPath;

    #[test]
    #[allow(clippy::expect_used)]
    fn mounted_locators_reject_paths_outside_the_mount() {
        let path = RepositoryPath::new("project/buoy/objects/xxh3-128/deadbeef.toml")
            .expect("object path");
        let other = RepositoryPath::new("authority").expect("other mount");
        let project = RepositoryPath::new("project").expect("project mount");
        assert_eq!(binding_object_locator_in_mount(&path, Some(&other)), None);
        assert_eq!(binding_object_locator_in_mount(&path, Some(&project)), None);
        assert_eq!(binding_attachment_key_locator(&path, Some(&other)), None);
        let nested = RepositoryPath::new("buoy/objects/v1/xxh3-128/nested/deadbeef.toml")
            .expect("nested object path");
        assert_eq!(binding_object_locator_in_mount(&nested, None), None);
    }
}

fn mount_relative_text<'a>(
    path: &'a RepositoryPath,
    mount_prefix: Option<&RepositoryPath>,
) -> Option<&'a str> {
    match mount_prefix {
        Some(prefix) => path
            .as_str()
            .strip_prefix(prefix.as_str())?
            .strip_prefix('/'),
        None => Some(path.as_str()),
    }
}

fn all_collision_resistant(digests: &[SemanticDigest]) -> bool {
    for digest in digests {
        if digest.algorithm().assurance_class() != DigestAssuranceClass::CollisionResistant {
            return false;
        }
    }
    true
}

fn path_digest(remainder: &str) -> Option<SemanticDigest> {
    let (algorithm, filename) = remainder.split_once('/')?;
    if filename.contains('/') {
        return None;
    }
    let payload = filename.strip_suffix(".toml")?;
    if payload.is_empty()
        || !payload
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return None;
    }
    SemanticDigest::parse_canonical(&format!("{algorithm}:{payload}")).ok()
}

fn parse_digest(value: &str) -> Result<SemanticDigest, &'static str> {
    SemanticDigest::parse_canonical(value).map_err(|_| "CAB-OBJECT-FIELD")
}

#[cfg(all(test, feature = "coverage-controls"))]
#[allow(clippy::expect_used, clippy::too_many_lines)]
mod coverage_tests {
    use super::*;
    use crate::core::DigestAlgorithm;

    #[test]
    fn binding_controls_cover_unmounted_wrapper_and_change_detection_sidecar() {
        let active = ActiveAlgorithm::new(DigestAlgorithm::Xxh3_128).expect("XXH3 member");
        let unrelated = RepositoryPath::new("unrelated.toml").expect("unrelated path");
        assert!(
            extract_binding_in_mount(active, None, &unrelated, b"not a binding")
                .stored
                .is_none()
        );

        let relation = StableId::new("COVERAGE-SIDECAR").expect("relation");
        let attachment = ArtifactBindingAttachment::new_with(
            active,
            relation.clone(),
            RepositoryPath::new("artifact.bin").expect("artifact path"),
            SourceNodeRef::new(SemanticDigest::xxh3_128([1; 16])),
            ImplementationBindingRef::new(SemanticDigest::xxh3_128([2; 16])),
        )
        .expect("XXH3 sidecar value");
        let path = RepositoryPath::new(
            &binding_attachment_path(active, &relation).expect("attachment path"),
        )
        .expect("repository attachment path");
        let canonical = canonical_binding_attachment(&attachment);
        assert_eq!(
            parse_artifact_attachment(active, None, &path, canonical.as_bytes()),
            Err("CAB-DIGEST-CLASS")
        );
        let mount_prefix = RepositoryPath::new("project").expect("mount prefix");
        let outside = RepositoryPath::new("outside/object.toml").expect("outside path");
        assert_eq!(
            binding_object_locator_in_mount(&outside, Some(&mount_prefix)),
            None
        );
        assert_eq!(
            binding_attachment_key_locator(&outside, Some(&mount_prefix)),
            None
        );
        assert_eq!(
            binding_attachment_key_locator(&mount_prefix, Some(&mount_prefix)),
            None
        );
        assert_eq!(
            parse_artifact_attachment(active, None, &outside, canonical.as_bytes()),
            Err("CAB-ATTACHMENT-PATH")
        );
        assert_eq!(
            parse_artifact_attachment(active, None, &path, &[0xff]),
            Err("CAB-ATTACHMENT-SYNTAX")
        );
        assert_eq!(
            parse_artifact_attachment(active, None, &path, b"invalid"),
            Err("CAB-ATTACHMENT-SYNTAX")
        );
        for (source, expected) in [
            (
                canonical.replace(
                    "relation = \"COVERAGE-SIDECAR\"",
                    "relation = \"bad relation\"",
                ),
                "CAB-ATTACHMENT-FIELD",
            ),
            (
                canonical.replace("artifact = \"artifact.bin\"", "artifact = \"/invalid\""),
                "CAB-ATTACHMENT-FIELD",
            ),
            (
                canonical.replace(
                    &format!("subject = \"{}\"", attachment.subject().canonical_text()),
                    "subject = \"invalid\"",
                ),
                "CAB-ATTACHMENT-FIELD",
            ),
            (
                canonical.replace(
                    &format!("record = \"{}\"", attachment.record().canonical_text()),
                    "record = \"invalid\"",
                ),
                "CAB-ATTACHMENT-FIELD",
            ),
        ] {
            assert_eq!(
                parse_artifact_attachment(active, None, &path, source.as_bytes()),
                Err(expected)
            );
        }

        let subject = SourceNodeRef::new(SemanticDigest::xxh3_128([3; 16]));
        let requirement = SourceNodeRef::new(SemanticDigest::xxh3_128([4; 16]));
        let record = ImplementationBinding::new_with(
            active,
            StableId::new("COVERAGE-OBJECT").expect("relation"),
            NavigationKind::Implements,
            subject,
            requirement,
            AtomRevisionRef::new(
                StableId::new("COVERAGE-REQUIREMENT").expect("requirement"),
                requirement.digest(),
            ),
        )
        .expect("XXH3 binding record");
        let object_path = RepositoryPath::new(&binding_object_path(record.record()))
            .expect("binding object path");
        assert_eq!(
            parse_stored_binding(
                active,
                None,
                &object_path,
                canonical_binding_record(&record).as_bytes(),
            ),
            Err("CAB-DIGEST-CLASS")
        );

        let resistant = ActiveAlgorithm::new(DigestAlgorithm::Blake3_256).expect("BLAKE3 member");
        let resistant_subject = SourceNodeRef::new(SemanticDigest::blake3_256([5; 32]));
        let resistant_requirement = SourceNodeRef::new(SemanticDigest::blake3_256([6; 32]));
        let resistant_record = ImplementationBinding::new_with(
            resistant,
            StableId::new("COVERAGE-MIXED-OBJECT").expect("relation"),
            NavigationKind::Implements,
            resistant_subject,
            resistant_requirement,
            AtomRevisionRef::new(
                StableId::new("COVERAGE-MIXED-REQUIREMENT").expect("requirement"),
                resistant_requirement.digest(),
            ),
        )
        .expect("BLAKE3 binding record");
        let resistant_path = RepositoryPath::new(&binding_object_path(resistant_record.record()))
            .expect("BLAKE3 object path");
        let mixed = canonical_binding_record(&resistant_record).replace(
            &resistant_subject.canonical_text(),
            &SemanticDigest::sha_256([7; 32]).canonical_text(),
        );
        assert_eq!(
            parse_stored_binding(resistant, None, &resistant_path, mixed.as_bytes()),
            Err("CAB-DIGEST-MEMBER")
        );
    }
}
