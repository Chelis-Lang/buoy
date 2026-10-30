#![doc = "Independent reference malformed-input and fallible-encoding controls."]
#![allow(clippy::expect_used, clippy::too_many_lines)]

use buoy_reference::{Error, Value, algorithms, semantic_preimage};
use std::collections::{BTreeMap, BTreeSet};

fn decode_hex(text: &str) -> Vec<u8> {
    text.as_bytes()
        .chunks_exact(2)
        .map(|pair| {
            let digit = |byte: u8| match byte {
                b'0'..=b'9' => byte - b'0',
                b'a'..=b'f' => byte - b'a' + 10,
                _ => 0,
            };
            (digit(pair[0]) << 4) | digit(pair[1])
        })
        .collect()
}

#[test]
fn malformed_production_vectors_are_rejected_by_the_independent_decoder() {
    let source = include_str!("../../../fixtures/hardening/v2/malformed-canonical-vectors.tsv");
    let mut rejected = 0;
    for line in source.lines().skip(1) {
        let fields = line.split('\t').collect::<Vec<_>>();
        let bytes = decode_hex(fields[2]);
        let nesting = fields[4].parse::<u16>().expect("nesting");
        let values = fields[5].parse::<u64>().expect("values");
        rejected += usize::from(Value::decode(&bytes, nesting, values).is_err());
    }
    assert!(rejected >= 20);
    let mut trailing = Value::True.encode().expect("true encodes");
    trailing.push(0);
    assert_eq!(Value::decode(&trailing, 128, 100), Err(Error::Trailing));
    assert_eq!(
        Value::decode(&Value::True.encode().expect("true"), 0, 100),
        Err(Error::Nesting)
    );
    assert_eq!(
        Value::decode(&Value::True.encode().expect("true"), 128, 0),
        Err(Error::Count)
    );
    let mut mismatched_digest_payload = vec![12];
    mismatched_digest_payload.extend_from_slice(&20_u64.to_be_bytes());
    mismatched_digest_payload.extend_from_slice(&3_u16.to_be_bytes());
    mismatched_digest_payload.extend_from_slice(&32_u16.to_be_bytes());
    mismatched_digest_payload.extend_from_slice(&[0_u8; 16]);
    assert_eq!(
        Value::decode(&mismatched_digest_payload, 128, 100),
        Err(Error::Digest)
    );
}

#[test]
fn fallible_reference_encodings_reject_each_noncanonical_shape() {
    for value in [
        Value::Text(String::from("e\u{301}")),
        Value::Record(vec![(1, Value::True), (1, Value::False)]),
        Value::Record(vec![(0, Value::True)]),
        Value::Record(vec![(0, Value::True), (1, Value::False)]),
        Value::Set(vec![Value::True, Value::True]),
        Value::Enum(0, None),
        Value::Enum(1, Some(Box::new(Value::Text(String::from("e\u{301}"))))),
        Value::Digest(1, vec![0; 16]),
        Value::Digest(2, vec![0; 15]),
        Value::Digest(2, vec![0; 32]),
        Value::Digest(3, vec![0; 16]),
        Value::Digest(4, vec![0; 16]),
        Value::Some(Box::new(Value::Text(String::from("e\u{301}")))),
        Value::Sequence(vec![Value::Text(String::from("e\u{301}"))]),
    ] {
        assert!(value.encode().is_err());
    }
    assert!(semantic_preimage(2, "e\u{301}", Vec::new()).is_err());
    assert!(semantic_preimage(2, "domain", vec![(0, Value::True)]).is_err());
    assert!(semantic_preimage(2, "domain", vec![(1, Value::True)]).is_ok());
    assert!(semantic_preimage(0, "domain", vec![(1, Value::True)]).is_err());
    assert!(
        Value::Enum(1, Some(Box::new(Value::Unsigned(1))))
            .encode()
            .is_ok()
    );
    assert!(Value::Digest(2, vec![0; 16]).encode().is_ok());
    assert!(
        Value::Record(vec![(1, Value::Text(String::from("e\u{301}")))])
            .encode()
            .is_err()
    );
}

