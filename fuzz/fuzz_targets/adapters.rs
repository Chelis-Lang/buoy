#![no_main]

use buoy::adapters::{AuthorityExpectation, extract_markdown, extract_rust};
use buoy::core::RepositoryPath;
use libfuzzer_sys::fuzz_target;

const MAX_INPUT_BYTES: usize = 8_192;

fuzz_target!(|data: &[u8]| {
    if data.len() > MAX_INPUT_BYTES {
        return;
    }
    let markdown_path = RepositoryPath::new("fuzz/input.md");
    let rust_path = RepositoryPath::new("fuzz/input.rs");
    if let (Ok(markdown_path), Ok(rust_path)) = (markdown_path, rust_path) {
        for expectation in [
            AuthorityExpectation::Forbidden,
            AuthorityExpectation::Optional,
            AuthorityExpectation::Required,
        ] {
            let _ = extract_markdown(&markdown_path, data, expectation);
        }
        let _ = extract_rust(&rust_path, data);
    }
});
