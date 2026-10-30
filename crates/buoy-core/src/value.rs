//! Versioned canonical values, digests, and repository-relative paths.

use crate::casefold_table::FULL_DEFAULT_CASE_FOLD;

mod hash;

/// Historical version 2 semantic schema identifier retained for migration tools.
pub const LEGACY_SEMANTIC_SCHEMA_V2: &str = "spec-provenance/v2";
/// Pinned Unicode release used for NFC and portable full case folding.
pub const UNICODE_DATA_VERSION: &str = crate::casefold_table::UNICODE_CASE_FOLD_VERSION;
/// Version 3 raw-artifact digest of the committed Unicode full case-fold source data.
pub const UNICODE_CASE_FOLD_SOURCE_DIGEST: &str =
    crate::casefold_table::UNICODE_CASE_FOLD_SOURCE_DIGEST;
use alloc::boxed::Box;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::fmt;
use unicode_normalization::UnicodeNormalization;

/// The assurance statement that one registered digest member supports.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Hash)]
pub enum DigestAssuranceClass {
    /// Deterministic change detection without a collision-resistance claim.
    ChangeDetection,
    /// Collision resistance without an authenticity or proof claim.
    CollisionResistant,
}

impl DigestAssuranceClass {
    /// Returns the canonical assurance-class name.
    #[must_use]
    pub const fn canonical_name(self) -> &'static str {
        match self {
            Self::ChangeDetection => "change-detection",
            Self::CollisionResistant => "collision-resistant",
        }
    }
}

/// One member of the closed version 3 digest registry.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Hash)]
#[repr(u16)]
pub enum DigestAlgorithm {
    /// Full XXH3-128 output with seed zero and the standard default secret.
    Xxh3_128 = 2,
    /// Unkeyed BLAKE3 hash-mode output at 32 bytes.
    Blake3_256 = 3,
    /// Full SHA-256 output from FIPS 180-4.
    Sha256 = 4,
}

impl DigestAlgorithm {
    /// Every member in canonical numeric-tag order.
    pub const ALL: [Self; 3] = [Self::Xxh3_128, Self::Blake3_256, Self::Sha256];

    /// Parses one exact canonical registry name.
    #[must_use]
    pub const fn from_canonical_name(name: &str) -> Option<Self> {
        match name.as_bytes() {
            b"xxh3-128" => Some(Self::Xxh3_128),
            b"blake3-256" => Some(Self::Blake3_256),
            b"sha-256" => Some(Self::Sha256),
            _ => None,
        }
    }

    /// Parses one registered numeric tag. Obsolete tags 0 and 1 return `None`.
    #[must_use]
    pub const fn from_tag(tag: u16) -> Option<Self> {
        match tag {
            2 => Some(Self::Xxh3_128),
            3 => Some(Self::Blake3_256),
            4 => Some(Self::Sha256),
            _ => None,
        }
    }

    /// Returns the immutable canonical registry name.
    #[must_use]
    pub const fn canonical_name(self) -> &'static str {
        match self {
            Self::Xxh3_128 => "xxh3-128",
            Self::Blake3_256 => "blake3-256",
            Self::Sha256 => "sha-256",
        }
    }

    /// Returns the immutable numeric registry tag.
    #[must_use]
    pub const fn tag(self) -> u16 {
        self as u16
    }

    /// Returns the exact output length in bytes.
    #[must_use]
    pub const fn output_length(self) -> usize {
        match self {
            Self::Xxh3_128 => 16,
            Self::Blake3_256 | Self::Sha256 => 32,
        }
    }

    /// Returns the declared assurance class.
    #[must_use]
    pub const fn assurance_class(self) -> DigestAssuranceClass {
        match self {
            Self::Xxh3_128 => DigestAssuranceClass::ChangeDetection,
            Self::Blake3_256 | Self::Sha256 => DigestAssuranceClass::CollisionResistant,
        }
    }

    /// Returns the Cargo feature that provides this member.
    #[must_use]
    pub const fn cargo_feature(self) -> &'static str {
        match self {
            Self::Xxh3_128 => "xxh3-128",
            Self::Blake3_256 => "blake3-256",
            Self::Sha256 => "sha-256",
        }
    }

    /// Reports whether this build contains the member adapter.
    #[must_use]
    pub const fn is_available(self) -> bool {
        match self {
            Self::Xxh3_128 => cfg!(feature = "xxh3-128"),
            Self::Blake3_256 => cfg!(feature = "blake3-256"),
            Self::Sha256 => cfg!(feature = "sha-256"),
        }
    }
}

/// A parsed active digest member that is available in this build.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ActiveAlgorithm {
    algorithm: DigestAlgorithm,
}

const fn active_algorithm_from_availability(
    algorithm: DigestAlgorithm,
    available: bool,
) -> Result<ActiveAlgorithm, ActiveAlgorithmError> {
    if available {
        Ok(ActiveAlgorithm { algorithm })
    } else {
        Err(ActiveAlgorithmError::Unavailable(algorithm))
    }
}

impl ActiveAlgorithm {
    /// Parses one registered and available member into the active-member type.
    ///
    /// # Errors
    ///
    /// Returns an unavailable-member error when the required Cargo feature is absent.
    pub const fn new(algorithm: DigestAlgorithm) -> Result<Self, ActiveAlgorithmError> {
        active_algorithm_from_availability(algorithm, algorithm.is_available())
    }

    /// Selects the first available build member for compatibility callers.
    ///
    /// # Errors
    ///
    /// Returns an unavailable-member error when the build contains no hash adapter.
    pub const fn build_default() -> Result<Self, ActiveAlgorithmError> {
        #[cfg(feature = "xxh3-128")]
        return Self::new(DigestAlgorithm::Xxh3_128);
        #[cfg(all(not(feature = "xxh3-128"), feature = "blake3-256"))]
        return Self::new(DigestAlgorithm::Blake3_256);
        #[cfg(all(not(feature = "xxh3-128"), not(feature = "blake3-256")))]
        Self::new(DigestAlgorithm::Sha256)
    }

    /// Returns the selected registry member.
    #[must_use]
    pub const fn algorithm(self) -> DigestAlgorithm {
        self.algorithm
    }