#[test]
fn every_reference_value_and_nested_error_path_is_exercised() {
    let values = vec![
        Value::False,
        Value::True,
        Value::Unsigned(u64::MAX),
        Value::Signed(i64::MIN),
        Value::Bytes(vec![0, 1, 255]),
        Value::Text(String::from("é")),
        Value::Record(vec![(1, Value::True)]),
        Value::Sequence(vec![Value::False, Value::True]),
        Value::Set(vec![Value::False, Value::True]),
        Value::None,
        Value::Some(Box::new(Value::True)),
        Value::Enum(1, None),
        Value::Enum(1, Some(Box::new(Value::Unsigned(1)))),
        Value::Digest(2, vec![0; 16]),
        Value::Digest(3, vec![0; 32]),
        Value::Digest(4, vec![0; 32]),
    ];
    for value in values {
        let encoded = value.encode().expect("reference value encodes");
        assert_eq!(Value::decode(&encoded, 128, 1_000), Ok(value));
    }

    let tlv = |tag: u8, payload: &[u8]| {
        let mut bytes = vec![tag];
        bytes.extend_from_slice(&(payload.len() as u64).to_be_bytes());
        bytes.extend_from_slice(payload);
        bytes
    };
    assert_eq!(Value::decode(&tlv(1, &[0]), 128, 100), Err(Error::Length));
    assert_eq!(Value::decode(&tlv(2, &[0]), 128, 100), Err(Error::Length));
    assert_eq!(Value::decode(&tlv(3, &[0]), 128, 100), Err(Error::Length));
    assert_eq!(Value::decode(&tlv(11, &[]), 128, 100), Err(Error::Enum));
    let mut truncated_enum = 1_u16.to_be_bytes().to_vec();
    truncated_enum.push(0);
    assert_eq!(
        Value::decode(&tlv(11, &truncated_enum), 128, 100),
        Err(Error::Enum)
    );
    assert_eq!(Value::decode(&tlv(7, &[]), 128, 100), Err(Error::Length));
    assert_eq!(Value::decode(&tlv(6, &[]), 128, 100), Err(Error::Length));
    let mut trailing_record = 0_u64.to_be_bytes().to_vec();
    trailing_record.push(0);
    assert_eq!(
        Value::decode(&tlv(6, &trailing_record), 128, 100),
        Err(Error::Record)
    );
    let mut invalid_sequence = 1_u64.to_be_bytes().to_vec();
    invalid_sequence.push(0);
    assert_eq!(
        Value::decode(&tlv(7, &invalid_sequence), 128, 100),
        Err(Error::Length)
    );
    let mut invalid_record_text = 1_u64.to_be_bytes().to_vec();
    invalid_record_text.extend_from_slice(&1_u16.to_be_bytes());
    invalid_record_text.extend(tlv(5, &[b'e', 0xcc, 0x81]));
    assert_eq!(
        Value::decode(&tlv(6, &invalid_record_text), 128, 100),
        Err(Error::Text)
    );

    let mut option_payload = Value::None.encode().expect("none");
    option_payload.push(0);
    assert_eq!(
        Value::decode(&tlv(10, &option_payload), 128, 100),
        Err(Error::Option)
    );
    let mut enum_payload = 1_u16.to_be_bytes().to_vec();
    enum_payload.extend(Value::None.encode().expect("enum none"));
    enum_payload.push(0);
    assert_eq!(
        Value::decode(&tlv(11, &enum_payload), 128, 100),
        Err(Error::Enum)
    );
    let mut sequence_payload = 0_u64.to_be_bytes().to_vec();
    sequence_payload.push(0);
    assert_eq!(
        Value::decode(&tlv(7, &sequence_payload), 128, 100),
        Err(Error::Length)
    );
    let mut set_payload = 0_u64.to_be_bytes().to_vec();
    set_payload.push(0);
    assert_eq!(
        Value::decode(&tlv(8, &set_payload), 128, 100),
        Err(Error::Set)
    );
    let mut record_payload = 1_u64.to_be_bytes().to_vec();
    record_payload.extend_from_slice(&1_u16.to_be_bytes());
    record_payload.extend(tlv(10, &[]));
    assert_eq!(
        Value::decode(&tlv(6, &record_payload), 128, 100),
        Err(Error::Record)
    );
}

