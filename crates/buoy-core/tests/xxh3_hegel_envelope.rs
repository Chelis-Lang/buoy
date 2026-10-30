//! Fixed-seed Hegel checks for the production v3 semantic preimage envelope.

#[path = "algorithms_model_coverage_controls.rs"]
mod model_coverage_controls;

use buoy_core::{CanonicalValue, semantic_digest, semantic_digest_observation};
use buoy_reference::Value;
use hegel::generators as gs;
use hegel::{Hegel, TestCase};

#[path = "../../../tests/support/hegel_settings.rs"]
mod hegel_settings;
use hegel_settings::settings;

#[test]
fn hegel_production_envelope_matches_the_explicit_profile() {
    Hegel::new(|test_case: TestCase| {
        let bytes = test_case.draw(gs::vecs(gs::integers::<u8>()).max_size(1024));
        let selector = test_case.draw(gs::integers::<u8>());
        let domain = if selector % 2 == 0 {
            "hegel-envelope-a"
        } else {
            "hegel-envelope-b"
        };
        let production_fields = vec![(1, CanonicalValue::Bytes(bytes.clone()))];
        let reference_fields = vec![(1, Value::Bytes(bytes))];
        assert_eq!(
            semantic_digest_observation(domain, production_fields)
                .ok()
                .map(|value| value.canonical_preimage().to_vec()),
            buoy_reference::semantic_preimage(2, domain, reference_fields).ok()
        );
    })
    .settings(settings(201))
    .run();
}

#[test]
fn hegel_production_envelope_is_deterministic_and_exercises_bindings() {
    Hegel::new(|test_case: TestCase| {
        let sample = test_case.draw(gs::integers::<u8>());
        let first = CanonicalValue::Bytes(vec![sample, 0]);
        let second = CanonicalValue::Bytes(vec![sample, 1]);
        let baseline_fields = vec![(1, first.clone()), (2, second.clone())];
        let baseline = semantic_digest("hegel-binding-a", baseline_fields.clone()).ok();
        assert_eq!(
            baseline,
            semantic_digest("hegel-binding-a", baseline_fields).ok()
        );
        assert_ne!(
            baseline,
            semantic_digest(
                "hegel-binding-b",
                vec![(1, first.clone()), (2, second.clone())],
            )
            .ok()
        );
        assert_ne!(
            baseline,
            semantic_digest("hegel-binding-a", vec![(1, second), (2, first)]).ok()
        );
    })
    .settings(settings(202))
    .run();
}
