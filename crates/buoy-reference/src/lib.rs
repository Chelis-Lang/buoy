#![doc = "Independent executable reference authority for canonical Buoy bytes under the frozen spec-provenance/v3 envelope."]
#![forbid(unsafe_code)]

use unicode_normalization::UnicodeNormalization;

/// Dependency-independent value representation used only as a test authority.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Value {
    /// Boolean false.
    False,
    /// Boolean true.
    True,
    /// Unsigned 64-bit integer.
    Unsigned(u64),
    /// Signed 64-bit integer.
    Signed(i64),
    /// Arbitrary bytes.
    Bytes(Vec<u8>),
    /// NFC UTF-8 text.
    Text(String),
    /// Ascending unique field tags.
    Record(Vec<(u16, Self)>),
    /// Ordered values.
    Sequence(Vec<Self>),
    /// Values ordered by their complete encoding.
    Set(Vec<Self>),
    /// Explicit absence.
    None,
    /// Explicit presence.
    Some(Box<Self>),
    /// Closed variant tag and explicit optional payload.
    Enum(u16, Option<Box<Self>>),
    /// Algorithm tag and complete digest bytes.
    Digest(u16, Vec<u8>),
}

/// Stable independent-reference failure category.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Error {
    /// Unknown value tag.
    Tag,
    /// Invalid or truncated length/count.
    Length,
    /// Invalid UTF-8 or non-NFC text.
    Text,
    /// Duplicate or unordered fields.
    Record,
    /// Duplicate or unordered set values.
    Set,
    /// Invalid option encoding.
    Option,
    /// Invalid enum tag or payload.
    Enum,
    /// Invalid algorithm or digest length.
    Digest,
    /// Configured nesting bound exceeded.
    Nesting,
    /// Configured cumulative value bound exceeded.
    Count,
    /// More than one complete value was supplied.
    Trailing,
}

impl Value {
    /// Encodes one independent reference value using frozen v3 bytes.
    ///
    /// # Errors
    ///
    /// Returns a stable reference error for noncanonical input values.
    pub fn encode(&self) -> Result<Vec<u8>, Error> {
        let mut output = Vec::new();
        emit(self, &mut output)?;
        Ok(output)
    }

    /// Decodes exactly one independent reference value with explicit limits.
    ///
    /// # Errors
    ///
    /// Returns a stable reference error for malformed or noncanonical bytes.
    pub fn decode(bytes: &[u8], max_nesting: u16, max_values: u64) -> Result<Self, Error> {
        let mut reader = Reader {
            bytes,
            position: 0,
            max_nesting: u32::from(max_nesting),
            max_values,
            values: 0,
        };
        let value = reader.read_value(1)?;
        if reader.position != reader.bytes.len() {
            return Err(Error::Trailing);
        }
        Ok(value)
    }
}

/// Computes the independently encoded frozen-v3 semantic preimage.
///
/// Official vectors anchor each registered hashing implementation. This
/// reference covers the independent envelope and value encoder only.
///
/// # Errors
///
/// Returns an encoding error for a noncanonical field record.
#[allow(clippy::expect_used, clippy::missing_panics_doc)]
pub fn semantic_preimage(
    algorithm_tag: u16,
    domain: &str,
    fields: Vec<(u16, Value)>,
) -> Result<Vec<u8>, Error> {
    if !matches!(algorithm_tag, 2..=4) {
        return Err(Error::Digest);
    }
    let mut preimage = b"SPEC-PROVENANCE\0".to_vec();
    preimage.extend_from_slice(&3_u16.to_be_bytes());
    preimage.extend(
        Value::Text(String::from("spec-provenance/v3"))
            .encode()
            .expect("frozen schema text canonically encodes"),
    );
    preimage.extend(
        Value::Enum(algorithm_tag, None)
            .encode()
            .expect("registered algorithm enum canonically encodes"),
    );
    preimage.extend(Value::Text(domain.to_owned()).encode()?);
    preimage.extend(Value::Record(fields).encode()?);
    Ok(preimage)
}

