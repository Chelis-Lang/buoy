#[buoy::implements("blake3-256:0e857e33e387325fbf457e426bc42286d9d99926b19f3efa3c203795cfeeb6c4")]
pub fn render_catalog(libraries: &[&str]) -> String {
    libraries.join("\n")
}
