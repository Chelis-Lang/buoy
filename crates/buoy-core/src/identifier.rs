//! Validated identifier newtypes for the schema-v2 domain.
//!
//! Every frozen identifier grammar is parsed exactly once, here, and the parse
//! yields a type. Model constructors accept the typed value, so an unvalidated
//! identifier is unrepresentable after construction and no downstream code
//! re-runs a grammar check.

use alloc::string::String;
use unicode_normalization::UnicodeNormalization;

/// A deterministic identifier validation failure.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Hash)]
pub enum IdentifierError {
    /// A stable object ID violated the frozen uppercase grammar.
    StableIdGrammar,
    /// A scope ID or namespace prefix violated the frozen lowercase grammar.
    ScopeIdGrammar,
    /// A schema ID violated the frozen versioned grammar.
    SchemaIdGrammar,
    /// A source item kind violated the versioned lower-case grammar.
    SourceItemKindGrammar,
    /// An external trace target was unsafe or noncanonical.
    ExternalTraceTargetGrammar,
}

impl IdentifierError {
    /// Stable machine diagnostic code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::StableIdGrammar => "SCF-CONSTRUCT-STABLE-ID",
            Self::ScopeIdGrammar => "SCF-CONSTRUCT-SCOPE-ID",
            Self::SchemaIdGrammar => "SCF-CONSTRUCT-SCHEMA-ID",
            Self::SourceItemKindGrammar => "SPG-SOURCE-ITEM-KIND",
            Self::ExternalTraceTargetGrammar => "SPG-EXTERNAL-TRACE-TARGET",
        }
    }
}

/// A validated stable object identifier.
///
/// The frozen grammar is one uppercase-or-digit head component that starts with
/// an uppercase letter, followed by at least one uppercase-or-digit component,
/// separated by `-`, with a total length of 3 through 128 bytes.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Hash)]
pub struct StableId(String);

impl StableId {
    /// Validates and NFC-normalizes a stable object identifier.
    ///
    /// # Errors
    ///
    /// Returns [`IdentifierError::StableIdGrammar`] when the normalized input
    /// falls outside the frozen uppercase stable-ID grammar.
    pub fn new(input: &str) -> Result<Self, IdentifierError> {
        let normalized = input.nfc().collect::<String>();
        if stable_id_grammar(&normalized) {
            Ok(Self(normalized))
        } else {
            Err(IdentifierError::StableIdGrammar)
        }
    }

    /// Returns the canonical UTF-8 identifier text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

fn stable_id_grammar(text: &str) -> bool {
    if !(3..=128).contains(&text.len()) {
        return false;
    }
    let mut components = text.split('-');
    // `str::split` always yields at least one component, including for `""`.
    let first = components.next().unwrap_or_default();
    let first_valid = !first.is_empty()
        && first.as_bytes()[0].is_ascii_uppercase()
        && first
            .bytes()
            .all(|byte| byte.is_ascii_uppercase() || byte.is_ascii_digit());
    let mut suffix_count = 0_u64;
    let suffixes_valid = components.all(|component| {
        suffix_count += 1;
        !component.is_empty()
            && component
                .bytes()
                .all(|byte| byte.is_ascii_uppercase() || byte.is_ascii_digit())
    });
    first_valid && suffix_count > 0 && suffixes_valid
}

fn scope_id_grammar(text: &str) -> bool {
    let mut components = text.split(['.', '-']);
    components.next().is_some_and(|component| {
        !component.is_empty()
            && component.as_bytes()[0].is_ascii_lowercase()
            && component
                .bytes()
                .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit())
    }) && components.all(|component| {
        !component.is_empty()
            && component
                .bytes()
                .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit())
    })
}

fn prefix_grammar(text: &str) -> bool {
    !text.is_empty()
        && text.split('-').all(|component| {
            !component.is_empty()
                && component
                    .bytes()
                    .all(|byte| byte.is_ascii_uppercase() || byte.is_ascii_digit())
        })
        && text.as_bytes()[0].is_ascii_uppercase()
}