    /// Computes only under this active member.
    ///
    /// # Errors
    ///
    /// Returns an inactive-member error when `requested` differs from this value.
    ///
    /// # Panics
    ///
    /// Panics if this parsed active member loses its build adapter.
    #[allow(clippy::expect_used)]
    pub fn compute(
        self,
        requested: DigestAlgorithm,
        preimage: &[u8],
    ) -> Result<SemanticDigest, ActiveAlgorithmError> {
        if requested != self.algorithm {
            return Err(ActiveAlgorithmError::Inactive {
                active: self.algorithm,
                requested,
            });
        }
        let bytes = hash::hash(self.algorithm, preimage)
            .expect("the parsed active member must retain its build adapter");
        Ok(SemanticDigest::from_buffer(self.algorithm, bytes))
    }

    fn hash(self, preimage: &[u8]) -> Result<SemanticDigest, ActiveAlgorithmError> {
        self.compute(self.algorithm, preimage)
    }
}

/// Active-member construction or dispatch failure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ActiveAlgorithmError {
    /// The member adapter is absent from this build.
    Unavailable(DigestAlgorithm),
    /// A caller requested computation under an inactive member.
    Inactive {
        /// The configured active member.
        active: DigestAlgorithm,
        /// The requested member.
        requested: DigestAlgorithm,
    },
}

impl ActiveAlgorithmError {
    /// Returns the stable machine diagnostic code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::Unavailable(_) => "DIG-ALGORITHM-UNAVAILABLE",
            Self::Inactive { .. } => "DIG-INACTIVE-ALGORITHM",
        }
    }
}

/// A semantic comparison received values from different registry members.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DigestAlgorithmMismatch;

impl DigestAlgorithmMismatch {
    /// Returns the stable machine diagnostic code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        "DIG-ALGORITHM-MISMATCH"
    }
}

pub(crate) fn encoding_result<T>(
    result: Result<T, ActiveAlgorithmError>,
) -> Result<T, EncodingError> {
    result.map_err(|_| EncodingError::UnavailableAlgorithm)
}

/// Runs the active-algorithm error translation for coverage controls.
///
/// # Errors
///
/// Always returns the stable unavailable-algorithm error.
#[cfg(feature = "coverage-controls")]
#[doc(hidden)]
pub fn encoding_error_control() -> Result<ActiveAlgorithm, EncodingError> {
    encoding_result(active_algorithm_from_availability(
        DigestAlgorithm::Xxh3_128,
        false,
    ))
}

/// Runs digest-result error translation for coverage controls.
///
/// # Errors
///
/// Always returns the stable unavailable-algorithm error.
#[cfg(feature = "coverage-controls")]
#[doc(hidden)]
pub fn encoding_digest_error_control() -> Result<SemanticDigest, EncodingError> {
    encoding_result(Err(ActiveAlgorithmError::Unavailable(
        DigestAlgorithm::Xxh3_128,
    )))
}

/// A full algorithm-qualified semantic digest.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Hash)]
pub struct SemanticDigest {
    algorithm: DigestAlgorithm,
    buffer: [u8; 32],
}

impl SemanticDigest {
    const fn from_buffer(algorithm: DigestAlgorithm, buffer: [u8; 32]) -> Self {
        Self { algorithm, buffer }
    }

    /// Constructs an XXH3-128 digest from all 16 canonical bytes.
    #[must_use]
    pub const fn xxh3_128(bytes: [u8; 16]) -> Self {
        let mut buffer = [0_u8; 32];
        let mut index = 0;
        while index < 16 {
            buffer[index] = bytes[index];
            index += 1;
        }
        Self::from_buffer(DigestAlgorithm::Xxh3_128, buffer)
    }

    /// Constructs a BLAKE3-256 digest from all 32 canonical bytes.
    #[must_use]
    pub const fn blake3_256(bytes: [u8; 32]) -> Self {
        Self::from_buffer(DigestAlgorithm::Blake3_256, bytes)
    }

    /// Constructs a SHA-256 digest from all 32 canonical bytes.
    #[must_use]
    pub const fn sha_256(bytes: [u8; 32]) -> Self {
        Self::from_buffer(DigestAlgorithm::Sha256, bytes)
    }

    /// Parses exact output bytes for one registry member.
    ///
    /// # Errors
    ///
    /// Returns a length error when the slice length differs from the registry.
    pub fn from_bytes(algorithm: DigestAlgorithm, bytes: &[u8]) -> Result<Self, DigestBinaryError> {
        if bytes.len() != algorithm.output_length() {
            return Err(DigestBinaryError::Length);
        }
        let mut buffer = [0_u8; 32];
        buffer[..bytes.len()].copy_from_slice(bytes);
        Ok(Self::from_buffer(algorithm, buffer))
    }

    /// Returns the bound algorithm.
    #[must_use]
    pub const fn algorithm(self) -> DigestAlgorithm {
        self.algorithm
    }

    /// Compares semantic equality only when both values use the same member.
    ///
    /// # Errors
    ///
    /// Returns an algorithm-mismatch error for values from different members.
    pub fn checked_eq(&self, other: &Self) -> Result<bool, DigestAlgorithmMismatch> {
        if self.algorithm != other.algorithm {
            return Err(DigestAlgorithmMismatch);
        }
        Ok(self.bytes() == other.bytes())
    }

    /// Compares semantic order only when both values use the same member.
    ///
    /// Canonical container encoders can still use [`Ord`] over complete tagged values.
    ///
    /// # Errors
    ///
    /// Returns an algorithm-mismatch error for values from different members.
    pub fn checked_cmp(
        &self,
        other: &Self,
    ) -> Result<core::cmp::Ordering, DigestAlgorithmMismatch> {
        if self.algorithm != other.algorithm {
            return Err(DigestAlgorithmMismatch);
        }
        Ok(self.bytes().cmp(other.bytes()))
    }

    /// Returns exactly the number of bytes declared by the bound algorithm.
    #[must_use]
    pub fn bytes(&self) -> &[u8] {
        &self.buffer[..self.algorithm.output_length()]
    }

    /// Parses the exact version 3 lowercase algorithm-qualified text form.
    ///
    /// # Errors
    ///
    /// Returns a digest-text error for unknown algorithms, noncanonical case,
    /// wrong length, or nonhexadecimal digits.
    pub fn parse_canonical(input: &str) -> Result<Self, DigestTextError> {
        let (name, hex) = input
            .split_once(':')
            .ok_or(DigestTextError::AlgorithmOrCase)?;
        let algorithm =
            DigestAlgorithm::from_canonical_name(name).ok_or(DigestTextError::AlgorithmOrCase)?;
        if hex.len() != algorithm.output_length() * 2 {
            return Err(DigestTextError::Length);
        }
        let mut buffer = [0_u8; 32];
        for (index, pair) in hex.as_bytes().chunks_exact(2).enumerate() {
            let high = hex_nibble(pair[0]).ok_or(DigestTextError::Hex)?;
            let low = hex_nibble(pair[1]).ok_or(DigestTextError::Hex)?;
            buffer[index] = (high << 4) | low;
        }
        Ok(Self::from_buffer(algorithm, buffer))
    }

