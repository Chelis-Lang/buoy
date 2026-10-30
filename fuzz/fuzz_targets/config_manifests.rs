#![no_main]

use buoy::cli::validate_change_manifest_bytes;
use buoy::repository::RepositoryConfig;
use libfuzzer_sys::fuzz_target;

const MAX_INPUT_BYTES: usize = 16_384;

fuzz_target!(|data: &[u8]| {
    if data.len() > MAX_INPUT_BYTES {
        return;
    }
    if let Ok(configuration) = RepositoryConfig::parse(data) {
        let _ = configuration.semantic_digest();
    }
    let _ = validate_change_manifest_bytes(data);
});