fn versioned_lowercase_id_grammar(text: &str) -> bool {
    let Some((path, version)) = text.rsplit_once('/') else {
        return false;
    };
    let Some(version) = version.strip_prefix('v') else {
        return false;
    };
    !path.is_empty()
        && path.split('/').all(scope_id_grammar)
        && !version.is_empty()
        && version.bytes().all(|byte| byte.is_ascii_digit())
        && !version.starts_with('0')
}

fn schema_id_grammar(text: &str) -> bool {
    scope_id_grammar(text) || versioned_lowercase_id_grammar(text)
}

macro_rules! identifier_newtype {
    ($name:ident, $grammar:ident, $error:ident) => {
        #[doc = "A validated identifier newtype."]
        #[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Hash)]
        pub struct $name(String);

        impl $name {
            /// Validates and NFC-normalizes the identifier.
            ///
            /// # Errors
            ///
            /// Returns the role-specific grammar error when validation fails.
            pub fn new(input: &str) -> Result<Self, IdentifierError> {
                let normalized = input.nfc().collect::<String>();
                if $grammar(&normalized) {
                    Ok(Self(normalized))
                } else {
                    Err(IdentifierError::$error)
                }
            }

            /// Returns the canonical UTF-8 identifier text.
            #[must_use]
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }
    };
}

identifier_newtype!(ScopeId, scope_id_grammar, ScopeIdGrammar);
identifier_newtype!(SchemaId, schema_id_grammar, SchemaIdGrammar);
identifier_newtype!(
    SourceItemKind,
    versioned_lowercase_id_grammar,
    SourceItemKindGrammar
);

/// A normalized absolute HTTPS URI or versioned URN.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Hash)]
pub struct ExternalTraceTarget(String);

#[allow(clippy::needless_bitwise_bool)]
fn https_trace_target_grammar(input: &str) -> bool {
    let Some(rest) = input.strip_prefix("https://") else {
        return false;
    };
    let authority = rest.split(['/', '?', '#']).next().unwrap_or_default();
    !authority.is_empty()
        & authority.split('.').all(|label| {
            !label.is_empty()
                & label
                    .as_bytes()
                    .first()
                    .is_some_and(u8::is_ascii_alphanumeric)
                & label
                    .as_bytes()
                    .last()
                    .is_some_and(u8::is_ascii_alphanumeric)
                & label
                    .bytes()
                    .all(|byte| byte.is_ascii_lowercase() | byte.is_ascii_digit() | (byte == b'-'))
        })
        & !input.contains('#')
        & !input.contains('\\')
        & !input
            .bytes()
            .any(|byte| byte.is_ascii_whitespace() | byte.is_ascii_control())
}

impl ExternalTraceTarget {
    /// Parses one safe canonical trace target.
    ///
    /// # Errors
    ///
    /// Returns the external target grammar error for unsafe or noncanonical text.
    #[allow(clippy::needless_bitwise_bool)]
    pub fn new(input: &str) -> Result<Self, IdentifierError> {
        let normalized = input.nfc().collect::<String>();
        let valid_https = https_trace_target_grammar(&normalized);
        let valid_urn = normalized.strip_prefix("urn:").is_some_and(|rest| {
            let parts = rest.split(':').collect::<alloc::vec::Vec<_>>();
            (parts.len() >= 3)
                & parts.iter().all(|part| {
                    !part.is_empty()
                        & part.bytes().all(|byte| {
                            byte.is_ascii_lowercase()
                                | byte.is_ascii_digit()
                                | matches!(byte, b'-' | b'.' | b'/')
                        })
                })
                & parts.last().is_some_and(|part| {
                    part.strip_prefix('v').is_some_and(|version| {
                        !version.is_empty()
                            & !version.starts_with('0')
                            & version.bytes().all(|byte| byte.is_ascii_digit())
                    })
                })
        });
        if valid_https | valid_urn {
            Ok(Self(normalized))
        } else {
            Err(IdentifierError::ExternalTraceTargetGrammar)
        }
    }

    /// Returns the canonical target text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<ScopeId> for String {
    fn from(value: ScopeId) -> Self {
        value.0
    }
}

