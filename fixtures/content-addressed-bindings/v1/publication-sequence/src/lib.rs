#[buoy::implements("blake3-256:5be031013feeaccb77f755d96d8f04dd9d131a4a493f7842483a734b6a38088f")]
pub fn render_catalog(libraries: &[&str]) -> String {
    libraries.join("\n")
}