/// Independent logical-path projection failure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LogicalProjectionError {
    /// A prefix or relative path is not a canonical repository path.
    Path,
    /// Two mount prefixes are equal or overlap by complete path segments.
    PrefixOverlap,
}

/// Projects one canonical path below one canonical logical mount prefix.
///
/// This reference does not import Buoy production path or mount code.
///
/// # Errors
///
/// Returns [`LogicalProjectionError::Path`] for a noncanonical input path.
pub fn logical_projection(prefix: &str, relative: &str) -> Result<String, LogicalProjectionError> {
    if !reference_repository_path(prefix) || !reference_repository_path(relative) {
        return Err(LogicalProjectionError::Path);
    }
    Ok(format!("{prefix}/{relative}"))
}

/// Canonicalizes independent mount declarations by ID and rejects overlaps.
///
/// # Errors
///
/// Returns a path or overlap error for an invalid declaration set.
pub fn logical_mount_set(
    declarations: &[(&str, &str)],
) -> Result<Vec<(String, String)>, LogicalProjectionError> {
    let mut rows = declarations
        .iter()
        .map(|(id, prefix)| {
            if !reference_scope_id(id) || !reference_repository_path(prefix) {
                return Err(LogicalProjectionError::Path);
            }
            Ok(((*id).to_owned(), (*prefix).to_owned()))
        })
        .collect::<Result<Vec<_>, _>>()?;
    rows.sort_unstable();
    for pair in rows.windows(2) {
        if pair[0].0 == pair[1].0 {
            return Err(LogicalProjectionError::PrefixOverlap);
        }
    }
    for (index, (_, left)) in rows.iter().enumerate() {
        for (_, right) in rows.iter().skip(index + 1) {
            let left = portable_path_key(left);
            let right = portable_path_key(right);
            if path_prefix(&left, &right) || path_prefix(&right, &left) {
                return Err(LogicalProjectionError::PrefixOverlap);
            }
        }
    }
    Ok(rows)
}

/// Computes the independent canonical preimage for one attachment relation key.
///
/// # Errors
///
/// Returns an encoding error for an unknown algorithm tag or noncanonical relation text.
pub fn attachment_key_preimage(algorithm_tag: u16, relation: &str) -> Result<Vec<u8>, Error> {
    semantic_preimage(
        algorithm_tag,
        "buoy/binding-attachment-key/v1",
        vec![(1, Value::Text(relation.to_owned()))],
    )
}

/// One effect stage in the independent artifact-binding transaction model.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BindingTransactionStage {
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
    /// Remove an uncommitted immutable object.
    RemoveUncommittedObject,
    /// Retry one failed cleanup operation.
    RetryCleanup,
}

/// Final observable state from the independent binding transaction model.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BindingTransactionState {
    /// Whether an immutable object is present after the modeled operation.
    pub object_present: bool,
    /// Exact current attachment bytes.
    pub attachment: Option<Vec<u8>>,
    /// Whether any bounded temporary path remains.
    pub temporary_present: bool,
    /// Whether the transaction completed.
    pub committed: bool,
    /// Number of forward stages completed before success or the injected failure.
    pub completed_steps: u8,
}

/// Evaluates one independent transaction outcome for a single one-shot failure.
///
/// A failed transaction restores the exact prior attachment and object state.
#[must_use]
pub fn binding_transaction_state(
    prior_object_present: bool,
    prior_attachment: Option<&[u8]>,
    next_attachment: &[u8],
    failure: Option<BindingTransactionStage>,
) -> BindingTransactionState {
    let completed_steps = failure.map_or(11, binding_stage_ordinal);
    if failure.is_some() {
        return BindingTransactionState {
            object_present: prior_object_present,
            attachment: prior_attachment.map(<[u8]>::to_vec),
            temporary_present: false,
            committed: false,
            completed_steps,
        };
    }
    BindingTransactionState {
        object_present: true,
        attachment: Some(next_attachment.to_vec()),
        temporary_present: false,
        committed: true,
        completed_steps,
    }
}

