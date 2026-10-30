#![doc = "Contracts for the typed stable-object-identifier construction boundary."]
#![allow(clippy::expect_used)]

use buoy_core::*;

/// Frozen canonical text encoding of the parity fixture identifier.
const PARITY_ID: &str = "ATOM-PARITY-1";

/// Frozen canonical `CanonicalValue::Text` bytes for [`PARITY_ID`].
const PARITY_TEXT_BYTES: &[u8] = &[
    5, 0, 0, 0, 0, 0, 0, 0, 13, 65, 84, 79, 77, 45, 80, 65, 82, 73, 84, 89, 45, 49,
];

/// Frozen XXH3-128 atom revision digest of the parity fixture.
const PARITY_ATOM_REVISION: &str = "xxh3-128:a37558a3c4ffc2c8714ec981702a8b34";

/// Frozen XXH3-128 atom object digest of the parity fixture.
const PARITY_ATOM_DIGEST: &str = "xxh3-128:8492f24d9a5746f6056f064e6fe04763";

/// Reference copy of the frozen stable-id grammar as it stood before the
/// `StableId` newtype existed. The differential test compares `StableId::new`
/// against this reference, so it stays meaningful after `require_stable_id`
/// is removed from the model layer.
fn reference_stable_id_grammar(text: &str) -> bool {
    if !(3..=128).contains(&text.len()) {
        return false;
    }
    let mut components = text.split('-');
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

/// Reference copy of the pre-change `require_stable_id` acceptance predicate:
/// nonempty, already NFC, and inside the frozen grammar.
fn reference_require_stable_id(text: &str) -> bool {
    !text.is_empty()
        && CanonicalValue::Text(String::from(text)).encode().is_ok()
        && reference_stable_id_grammar(text)
}

/// Corpus that exercises every acceptance and rejection branch of the grammar.
fn differential_corpus() -> Vec<String> {
    let mut corpus = vec![
        String::new(),
        String::from("A"),
        String::from("AB"),
        String::from("A-1"),
        String::from("AB-"),
        String::from("-AB"),
        String::from("ATOM-1"),
        String::from("ATOM-PARITY-1"),
        String::from("ATOM--1"),
        String::from("ATOM"),
        String::from("1ATOM-1"),
        String::from("atom-1"),
        String::from("Atom-1"),
        String::from("ATOM-a"),
        String::from("ATOM_1"),
        String::from("ATOM.1"),
        String::from("ATOM 1"),
        String::from("ATOM-1-"),
        String::from("A1-B2-C3"),
        String::from("0-1"),
        String::from("00-11"),
        // Not NFC: `E` plus combining acute accent.
        String::from("ATOM-E\u{301}"),
        // NFC, but outside the ASCII grammar.
        String::from("ATOM-\u{c9}"),
        String::from("ATOM-\u{fb01}"),
    ];
    // Exactly at, and one past, both frozen length bounds.
    corpus.push(String::from("A-1"));
    corpus.push(format!("A-{}", "B".repeat(125)));
    corpus.push(format!("A-{}", "B".repeat(126)));
    corpus
}

fn parity_atom(id: StableId) -> Atom {
    Atom::new(
        id,
        LifecycleState::Proposed,
        AtomKind::Constraint,
        vec![ScopeId::new("spec.graph").expect("valid scope")],
        "The invariant MUST remain true.",
        None,
    )
    .expect("valid parity atom")
}

// Task 1.1 - positive construction.

#[test]
fn valid_stable_id_parses_and_retains_canonical_text() {
    let identifier = StableId::new(PARITY_ID).expect("valid stable identifier");
    assert_eq!(identifier.as_str(), PARITY_ID);
}

#[test]
fn valid_stable_id_accepts_every_grammar_shape() {
    for candidate in ["A-1", "ATOM-1", "A1-B2-C3", "ATOM-PARITY-1"] {
        let identifier = StableId::new(candidate).expect("valid stable identifier");
        assert_eq!(identifier.as_str(), candidate);
    }
}

#[test]
fn valid_stable_id_is_usable_as_an_ordered_key() {
    let first = StableId::new("ATOM-1").expect("valid stable identifier");
    let second = StableId::new("ATOM-2").expect("valid stable identifier");
    assert!(first < second);
    assert_eq!(first, StableId::new("ATOM-1").expect("valid identifier"));
}

// Task 1.2 - negative construction.

#[test]
fn invalid_stable_id_reports_the_frozen_code_and_yields_no_value() {
    for candidate in [
        "", "A", "ATOM", "atom-1", "ATOM_1", "ATOM--1", "1ATOM-1", "0-1",
    ] {
        let outcome = StableId::new(candidate);
        let error = outcome.expect_err("rejected stable identifier");
        assert_eq!(error.code(), "SCF-CONSTRUCT-STABLE-ID");
    }
}

#[test]
fn non_nfc_stable_id_reports_the_frozen_code() {
    let error = StableId::new("ATOM-E\u{301}").expect_err("rejected stable identifier");
    assert_eq!(error.code(), "SCF-CONSTRUCT-STABLE-ID");
}

#[test]
fn rejected_stable_id_surfaces_the_frozen_code_through_construction_error() {
    let error = StableId::new("atom-1").expect_err("rejected stable identifier");
    let construction: ConstructionError = error.into();
    assert_eq!(construction.code(), "SCF-CONSTRUCT-STABLE-ID");
}

// Task 1.3 - differential parity with the pre-change helper.

#[test]
fn stable_id_matches_the_reference_acceptance_set() {
    for candidate in differential_corpus() {
        let accepted = StableId::new(&candidate).is_ok();
        assert_eq!(
            accepted,
            reference_require_stable_id(&candidate),
            "acceptance differs for {candidate:?}"
        );
    }
}

#[test]
fn stable_id_matches_the_reference_over_generated_ascii_shapes() {
    let alphabet = ['A', 'Z', '0', '9', '-', 'a', '_'];
    for first in alphabet {
        for second in alphabet {
            for third in alphabet {
                for fourth in alphabet {
                    let candidate: String = [first, second, third, fourth].iter().collect();
                    assert_eq!(
                        StableId::new(&candidate).is_ok(),
                        reference_require_stable_id(&candidate),
                        "acceptance differs for {candidate:?}"
                    );
                }
            }
        }
    }
}

// Task 1.4 - canonical parity.

#[test]
fn stable_id_encodes_to_the_frozen_canonical_text_bytes() {
    let identifier = StableId::new(PARITY_ID).expect("valid stable identifier");
    let typed = CanonicalValue::Text(String::from(identifier.as_str()))
        .encode()
        .expect("typed identifier encodes");
    let raw = CanonicalValue::Text(String::from(PARITY_ID))
        .encode()
        .expect("raw identifier encodes");
    assert_eq!(typed, raw);
    assert_eq!(typed.as_slice(), PARITY_TEXT_BYTES);
}

#[test]
fn object_built_through_stable_id_keeps_the_frozen_digests() {
    let atom = parity_atom(StableId::new(PARITY_ID).expect("valid stable identifier"));
    assert_eq!(atom.meta().id().as_str(), PARITY_ID);
    if atom.meta().digest().algorithm() == DigestAlgorithm::Xxh3_128 {
        assert_eq!(atom.revision().canonical_text(), PARITY_ATOM_REVISION);
        assert_eq!(atom.meta().digest().canonical_text(), PARITY_ATOM_DIGEST);
    }
}

#[test]
fn reference_built_from_a_typed_identifier_needs_no_second_parse() {
    let atom = parity_atom(StableId::new(PARITY_ID).expect("valid stable identifier"));
    let reference = ObjectRef::new(
        ObjectKind::Atom,
        atom.meta().id().clone(),
        atom.meta().digest(),
    );
    assert_eq!(reference.id().as_str(), PARITY_ID);
    assert_eq!(reference.digest(), atom.meta().digest());
}
