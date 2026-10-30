//! Shell-side Chelis host integration over the repository-independent Buoy
//! interfaces.
//!
//! This crate depends inward on the published `buoy` shell and `buoy-core`
//! value contracts. It adds no Chelis semantics to the generic core. Canonical
//! inputs are saved repository paths, saved bytes, and explicit configuration;
//! daemon state, editor buffers, provider metadata, Git state, and clocks
//! cannot supply authority or alter canonical output.

pub mod adapter;
pub mod advisory;
pub mod bind;
pub mod command;
pub mod editor;
pub mod execution;
pub mod map;
pub mod rebind;
pub mod records;
pub mod snapshot;
pub mod trace;