#[allow(clippy::match_same_arms)]
const fn binding_stage_ordinal(stage: BindingTransactionStage) -> u8 {
    match stage {
        BindingTransactionStage::Validate => 0,
        BindingTransactionStage::CreateObjectDirectory => 1,
        BindingTransactionStage::CreateObjectTemporary => 2,
        BindingTransactionStage::WriteObject => 3,
        BindingTransactionStage::SyncObject => 4,
        BindingTransactionStage::ReplaceObject => 5,
        BindingTransactionStage::CreateAttachmentDirectory => 6,
        BindingTransactionStage::CreateAttachmentTemporary => 7,
        BindingTransactionStage::WriteAttachment => 8,
        BindingTransactionStage::SyncAttachment => 9,
        BindingTransactionStage::ReplaceAttachment => 10,
        BindingTransactionStage::RemoveUncommittedObject => 11,
        BindingTransactionStage::RetryCleanup => 11,
    }
}

fn reference_scope_id(value: &str) -> bool {
    let mut components = value.split(['.', '-']);
    components.next().is_some_and(reference_scope_component)
        && components.all(reference_scope_component)
}

fn reference_scope_component(value: &str) -> bool {
    !value.is_empty()
        && value.as_bytes()[0].is_ascii_lowercase()
        && value
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit())
}

fn reference_repository_path(value: &str) -> bool {
    !value.is_empty()
        && !value.starts_with('/')
        && !value.ends_with('/')
        && !value.contains('\\')
        && value.nfc().eq(value.chars())
        && value.split('/').all(|segment| {
            !segment.is_empty()
                && !matches!(segment, "." | "..")
                && !segment.ends_with([' ', '.'])
                && !segment.chars().any(|character| {
                    character.is_ascii_control()
                        || matches!(character, '<' | '>' | ':' | '"' | '|' | '?' | '*')
                })
        })
}

fn portable_path_key(path: &str) -> String {
    path.chars()
        .flat_map(|character| match character {
            'ß' | 'ẞ' => "ss".chars().collect::<Vec<_>>(),
            'ς' => vec!['σ'],
            'ﬀ' => vec!['f', 'f'],
            'İ' => vec!['i', '\u{307}'],
            character => character.to_lowercase().collect::<Vec<_>>(),
        })
        .collect::<String>()
        .nfc()
        .collect()
}

fn path_prefix(prefix: &str, path: &str) -> bool {
    prefix == path
        || path
            .strip_prefix(prefix)
            .is_some_and(|remainder| remainder.starts_with('/'))
}

#[cfg(test)]
mod coverage_tests {
    use super::*;

    #[test]
    fn logical_mount_reference_controls_cover_closed_path_and_casefold_rules() {
        for (prefix, relative) in [
            ("/absolute", "file"),
            ("project", "/absolute"),
            ("project", ""),
            ("project", "trailing/"),
            ("project", "back\\slash"),
            ("project", "cafe\u{301}"),
            ("project", "two//segments"),
            ("project", "dot/./segment"),
            ("project", "dot/../segment"),
            ("project", "trailing-space "),
            ("project", "trailing-dot."),
            ("project", "control/\u{7f}"),
            ("project", "reserved/*"),
        ] {
            assert_eq!(
                logical_projection(prefix, relative),
                Err(LogicalProjectionError::Path),
                "{prefix}/{relative:?}"
            );
        }
        for declarations in [
            vec![("Bad", "project")],
            vec![("valid", "/project")],
            vec![("valid", "project"), ("valid", "authority")],
        ] {
            assert!(logical_mount_set(&declarations).is_err());
        }
        assert_eq!(
            logical_mount_set(&[("first", "source/specs"), ("second", "source")]),
            Err(LogicalProjectionError::PrefixOverlap)
        );
        for (left, right) in [
            ("Straße", "STRASSE"),
            ("ος", "οσ"),
            ("ﬀ", "ff"),
            ("İ", "i\u{307}"),
            ("Docs", "docs"),
        ] {
            assert_eq!(
                logical_mount_set(&[("first", left), ("second", right)]),
                Err(LogicalProjectionError::PrefixOverlap),
                "{left}/{right}"
            );
        }
        for stage in [
            BindingTransactionStage::RemoveUncommittedObject,
            BindingTransactionStage::RetryCleanup,
        ] {
            assert_eq!(binding_stage_ordinal(stage), 11);
            let state = binding_transaction_state(true, Some(b"prior"), b"next", Some(stage));
            assert!(state.object_present);
            assert_eq!(state.attachment.as_deref(), Some(b"prior".as_slice()));
        }
        assert!(path_prefix("src", "src"));
        assert!(path_prefix("src", "src/control"));
        assert!(!path_prefix("src", "src-other"));
        assert!(!path_prefix("src", "other"));
        for invalid_id in ["", "a..b", "A", "a_b"] {
            assert_eq!(
                logical_mount_set(&[(invalid_id, "project")]),
                Err(LogicalProjectionError::Path)
            );
        }
    }
}

