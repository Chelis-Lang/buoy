#![doc = "Independent malformed canonical vectors and every frozen decoder budget family."]

use buoy::core::{CanonicalValue, DecodeBudget};
use std::collections::BTreeSet;

const VECTORS: &str = include_str!("../fixtures/hardening/v2/malformed-canonical-vectors.tsv");

fn hex(input: &str) -> Option<Vec<u8>> {
    if input.len() % 2 != 0 {
        return None;
    }
    input
        .as_bytes()
        .chunks_exact(2)
        .map(|pair| Some((nibble(pair[0])? << 4) | nibble(pair[1])?))
        .collect()
}

const fn nibble(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        _ => None,
    }
}

#[test]
fn every_independent_malformed_vector_fails_with_its_stable_family() {
    let mut lines = VECTORS.lines();
    assert_eq!(
        lines.next(),
        Some("schema\tid\thex\tmax_bytes\tmax_nesting\tmax_values\texpected_code")
    );
    let mut ids = BTreeSet::new();
    let mut codes = BTreeSet::new();
    let mut count = 0_usize;
    for line in lines {
        let columns = line.split('\t').collect::<Vec<_>>();
        assert_eq!(columns.len(), 7);
        assert_eq!(columns[0], "spec-provenance-malformed-vector/v2");
        assert!(ids.insert(columns[1]));
        let bytes = hex(columns[2]);
        let max_bytes = columns[3].parse::<u64>().ok();
        let max_nesting = columns[4].parse::<u16>().ok();
        let max_values = columns[5].parse::<u64>().ok();
        assert!(bytes.is_some());
        assert!(max_bytes.is_some() && max_nesting.is_some() && max_values.is_some());
        let (Some(bytes), Some(max_bytes), Some(max_nesting), Some(max_values)) =
            (bytes, max_bytes, max_nesting, max_values)
        else {
            continue;
        };
        let error = CanonicalValue::decode(
            &bytes,
            DecodeBudget {
                max_bytes,
                max_nesting,
                max_values,
            },
        )
        .err();
        assert!(error.is_some(), "{} unexpectedly decoded", columns[1]);
        assert_eq!(
            error.map(buoy::core::DecodingError::code),
            Some(columns[6]),
            "{}",
            columns[1]
        );
        codes.insert(columns[6]);
        count += 1;
    }
    assert_eq!(count, 24);
    for required in [
        "DET-ENC-BUDGET-BYTES",
        "DET-ENC-BUDGET-NESTING",
        "DET-ENC-BUDGET-VALUES",
        "DET-ENC-RECORD-TAGS",
        "DET-ORDER-SET",
        "DET-ENC-OPTION",
        "DET-ENC-ENUM",
        "DET-ENC-DIGEST",
        "DET-ENC-TRAILING-BYTES",
    ] {
        assert!(
            codes.contains(required),
            "missing diagnostic family {required}"
        );
    }
}
