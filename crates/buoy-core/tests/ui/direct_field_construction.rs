#![allow(unreachable_code)]

use buoy_core::CoveragePolicy;

fn main() {
    let _ = CoveragePolicy {
        meta: unreachable!(),
        revision: unreachable!(),
        active_approval: unreachable!(),
        lifecycle: unreachable!(),
        replacements: Vec::new(),
        terminal_approval: None,
        materialized_subjects: Vec::new(),
    };
}