fn emit(value: &Value, output: &mut Vec<u8>) -> Result<(), Error> {
    let (tag, payload) = payload(value)?;
    output.push(tag);
    output.extend_from_slice(&u64_len(payload.len()).to_be_bytes());
    output.extend(payload);
    Ok(())
}

#[allow(clippy::cast_possible_truncation)]
fn payload(value: &Value) -> Result<(u8, Vec<u8>), Error> {
    match value {
        Value::False => Ok((0, Vec::new())),
        Value::True => Ok((1, Vec::new())),
        Value::Unsigned(number) => Ok((2, number.to_be_bytes().to_vec())),
        Value::Signed(number) => Ok((3, number.to_be_bytes().to_vec())),
        Value::Bytes(bytes) => Ok((4, bytes.clone())),
        Value::Text(text) => {
            if text.nfc().ne(text.chars()) {
                return Err(Error::Text);
            }
            Ok((5, text.as_bytes().to_vec()))
        }
        Value::Record(fields) => {
            if fields
                .windows(2)
                .any(|pair| pair[0].0 == 0 || pair[0].0 >= pair[1].0)
                || fields.last().is_some_and(|field| field.0 == 0)
            {
                return Err(Error::Record);
            }
            let mut bytes = u64_len(fields.len()).to_be_bytes().to_vec();
            for (field, value) in fields {
                bytes.extend_from_slice(&field.to_be_bytes());
                emit(value, &mut bytes)?;
            }
            Ok((6, bytes))
        }
        Value::Sequence(values) => Ok((7, emit_collection(values, false)?)),
        Value::Set(values) => Ok((8, emit_collection(values, true)?)),
        Value::None => Ok((9, Vec::new())),
        Value::Some(value) => Ok((10, value.encode()?)),
        Value::Enum(tag, payload) => {
            if *tag == 0 {
                return Err(Error::Enum);
            }
            let mut bytes = tag.to_be_bytes().to_vec();
            if let Some(payload) = payload {
                emit(&Value::Some(payload.clone()), &mut bytes)?;
            } else {
                bytes.push(9);
                bytes.extend_from_slice(&0_u64.to_be_bytes());
            }
            Ok((11, bytes))
        }
        Value::Digest(algorithm, bytes)
            if matches!((*algorithm, bytes.len()), (2, 16) | (3 | 4, 32)) =>
        {
            let mut payload = algorithm.to_be_bytes().to_vec();
            payload.extend_from_slice(&(bytes.len() as u16).to_be_bytes());
            payload.extend(bytes);
            Ok((12, payload))
        }
        Value::Digest(_, _) => Err(Error::Digest),
    }
}

fn emit_collection(values: &[Value], set: bool) -> Result<Vec<u8>, Error> {
    let mut encodings = values
        .iter()
        .map(Value::encode)
        .collect::<Result<Vec<_>, _>>()?;
    if set {
        encodings.sort_unstable();
        if encodings.windows(2).any(|pair| pair[0] == pair[1]) {
            return Err(Error::Set);
        }
    }
    let mut output = u64_len(encodings.len()).to_be_bytes().to_vec();
    for encoding in encodings {
        output.extend(encoding);
    }
    Ok(output)
}