    /// Writes the canonical lowercase algorithm-qualified display.
    #[must_use]
    pub fn canonical_text(&self) -> String {
        const HEX: &[u8; 16] = b"0123456789abcdef";
        let mut output = String::from(self.algorithm.canonical_name());
        output.push(':');
        for byte in self.bytes() {
            output.push(char::from(HEX[usize::from(byte >> 4)]));
            output.push(char::from(HEX[usize::from(byte & 0x0f)]));
        }
        output
    }

    /// Writes the exact canonical-value digest encoding.
    #[must_use]
    #[allow(clippy::cast_possible_truncation)]
    pub fn canonical_binary(&self) -> Vec<u8> {
        let payload_length = self.algorithm.output_length() + 4;
        let mut output = Vec::with_capacity(payload_length + 9);
        output.push(12);
        output.extend_from_slice(&(payload_length as u64).to_be_bytes());
        output.extend_from_slice(&self.algorithm.tag().to_be_bytes());
        output.extend_from_slice(&(self.algorithm.output_length() as u16).to_be_bytes());
        output.extend_from_slice(self.bytes());
        output
    }

    /// Parses one exact canonical-value version 3 digest encoding.
    ///
    /// # Errors
    ///
    /// Returns a stable error for a wrong type tag, unknown algorithm tag,
    /// wrong declared length, or truncated input.
    pub fn parse_canonical_binary(input: &[u8]) -> Result<Self, DigestBinaryError> {
        if input.len() < 13 {
            return Err(DigestBinaryError::Length);
        }
        if input[0] != 12 {
            return Err(DigestBinaryError::TypeTag);
        }
        let payload_length = u64::from_be_bytes([
            input[1], input[2], input[3], input[4], input[5], input[6], input[7], input[8],
        ]);
        if payload_length != (input.len() - 9) as u64 {
            return Err(DigestBinaryError::Length);
        }
        let tag = u16::from_be_bytes([input[9], input[10]]);
        let algorithm = DigestAlgorithm::from_tag(tag).ok_or(DigestBinaryError::Algorithm)?;
        let declared_length = usize::from(u16::from_be_bytes([input[11], input[12]]));
        if declared_length != algorithm.output_length()
            || input.len() != 13 + algorithm.output_length()
        {
            return Err(DigestBinaryError::Length);
        }
        Self::from_bytes(algorithm, &input[13..])
    }
}

fn hex_nibble(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        _ => None,
    }
}

/// Canonical digest-text validation failure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DigestTextError {
    /// Missing prefix or an unknown or noncanonical algorithm spelling.
    AlgorithmOrCase,
    /// Wrong hexadecimal payload length.
    Length,
    /// Invalid or uppercase hexadecimal digit.
    Hex,
}

impl DigestTextError {
    /// Stable machine diagnostic code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::AlgorithmOrCase => "DET-DIGEST-ALGORITHM",
            Self::Length => "DET-DIGEST-LENGTH",
            Self::Hex => "DET-DIGEST-HEX",
        }
    }
}

/// Canonical digest-binary validation failure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DigestBinaryError {
    /// The canonical-value type tag is not digest tag 12.
    TypeTag,
    /// The algorithm tag is obsolete or unknown.
    Algorithm,
    /// The envelope or digest length is not exact.
    Length,
}

impl DigestBinaryError {
    /// Stable machine diagnostic code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::TypeTag => "DET-DIGEST-TYPE-TAG",
            Self::Algorithm => "DET-DIGEST-ALGORITHM",
            Self::Length => "DET-DIGEST-LENGTH",
        }
    }
}

impl fmt::Display for SemanticDigest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.canonical_text())
    }
}

/// An owned value in the closed v2 canonical value algebra.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CanonicalValue {
    /// Boolean false.
    False,
    /// Boolean true.
    True,
    /// Unsigned 64-bit integer.
    U64(u64),
    /// Signed 64-bit integer.
    I64(i64),
    /// Arbitrary bytes.
    Bytes(Vec<u8>),
    /// NFC UTF-8 text.
    Text(String),
    /// Record fields keyed by schema field tag.
    Record(Vec<(u16, Self)>),
    /// Semantically ordered values.
    Sequence(Vec<Self>),
    /// Semantically unordered unique values.
    Set(Vec<Self>),
    /// Explicit absent option.
    None,
    /// Explicit present option.
    Some(Box<Self>),
    /// Closed enum variant and optional payload.
    Enum(u16, Option<Box<Self>>),
    /// Full algorithm-qualified digest.
    Digest(SemanticDigest),
}

/// A deterministic canonicalization failure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EncodingError {
    /// Record tags were duplicate or not strictly increasing.
    InvalidRecordTags,
    /// Set values contained duplicate complete encodings.
    DuplicateSetValue,
    /// A text value was not already NFC.
    NonCanonicalText,
    /// The build contains no available active digest member.
    UnavailableAlgorithm,
}

impl EncodingError {
    /// Stable machine diagnostic code.
    #[must_use]
    pub const fn code(&self) -> &'static str {
        match self {
            Self::InvalidRecordTags => "DET-ENC-RECORD-TAGS",
            Self::DuplicateSetValue => "DET-DUP-SET-VALUE",
            Self::NonCanonicalText => "DET-ENC-TEXT-NFC",
            Self::UnavailableAlgorithm => "DIG-ALGORITHM-UNAVAILABLE",
        }
    }
}

/// Resource limits applied before canonical decoder allocation or descent.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DecodeBudget {
    /// Maximum complete encoded input bytes.
    pub max_bytes: u64,
    /// Maximum recursive canonical-value nesting levels.
    pub max_nesting: u16,
    /// Maximum cumulative decoded values, including the root.
    pub max_values: u64,
}

impl DecodeBudget {
    /// Frozen production defaults from `schema/v3/resource-budgets.toml`.
    #[must_use]
    pub const fn v2_default() -> Self {
        Self {
            max_bytes: 536_870_912,
            max_nesting: 128,
            max_values: 5_000_000,
        }
    }
}

