#![doc = "Official XXH3 and simultaneous-observation contracts owned by the core package artifact."]
#![allow(clippy::cast_possible_truncation, clippy::expect_used)]

#[path = "algorithms_model_coverage_controls.rs"]
mod model_coverage_controls;

use buoy_core::*;

const OFFICIAL: &str = include_str!("../../../fixtures/xxh3/v2/official-xxh3-128-vectors.tsv");
const MALFORMED: &str = include_str!("../../../fixtures/xxh3/v2/malformed-digest-vectors.tsv");

fn sanity_buffer(length: usize) -> Vec<u8> {
    let mut generator = 2_654_435_761_u64;
    let mut bytes = Vec::with_capacity(length);
    for _ in 0..length {
        bytes.push((generator >> 56) as u8);
        generator = generator.wrapping_mul(11_400_714_785_074_694_797);
    }
    bytes
}

fn parse_hex(input: &str) -> Option<[u8; 16]> {
    if input.len() != 32 {
        return None;
    }
    let mut bytes = [0_u8; 16];
    for (output, pair) in bytes.iter_mut().zip(input.as_bytes().chunks_exact(2)) {
        let high = char::from(pair[0]).to_digit(16)?;
        let low = char::from(pair[1]).to_digit(16)?;
        *output = ((high << 4) | low) as u8;
    }
    Some(bytes)
}

#[test]
fn sole_hasher_matches_all_official_one_shot_and_incremental_rows() {
    let mut count = 0;
    for line in OFFICIAL.lines().skip(1) {
        let fields = line.split('\t').collect::<Vec<_>>();
        assert_eq!(fields.len(), 13);
        let length = fields[4]
            .parse::<usize>()
            .expect("reviewed official vector length");
        let observations = observe_canonical_preimages(vec![sanity_buffer(length)])
            .expect("one preimage cannot collide");
        let expected = parse_hex(fields[12]).expect("reviewed official digest bytes");
        assert_eq!(observations[0].digest().bytes(), expected);
        count += 1;
    }
    assert_eq!(count, 25);
}

#[test]
fn strict_fixture_hex_parser_rejects_wrong_length_and_nonhexadecimal_input() {
    assert_eq!(parse_hex("00"), None);
    assert_eq!(parse_hex("gggggggggggggggggggggggggggggggg"), None);
    assert_eq!(parse_hex("0ggggggggggggggggggggggggggggggg"), None);
}

#[cfg(feature = "coverage-controls")]
#[test]
fn simultaneous_observations_reject_duplicates_and_forced_collisions() {
    assert_eq!(
        observe_canonical_preimages(vec![b"left".to_vec(), b"left".to_vec()]),
        Err(DigestObservationError::DuplicateCanonicalPreimage)
    );
    let digest = SemanticDigest::xxh3_128([42; 16]);
    assert_eq!(
        validate_digest_observations_control(vec![
            (digest, b"left".to_vec()),
            (digest, b"right".to_vec()),
        ])
        .err()
        .map(DigestObservationError::code),
        MALFORMED
            .lines()
            .find(|line| line.contains("\tobserved-collision\t"))
            .and_then(|line| line.split('\t').nth(4))
    );
}

#[test]
fn simultaneous_observations_have_input_order_independent_total_order() {
    let forward =
        observe_canonical_preimages(vec![b"alpha".to_vec(), b"beta".to_vec(), b"gamma".to_vec()]);
    let reverse =
        observe_canonical_preimages(vec![b"gamma".to_vec(), b"beta".to_vec(), b"alpha".to_vec()]);
    assert_eq!(forward, reverse);
    assert!(forward.as_ref().is_ok_and(|items| {
        items
            .windows(2)
            .all(|pair| pair[0].digest() < pair[1].digest())
    }));
}
