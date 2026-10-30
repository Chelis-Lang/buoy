#![doc = "Exact proof-source and correspondence bindings for logical mount projection."]
#![allow(clippy::expect_used)]

#[path = "../tools/digest_binding.rs"]
mod content_digest;

use std::collections::BTreeMap;
use std::path::Path;
use toml::Value;

fn digest(path: &str) -> String {
    content_digest::file_binding(Path::new(path)).expect("proof input digest")
}

#[test]
fn logical_mount_proof_binds_every_input_assumption_exclusion_and_false_control() {
    let record =
        toml::from_str::<Value>(include_str!("../proofs/logical-mounts/proof-record.toml"))
            .expect("logical mount proof record");
    let derivation = toml::from_str::<Value>(include_str!(
        "../proofs/logical-mounts/derivation-record.toml"
    ))
    .expect("logical mount derivation record");
    let approval = toml::from_str::<Value>(include_str!(
        "../proofs/logical-mounts/correspondence-approval.toml"
    ))
    .expect("logical mount correspondence approval");

    assert_eq!(
        record["schema"].as_str(),
        Some("spec-provenance-proof-binding/v2")
    );
    assert_eq!(record["lane"].as_str(), Some("verus"));
    assert_eq!(record["verdict"].as_str(), Some("pass"));
    assert_eq!(record["negative_control"].as_str(), Some("expected-fail"));
    assert_eq!(
        record["implementation"]["digest"].as_str(),
        Some(digest("crates/buoy-core/src/mount.rs").as_str())
    );
    assert_eq!(
        record["proof_source"]["digest"].as_str(),
        Some(digest("proofs/spikes/verus/logical_mounts.rs").as_str())
    );
    assert_eq!(
        record["negative_source"]["digest"].as_str(),
        Some(digest("proofs/spikes/verus/logical_mounts_negative.rs").as_str())
    );
    assert_eq!(
        record["negative_source"]["expected_diagnostic"].as_str(),
        Some("postcondition not satisfied")
    );

    let inputs = record["additional_inputs"]
        .as_array()
        .expect("additional proof inputs")
        .iter()
        .map(|input| {
            (
                input["role"].as_str().expect("input role"),
                input["digest"].as_str().expect("input digest"),
            )
        })
        .collect::<BTreeMap<_, _>>();
    assert_eq!(
        inputs.get("attachment-key-production").copied(),
        Some(digest("crates/buoy-core/src/attachment.rs").as_str())
    );
    assert_eq!(
        inputs.get("scope-and-exclusions").copied(),
        Some(
            digest(
                "openspec/changes/archive/2026-08-20-add-logical-mount-artifact-bindings/evidence/formal-spike-scope.md"
            )
            .as_str()
        )
    );
    assert_eq!(record["assumptions"].as_array().map(Vec::len), Some(2));
    assert!(
        record["exclusions"]
            .as_array()
            .is_some_and(|values| values.len() >= 7)
    );

    assert_eq!(derivation["id"].as_str(), record["derivation"].as_str());
    assert_eq!(
        approval["id"].as_str(),
        record["correspondence_approval"].as_str()
    );
    assert_eq!(approval["status"].as_str(), Some("approved"));
    assert_eq!(
        derivation["correspondence"].as_array().map(Vec::len),
        Some(3)
    );

    for source in [
        include_str!("../proofs/spikes/verus/logical_mounts.rs"),
        include_str!("../proofs/spikes/verus/logical_mounts_negative.rs"),
        include_str!("../proofs/spikes/aeneas/lean/IntegrityProofs.lean"),
    ] {
        assert!(!source.contains("sorry"));
        assert!(!source.contains("admit"));
    }
    let command = include_str!("../nix/devenv/scripts/assurance.nix");
    assert!(command.contains("verus proofs/spikes/verus/logical_mounts.rs"));
    assert!(command.contains("verus proofs/spikes/verus/logical_mounts_negative.rs"));
    assert!(
        command.contains("expected postcondition diagnostic")
            || command.contains("postcondition not satisfied")
    );
}
