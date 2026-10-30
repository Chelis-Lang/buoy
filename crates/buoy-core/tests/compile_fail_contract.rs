#![doc = "Compile-fail controls for frozen public core API boundaries."]

#[path = "algorithms_model_coverage_controls.rs"]
mod model_coverage_controls;

#[test]
fn frozen_public_api_rejects_invalid_construction_and_callable_effects() {
    let cases = trybuild::TestCases::new();
    cases.pass("tests/ui/valid_constructor.rs");
    cases.compile_fail("tests/ui/direct_field_construction.rs");
    cases.compile_fail("tests/ui/metadata_kind_substitution.rs");
    cases.compile_fail("tests/ui/callable_effect_input.rs");
    cases.compile_fail("tests/ui/raw_source_item_kind.rs");
}