impl PartialEq<str> for ScopeId {
    fn eq(&self, other: &str) -> bool {
        self.as_str() == other
    }
}

impl PartialEq<String> for ScopeId {
    fn eq(&self, other: &String) -> bool {
        self.as_str() == other
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[allow(clippy::unwrap_used)]
    fn remaining_identifier_roles_parse() {
        assert_eq!(ScopeId::new("spec.graph").unwrap().as_str(), "spec.graph");
        assert_eq!(
            SchemaId::new("spec.graph/v1").unwrap().as_str(),
            "spec.graph/v1"
        );
        assert_eq!(NamespacePrefix::new("ATOM-1").unwrap().as_str(), "ATOM-1");
        assert_eq!(
            SourceItemKind::new("openspec/requirement/v1")
                .unwrap()
                .as_str(),
            "openspec/requirement/v1"
        );
    }

    #[test]
    #[allow(clippy::unwrap_used)]
    fn remaining_identifier_roles_report_frozen_codes() {
        assert_eq!(
            ScopeId::new("Bad").unwrap_err().code(),
            "SCF-CONSTRUCT-SCOPE-ID"
        );
        assert_eq!(
            SchemaId::new("bad/").unwrap_err().code(),
            "SCF-CONSTRUCT-SCHEMA-ID"
        );
        assert_eq!(
            NamespacePrefix::new("atom-1").unwrap_err().code(),
            "SCF-CONSTRUCT-SCOPE-ID"
        );
        assert!(ScopeId::new("bad\u{301}").is_err());
        assert!(SchemaId::new("bad/\u{301}").is_err());
        assert_eq!(
            SourceItemKind::new("rust-item").unwrap_err().code(),
            "SPG-SOURCE-ITEM-KIND"
        );
    }

    #[test]
    fn constructors_match_the_frozen_role_grammars() {
        for value in ["scope", "scope.part-2", "a1"] {
            assert_eq!(ScopeId::new(value).is_ok(), scope_id_grammar(value));
        }
        for value in ["schema", "scope/v1", "scope/path/v2", "scope/v01"] {
            assert_eq!(SchemaId::new(value).is_ok(), schema_id_grammar(value));
        }
        for value in ["markdown/fence/v1", "rust/item/v2", "rust-item", "bad/v0"] {
            assert_eq!(
                SourceItemKind::new(value).is_ok(),
                versioned_lowercase_id_grammar(value)
            );
        }
        for value in ["ATOM", "ATOM-1", "ATOM--1", "atom"] {
            assert_eq!(NamespacePrefix::new(value).is_ok(), prefix_grammar(value));
        }
    }

    #[test]
    #[allow(clippy::expect_used)]
    fn external_trace_targets_normalize_unicode_paths() {
        let target = ExternalTraceTarget::new("https://example.com/cafe\u{301}")
            .expect("safe decomposed target");
        assert_eq!(target.as_str(), "https://example.com/caf\u{e9}");
    }

    #[test]
    #[allow(clippy::expect_used)]
    fn typed_text_has_raw_canonical_parity() {
        let typed = ScopeId::new("spec.graph").expect("valid scope");
        let typed_bytes = crate::CanonicalValue::Text(alloc::string::String::from(typed.as_str()))
            .encode()
            .expect("typed text encodes");
        let raw_bytes = crate::CanonicalValue::Text(alloc::string::String::from("spec.graph"))
            .encode()
            .expect("raw text encodes");
        assert_eq!(typed_bytes, raw_bytes);
    }
}

/// A validated uppercase namespace prefix.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Hash)]
pub struct NamespacePrefix(String);

impl NamespacePrefix {
    /// Validates and NFC-normalizes a namespace prefix.
    ///
    /// # Errors
    ///
    /// Returns the scope grammar error when validation fails.
    pub fn new(input: &str) -> Result<Self, IdentifierError> {
        let normalized = input.nfc().collect::<String>();
        if prefix_grammar(&normalized) {
            Ok(Self(normalized))
        } else {
            Err(IdentifierError::ScopeIdGrammar)
        }
    }

    /// Returns the canonical UTF-8 prefix text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
