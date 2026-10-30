//! Compact bindings under a change-detection active member.
//!
//! A content address is only a claim about content while the member resisting
//! collisions makes it one. The default build's member is `xxh3-128`, which
//! does not, so every compact binding must fail closed there rather than
//! resolve weakly. This suite is the default-build counterpart of the
//! collision-resistant suites.

#![cfg(feature = "xxh3-128")]
#![allow(clippy::expect_used, clippy::panic)]

use chelis_provenance::command::{EXIT_STATIC, static_check};
use chelis_provenance::records::ChelisDialect;
use chelis_provenance::snapshot::{ChelisConfiguration, ChelisSnapshot};

const STORE: &str = "chelis/objects";
const DECLARATION: &str = "def weak_impl(x: f32) -> f32 = x";

fn configuration() -> ChelisConfiguration {
    ChelisConfiguration::parse(
        format!(
            "schema = \"chelis-adapter-config/v1\"\nbinding-store = \"{STORE}\"\n\n[[selector]]\nroot = \"chelis\"\ndialect = \"surf\"\n"
        )
        .as_bytes(),
    )
    .expect("configuration")
}

/// One marker carrying a well-formed collision-resistant address.
fn source(address: &str) -> String {
    format!(
        "-- chelis:provenance/v1 authority\n-- id = WEAK-ATOM\n-- kind = behavioral\n-- scopes = chelis-core\n-- statement = The weak member MUST resolve no binding.\ndef weak_subject(x: f32) -> f32 = x\n\
         -- chelis:provenance/v1 binding\n-- record = {address}\n{DECLARATION}\n"
    )
}

fn snapshot_of(address: &str) -> ChelisSnapshot {
    ChelisSnapshot::from_files(vec![(
        String::from("chelis/source.ch"),
        ChelisDialect::Surf,
        source(address).into_bytes(),
    )])
    .expect("snapshot")
}

#[test]
fn this_build_refuses_to_address_content() {
    let reason =
        chelis_provenance::command::binding_algorithm().expect_err("xxh3-128 addresses nothing");
    assert_eq!(reason, "algorithm-not-collision-resistant:xxh3-128");
}

#[test]
fn every_compact_binding_fails_closed() {
    let address = format!("blake3-256:{}", "11".repeat(32));
    let outcome = static_check(&snapshot_of(&address), &configuration()).expect("static outcome");
    assert_eq!(outcome.exit, EXIT_STATIC);
    let diagnostic = outcome
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == "CHELIS-PROV-BINDING-ADDRESS")
        .expect("a change-detection build resolves no binding");
    assert_eq!(
        diagnostic.context, "algorithm-not-collision-resistant:xxh3-128",
        "the refusal names why this build cannot address content"
    );
    assert!(
        outcome.graph.implementation_bindings.is_empty(),
        "nothing materializes under a change-detection member"
    );
}

/// The refusal precedes every store lookup, so a marker whose own address is
/// change detection fails for the member, not for a missing object.
#[test]
fn a_change_detection_address_fails_for_the_member() {
    let address = format!("xxh3-128:{}", "22".repeat(16));
    let outcome = static_check(&snapshot_of(&address), &configuration()).expect("static outcome");
    assert_eq!(outcome.exit, EXIT_STATIC);
    assert!(
        outcome.diagnostics.iter().any(|diagnostic| diagnostic.code
            == "CHELIS-PROV-BINDING-ADDRESS"
            && diagnostic.context == "algorithm-not-collision-resistant:xxh3-128"),
        "{:?}",
        outcome.diagnostics
    );
}