fn u64_len(length: usize) -> u64 {
    length as u64
}

struct Reader<'a> {
    bytes: &'a [u8],
    position: usize,
    max_nesting: u32,
    max_values: u64,
    values: u64,
}

impl Reader<'_> {
    #[allow(
        clippy::cast_possible_truncation,
        clippy::expect_used,
        clippy::too_many_lines
    )]
    fn read_value(&mut self, depth: u32) -> Result<Value, Error> {
        if depth > self.max_nesting {
            return Err(Error::Nesting);
        }
        self.values += 1;
        if self.values > self.max_values {
            return Err(Error::Count);
        }
        let tag = self.read(1)?[0];
        let payload_size_u64 = self.read_u64()?;
        let remaining = (self.bytes.len() - self.position) as u64;
        if payload_size_u64 > remaining {
            return Err(Error::Length);
        }
        let payload_size = payload_size_u64 as usize;
        let end = self.position + payload_size;
        let payload_bytes = &self.bytes[self.position..end];
        let mut payload = Reader {
            bytes: payload_bytes,
            position: 0,
            max_nesting: self.max_nesting,
            max_values: self.max_values,
            values: self.values,
        };
        let value = match tag {
            0 if payload_size == 0 => Value::False,
            1 if payload_size == 0 => Value::True,
            2 if payload_size == 8 => Value::Unsigned(
                payload
                    .read_u64()
                    .expect("fixed-width unsigned payload was length-checked"),
            ),
            3 if payload_size == 8 => {
                let mut bytes = [0; 8];
                bytes.copy_from_slice(
                    payload
                        .read(8)
                        .expect("fixed-width signed payload was length-checked"),
                );
                Value::Signed(i64::from_be_bytes(bytes))
            }
            4 => Value::Bytes(
                payload
                    .read(payload_size)
                    .expect("byte payload size is bounded by its frame")
                    .to_vec(),
            ),
            5 => {
                let text = core::str::from_utf8(
                    payload
                        .read(payload_size)
                        .expect("text payload size is bounded by its frame"),
                )
                .map_err(|_| Error::Text)?;
                if text.nfc().ne(text.chars()) {
                    return Err(Error::Text);
                }
                Value::Text(text.to_owned())
            }
            6 => Value::Record(payload.read_record(depth)?),
            7 => Value::Sequence(payload.read_collection(depth, false)?),
            8 => Value::Set(payload.read_collection(depth, true)?),
            9 if payload_size == 0 => Value::None,
            10 => Value::Some(Box::new(payload.read_value(increment(depth))?)),
            11 => {
                let variant = payload.read_u16().map_err(|_| Error::Enum)?;
                if variant == 0 {
                    return Err(Error::Enum);
                }
                match payload
                    .read_value(increment(depth))
                    .map_err(|_| Error::Enum)?
                {
                    Value::None => Value::Enum(variant, None),
                    Value::Some(value) => Value::Enum(variant, Some(value)),
                    _ => return Err(Error::Enum),
                }
            }
            12 if matches!(payload_size, 20 | 36) => {
                let algorithm = payload
                    .read_u16()
                    .expect("digest algorithm field was length-checked");
                let length = payload
                    .read_u16()
                    .expect("digest length field was length-checked");
                if !matches!((algorithm, length), (2, 16) | (3 | 4, 32))
                    || payload_size != usize::from(length) + 4
                {
                    return Err(Error::Digest);
                }
                Value::Digest(
                    algorithm,
                    payload
                        .read(usize::from(length))
                        .expect("digest payload length was checked")
                        .to_vec(),
                )
            }
            0..=12 => return Err(Error::Length),
            _ => return Err(Error::Tag),
        };
        if payload.position != payload.bytes.len() {
            return Err(if tag == 11 {
                Error::Enum
            } else {
                Error::Option
            });
        }
        self.values = payload.values;
        self.position = end;
        Ok(value)
    }

    fn read_record(&mut self, depth: u32) -> Result<Vec<(u16, Value)>, Error> {
        let count = self.read_count()?;
        let mut fields = Vec::with_capacity(count);
        let mut prior = 0;
        for _ in 0..count {
            let tag = self.read_u16().map_err(|_| Error::Record)?;
            if tag == 0 || tag <= prior {
                return Err(Error::Record);
            }
            fields.push((tag, self.read_value(increment(depth)).map_err(map_record)?));
            prior = tag;
        }
        if self.position != self.bytes.len() {
            return Err(Error::Record);
        }
        Ok(fields)
    }

    fn read_collection(&mut self, depth: u32, set: bool) -> Result<Vec<Value>, Error> {
        let count = self.read_count()?;
        let mut values = Vec::with_capacity(count);
        let mut prior: Option<Vec<u8>> = None;
        for _ in 0..count {
            let start = self.position;
            let value = self.read_value(increment(depth))?;
            let encoding = self.bytes[start..self.position].to_vec();
            if set && prior.as_ref().is_some_and(|previous| previous >= &encoding) {
                return Err(Error::Set);
            }
            prior = Some(encoding);
            values.push(value);
        }
        if self.position != self.bytes.len() {
            return Err(if set { Error::Set } else { Error::Length });
        }
        Ok(values)
    }

    #[allow(clippy::cast_possible_truncation)]
    fn read_count(&mut self) -> Result<usize, Error> {
        let count = self.read_u64()?;
        if count > self.max_values.saturating_sub(self.values) {
            return Err(Error::Count);
        }
        Ok(count as usize)
    }

    fn read_u16(&mut self) -> Result<u16, Error> {
        let mut bytes = [0; 2];
        bytes.copy_from_slice(self.read(2)?);
        Ok(u16::from_be_bytes(bytes))
    }

    fn read_u64(&mut self) -> Result<u64, Error> {
        let mut bytes = [0; 8];
        bytes.copy_from_slice(self.read(8)?);
        Ok(u64::from_be_bytes(bytes))
    }

    fn read(&mut self, count: usize) -> Result<&[u8], Error> {
        let end = self.position.saturating_add(count);
        let result = self.bytes.get(self.position..end).ok_or(Error::Length)?;
        self.position = end;
        Ok(result)
    }
}

