//! Compiles the vendored generated Chelis Surf grammar sources.
//!
//! The vendored bytes are pinned to one exact Chelis revision. The drift
//! check in `chelis-provenance` compares them against the registered
//! derivation digests before any gate trusts this build.

fn main() {
    println!("cargo:rerun-if-changed=vendor");
    cc::Build::new()
        .include("vendor")
        .file("vendor/parser.c")
        .warnings(false)
        .compile("tree-sitter-chelis-surf");
    cc::Build::new()
        .cpp(true)
        .std("c++17")
        .include("vendor")
        .file("vendor/scanner.cc")
        .warnings(false)
        .compile("tree-sitter-chelis-surf-scanner");
}