/// Strict canonical decoding failure with a stable diagnostic family.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DecodingError {
    /// Complete encoded input exceeds the configured byte budget.
    ByteBudget,
    /// Recursive nesting exceeds the configured bound.
    NestingBudget,
    /// Declared or cumulative value count exceeds the configured bound.
    ValueBudget,
    /// A tag is unknown to the closed canonical value algebra.
    UnknownTypeTag,
    /// A payload length is truncated, overflows, or is wrong for its tag.
    InvalidLength,
    /// Bytes do not contain canonical UTF-8 NFC text.
    NonCanonicalText,
    /// Record fields are duplicate, unordered, omitted from their declared count, or truncated.
    InvalidRecord,
    /// A sequence count or payload boundary is invalid.
    InvalidSequence,
    /// A set is duplicate or not ordered by complete encoded bytes.
    InvalidSet,
    /// An option has an unknown/noncanonical form or wrong payload shape.
    InvalidOption,
    /// An enum has tag zero or a malformed explicit option payload.
    InvalidEnum,
    /// A digest algorithm, declared length, or payload shape is unsupported.
    InvalidDigest,
    /// Bytes remain after one complete canonical value.
    TrailingBytes,
}

impl DecodingError {
    /// Stable machine diagnostic code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::ByteBudget => "DET-ENC-BUDGET-BYTES",
            Self::NestingBudget => "DET-ENC-BUDGET-NESTING",
            Self::ValueBudget => "DET-ENC-BUDGET-VALUES",
            Self::UnknownTypeTag => "DET-ENC-UNKNOWN-TYPE-TAG",
            Self::InvalidLength => "DET-ENC-LENGTH",
            Self::NonCanonicalText => "DET-ENC-TEXT-NFC",
            Self::InvalidRecord => "DET-ENC-RECORD-TAGS",
            Self::InvalidSequence => "DET-ENC-SEQUENCE",
            Self::InvalidSet => "DET-ORDER-SET",
            Self::InvalidOption => "DET-ENC-OPTION",
            Self::InvalidEnum => "DET-ENC-ENUM",
            Self::InvalidDigest => "DET-ENC-DIGEST",
            Self::TrailingBytes => "DET-ENC-TRAILING-BYTES",
        }
    }
}

impl CanonicalValue {
    /// Encodes this value using the v2 tag-length-payload format.
    ///
    /// # Errors
    ///
    /// Returns an encoding error for noncanonical text, record tags, or set
    /// duplicates.
    pub fn encode(&self) -> Result<Vec<u8>, EncodingError> {
        let (tag, payload) = self.payload()?;
        let mut encoded = Vec::with_capacity(9 + payload.len());
        encoded.push(tag);
        push_u64(&mut encoded, payload.len() as u64);
        encoded.extend_from_slice(&payload);
        Ok(encoded)
    }

    /// Strictly decodes exactly one complete v2 canonical value.
    ///
    /// # Errors
    ///
    /// Rejects noncanonical values, malformed lengths/counts, configured
    /// budget violations, unknown value/algorithm tags, and trailing bytes.
    pub fn decode(bytes: &[u8], budget: DecodeBudget) -> Result<Self, DecodingError> {
        let byte_count = bytes.len() as u64;
        if byte_count > budget.max_bytes {
            return Err(DecodingError::ByteBudget);
        }
        let mut decoder = Decoder {
            bytes,
            offset: 0,
            budget,
            values: 0,
        };
        let value = decoder.value(1)?;
        if decoder.offset != bytes.len() {
            return Err(DecodingError::TrailingBytes);
        }
        Ok(value)
    }

    #[allow(clippy::cast_possible_truncation)]
    fn payload(&self) -> Result<(u8, Vec<u8>), EncodingError> {
        match self {
            Self::False => Ok((0, Vec::new())),
            Self::True => Ok((1, Vec::new())),
            Self::U64(value) => Ok((2, value.to_be_bytes().to_vec())),
            Self::I64(value) => Ok((3, value.to_be_bytes().to_vec())),
            Self::Bytes(value) => Ok((4, value.clone())),
            Self::Text(value) => {
                if value.nfc().ne(value.chars()) {
                    return Err(EncodingError::NonCanonicalText);
                }
                Ok((5, value.as_bytes().to_vec()))
            }
            Self::Record(fields) => {
                if fields.windows(2).any(|pair| pair[0].0 >= pair[1].0) {
                    return Err(EncodingError::InvalidRecordTags);
                }
                let mut payload = Vec::new();
                push_u64(&mut payload, fields.len() as u64);
                for (field_tag, value) in fields {
                    payload.extend_from_slice(&field_tag.to_be_bytes());
                    payload.extend_from_slice(&value.encode()?);
                }
                Ok((6, payload))
            }
            Self::Sequence(values) => Ok((7, encode_collection(values, false)?)),
            Self::Set(values) => Ok((8, encode_collection(values, true)?)),
            Self::None => Ok((9, Vec::new())),
            Self::Some(value) => Ok((10, value.encode()?)),
            Self::Enum(variant, value) => {
                let mut payload = Vec::new();
                payload.extend_from_slice(&variant.to_be_bytes());
                payload.extend_from_slice(&value.as_ref().map_or_else(
                    || Self::None.encode(),
                    |item| Self::Some(item.clone()).encode(),
                )?);
                Ok((11, payload))
            }
            Self::Digest(digest) => {
                let mut payload = Vec::with_capacity(digest.algorithm().output_length() + 4);
                payload.extend_from_slice(&digest.algorithm().tag().to_be_bytes());
                payload
                    .extend_from_slice(&(digest.algorithm().output_length() as u16).to_be_bytes());
                payload.extend_from_slice(digest.bytes());
                Ok((12, payload))
            }
        }
    }
}

struct Decoder<'a> {
    bytes: &'a [u8],
    offset: usize,
    budget: DecodeBudget,
    values: u64,
}