const fn increment(depth: u32) -> u32 {
    depth + 1
}

const fn map_record(error: Error) -> Error {
    match error {
        Error::Nesting | Error::Count | Error::Text | Error::Tag | Error::Digest => error,
        _ => Error::Record,
    }
}

/// Reviewably separate reference algorithms used by production differential tests.
pub mod algorithms {
    use std::collections::{BTreeMap, BTreeSet, VecDeque};
    use unicode_normalization::UnicodeNormalization;

    /// Returns the frozen lifecycle target tag, or `None` for an illegal transition.
    #[must_use]
    pub const fn lifecycle_transition(state: u16, event: u16) -> Option<u16> {
        match (state, event) {
            (1, 1) => Some(2),
            (1 | 2, 2) => Some(4),
            (2, 3) => Some(3),
            _ => None,
        }
    }

    /// Independently evaluates the combined policy/debt/waiver transition relation.
    #[must_use]
    pub fn governance_transition_valid(
        base_policy: Option<u16>,
        head_policy: Option<u16>,
        base_debt: Option<u16>,
        head_debt: Option<u16>,
        base_waiver: Option<u16>,
        head_waiver: Option<u16>,
    ) -> bool {
        member_pair(
            base_policy,
            head_policy,
            &[(0, 1), (1, 1), (1, 2), (1, 3), (2, 2), (3, 3)],
        ) && member_pair(
            base_debt,
            head_debt,
            &[(0, 1), (1, 1), (1, 2), (1, 3), (2, 2), (3, 3)],
        ) && member_pair(
            base_waiver,
            head_waiver,
            &[
                (0, 1),
                (1, 1),
                (1, 2),
                (1, 3),
                (1, 4),
                (2, 2),
                (3, 3),
                (4, 4),
            ],
        )
    }

