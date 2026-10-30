#![doc = "Independent frozen positive vectors for every canonical tag, nested value, fact, and domain."]

use buoy::core::{CanonicalValue, DecodeBudget, semantic_digest};
use buoy_reference::Value;
use std::collections::BTreeSet;

const VECTORS: &str = include_str!("../fixtures/hardening/v3/canonical-vectors.tsv");

fn hex(input: &str) -> Option<Vec<u8>> {
    if input.len() % 2 != 0 {
        return None;
    }
    input
        .as_bytes()
        .chunks_exact(2)
        .map(|pair| {
            let high = nibble(pair[0])?;
            let low = nibble(pair[1])?;
            Some((high << 4) | low)
        })
        .collect()
}

const fn nibble(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        _ => None,
    }
}

fn assert_vectors(source: &str, expected_domains: usize) {
    let mut identities = BTreeSet::new();
    let mut type_tags = 0_usize;
    let mut domains = 0_usize;
    let mut facts = 0_usize;
    let mut nested = 0_usize;
    let mut unknown = 0_usize;
    let mut lines = source.lines();
    assert_eq!(
        lines.next(),
        Some("schema\tkind\tid\tcanonical_hex\tsemantic_digest")
    );
    for line in lines {
        let columns = line.split('\t').collect::<Vec<_>>();
        assert_eq!(columns.len(), 5);
        assert_eq!(columns[0], "spec-provenance-canonical-vector/v2");
        assert!(identities.insert((columns[1], columns[2])));
        let bytes = hex(columns[3]);
        assert!(bytes.is_some());
        let Some(bytes) = bytes else { continue };
        let value = CanonicalValue::decode(&bytes, DecodeBudget::v2_default());
        assert!(value.is_ok(), "{}/{}", columns[1], columns[2]);
        let Ok(value) = value else { continue };
        assert_eq!(value.encode().ok().as_deref(), Some(bytes.as_slice()));
        assert_eq!(
            Value::decode(&bytes, 128, 5_000_000).and_then(|value| value.encode()),
            Ok(bytes.clone()),
            "independent replay {}/{}",
            columns[1],
            columns[2]
        );
        match columns[1] {
            "type-tag" => type_tags += 1,
            "nested" => nested += 1,
            "report-fact" => facts += 1,
            "domain" => {
                domains += 1;
                let CanonicalValue::Record(fields) = value else {
                    continue;
                };
                let digest = semantic_digest(columns[2], fields);
                assert!(digest.is_ok());
                assert_eq!(
                    digest.ok().map(|value| value.canonical_text()).as_deref(),
                    Some(columns[4])
                );
            }
            _ => unknown += 1,
        }
    }
    assert_eq!(type_tags, 13);
    assert_eq!(domains, expected_domains);
    assert_eq!(facts, 11);
    assert!(nested >= 10);
    assert_eq!(unknown, 0);
}

#[test]
fn independent_positive_vectors_decode_reencode_and_rehash_exactly() {
    assert_vectors(VECTORS, 49);
}
