#![no_std]
#![doc = "Pure deterministic values and transformations for Buoy."]
#![forbid(unsafe_code)]

extern crate alloc;

mod algorithms;
mod attachment;
mod casefold_table;
mod identifier;
mod model;
mod mount;
mod report;
mod trace;
mod value;

pub use algorithms::*;
pub use attachment::*;
pub use identifier::*;
pub use model::*;
pub use mount::*;
pub use report::*;
pub use trace::*;
pub use value::*;

// This crate is the functional core. It accepts complete immutable values and
// returns values; filesystem, environment, clock, network, parser, and process
// access belong to shell crates.