impl Decoder<'_> {
    #[allow(clippy::cast_possible_truncation, clippy::too_many_lines)]
    fn value(&mut self, depth: u32) -> Result<CanonicalValue, DecodingError> {
        if depth > u32::from(self.budget.max_nesting) {
            return Err(DecodingError::NestingBudget);
        }
        self.values += 1;
        if self.values > self.budget.max_values {
            return Err(DecodingError::ValueBudget);
        }
        let tag = self.take(1)?[0];
        let payload_length = self.u64()?;
        let remaining = self.bytes.len() - self.offset;
        if payload_length > remaining as u64 {
            return Err(DecodingError::InvalidLength);
        }
        let length = payload_length as usize;
        let end = self.offset + length;
        let mut payload = Decoder {
            bytes: &self.bytes[self.offset..end],
            offset: 0,
            budget: self.budget,
            values: self.values,
        };
        let value = match tag {
            0 if length == 0 => CanonicalValue::False,
            1 if length == 0 => CanonicalValue::True,
            2 if length == 8 => CanonicalValue::U64(u64::from_be_bytes([
                payload.bytes[0],
                payload.bytes[1],
                payload.bytes[2],
                payload.bytes[3],
                payload.bytes[4],
                payload.bytes[5],
                payload.bytes[6],
                payload.bytes[7],
            ])),
            3 if length == 8 => CanonicalValue::I64(i64::from_be_bytes([
                payload.bytes[0],
                payload.bytes[1],
                payload.bytes[2],
                payload.bytes[3],
                payload.bytes[4],
                payload.bytes[5],
                payload.bytes[6],
                payload.bytes[7],
            ])),
            4 => CanonicalValue::Bytes(payload.bytes.to_vec()),
            5 => {
                let text = core::str::from_utf8(payload.bytes)
                    .map_err(|_| DecodingError::NonCanonicalText)?;
                if text.nfc().ne(text.chars()) {
                    return Err(DecodingError::NonCanonicalText);
                }
                CanonicalValue::Text(text.to_string())
            }
            6 => CanonicalValue::Record(payload.record(depth)?),
            7 => CanonicalValue::Sequence(payload.collection(depth, false)?),
            8 => CanonicalValue::Set(payload.collection(depth, true)?),
            9 if length == 0 => CanonicalValue::None,
            10 => {
                let nested = payload.value(depth + 1).map_err(map_nested_option_error)?;
                if payload.offset != payload.bytes.len() {
                    return Err(DecodingError::InvalidOption);
                }
                CanonicalValue::Some(Box::new(nested))
            }
            11 => {
                let variant = payload.u16().map_err(|_| DecodingError::InvalidEnum)?;
                if variant == 0 {
                    return Err(DecodingError::InvalidEnum);
                }
                let option = payload
                    .value(depth + 1)
                    .map_err(|_| DecodingError::InvalidEnum)?;
                let option = match option {
                    CanonicalValue::None => None,
                    CanonicalValue::Some(value) => Some(value),
                    _ => return Err(DecodingError::InvalidEnum),
                };
                if payload.offset != payload.bytes.len() {
                    return Err(DecodingError::InvalidEnum);
                }
                CanonicalValue::Enum(variant, option)
            }
            12 if length >= 4 => {
                let tag = u16::from_be_bytes([payload.bytes[0], payload.bytes[1]]);
                let digest_length =
                    usize::from(u16::from_be_bytes([payload.bytes[2], payload.bytes[3]]));
                let algorithm =
                    DigestAlgorithm::from_tag(tag).ok_or(DecodingError::InvalidDigest)?;
                if digest_length != algorithm.output_length() || length != digest_length + 4 {
                    return Err(DecodingError::InvalidDigest);
                }
                let mut buffer = [0_u8; 32];
                buffer[..digest_length].copy_from_slice(&payload.bytes[4..]);
                CanonicalValue::Digest(SemanticDigest::from_buffer(algorithm, buffer))
            }
            9 => return Err(DecodingError::InvalidOption),
            12 => return Err(DecodingError::InvalidDigest),
            0..=8 => return Err(DecodingError::InvalidLength),
            _ => return Err(DecodingError::UnknownTypeTag),
        };
        self.values = payload.values;
        self.offset = end;
        Ok(value)
    }

    fn record(&mut self, depth: u32) -> Result<Vec<(u16, CanonicalValue)>, DecodingError> {
        let count = self.count()?;
        let mut fields = Vec::with_capacity(count);
        let mut prior = None;
        for _ in 0..count {
            let tag = self.u16().map_err(|_| DecodingError::InvalidRecord)?;
            if tag == 0 || prior.is_some_and(|previous| previous >= tag) {
                return Err(DecodingError::InvalidRecord);
            }
            let value = self.value(depth + 1).map_err(map_nested_record_error)?;
            fields.push((tag, value));
            prior = Some(tag);
        }
        if self.offset != self.bytes.len() {
            return Err(DecodingError::InvalidRecord);
        }
        Ok(fields)
    }

    fn collection(
        &mut self,
        depth: u32,
        is_set: bool,
    ) -> Result<Vec<CanonicalValue>, DecodingError> {
        let count = self.count()?;
        let mut values = Vec::with_capacity(count);
        let mut prior: Option<&[u8]> = None;
        for _ in 0..count {
            let start = self.offset;
            let value = self.value(depth + 1)?;
            let encoded = &self.bytes[start..self.offset];
            if is_set && prior.is_some_and(|previous| previous >= encoded) {
                return Err(DecodingError::InvalidSet);
            }
            values.push(value);
            prior = Some(encoded);
        }
        if self.offset != self.bytes.len() {
            return Err(if is_set {
                DecodingError::InvalidSet
            } else {
                DecodingError::InvalidSequence
            });
        }
        Ok(values)
    }

    #[allow(clippy::cast_possible_truncation)]
    fn count(&mut self) -> Result<usize, DecodingError> {
        let count = self.u64()?;
        if count > self.budget.max_values.saturating_sub(self.values)
            || count > self.bytes.len() as u64
        {
            return Err(DecodingError::ValueBudget);
        }
        Ok(count as usize)
    }

    fn u16(&mut self) -> Result<u16, DecodingError> {
        let bytes = self.take(2)?;
        Ok(u16::from_be_bytes([bytes[0], bytes[1]]))
    }

    fn u64(&mut self) -> Result<u64, DecodingError> {
        let bytes = self.take(8)?;
        Ok(u64::from_be_bytes([
            bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
        ]))
    }

    fn take(&mut self, length: usize) -> Result<&[u8], DecodingError> {
        let remaining = self.bytes.len() - self.offset;
        if length > remaining {
            return Err(DecodingError::InvalidLength);
        }
        let end = self.offset + length;
        let slice = &self.bytes[self.offset..end];
        self.offset = end;
        Ok(slice)
    }
}

const fn map_nested_option_error(error: DecodingError) -> DecodingError {
    match error {
        DecodingError::NestingBudget
        | DecodingError::ValueBudget
        | DecodingError::ByteBudget
        | DecodingError::NonCanonicalText
        | DecodingError::UnknownTypeTag
        | DecodingError::InvalidDigest => error,
        _ => DecodingError::InvalidOption,
    }
}

const fn map_nested_record_error(error: DecodingError) -> DecodingError {
    match error {
        DecodingError::NestingBudget
        | DecodingError::ValueBudget
        | DecodingError::ByteBudget
        | DecodingError::NonCanonicalText
        | DecodingError::UnknownTypeTag
        | DecodingError::InvalidDigest => error,
        _ => DecodingError::InvalidRecord,
    }
}

