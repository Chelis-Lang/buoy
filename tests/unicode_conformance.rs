#![doc = "Pinned Unicode 16.0 full default case-fold and portable-collision conformance."]

use buoy::core::{RepositoryPath, canonical_paths, portable_case_fold};
use unicode_normalization::UnicodeNormalization;

const CASE_FOLDING: &str = include_str!("../unicode/16.0.0/CaseFolding.txt");

#[test]
fn every_unicode_16_default_full_case_fold_mapping_conforms() {
    assert!(CASE_FOLDING.starts_with("# CaseFolding-16.0.0.txt\n"));
    assert!(CASE_FOLDING.trim_end().ends_with("# EOF"));
    let mut checked = 0_usize;
    for raw in CASE_FOLDING.lines() {
        let content = raw.split('#').next().unwrap_or("").trim();
        if content.is_empty() {
            continue;
        }
        let columns = content.split(';').map(str::trim).collect::<Vec<_>>();
        assert_eq!(columns.len(), 4);
        if !matches!(columns[1], "C" | "F") {
            continue;
        }
        let source = u32::from_str_radix(columns[0], 16)
            .ok()
            .and_then(char::from_u32);
        assert!(source.is_some());
        let Some(source) = source else { continue };
        let expected = columns[2]
            .split_ascii_whitespace()
            .filter_map(|value| u32::from_str_radix(value, 16).ok())
            .filter_map(char::from_u32)
            .collect::<String>()
            .nfc()
            .collect::<String>();
        assert_eq!(portable_case_fold(&source.to_string()), expected);
        checked += 1;
    }
    assert_eq!(checked, 1_557);
}

#[test]
fn portable_path_collisions_use_full_default_fold_not_simple_lowercase() {
    let pairs = [
        ("spec/MASSE.md", "spec/Maße.md"),
        ("spec/Σ.md", "spec/ς.md"),
        ("spec/ﬀ.md", "spec/ff.md"),
        ("spec/İ.md", "spec/i\u{307}.md"),
    ];
    for (left, right) in pairs {
        let left = RepositoryPath::new(left);
        let right = RepositoryPath::new(right);
        assert!(left.is_ok() && right.is_ok());
        let (Ok(left), Ok(right)) = (left, right) else {
            continue;
        };
        assert_eq!(
            left.portable_collision_key(),
            right.portable_collision_key()
        );
        assert!(canonical_paths(vec![left, right]).is_err());
    }
}
