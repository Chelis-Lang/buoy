//! Structural checks for the official XXH3-128 vector authority.

const OFFICIAL: &str = include_str!("../../../fixtures/xxh3/v2/official-xxh3-128-vectors.tsv");

#[test]
fn official_vector_rows_are_complete_and_canonical() {
    let mut count = 0;
    for line in OFFICIAL.lines().skip(1) {
        let fields = line.split('\t').collect::<Vec<_>>();
        assert_eq!(fields.len(), 13);
        assert!(fields[4].parse::<usize>().is_ok(), "{}", fields[1]);
        assert_eq!(fields[12].len(), 32, "{}", fields[1]);
        assert!(
            fields[12]
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte)),
            "{}",
            fields[1]
        );
        assert!(!fields[8].is_empty(), "{}", fields[1]);
        assert!(!fields[9].is_empty(), "{}", fields[1]);
        count += 1;
    }
    assert_eq!(count, 25);
}
