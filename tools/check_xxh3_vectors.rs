//! Compatibility entry point for the version 3 registry fixture checker.
//!
//! The implementation checks all registered digest members. The historical
//! binary name remains available for existing local automation.

#[path = "check_digest_fixtures.rs"]
mod implementation;

fn main() {
    implementation::main();
}
