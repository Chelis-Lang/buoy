//! Authoritative deterministic Hegel runner settings for repository evidence.

use hegel::{Backend, Phase, Settings};

/// Exact sampled-case count for every authoritative Hegel property.
pub const TEST_CASES: u64 = 256;
/// Stable namespace reserved for Buoy v1 property seeds.
pub const SEED_PREFIX: u64 = 0x5350_5631_0000_0000;
/// Ordered phases required for every authoritative run.
pub const PHASES: [Phase; 5] = [
    Phase::Explicit,
    Phase::Reuse,
    Phase::Generate,
    Phase::Target,
    Phase::Shrink,
];

/// Construct the only Hegel settings accepted for authoritative property evidence.
pub fn settings(property: u16) -> Settings {
    assert!(property != 0, "property seed suffix must be nonzero");
    Settings::new()
        .backend(Backend::Default)
        .test_cases(TEST_CASES)
        .seed(Some(SEED_PREFIX | u64::from(property)))
        .derandomize(true)
        .database(None)
        .phases(PHASES)
}
