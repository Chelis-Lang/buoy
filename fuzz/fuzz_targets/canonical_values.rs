#![no_main]

use buoy::core::{CanonicalValue, DecodeBudget};
use libfuzzer_sys::fuzz_target;

const MAX_INPUT_BYTES: usize = 4_096;

fuzz_target!(|data: &[u8]| {
    if data.len() > MAX_INPUT_BYTES {
        return;
    }
    let budget = DecodeBudget {
        max_bytes: MAX_INPUT_BYTES as u64,
        max_nesting: 32,
        max_values: 512,
    };
    if let Ok(value) = CanonicalValue::decode(data, budget) {
        let encoded = value
            .encode()
            .expect("a strictly decoded canonical value must re-encode");
        assert_eq!(encoded, data);
    }
});