#[test]
fn independent_algorithm_controls_cover_all_closed_relations() {
    for (state, event, expected) in [
        (1, 1, Some(2)),
        (1, 2, Some(4)),
        (2, 2, Some(4)),
        (2, 3, Some(3)),
        (9, 9, None),
    ] {
        assert_eq!(algorithms::lifecycle_transition(state, event), expected);
    }
    assert!(algorithms::governance_transition_valid(
        None,
        Some(1),
        None,
        Some(1),
        None,
        Some(1),
    ));
    assert!(!algorithms::governance_transition_valid(
        Some(9),
        Some(9),
        Some(1),
        Some(1),
        Some(1),
        Some(1),
    ));
    assert!(!algorithms::governance_transition_valid(
        Some(1),
        Some(1),
        Some(9),
        Some(9),
        Some(1),
        Some(1),
    ));
    assert!(!algorithms::governance_transition_valid(
        Some(1),
        Some(1),
        Some(1),
        Some(1),
        Some(9),
        Some(9),
    ));

    let key = b"key".to_vec();
    assert!(algorithms::surface_complete(
        std::slice::from_ref(&key),
        &[(key.clone(), true, true)],
    ));
    assert!(!algorithms::surface_complete(
        std::slice::from_ref(&key),
        &[]
    ));
    for row in [
        (key.clone(), false, true),
        (key.clone(), true, false),
        (b"other".to_vec(), true, true),
    ] {
        assert!(!algorithms::surface_complete(
            std::slice::from_ref(&key),
            &[row],
        ));
    }
    assert!(!algorithms::surface_complete(
        &[b"a".to_vec(), b"b".to_vec()],
        &[(b"a".to_vec(), true, true), (b"a".to_vec(), true, true)],
    ));
    assert!(!algorithms::all_current(&[]));
    assert!(algorithms::all_current(&[true, true]));
    assert!(!algorithms::all_current(&[true, false]));
    assert_eq!(
        algorithms::required_roles(&[2, 1, 2], &[(1, true), (2, false)]),
        (vec![1], vec![2])
    );

    let base_subjects = BTreeSet::from([String::from("a"), String::from("b")]);
    let head_subjects = BTreeSet::from([String::from("b")]);
    let base_roles = BTreeSet::from([1, 2]);
    let head_roles = BTreeSet::from([2]);
    assert_eq!(
        algorithms::policy_ratchet(
            &base_subjects,
            &head_subjects,
            &base_roles,
            &head_roles,
            true,
            false,
            false,
        ),
        (vec![String::from("a")], vec![1], true, true)
    );
    assert_eq!(
        algorithms::policy_ratchet(
            &head_subjects,
            &head_subjects,
            &head_roles,
            &head_roles,
            false,
            false,
            true,
        ),
        (Vec::new(), Vec::new(), false, false)
    );
    assert_eq!(
        algorithms::assurance_classes(true, true, true, &[2], true, true, true, true),
        vec![1, 2, 3, 4, 5, 6]
    );
    assert!(
        algorithms::assurance_classes(false, true, true, &[2], false, false, false, false)
            .is_empty()
    );
    for verdicts in [&[][..], &[1][..]] {
        assert_eq!(
            algorithms::assurance_classes(true, true, true, verdicts, false, false, false, false),
            vec![1]
        );
    }
    for (fresh, selected) in [(false, true), (true, false)] {
        assert_eq!(
            algorithms::assurance_classes(true, fresh, selected, &[2], false, false, false, false,),
            vec![1]
        );
    }

    let base = BTreeMap::from([
        (String::from("A"), (1, Vec::new())),
        (String::from("B"), (1, vec![String::from("A")])),
    ]);
    let head = BTreeMap::from([
        (String::from("A"), (2, Vec::new())),
        (String::from("B"), (1, vec![String::from("A")])),
    ]);
    assert_eq!(
        algorithms::transitive_impacts(&base, &head),
        BTreeSet::from([(String::from("A"), true), (String::from("B"), false)])
    );
    assert!(algorithms::disposition_closed(
        &[String::from("A")],
        &[String::from("A")],
    ));
    assert!(!algorithms::disposition_closed(
        &[String::from("A"), String::from("A")],
        &[String::from("A")],
    ));
    assert!(!algorithms::disposition_closed(
        &[String::from("A")],
        &[String::from("B")],
    ));
    assert!(!algorithms::disposition_closed(
        &[String::from("A")],
        &[String::from("A"), String::from("A")],
    ));

    for invalid in [
        "", "/a", "a/", "a\\b", "a\0b", "C:/x", "a//b", "a/./b", "a/../b", "a ", "a.", "a?",
        "a\u{7}", "CON", "COM1", "LPT9",
    ] {
        assert!(algorithms::portable_path(invalid).is_err(), "{invalid:?}");
    }
    assert_eq!(
        algorithms::portable_path("é/x.txt"),
        Ok(String::from("é/x.txt"))
    );
    assert_eq!(
        algorithms::portable_path_set(&[String::from("b"), String::from("a")]),
        Ok(vec![String::from("a"), String::from("b")])
    );
    assert!(algorithms::portable_path_set(&[String::from("a"), String::from("a")]).is_err());
    assert!(algorithms::portable_path_set(&[String::from("ß"), String::from("ss")]).is_err());
    assert!(algorithms::portable_path_set(&[String::from("bad?")]).is_err());
    assert_eq!(
        algorithms::reachable(
            &[String::from("A")],
            &[
                (String::from("A"), String::from("B")),
                (String::from("B"), String::from("C")),
                (String::from("C"), String::from("A")),
            ],
        ),
        BTreeSet::from([String::from("A"), String::from("B"), String::from("C")])
    );
}
