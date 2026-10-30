#![no_main]

use buoy::publication::{
    PackageIdentity, PublicationEvidence, PublicationReport, join_target_base, parse_plan,
    render_path,
};
use libfuzzer_sys::fuzz_target;

const MAX_INPUT_BYTES: usize = 4_096;

fuzz_target!(|data: &[u8]| {
    if data.len() > MAX_INPUT_BYTES {
        return;
    }

    // Task 12.1: the publication-plan parser must never panic or allocate
    // unboundedly for any byte sequence, and every accepted plan must
    // satisfy the same closed invariants `trace_publication_contract.rs`
    // already checks for known-good and known-bad fixtures.
    if let Ok(plan) = parse_plan(data) {
        assert_eq!(plan.schema(), "buoy/trace-publication-plan/v1");
        assert!(!plan.publication_id().is_empty());
        assert!(plan.expected_rows() > 0);
        for output in plan.outputs() {
            assert!(!output.path.is_empty());
            assert!(!output.mount.is_empty());
        }
    }

    // Split the input roughly in half to exercise the path-template
    // renderer and the target-base joiner with independent fuzzed bytes,
    // both of which parse untrusted operational strings.
    let midpoint = data.len() / 2;
    let (first_half, second_half) = data.split_at(midpoint);
    if let Ok(first_text) = core::str::from_utf8(first_half) {
        if let Ok(second_text) = core::str::from_utf8(second_half) {
            let identities = buoy::publication::PathIdentities {
                basis: second_text,
                atom_revision: Some(second_text),
                record_digest: Some(second_text),
            };
            let _ = render_path(first_text, &identities);
            let _ = join_target_base(first_text, second_text);
        }
    }

    // Report, evidence, and package-identity encoders must render
    // deterministically for any fuzzed diagnostic/publication-id/feature
    // text, or reject it outright; neither may panic.
    if let Ok(text) = core::str::from_utf8(data) {
        let report = PublicationReport::new("check", text).with_diagnostic("TPT-000", text);
        let _ = buoy::publication::render_report(&report);

        let evidence = PublicationEvidence::new(text);
        let _ = buoy::publication::render_evidence(&evidence);

        let identity = PackageIdentity::new("0.0.0", text, text, vec![text]);
        let _ = identity.render();
    }
});