    fn member_pair(base: Option<u16>, head: Option<u16>, allowed: &[(u16, u16)]) -> bool {
        let pair = (base.unwrap_or(0), head.unwrap_or(0));
        allowed.contains(&pair)
    }

    /// Evaluates finite membership, uniqueness, authority, and payload completeness.
    #[must_use]
    pub fn surface_complete(expected: &[Vec<u8>], actual: &[(Vec<u8>, bool, bool)]) -> bool {
        let expected = expected.iter().collect::<BTreeSet<_>>();
        if expected.len() != actual.len() {
            return false;
        }
        let mut seen = BTreeSet::new();
        actual.iter().all(|(key, authority, payload)| {
            expected.contains(key) && seen.insert(key) && *authority && *payload
        })
    }

    /// True only when a nonempty binding list is entirely current.
    #[must_use]
    pub fn all_current(bindings: &[bool]) -> bool {
        !bindings.is_empty() && bindings.iter().all(|binding| *binding)
    }

    /// Computes sorted present and missing required role tags.
    #[must_use]
    pub fn required_roles(required: &[u16], carriers: &[(u16, bool)]) -> (Vec<u16>, Vec<u16>) {
        let mut required = required.to_vec();
        required.sort_unstable();
        required.dedup();
        required.into_iter().partition(|role| {
            carriers
                .iter()
                .any(|(carrier_role, current)| carrier_role == role && *current)
        })
    }

    /// Independently classifies policy scope/role/mode/lineage weakening.
    #[must_use]
    pub fn policy_ratchet(
        base_subjects: &BTreeSet<String>,
        head_subjects: &BTreeSet<String>,
        base_roles: &BTreeSet<u16>,
        head_roles: &BTreeSet<u16>,
        base_blocking: bool,
        head_blocking: bool,
        same_lineage: bool,
    ) -> (Vec<String>, Vec<u16>, bool, bool) {
        (
            base_subjects.difference(head_subjects).cloned().collect(),
            base_roles.difference(head_roles).copied().collect(),
            base_blocking && !head_blocking,
            !same_lineage,
        )
    }

    /// Independently derives orthogonal assurance tag identities.
    #[must_use]
    #[allow(clippy::too_many_arguments, clippy::fn_params_excessive_bools)]
    pub fn assurance_classes(
        linked: bool,
        fresh: bool,
        selected: bool,
        verdicts: &[u16],
        exhaustive: bool,
        formal: bool,
        structural: bool,
        reviewed: bool,
    ) -> Vec<u16> {
        let mut result = Vec::new();
        if linked {
            result.push(1);
        }
        if linked
            && fresh
            && selected
            && !verdicts.is_empty()
            && verdicts.iter().all(|verdict| *verdict == 2)
        {
            result.push(2);
        }
        if exhaustive {
            result.push(3);
        }
        if formal {
            result.push(4);
        }
        if structural {
            result.push(5);
        }
        if reviewed {
            result.push(6);
        }
        result
    }

    /// Computes a graph's direct changes and reverse-reference fixed point.
    #[must_use]
    pub fn transitive_impacts(
        base: &BTreeMap<String, (u8, Vec<String>)>,
        head: &BTreeMap<String, (u8, Vec<String>)>,
    ) -> BTreeSet<(String, bool)> {
        let keys = base
            .keys()
            .chain(head.keys())
            .cloned()
            .collect::<BTreeSet<_>>();
        let direct = keys
            .into_iter()
            .filter(|key| base.get(key).map(|value| value.0) != head.get(key).map(|value| value.0))
            .collect::<BTreeSet<_>>();
        let mut impacted = direct.clone();
        loop {
            let before = impacted.len();
            for graph in [base, head] {
                for (key, (_, references)) in graph {
                    if references
                        .iter()
                        .any(|reference| impacted.contains(reference))
                    {
                        impacted.insert(key.clone());
                    }
                }
            }
            if impacted.len() == before {
                break;
            }
        }
        impacted
            .into_iter()
            .map(|key| {
                let direct = direct.contains(&key);
                (key, direct)
            })
            .collect()
    }