fn encode_collection(
    values: &[CanonicalValue],
    sort_and_reject_duplicates: bool,
) -> Result<Vec<u8>, EncodingError> {
    let mut elements = values
        .iter()
        .map(CanonicalValue::encode)
        .collect::<Result<Vec<_>, _>>()?;
    if sort_and_reject_duplicates {
        elements.sort_unstable();
        if elements.windows(2).any(|pair| pair[0] == pair[1]) {
            return Err(EncodingError::DuplicateSetValue);
        }
    }
    let mut payload = Vec::new();
    push_u64(&mut payload, elements.len() as u64);
    for element in elements {
        payload.extend_from_slice(&element);
    }
    Ok(payload)
}

fn push_u64(buffer: &mut Vec<u8>, value: u64) {
    buffer.extend_from_slice(&value.to_be_bytes());
}

fn hash_canonical_preimage(
    active: ActiveAlgorithm,
    preimage: &[u8],
) -> Result<SemanticDigest, ActiveAlgorithmError> {
    active.hash(preimage)
}

fn append_encoded_value(buffer: &mut Vec<u8>, tag: u8, payload: &[u8]) {
    buffer.push(tag);
    push_u64(buffer, payload.len() as u64);
    buffer.extend_from_slice(payload);
}

/// Builds the version 3 preimage for an encoded canonical record.
fn semantic_preimage_from_encoded_record(
    active: ActiveAlgorithm,
    domain: &str,
    encoded_record: &[u8],
) -> Vec<u8> {
    let mut preimage = Vec::from(&b"SPEC-PROVENANCE\0"[..]);
    preimage.extend_from_slice(&3_u16.to_be_bytes());
    append_encoded_value(&mut preimage, 5, b"spec-provenance/v3");
    let mut algorithm = Vec::from(active.algorithm().tag().to_be_bytes());
    append_encoded_value(&mut algorithm, 9, &[]);
    append_encoded_value(&mut preimage, 11, &algorithm);
    append_encoded_value(&mut preimage, 5, domain.as_bytes());
    preimage.extend_from_slice(encoded_record);
    preimage
}

pub(crate) fn semantic_digest_from_encoded_record_with(
    active: ActiveAlgorithm,
    domain: &str,
    encoded_record: &[u8],
) -> Result<SemanticDigest, ActiveAlgorithmError> {
    hash_canonical_preimage(
        active,
        &semantic_preimage_from_encoded_record(active, domain, encoded_record),
    )
}

#[allow(clippy::expect_used)]
pub(crate) fn semantic_digest_from_encoded_record(
    domain: &str,
    encoded_record: &[u8],
) -> SemanticDigest {
    let active = ActiveAlgorithm::build_default()
        .expect("a digest-producing build must contain one registry adapter");
    semantic_digest_from_encoded_record_with(active, domain, encoded_record)
        .expect("a parsed active algorithm must retain its adapter")
}

/// Computes a canonical digest for a closed-media saved-bytes artifact.
///
/// # Panics
///
/// Panics if `active` does not retain its selected build adapter.
#[must_use]
#[allow(clippy::expect_used)]
pub fn raw_artifact_digest_with(
    active: ActiveAlgorithm,
    media_type: crate::model::RawArtifactMediaType,
    bytes: &[u8],
) -> SemanticDigest {
    let mut record_payload = Vec::new();
    push_u64(&mut record_payload, 2);
    record_payload.extend_from_slice(&1_u16.to_be_bytes());
    append_encoded_value(&mut record_payload, 5, media_type.as_str().as_bytes());
    record_payload.extend_from_slice(&2_u16.to_be_bytes());
    append_encoded_value(&mut record_payload, 4, bytes);
    let mut encoded_record = Vec::new();
    append_encoded_value(&mut encoded_record, 6, &record_payload);
    semantic_digest_from_encoded_record_with(active, "raw-artifact", &encoded_record)
        .expect("a parsed active algorithm must retain its adapter")
}

/// Computes a canonical digest with the first available build member.
///
/// # Panics
///
/// Panics if the build contains no digest adapter or the active member loses its adapter.
#[must_use]
#[allow(clippy::expect_used)]
pub fn raw_artifact_digest(
    media_type: crate::model::RawArtifactMediaType,
    bytes: &[u8],
) -> SemanticDigest {
    let active = ActiveAlgorithm::build_default()
        .expect("a digest-producing build must contain one registry adapter");
    raw_artifact_digest_with(active, media_type, bytes)
}

/// One computed digest paired with the canonical preimage available at construction time.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DigestObservation {
    digest: SemanticDigest,
    canonical_preimage: Vec<u8>,
}

impl DigestObservation {
    /// Returns the computed digest.
    #[must_use]
    pub const fn digest(&self) -> SemanticDigest {
        self.digest
    }

    /// Returns the complete canonical preimage used to compute the digest.
    #[must_use]
    pub fn canonical_preimage(&self) -> &[u8] {
        &self.canonical_preimage
    }
}

/// Failure while ordering simultaneously available digest/preimage observations.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DigestObservationError {
    /// The same canonical preimage was supplied more than once.
    DuplicateCanonicalPreimage,
    /// Unequal canonical preimages produced the same full XXH3-128 digest.
    ObservedCollision,
}

impl DigestObservationError {
    /// Stable machine diagnostic code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::DuplicateCanonicalPreimage => "DET-DUP-CANONICAL-PREIMAGE",
            Self::ObservedCollision => "DET-DIGEST-OBSERVED-COLLISION",
        }
    }
}

fn validate_digest_observations(
    mut observations: Vec<DigestObservation>,
) -> Result<Vec<DigestObservation>, DigestObservationError> {
    observations.sort_unstable_by(|left, right| {
        left.digest
            .cmp(&right.digest)
            .then_with(|| left.canonical_preimage.cmp(&right.canonical_preimage))
    });
    for pair in observations.windows(2) {
        if pair[0].digest == pair[1].digest {
            if pair[0].canonical_preimage == pair[1].canonical_preimage {
                return Err(DigestObservationError::DuplicateCanonicalPreimage);
            }
            return Err(DigestObservationError::ObservedCollision);
        }
    }
    Ok(observations)
}

