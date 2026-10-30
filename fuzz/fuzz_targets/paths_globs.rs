#![no_main]

use buoy::core::{RepositoryPath, canonical_paths};
use buoy::glob::PortableGlob;
use libfuzzer_sys::fuzz_target;

const MAX_INPUT_BYTES: usize = 1_024;

fuzz_target!(|data: &[u8]| {
    if data.len() > MAX_INPUT_BYTES || data.is_empty() {
        return;
    }
    let split = usize::from(data[0]) % data.len();
    let pattern = core::str::from_utf8(&data[..split]);
    let path = core::str::from_utf8(&data[split..]);
    if let (Ok(pattern), Ok(path)) = (pattern, path) {
        let parsed_pattern = PortableGlob::parse(pattern);
        let parsed_path = RepositoryPath::new(path);
        if let Ok(pattern) = &parsed_pattern {
            assert!(pattern.overlaps(pattern));
        }
        if let (Ok(pattern), Ok(path)) = (parsed_pattern, parsed_path) {
            let _ = pattern.matches(&path);
            let _ = path.portable_collision_key();
            let _ = canonical_paths(vec![path]);
        }
    }
});
