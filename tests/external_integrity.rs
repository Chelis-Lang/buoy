#![doc = "Exact opaque external-integrity trust-boundary fixtures."]
#![allow(clippy::expect_used, clippy::too_many_lines)]

use buoy::external_integrity::{
    ExternalEcosystem, ExternalIntegrityError, parse_opaque_external_integrity,
};
use std::collections::BTreeSet;
use std::fs;

const CASES: &str = "fixtures/xxh3/v2/external-integrity-cases.tsv";

#[test]
fn external_integrity_cases_accept_only_exact_fields_and_reject_promotion() {
    let source = fs::read_to_string(CASES);
    assert!(source.is_ok());
    let Some(source) = source.ok() else {
        return;
    };
    let mut cases = 0;
    for line in source.lines().skip(1) {
        let columns = line.split('\t').collect::<Vec<_>>();
        assert_eq!(columns.len(), 7);
        let fixture = fs::read_to_string(columns[4]);
        assert!(fixture.is_ok(), "{}", columns[1]);
        let Some(fixture) = fixture.ok() else {
            continue;
        };
        let result = parse_opaque_external_integrity(columns[3], &fixture);
        if columns[2] == "positive" {
            assert!(result.as_ref().is_ok_and(|fields| {
                !fields.is_empty()
                    && fields
                        .iter()
                        .all(|field| !field.field().is_empty() && !field.value().is_empty())
            }));
        } else {
            let actual = result
                .err()
                .map(ExternalIntegrityError::code)
                .into_iter()
                .collect::<BTreeSet<_>>();
            let expected = columns[6].split(',').collect::<BTreeSet<_>>();
            assert_eq!(actual, expected, "{}", columns[1]);
        }
        cases += 1;
    }
    assert_eq!(cases, 8);
    for error in [
        ExternalIntegrityError::FieldSyntax,
        ExternalIntegrityError::BroadException,
        ExternalIntegrityError::SemanticPromotion,
    ] {
        assert!(error.code().starts_with("XXH-EXTERNAL-"));
    }
    let cargo = parse_opaque_external_integrity(
        "cargo",
        "[[package]]\nsource = \"registry+control\"\nchecksum = \"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\"\n",
    )
    .expect("cargo control");
    assert_eq!(cargo[0].ecosystem(), ExternalEcosystem::Cargo);
    for source in [
        "not = [toml",
        "other = true",
        "package = 1",
        "package = [1]",
        "package = [{}]",
        "package = [{ source = 1, checksum = \"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\" }]",
        "package = [{ source = \"registry+x\" }]",
        "package = [{ source = \"path+x\", checksum = \"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\" }]",
        "package = [{ source = \"registry+x\", checksum = \"short\" }]",
        "package = [{ source = \"registry+x\", checksum = \"zzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzz\" }]",
        "package = []",
    ] {
        assert_eq!(
            parse_opaque_external_integrity("cargo", source),
            Err(ExternalIntegrityError::FieldSyntax)
        );
    }
    assert!(
        parse_opaque_external_integrity("nix", "fetchurl {\nhash = \"sha512-control\";\n}",)
            .is_ok()
    );
    for source in [
        "not fetchurl {}",
        "fetchurl { }",
        "fetchurl { hash = 1; }",
        "fetchurl { hash = \"md5-control\"; }",
        "fetchurl {\nhash = \"md5-control\";\n}",
        "fetchurl {\nhash \"sha256-control\";\n}",
        "fetchurl {\nhash = sha256-control;\n}",
        "fetchurl {\nhash = \"sha256-control\"\n}",
    ] {
        assert_eq!(
            parse_opaque_external_integrity("nix", source),
            Err(ExternalIntegrityError::FieldSyntax)
        );
    }
    for source in [
        "not = [toml",
        "other = true",
        "dependencies = 1",
        "dependencies = { control = 1 }",
        "dependencies = { control = {} }",
        "dependencies = { control = { git = 1, rev = \"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\" } }",
        "dependencies = { control = { git = \"https://example.invalid\" } }",
        "dependencies = { control = { git = \"http://example.invalid\", rev = \"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\" } }",
        "dependencies = { control = { git = \"https://example.invalid\", rev = \"short\" } }",
        "dependencies = {}",
    ] {
        assert_eq!(
            parse_opaque_external_integrity("git", source),
            Err(ExternalIntegrityError::FieldSyntax)
        );
    }
    let upstream_prefix = "kind = \"upstream-release-metadata\"\n";
    for source in [
        String::from("not = [toml"),
        String::from("kind = \"wrong\""),
        upstream_prefix.to_owned(),
        format!("{upstream_prefix}tag = \"v1\"\n"),
        format!("{upstream_prefix}tag = \"v1\"\nasset = \"a\"\n"),
        format!("{upstream_prefix}tag = \"v1\"\nasset = \"a\"\nchecksum_algorithm = \"sha256\"\n"),
        format!(
            "{upstream_prefix}tag = \"v1\"\nasset = \"a\"\nchecksum_algorithm = \"sha256\"\nchecksum = \"x\"\n"
        ),
    ] {
        assert_eq!(
            parse_opaque_external_integrity("upstream", &source),
            Err(ExternalIntegrityError::FieldSyntax)
        );
    }
    assert_eq!(
        parse_opaque_external_integrity("unknown", "valid = true"),
        Err(ExternalIntegrityError::FieldSyntax)
    );
    assert_eq!(
        parse_opaque_external_integrity("exception", "valid = true"),
        Err(ExternalIntegrityError::BroadException)
    );
    for broad in ["allow_all_hash_strings", "always-external"] {
        assert_eq!(
            parse_opaque_external_integrity("cargo", broad),
            Err(ExternalIntegrityError::BroadException)
        );
    }
}
