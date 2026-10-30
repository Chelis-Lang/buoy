//! Positive API probe and deterministic expected-failure controls for Hegel-rs.

#![allow(clippy::eq_op, clippy::expect_used)]

#[path = "../tools/digest_binding.rs"]
mod content_digest;
#[path = "../tools/execution_evidence.rs"]
mod execution_evidence;

use hegel::generators as gs;
use hegel::{Hegel, TestCase};
use std::any::Any;
use std::panic::{AssertUnwindSafe, catch_unwind};

#[path = "support/hegel_settings.rs"]
mod hegel_settings;
use hegel_settings::settings;

const FALSE_PROPERTY_ID: &str = "hegel-control-intentionally-false";
const FALSE_PROPERTY_SEED_SUFFIX: u16 = 255;
const FALSE_REPLAY: &str = include_str!("../fixtures/property/v1/replays/intentionally-false.tsv");
const FALSE_EXECUTION: &str = "assurance/v1/hegel-false-control-execution.tsv";
const FALSE_SOURCE: &str = "tests/hegel_controls.rs";
const FALSE_TOOLS: [&str; 4] = [
    FALSE_SOURCE,
    "tests/support/hegel_settings.rs",
    "tools/execution_evidence.rs",
    "tools/digest_binding.rs",
];
const FALSE_CONFIGURATION: [&str; 5] = [
    "Cargo.toml",
    "Cargo.lock",
    "assurance/v1/hegel-tool-binding.toml",
    "assurance/v1/property-inventory.tsv",
    "fixtures/property/v1/replays/intentionally-false.tsv",
];

fn panic_message(payload: &(dyn Any + Send)) -> Option<&str> {
    payload
        .downcast_ref::<String>()
        .map(String::as_str)
        .or_else(|| payload.downcast_ref::<&'static str>().copied())
}

fn validate_expected_failure_execution_evidence(write: bool) {
    let mut replay_lines = FALSE_REPLAY.lines();
    assert_eq!(
        replay_lines.next(),
        Some("schema\tproperty_id\tseed\tinput\texpected_diagnostic")
    );
    let replay = replay_lines
        .next()
        .expect("reviewed false-property replay row must be present")
        .split('\t')
        .collect::<Vec<_>>();
    assert_eq!(replay_lines.next(), None);
    assert_eq!(
        replay,
        [
            "buoy-property-replay/v1",
            FALSE_PROPERTY_ID,
            "6003393071738257663",
            "false",
            "HEGEL-NEGATIVE-CONTROL",
        ]
    );

    let rejection = catch_unwind(AssertUnwindSafe(|| {
        Hegel::new(|test_case: TestCase| {
            let value = test_case.draw(gs::booleans());
            assert_ne!(value, value, "{FALSE_PROPERTY_ID}");
        })
        .settings(settings(FALSE_PROPERTY_SEED_SUFFIX))
        .run();
    }));
    let payload = rejection.expect_err("HEGEL-NEGATIVE-CONTROL: false property passed");
    let message = panic_message(payload.as_ref())
        .expect("HEGEL-EXECUTION-INCOMPLETE: rejection had an unknown panic payload");
    assert!(
        message.contains(FALSE_PROPERTY_ID),
        "HEGEL-EXECUTION-INCOMPLETE: rejection did not preserve property identity: {message}"
    );
    let source = include_bytes!("hegel_controls.rs");
    let record = execution_evidence::passing(
        "property",
        "hegel-intentionally-false-control",
        FALSE_SOURCE,
        source,
        &FALSE_TOOLS,
        &FALSE_CONFIGURATION,
        format!(
            "property={FALSE_PROPERTY_ID};execution=expected-failure-observed;diagnostic=HEGEL-NEGATIVE-CONTROL;cardinality=1;seed=6003393071738257663"
        ),
    )
    .expect("false-control evidence must bind exact source, tool, and configuration");
    let expected = execution_evidence::render(&[record])
        .expect("false-control evidence must render deterministically");
    execution_evidence::validate_materialized(FALSE_EXECUTION, &expected, write)
        .expect("false-control execution evidence must be current");
}

#[test]
fn hegel_expected_failure_is_rejected_exactly_once() {
    validate_expected_failure_execution_evidence(false);
}

#[test]
#[ignore = "explicit reviewed intentionally-false control regeneration only"]
fn write_hegel_expected_failure_execution_evidence() {
    validate_expected_failure_execution_evidence(true);
}
