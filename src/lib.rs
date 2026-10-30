#![doc = "Imperative shell and public facade for Buoy."]
#![forbid(unsafe_code)]
#![allow(clippy::duplicate_mod)]

pub mod adapter_sdk;
pub mod adapters;
pub mod adversarial;
pub mod check;
pub mod cli;
pub mod external_integrity;
pub mod external_link_generation;
pub mod glob;
pub mod hashing_policy;
pub mod identifier_policy;
pub mod migration;
pub mod oracle;
pub mod presentation;
pub mod product_identity;
pub mod publication;
pub mod publication_fs;
pub mod publication_models;
pub mod quality;
pub mod repository;
pub mod traceability;

/// Pure, `no_std` deterministic values and transformations.
pub use buoy_core as core;

// This crate owns filesystem, environment, parser, clock, network, and process
// effects. The syntax adapters remain shell-side and pass only explicit values
// into the deterministic core.
