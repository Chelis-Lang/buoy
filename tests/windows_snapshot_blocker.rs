#![doc = "Reviewed Windows snapshot blocker and reserved/collision/reparse fixture dispatch."]
#![allow(clippy::expect_used)]

use buoy::core::{RepositoryPath, canonical_paths};
use buoy::repository::{
    RepositoryError, WINDOWS_SNAPSHOT_BLOCKER_ID, windows_snapshot_blocker_disposition,
};
use serde::Deserialize;
use std::collections::BTreeSet;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct FixtureManifest {
    schema: String,
    review_status: String,
    reviewed_on: String,
    blocker_id: String,
    blocker_diagnostic: String,
    case: Vec<FixtureCase>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct FixtureCase {
    id: String,
    operation: String,
    inputs: Vec<String>,
    expected: String,
}

#[test]
fn windows_reserved_collision_and_reparse_blocker_fixtures_are_total() {
    let manifest: FixtureManifest = toml::from_str(include_str!(
        "../fixtures/repository/windows-snapshot-v1.toml"
    ))
    .expect("strict Windows fixture manifest");
    assert_eq!(manifest.schema, "buoy.windows-snapshot-fixtures/v1");
    assert_eq!(manifest.review_status, "accepted-blocking-disposition");
    assert_eq!(manifest.reviewed_on, "2026-07-22");
    assert_eq!(manifest.blocker_id, WINDOWS_SNAPSHOT_BLOCKER_ID);
    assert_eq!(
        manifest.blocker_diagnostic,
        RepositoryError::PlatformBlocker.code()
    );
    assert_eq!(manifest.case.len(), 3);
    assert_eq!(
        manifest
            .case
            .iter()
            .map(|case| case.id.as_str())
            .collect::<BTreeSet<_>>()
            .len(),
        manifest.case.len()
    );

    let mut dispatched = BTreeSet::new();
    for case in &manifest.case {
        let actual = match case.operation.as_str() {
            "portable-path" => case
                .inputs
                .first()
                .and_then(|path| RepositoryPath::new(path).err())
                .map(buoy::core::PathError::code),
            "portable-collision" => {
                let paths = case
                    .inputs
                    .iter()
                    .map(|path| RepositoryPath::new(path).expect("fixture path"))
                    .collect::<Vec<_>>();
                canonical_paths(paths)
                    .expect_err("fixture paths must collide")
                    .eq(&buoy::core::PathSetError::PortableCollision)
                    .then_some("DET-PATH-SELECTION-COLLISION")
            }
            "platform-blocker" => {
                let (blocker_id, error) = windows_snapshot_blocker_disposition();
                (blocker_id == manifest.blocker_id && !case.inputs.is_empty())
                    .then_some(error.code())
            }
            _ => None,
        };
        assert_eq!(actual, Some(case.expected.as_str()), "{}", case.id);
        assert!(dispatched.insert(case.id.as_str()));
    }
    assert_eq!(dispatched.len(), manifest.case.len());
}

#[cfg(windows)]
#[test]
fn windows_snapshot_entry_point_fails_before_path_based_traversal() {
    use buoy::repository::{RepositoryConfig, snapshot_repository};

    let config = RepositoryConfig::parse(
        br#"
schema = "spec-provenance-config/v2"
digest_algorithms = ["xxh3-128"]
[[selectors]]
root = "spec"
adapter = "markdown/v1"
include = ["**/*.md"]
exclude = []
authority_expectation = "optional"
"#,
    )
    .expect("valid Windows blocker control configuration");
    let root = tempfile::tempdir().expect("unused Windows blocker root");
    assert_eq!(
        snapshot_repository(root.path(), &config),
        Err(RepositoryError::PlatformBlocker)
    );
}