    /// True when exactly one disposition exists for each impact and no other disposition exists.
    #[must_use]
    pub fn disposition_closed(impacts: &[String], dispositions: &[String]) -> bool {
        let impact_counts = counts(impacts);
        let disposition_counts = counts(dispositions);
        impact_counts.keys().eq(disposition_counts.keys())
            && impact_counts.values().all(|count| *count == 1)
            && disposition_counts.values().all(|count| *count == 1)
    }

    fn counts(values: &[String]) -> BTreeMap<&str, usize> {
        let mut counts = BTreeMap::new();
        for value in values {
            *counts.entry(value.as_str()).or_default() += 1;
        }
        counts
    }

    /// Canonicalizes portable path text without consulting host filesystem state.
    ///
    /// # Errors
    ///
    /// Returns a stable category for nonportable path syntax.
    pub fn portable_path(input: &str) -> Result<String, &'static str> {
        if input.is_empty()
            || input.starts_with('/')
            || input.ends_with('/')
            || input.contains('\\')
            || input.as_bytes().contains(&0)
            || (input.len() >= 2 && input.as_bytes()[1] == b':')
        {
            return Err("relative");
        }
        let mut output = Vec::new();
        for segment in input.split('/') {
            if segment.is_empty() || matches!(segment, "." | "..") {
                return Err("segment");
            }
            if segment.ends_with([' ', '.'])
                || segment.chars().any(|character| {
                    character.is_ascii_control()
                        || matches!(character, '<' | '>' | ':' | '"' | '|' | '?' | '*')
                })
            {
                return Err("character");
            }
            let normalized = segment.nfc().collect::<String>();
            let base = normalized.split('.').next().unwrap_or(&normalized);
            let upper = base.to_ascii_uppercase();
            if matches!(upper.as_str(), "CON" | "PRN" | "AUX" | "NUL")
                || numbered(&upper, "COM")
                || numbered(&upper, "LPT")
            {
                return Err("reserved");
            }
            output.push(normalized);
        }
        Ok(output.join("/"))
    }

    /// Sorts independent portable paths and rejects canonical or folded collisions.
    ///
    /// # Errors
    ///
    /// Returns a stable category for invalid paths or canonical/portable collisions.
    pub fn portable_path_set(paths: &[String]) -> Result<Vec<String>, &'static str> {
        let mut canonical = paths
            .iter()
            .map(|path| portable_path(path))
            .collect::<Result<Vec<_>, _>>()?;
        canonical.sort_unstable();
        if canonical.windows(2).any(|pair| pair[0] == pair[1]) {
            return Err("canonical-collision");
        }
        let mut folded = canonical.iter().map(|path| fold(path)).collect::<Vec<_>>();
        folded.sort_unstable();
        if folded.windows(2).any(|pair| pair[0] == pair[1]) {
            return Err("portable-collision");
        }
        Ok(canonical)
    }

    fn fold(value: &str) -> String {
        value
            .chars()
            .flat_map(|character| match character {
                'ß' | 'ẞ' => "ss".chars().collect::<Vec<_>>(),
                _ => character.to_lowercase().collect::<Vec<_>>(),
            })
            .collect::<String>()
            .nfc()
            .collect()
    }

    fn numbered(value: &str, prefix: &str) -> bool {
        value.strip_prefix(prefix).is_some_and(|suffix| {
            matches!(suffix, "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9")
        })
    }

    /// Returns canonical breadth-first reachability for model cross-checks.
    #[must_use]
    pub fn reachable(initial: &[String], edges: &[(String, String)]) -> BTreeSet<String> {
        let mut reached = initial.iter().cloned().collect::<BTreeSet<_>>();
        let mut pending = initial.iter().cloned().collect::<VecDeque<_>>();
        while let Some(current) = pending.pop_front() {
            for (_, target) in edges.iter().filter(|(source, _)| source == &current) {
                if reached.insert(target.clone()) {
                    pending.push_back(target.clone());
                }
            }
        }
        reached
    }
}