/// Deterministic injected-observation control for exact collision-path testing.
#[cfg(feature = "coverage-controls")]
#[doc(hidden)]
pub fn validate_digest_observations_control(
    observations: Vec<(SemanticDigest, Vec<u8>)>,
) -> Result<(), DigestObservationError> {
    validate_digest_observations(
        observations
            .into_iter()
            .map(|(digest, canonical_preimage)| DigestObservation {
                digest,
                canonical_preimage,
            })
            .collect(),
    )
    .map(|_| ())
}

/// Computes and collision-checks preimages with one explicit active member.
///
/// # Errors
///
/// Rejects duplicate canonical preimages and observed unequal-preimage/equal-digest pairs.
///
/// # Panics
///
/// Panics if `active` does not retain its selected build adapter.
#[allow(clippy::expect_used)]
pub fn observe_canonical_preimages_with(
    active: ActiveAlgorithm,
    preimages: Vec<Vec<u8>>,
) -> Result<Vec<DigestObservation>, DigestObservationError> {
    validate_digest_observations(
        preimages
            .into_iter()
            .map(|canonical_preimage| DigestObservation {
                digest: hash_canonical_preimage(active, &canonical_preimage)
                    .expect("a parsed active algorithm must retain its adapter"),
                canonical_preimage,
            })
            .collect(),
    )
}

/// Computes and collision-checks preimages with the first available build member.
///
/// # Errors
///
/// Rejects duplicate canonical preimages and observed unequal-preimage/equal-digest pairs.
///
/// # Panics
///
/// Panics if the build contains no digest adapter or the active member loses its adapter.
#[allow(clippy::expect_used)]
pub fn observe_canonical_preimages(
    preimages: Vec<Vec<u8>>,
) -> Result<Vec<DigestObservation>, DigestObservationError> {
    let active = ActiveAlgorithm::build_default()
        .expect("a digest-producing build must contain one registry adapter");
    observe_canonical_preimages_with(active, preimages)
}

/// Computes a domain-separated version 3 digest with one explicit active member.
///
/// # Errors
///
/// Returns an encoding error when the payload violates canonical value rules.
pub fn semantic_digest_with(
    active: ActiveAlgorithm,
    domain: &str,
    payload_fields: Vec<(u16, CanonicalValue)>,
) -> Result<SemanticDigest, EncodingError> {
    let observation = semantic_digest_observation_with(active, domain, payload_fields)?;
    Ok(observation.digest())
}

/// Computes a historical version 2 digest for compatibility replay.
///
/// # Errors
///
/// Returns an encoding error when the XXH3-128 member is unavailable or the payload is invalid.
pub fn semantic_digest_v2(
    domain: &str,
    payload_fields: Vec<(u16, CanonicalValue)>,
) -> Result<SemanticDigest, EncodingError> {
    if domain.nfc().ne(domain.chars()) {
        return Err(EncodingError::NonCanonicalText);
    }
    let encoded_record = CanonicalValue::Record(payload_fields).encode()?;
    let mut preimage = Vec::from(&b"SPEC-PROVENANCE\0"[..]);
    preimage.extend_from_slice(&2_u16.to_be_bytes());
    append_encoded_value(&mut preimage, 5, LEGACY_SEMANTIC_SCHEMA_V2.as_bytes());
    let mut algorithm = Vec::from(DigestAlgorithm::Xxh3_128.tag().to_be_bytes());
    append_encoded_value(&mut algorithm, 9, &[]);
    append_encoded_value(&mut preimage, 11, &algorithm);
    append_encoded_value(&mut preimage, 5, domain.as_bytes());
    preimage.extend_from_slice(&encoded_record);
    encoding_result(ActiveAlgorithm::new(DigestAlgorithm::Xxh3_128))
        .and_then(|active| encoding_result(hash_canonical_preimage(active, &preimage)))
}

/// Computes a domain-separated version 3 digest with the first available member.
///
/// # Errors
///
/// Returns an encoding error when no member is available or the payload is invalid.
pub fn semantic_digest(
    domain: &str,
    payload_fields: Vec<(u16, CanonicalValue)>,
) -> Result<SemanticDigest, EncodingError> {
    encoding_result(ActiveAlgorithm::build_default())
        .and_then(|active| semantic_digest_with(active, domain, payload_fields))
}

/// Computes a digest for validated model fields with one active member.
#[allow(clippy::expect_used)]
pub(crate) fn semantic_digest_trusted_with(
    active: ActiveAlgorithm,
    domain: &str,
    payload_fields: Vec<(u16, CanonicalValue)>,
) -> SemanticDigest {
    semantic_digest_with(active, domain, payload_fields)
        .expect("validated model fields must retain canonical encoding")
}

/// Computes a digest for validated model fields with the first available member.
#[allow(clippy::expect_used)]
pub(crate) fn semantic_digest_trusted(
    domain: &str,
    payload_fields: Vec<(u16, CanonicalValue)>,
) -> SemanticDigest {
    semantic_digest(domain, payload_fields)
        .expect("validated model fields must retain canonical encoding")
}

/// Encodes, sorts, and rejects duplicate values at the shared value boundary.
pub(crate) fn canonicalize_set<T, F>(
    values: Vec<T>,
    canonical_value: F,
) -> Result<Vec<T>, EncodingError>
where
    F: Fn(&T) -> CanonicalValue,
{
    let canonical = values.iter().map(canonical_value).collect::<Vec<_>>();
    validated_set_keys(&canonical).map(|keys| {
        let mut encoded = keys.into_iter().zip(values).collect::<Vec<_>>();
        encoded.sort_unstable_by(|left, right| left.0.cmp(&right.0));
        encoded.into_iter().map(|(_, value)| value).collect()
    })
}

#[allow(clippy::expect_used)]
fn validated_set_keys(values: &[CanonicalValue]) -> Result<Vec<Vec<u8>>, EncodingError> {
    let keys = values
        .iter()
        .map(|value| {
            value
                .encode()
                .expect("validated model set members must retain canonical encoding")
        })
        .collect::<Vec<_>>();
    let mut sorted = keys.clone();
    sorted.sort_unstable();
    if sorted.windows(2).any(|pair| pair[0] == pair[1]) {
        return Err(EncodingError::DuplicateSetValue);
    }
    Ok(keys)
}

/// Sorts a set whose model-level validation already proved canonical uniqueness.
#[allow(clippy::expect_used)]
pub(crate) fn canonicalize_validated_set<T, F>(values: Vec<T>, canonical_value: F) -> Vec<T>
where
    F: Fn(&T) -> CanonicalValue,
{
    canonicalize_set(values, canonical_value)
        .expect("validated model set must retain canonical uniqueness")
}

