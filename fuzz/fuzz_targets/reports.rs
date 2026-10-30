#![no_main]

use buoy::core::{
    CanonicalExecutionReport, CanonicalImpactReport, CanonicalStaticReport, CanonicalValue,
    DecodeBudget,
};
use buoy::presentation::render_canonical_value_control;
use libfuzzer_sys::fuzz_target;

const MAX_INPUT_BYTES: usize = 16_384;

fuzz_target!(|data: &[u8]| {
    if data.len() > MAX_INPUT_BYTES {
        return;
    }
    let budget = DecodeBudget {
        max_bytes: MAX_INPUT_BYTES as u64,
        max_nesting: 32,
        max_values: 1_024,
    };
    if let Ok(value) = CanonicalValue::decode(data, budget) {
        let encoded = value
            .encode()
            .expect("a strictly decoded report candidate must re-encode");
        assert_eq!(encoded, data);
        let _ = render_canonical_value_control(&value);
    }
    if let Ok(report) = CanonicalStaticReport::decode(data, budget) {
        assert_eq!(report.bytes(), data);
    }
    if let Ok(report) = CanonicalExecutionReport::decode(data, budget) {
        assert_eq!(report.bytes(), data);
    }
    if let Ok(report) = CanonicalImpactReport::decode(data, budget) {
        assert_eq!(report.bytes(), data);
    }
});
