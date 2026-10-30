#![no_std]
#![doc = "Allowed FCIS boundary fixture."]

use core::cell::Cell;

/// Immutable `Freeze` statics do not create ambient mutable state.
pub static SCHEMA_TAG: &str = "spec-provenance-v1";

fn apply_core_owned(value: u8, transform: fn(u8) -> u8) -> u8 {
    transform(value)
}

fn increment(value: u8) -> u8 {
    value + 1
}

/// Public core APIs consume concrete values rather than caller-provided behavior.
pub fn normalize(value: u8) -> u8 {
    let invocation_local = Cell::new(value);
    invocation_local.set(apply_core_owned(invocation_local.get(), increment));
    invocation_local.get()
}