/// Computes a semantic digest and retains its exact version 3 preimage.
///
/// # Errors
///
/// Returns an encoding error for a noncanonical domain or payload record.
///
/// # Panics
///
/// Panics if `active` loses its selected build adapter.
#[allow(clippy::expect_used)]
pub fn semantic_digest_observation_with(
    active: ActiveAlgorithm,
    domain: &str,
    payload_fields: Vec<(u16, CanonicalValue)>,
) -> Result<DigestObservation, EncodingError> {
    if domain.nfc().ne(domain.chars()) {
        return Err(EncodingError::NonCanonicalText);
    }
    let encoded_record = CanonicalValue::Record(payload_fields).encode()?;
    let canonical_preimage = semantic_preimage_from_encoded_record(active, domain, &encoded_record);
    let digest = hash_canonical_preimage(active, &canonical_preimage)
        .expect("the parsed active member must retain its build adapter");
    Ok(DigestObservation {
        digest,
        canonical_preimage,
    })
}

/// Computes a semantic digest with the first available build member.
///
/// # Errors
///
/// Returns an encoding error when no member is available or the input is invalid.
pub fn semantic_digest_observation(
    domain: &str,
    payload_fields: Vec<(u16, CanonicalValue)>,
) -> Result<DigestObservation, EncodingError> {
    encoding_result(ActiveAlgorithm::build_default())
        .and_then(|active| semantic_digest_observation_with(active, domain, payload_fields))
}

/// A canonical repository-relative path.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Hash)]
pub struct RepositoryPath(String);

impl RepositoryPath {
    /// Validates and NFC-normalizes a schema-v2 path.
    ///
    /// # Errors
    ///
    /// Returns a stable path-policy error for nonrelative, malformed, reserved,
    /// or forbidden path forms.
    pub fn new(input: &str) -> Result<Self, PathError> {
        if input.is_empty() || input.starts_with('/') || input.ends_with('/') {
            return Err(PathError::NotRelative);
        }
        if input.contains('\\') || input.as_bytes().contains(&0) {
            return Err(PathError::ForbiddenCharacter);
        }
        if input.len() >= 2 && input.as_bytes()[1] == b':' {
            return Err(PathError::NotRelative);
        }
        let mut normalized = String::new();
        for (index, raw_segment) in input.split('/').enumerate() {
            if raw_segment.is_empty() || raw_segment == "." || raw_segment == ".." {
                return Err(PathError::InvalidSegment);
            }
            if raw_segment.ends_with([' ', '.'])
                || raw_segment.chars().any(forbidden_path_character)
            {
                return Err(PathError::ForbiddenCharacter);
            }
            let segment = raw_segment.nfc().collect::<String>();
            if is_reserved_basename(&segment) {
                return Err(PathError::ReservedBasename);
            }
            if index != 0 {
                normalized.push('/');
            }
            normalized.push_str(&segment);
        }
        Ok(Self(normalized))
    }

    /// Constructs a reviewed valid path for deterministic collision injection.
    #[cfg(feature = "coverage-controls")]
    #[doc(hidden)]
    #[must_use]
    pub fn reviewed_control(input: &str) -> Self {
        Self(String::from(input))
    }

    /// Returns canonical UTF-8 path text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Returns the portable case-fold collision key.
    #[must_use]
    pub fn portable_collision_key(&self) -> String {
        portable_case_fold(&self.0)
    }
}

/// Applies the pinned Unicode 16.0 full default non-Turkic case fold and NFC.
///
/// This function is pure, locale independent, and backed by the committed
/// `CaseFolding.txt` C+F mapping table. It is public so conformance tests and
/// host adapters can use exactly the collision-key transformation.
#[must_use]
pub fn portable_case_fold(input: &str) -> String {
    let mut output = String::new();
    for character in input.chars() {
        match FULL_DEFAULT_CASE_FOLD.binary_search_by_key(&character, |entry| entry.0) {
            Ok(index) => output.extend(FULL_DEFAULT_CASE_FOLD[index].1.iter().copied()),
            Err(_) => output.push(character),
        }
    }
    output.nfc().collect()
}

fn forbidden_path_character(character: char) -> bool {
    character.is_ascii_control() || matches!(character, '<' | '>' | ':' | '"' | '|' | '?' | '*')
}

fn is_reserved_basename(segment: &str) -> bool {
    let base = segment.split('.').next().unwrap_or(segment);
    let folded = base.to_ascii_uppercase();
    matches!(folded.as_str(), "CON" | "PRN" | "AUX" | "NUL")
        || reserved_numbered(&folded, "COM")
        || reserved_numbered(&folded, "LPT")
}

fn reserved_numbered(value: &str, prefix: &str) -> bool {
    value.strip_prefix(prefix).is_some_and(|suffix| {
        let bytes = suffix.as_bytes();
        bytes.len() == 1 && (b'1'..=b'9').contains(&bytes[0])
    })
}

/// A deterministic path validation failure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PathError {
    /// The path is empty, absolute, drive-qualified, or directory-shaped.
    NotRelative,
    /// A segment is empty, current-directory, or parent-directory.
    InvalidSegment,
    /// The path contains a v2-forbidden character or suffix.
    ForbiddenCharacter,
    /// A segment uses a portable reserved device basename.
    ReservedBasename,
}

impl PathError {
    /// Stable machine diagnostic code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::NotRelative => "DET-PATH-NOT-RELATIVE",
            Self::InvalidSegment => "DET-PATH-SEGMENT",
            Self::ForbiddenCharacter => "DET-PATH-FORBIDDEN",
            Self::ReservedBasename => "DET-PATH-RESERVED",
        }
    }
}

/// Sorts paths and rejects canonical or portable collisions.
///
/// # Errors
///
/// Returns a collision error when two inputs share canonical bytes or one
/// portable case-fold key.
pub fn canonical_paths(
    mut paths: Vec<RepositoryPath>,
) -> Result<Vec<RepositoryPath>, PathSetError> {
    paths.sort_unstable();
    if paths.windows(2).any(|pair| pair[0] == pair[1]) {
        return Err(PathSetError::CanonicalCollision);
    }
    let mut keys = paths
        .iter()
        .map(RepositoryPath::portable_collision_key)
        .collect::<Vec<_>>();
    keys.sort_unstable();
    if keys.windows(2).any(|pair| pair[0] == pair[1]) {
        return Err(PathSetError::PortableCollision);
    }
    Ok(paths)
}

/// A deterministic snapshot path-set failure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PathSetError {
    /// Two inputs normalize to one canonical path.
    CanonicalCollision,
    /// Two canonical paths share one portable collision key.
    PortableCollision,
}
