#![doc = "Fail-closed contracts for adversarial campaign, platform, and gate evidence."]
#![allow(clippy::expect_used)]

use buoy::adversarial::{
    CampaignError, CampaignKind, campaign_bindings, platform_effect_diagnostics, portable_corpus,
    validate_campaign_documents, validate_named_mutant_catalog, validate_repository_campaign,
};
#[cfg(feature = "coverage-controls")]
use buoy::adversarial::{
    campaign_bound_control, campaign_input_fault_control, portable_corpus_control,
    repository_path_text_control,
};
use std::fmt::Write as _;
use std::fs;
use std::path::Path;

const CAMPAIGN_AS_OF: &str = "2026-08-31";

fn root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

#[test]
fn committed_fuzz_and_mutation_campaigns_are_current_in_their_exact_scopes() {
    for kind in [CampaignKind::Fuzz, CampaignKind::Mutation] {
        let stem = match kind {
            CampaignKind::Fuzz => "fuzz",
            CampaignKind::Mutation => "mutation",
        };
        let artifact = fs::read_to_string(
            root().join(format!("assurance/adversarial/v1/{stem}-campaign.toml")),
        )
        .expect("campaign artifact");
        let artifact = toml::from_str::<toml::Value>(&artifact).expect("campaign artifact TOML");
        let generated_on = artifact["generated_on"]
            .as_str()
            .expect("campaign generation date");
        validate_repository_campaign(root(), kind, generated_on)
            .expect("committed campaign must match its exact inputs at generation");
    }
}

#[test]
fn stale_expired_or_discharged_campaign_evidence_cannot_satisfy_an_obligation() {
    let (policy, artifact, _, _) = policy_and_artifact(CampaignKind::Fuzz);
    let bindings = campaign_bindings(root(), CampaignKind::Fuzz).expect("fuzz bindings");

    let stale = artifact.replacen(
        &bindings.source_digest,
        "xxh3-128:00000000000000000000000000000000",
        1,
    );
    assert_eq!(
        validate_campaign_documents(
            &policy,
            &stale,
            &bindings.source_digest,
            &bindings.configuration_digest,
            CAMPAIGN_AS_OF,
        ),
        Err(CampaignError::SourceStale)
    );
    assert_eq!(
        validate_campaign_documents(
            &policy,
            &artifact,
            &bindings.source_digest,
            &bindings.configuration_digest,
            "2026-09-28",
        ),
        Err(CampaignError::Expired)
    );

    let discharged = format!(
        "{artifact}\n[[discharges]]\nid = \"DISCHARGE-FAKE\"\nscope = \"all fuzz obligations\"\nrationale = \"aggregate green\"\nreview = \"reviews/fake.md\"\ncounts_as_satisfied = true\n"
    );
    assert_eq!(
        validate_campaign_documents(
            &policy,
            &discharged,
            &bindings.source_digest,
            &bindings.configuration_digest,
            CAMPAIGN_AS_OF,
        ),
        Err(CampaignError::DischargeCannotSatisfy)
    );
}

#[test]
fn portable_corpus_is_host_independent_and_effect_diagnostics_are_scoped() {
    let first = portable_corpus().expect("portable corpus");
    let second = portable_corpus().expect("repeat portable corpus");
    assert_eq!(first, second);
    for required in [
        "canonical-values",
        "paths-globs",
        "adapters",
        "config-manifests",
        "graphs-governance",
        "reports",
    ] {
        assert!(
            first.contains(required),
            "missing portable corpus row {required}"
        );
    }

    let linux = platform_effect_diagnostics("linux").expect("Linux effects");
    let macos = platform_effect_diagnostics("macos").expect("macOS effects");
    let windows = platform_effect_diagnostics("windows").expect("Windows effects");
    assert!(linux.contains("descriptor-relative-supported"));
    assert!(macos.contains("descriptor-relative-supported"));
    assert!(windows.contains("PLATFORM-WINDOWS-SNAPSHOT-001"));
    assert!(windows.contains("PLATFORM-WINDOWS-JOB-001"));
    assert!(!windows.contains("production-supported"));
}

#[cfg(feature = "coverage-controls")]
#[test]
fn portable_corpus_failure_controls_reach_each_owned_stage() {
    for (control, expected) in [
        ("canonical-encode", CampaignError::CorpusCanonical),
        ("canonical-decode", CampaignError::CorpusCanonical),
        ("path-construct", CampaignError::CorpusPath),
        ("glob-parse", CampaignError::CorpusPath),
        ("markdown-path", CampaignError::Corpus),
        ("rust-path", CampaignError::Corpus),
        ("configuration-parse", CampaignError::CorpusConfiguration),
        ("configuration-digest", CampaignError::CorpusConfiguration),
        ("report-new", CampaignError::CorpusReport),
        ("report-decode", CampaignError::CorpusReport),
        ("canonical-row", CampaignError::Corpus),
        ("path-row", CampaignError::Corpus),
        ("adapter-row", CampaignError::Corpus),
        ("configuration-mismatch", CampaignError::CorpusConfiguration),
        ("configuration-row", CampaignError::Corpus),
        ("governance-mismatch", CampaignError::CorpusGovernance),
        ("governance-row", CampaignError::Corpus),
        ("report-mismatch", CampaignError::CorpusReport),
        ("report-row", CampaignError::Corpus),
    ] {
        assert_eq!(portable_corpus_control(control), Err(expected));
    }
    assert_eq!(
        portable_corpus_control("none"),
        portable_corpus(),
        "unknown controls must preserve the production corpus"
    );
}

#[test]
fn committed_named_mutant_catalog_is_valid() {
    validate_named_mutant_catalog(root()).expect("named mutant catalog");
}

fn policy_and_artifact(kind: CampaignKind) -> (String, String, String, String) {
    let stem = match kind {
        CampaignKind::Fuzz => "fuzz",
        CampaignKind::Mutation => "mutation",
    };
    let policy =
        fs::read_to_string(root().join(format!("assurance/adversarial/v1/{stem}-policy.toml")))
            .expect("campaign policy");
    let artifact =
        fs::read_to_string(root().join(format!("assurance/adversarial/v1/{stem}-campaign.toml")))
            .expect("campaign artifact");
    let mut artifact_value =
        toml::from_str::<toml::Value>(&artifact).expect("campaign artifact TOML");
    // Fixed fixture dates isolate boundary controls from campaign renewal.
    artifact_value["generated_on"] = toml::Value::String(CAMPAIGN_AS_OF.to_owned());
    artifact_value["expires_on"] = toml::Value::String("2026-09-27".to_owned());
    // Synthetic controls use the current command authority, not past execution metadata.
    let policy_value = toml::from_str::<toml::Value>(&policy).expect("campaign policy TOML");
    artifact_value["command"] = policy_value["campaign_command"].clone();
    let source_digest = artifact_value["source_digest"]
        .as_str()
        .expect("artifact source digest")
        .to_owned();
    let configuration_digest = artifact_value["configuration_digest"]
        .as_str()
        .expect("artifact configuration digest")
        .to_owned();
    let artifact = toml::to_string(&artifact_value).expect("campaign control TOML");
    (policy, artifact, source_digest, configuration_digest)
}

fn set_toml(source: &str, field: &str, value: toml::Value) -> String {
    let mut document = toml::from_str::<toml::Value>(source).expect("valid control TOML");
    document
        .as_table_mut()
        .expect("control document table")
        .insert(field.to_owned(), value);
    toml::to_string(&document).expect("render control TOML")
}

fn assert_document_error(
    policy: &str,
    artifact: &str,
    source_digest: &str,
    configuration_digest: &str,
    as_of: &str,
    expected: CampaignError,
) {
    assert_eq!(
        validate_campaign_documents(policy, artifact, source_digest, configuration_digest, as_of,),
        Err(expected)
    );
}

#[test]
fn every_closed_campaign_policy_field_has_a_negative_control() {
    let (fuzz_policy, fuzz_artifact, fuzz_source, fuzz_configuration) =
        policy_and_artifact(CampaignKind::Fuzz);
    assert_document_error(
        "not TOML",
        &fuzz_artifact,
        &fuzz_source,
        &fuzz_configuration,
        CAMPAIGN_AS_OF,
        CampaignError::PolicySyntax,
    );
    assert_document_error(
        &set_toml(
            &fuzz_policy,
            "kind",
            toml::Value::String("other".to_owned()),
        ),
        &fuzz_artifact,
        &fuzz_source,
        &fuzz_configuration,
        CAMPAIGN_AS_OF,
        CampaignError::PolicySchema,
    );

    for (field, value) in [
        ("schema", toml::Value::String("wrong/v1".to_owned())),
        ("id", toml::Value::String(String::new())),
        ("tool_id", toml::Value::String("wrong".to_owned())),
        ("tool_version", toml::Value::String("wrong".to_owned())),
        ("tool_binding", toml::Value::String("wrong".to_owned())),
        ("source_paths", toml::Value::Array(Vec::new())),
        ("configuration_paths", toml::Value::Array(Vec::new())),
        ("required_units", toml::Value::Array(Vec::new())),
        ("minimum_cases", toml::Value::Integer(0)),
        ("minimum_duration_seconds", toml::Value::Integer(0)),
        ("maximum_age_days", toml::Value::Integer(0)),
        ("campaign_command", toml::Value::String(String::new())),
        ("replay_command", toml::Value::String(String::new())),
        ("assurance_scope", toml::Value::String(String::new())),
        ("minimum_score_basis_points", toml::Value::Integer(1)),
    ] {
        assert_document_error(
            &set_toml(&fuzz_policy, field, value),
            &fuzz_artifact,
            &fuzz_source,
            &fuzz_configuration,
            CAMPAIGN_AS_OF,
            CampaignError::PolicySchema,
        );
    }
    assert_document_error(
        &set_toml(&fuzz_policy, "maximum_age_days", toml::Value::Integer(91)),
        &fuzz_artifact,
        &fuzz_source,
        &fuzz_configuration,
        CAMPAIGN_AS_OF,
        CampaignError::PolicySchema,
    );

    for field in [
        "source_paths",
        "configuration_paths",
        "required_units",
        "exclusions",
    ] {
        let mut document =
            toml::from_str::<toml::Value>(&fuzz_policy).expect("fuzz policy control");
        let values = document[field]
            .as_array_mut()
            .expect("policy array control");
        values.push(values[0].clone());
        assert_document_error(
            &toml::to_string(&document).expect("duplicate policy control"),
            &fuzz_artifact,
            &fuzz_source,
            &fuzz_configuration,
            CAMPAIGN_AS_OF,
            CampaignError::PolicySchema,
        );
    }

    let (mutation_policy, mutation_artifact, mutation_source, mutation_configuration) =
        policy_and_artifact(CampaignKind::Mutation);
    for score in [0, 10_001] {
        assert_document_error(
            &set_toml(
                &mutation_policy,
                "minimum_score_basis_points",
                toml::Value::Integer(score),
            ),
            &mutation_artifact,
            &mutation_source,
            &mutation_configuration,
            CAMPAIGN_AS_OF,
            CampaignError::PolicySchema,
        );
    }
}

#[test]
#[allow(clippy::too_many_lines)]
fn every_campaign_artifact_gate_has_a_negative_control() {
    let (fuzz_policy, fuzz_artifact, fuzz_source, fuzz_configuration) =
        policy_and_artifact(CampaignKind::Fuzz);
    assert_document_error(
        &fuzz_policy,
        "not TOML",
        &fuzz_source,
        &fuzz_configuration,
        CAMPAIGN_AS_OF,
        CampaignError::ArtifactSyntax,
    );
    for (field, value, expected) in [
        (
            "schema",
            toml::Value::String("wrong/v1".to_owned()),
            CampaignError::ArtifactSchema,
        ),
        (
            "campaign_id",
            toml::Value::String("wrong".to_owned()),
            CampaignError::ArtifactSchema,
        ),
        (
            "kind",
            toml::Value::String("mutation".to_owned()),
            CampaignError::ArtifactSchema,
        ),
        (
            "command",
            toml::Value::String("wrong".to_owned()),
            CampaignError::ArtifactSchema,
        ),
        (
            "assurance_scope",
            toml::Value::String("wrong".to_owned()),
            CampaignError::ArtifactSchema,
        ),
        (
            "tool_id",
            toml::Value::String("wrong".to_owned()),
            CampaignError::ToolBinding,
        ),
        (
            "tool_binding",
            toml::Value::String("wrong".to_owned()),
            CampaignError::ToolBinding,
        ),
        (
            "tool_version",
            toml::Value::String("wrong".to_owned()),
            CampaignError::ToolBinding,
        ),
        (
            "source_digest",
            toml::Value::String("not-a-digest".to_owned()),
            CampaignError::SourceStale,
        ),
        (
            "configuration_digest",
            toml::Value::String("not-a-digest".to_owned()),
            CampaignError::ConfigurationStale,
        ),
        (
            "result",
            toml::Value::String("fail".to_owned()),
            CampaignError::CampaignFailed,
        ),
        (
            "finding_count",
            toml::Value::Integer(1),
            CampaignError::CampaignFailed,
        ),
        (
            "unresolved_blocking_findings",
            toml::Value::Integer(1),
            CampaignError::CampaignFailed,
        ),
        (
            "duration_seconds",
            toml::Value::Integer(0),
            CampaignError::Bounds,
        ),
        (
            "total_cases",
            toml::Value::Integer(383),
            CampaignError::Bounds,
        ),
        (
            "accepted_mutants",
            toml::Value::Integer(1),
            CampaignError::ArtifactSchema,
        ),
        (
            "killed_mutants",
            toml::Value::Integer(1),
            CampaignError::ArtifactSchema,
        ),
        (
            "score_basis_points",
            toml::Value::Integer(1),
            CampaignError::ArtifactSchema,
        ),
    ] {
        assert_document_error(
            &fuzz_policy,
            &set_toml(&fuzz_artifact, field, value),
            &fuzz_source,
            &fuzz_configuration,
            CAMPAIGN_AS_OF,
            expected,
        );
    }
    assert_document_error(
        &fuzz_policy,
        &set_toml(
            &fuzz_artifact,
            "source_digest",
            toml::Value::String("xxh3-128:22222222222222222222222222222222".to_owned()),
        ),
        &fuzz_source,
        &fuzz_configuration,
        CAMPAIGN_AS_OF,
        CampaignError::SourceStale,
    );
    assert_document_error(
        &fuzz_policy,
        &set_toml(
            &fuzz_artifact,
            "configuration_digest",
            toml::Value::String("xxh3-128:33333333333333333333333333333333".to_owned()),
        ),
        &fuzz_source,
        &fuzz_configuration,
        CAMPAIGN_AS_OF,
        CampaignError::ConfigurationStale,
    );
    assert_document_error(
        &set_toml(&fuzz_policy, "minimum_cases", toml::Value::Integer(449)),
        &fuzz_artifact,
        &fuzz_source,
        &fuzz_configuration,
        CAMPAIGN_AS_OF,
        CampaignError::Bounds,
    );
    let survivor = "[[survivors]]\nid = \"CONTROL\"\ndisposition = \"equivalent\"\nrationale = \"reviewed\"\nreview = \"review\"\ncounts_as_killed = false\n";
    assert_document_error(
        &fuzz_policy,
        &format!("{fuzz_artifact}\n{survivor}"),
        &fuzz_source,
        &fuzz_configuration,
        CAMPAIGN_AS_OF,
        CampaignError::ArtifactSchema,
    );

    for (field, value) in [("generated_on", "invalid"), ("expires_on", "invalid")] {
        assert_document_error(
            &fuzz_policy,
            &set_toml(&fuzz_artifact, field, toml::Value::String(value.to_owned())),
            &fuzz_source,
            &fuzz_configuration,
            CAMPAIGN_AS_OF,
            CampaignError::ArtifactSchema,
        );
    }
    assert_document_error(
        &fuzz_policy,
        &fuzz_artifact,
        &fuzz_source,
        &fuzz_configuration,
        "invalid",
        CampaignError::ArtifactSchema,
    );
    for (generated, expires, as_of) in [
        (CAMPAIGN_AS_OF, "2026-07-21", CAMPAIGN_AS_OF),
        (CAMPAIGN_AS_OF, "2026-08-21", "2026-07-21"),
        (CAMPAIGN_AS_OF, "2026-08-21", "2026-08-31"),
        ("2026-06-30", CAMPAIGN_AS_OF, CAMPAIGN_AS_OF),
    ] {
        let dated = set_toml(
            &set_toml(
                &fuzz_artifact,
                "generated_on",
                toml::Value::String(generated.to_owned()),
            ),
            "expires_on",
            toml::Value::String(expires.to_owned()),
        );
        assert_document_error(
            &fuzz_policy,
            &dated,
            &fuzz_source,
            &fuzz_configuration,
            as_of,
            CampaignError::Expired,
        );
    }

    let mut missing_unit =
        toml::from_str::<toml::Value>(&fuzz_artifact).expect("fuzz artifact control");
    missing_unit["units"]
        .as_array_mut()
        .expect("units control")
        .pop();
    assert_document_error(
        &fuzz_policy,
        &toml::to_string(&missing_unit).expect("missing unit control"),
        &fuzz_source,
        &fuzz_configuration,
        CAMPAIGN_AS_OF,
        CampaignError::UnitSet,
    );
    let mut duplicate_unit =
        toml::from_str::<toml::Value>(&fuzz_artifact).expect("fuzz artifact control");
    let units = duplicate_unit["units"]
        .as_array_mut()
        .expect("units control");
    units.push(units[0].clone());
    assert_document_error(
        &fuzz_policy,
        &toml::to_string(&duplicate_unit).expect("duplicate unit control"),
        &fuzz_source,
        &fuzz_configuration,
        CAMPAIGN_AS_OF,
        CampaignError::UnitSet,
    );
    for (field, value) in [
        ("status", toml::Value::String("fail".to_owned())),
        ("finding_count", toml::Value::Integer(1)),
        ("cases", toml::Value::Integer(0)),
    ] {
        let mut failed_unit =
            toml::from_str::<toml::Value>(&fuzz_artifact).expect("fuzz artifact control");
        failed_unit["units"][0]
            .as_table_mut()
            .expect("unit control")
            .insert(field.to_owned(), value);
        assert_document_error(
            &fuzz_policy,
            &toml::to_string(&failed_unit).expect("failed unit control"),
            &fuzz_source,
            &fuzz_configuration,
            CAMPAIGN_AS_OF,
            CampaignError::UnitFailed,
        );
    }

    let mut overflow_policy =
        toml::from_str::<toml::Value>(&fuzz_policy).expect("overflow policy control");
    overflow_policy["required_units"] = toml::Value::Array(
        ["one", "two", "three"]
            .into_iter()
            .map(|id| toml::Value::String(id.to_owned()))
            .collect(),
    );
    let mut overflow_artifact =
        toml::from_str::<toml::Value>(&fuzz_artifact).expect("overflow artifact control");
    let template = overflow_artifact["units"][0].clone();
    let units = ["one", "two", "three"]
        .into_iter()
        .map(|id| {
            let mut unit = template.clone();
            unit["id"] = toml::Value::String(id.to_owned());
            unit["cases"] = toml::Value::Integer(i64::MAX);
            unit
        })
        .collect();
    overflow_artifact["units"] = toml::Value::Array(units);
    overflow_artifact["total_cases"] = toml::Value::Integer(i64::MAX);
    assert_document_error(
        &toml::to_string(&overflow_policy).expect("overflow policy TOML"),
        &toml::to_string(&overflow_artifact).expect("overflow artifact TOML"),
        &fuzz_source,
        &fuzz_configuration,
        CAMPAIGN_AS_OF,
        CampaignError::Bounds,
    );

    for discharge in [
        "id = \"X\"\nscope = \"all\"\nrationale = \"reviewed\"\nreview = \"review\"\ncounts_as_satisfied = true",
        "id = \"\"\nscope = \"all\"\nrationale = \"reviewed\"\nreview = \"review\"\ncounts_as_satisfied = false",
        "id = \"X\"\nscope = \"\"\nrationale = \"reviewed\"\nreview = \"review\"\ncounts_as_satisfied = false",
        "id = \"X\"\nscope = \"all\"\nrationale = \"\"\nreview = \"review\"\ncounts_as_satisfied = false",
        "id = \"X\"\nscope = \"all\"\nrationale = \"reviewed\"\nreview = \"\"\ncounts_as_satisfied = false",
    ] {
        assert_document_error(
            &fuzz_policy,
            &format!("{fuzz_artifact}\n[[discharges]]\n{discharge}\n"),
            &fuzz_source,
            &fuzz_configuration,
            CAMPAIGN_AS_OF,
            CampaignError::DischargeCannotSatisfy,
        );
    }

    let (mutation_policy, mutation_artifact, mutation_source, mutation_configuration) =
        policy_and_artifact(CampaignKind::Mutation);
    for (field, value) in [
        ("accepted_mutants", toml::Value::Integer(0)),
        ("killed_mutants", toml::Value::Integer(4)),
        ("score_basis_points", toml::Value::Integer(9_999)),
    ] {
        assert_document_error(
            &mutation_policy,
            &set_toml(&mutation_artifact, field, value),
            &mutation_source,
            &mutation_configuration,
            CAMPAIGN_AS_OF,
            CampaignError::MutationScore,
        );
    }
    let survivor_control = format!(
        "{mutation_artifact}\n[[survivors]]\nid = \"control\"\ndisposition = \"unsupported\"\nrationale = \"tool rejected this control before execution\"\nreview = \"adversarial-assurance-review\"\ncounts_as_killed = false\n"
    );
    for (field, value) in [
        ("disposition", toml::Value::String("accepted".to_owned())),
        ("id", toml::Value::String(String::new())),
        ("rationale", toml::Value::String(String::new())),
        ("review", toml::Value::String(String::new())),
        ("counts_as_killed", toml::Value::Boolean(true)),
    ] {
        let mut survivor =
            toml::from_str::<toml::Value>(&survivor_control).expect("mutation artifact control");
        survivor["survivors"][0]
            .as_table_mut()
            .expect("survivor control")
            .insert(field.to_owned(), value);
        assert_document_error(
            &mutation_policy,
            &toml::to_string(&survivor).expect("survivor control TOML"),
            &mutation_source,
            &mutation_configuration,
            CAMPAIGN_AS_OF,
            CampaignError::SurvivorDisposition,
        );
    }
}

#[test]
fn campaign_error_codes_and_date_boundaries_are_closed() {
    let cases = [
        (CampaignError::PolicySyntax, "ADV-CAMPAIGN-POLICY-SYNTAX"),
        (
            CampaignError::ArtifactSyntax,
            "ADV-CAMPAIGN-ARTIFACT-SYNTAX",
        ),
        (CampaignError::PolicySchema, "ADV-CAMPAIGN-POLICY-SCHEMA"),
        (
            CampaignError::ArtifactSchema,
            "ADV-CAMPAIGN-ARTIFACT-SCHEMA",
        ),
        (CampaignError::BoundInput, "ADV-CAMPAIGN-BOUND-INPUT"),
        (CampaignError::SourceStale, "ADV-CAMPAIGN-SOURCE-STALE"),
        (
            CampaignError::ConfigurationStale,
            "ADV-CAMPAIGN-CONFIGURATION-STALE",
        ),
        (CampaignError::ToolBinding, "ADV-CAMPAIGN-TOOL-STALE"),
        (CampaignError::Expired, "ADV-CAMPAIGN-EXPIRED"),
        (CampaignError::CampaignFailed, "ADV-CAMPAIGN-FAILED"),
        (CampaignError::UnitSet, "ADV-CAMPAIGN-UNIT-SET"),
        (CampaignError::UnitFailed, "ADV-CAMPAIGN-UNIT-FAILED"),
        (CampaignError::Bounds, "ADV-CAMPAIGN-BOUNDS"),
        (CampaignError::MutationScore, "ADV-MUTATION-SCORE"),
        (
            CampaignError::SurvivorDisposition,
            "ADV-MUTATION-SURVIVOR-DISPOSITION",
        ),
        (
            CampaignError::DischargeCannotSatisfy,
            "ADV-CAMPAIGN-DISCHARGE-NONPASSING",
        ),
        (CampaignError::NamedMutants, "ADV-MUTATION-NAMED-CATALOG"),
        (CampaignError::Platform, "ADV-PLATFORM-UNKNOWN"),
        (CampaignError::Corpus, "ADV-PLATFORM-CORPUS"),
        (
            CampaignError::CorpusCanonical,
            "ADV-PLATFORM-CORPUS-CANONICAL",
        ),
        (CampaignError::CorpusPath, "ADV-PLATFORM-CORPUS-PATH"),
        (
            CampaignError::CorpusConfiguration,
            "ADV-PLATFORM-CORPUS-CONFIGURATION",
        ),
        (
            CampaignError::CorpusGovernance,
            "ADV-PLATFORM-CORPUS-GOVERNANCE",
        ),
        (CampaignError::CorpusReport, "ADV-PLATFORM-CORPUS-REPORT"),
    ];
    for (error, code) in cases {
        assert_eq!(error.code(), code);
    }

    let (policy, artifact, source, configuration) = policy_and_artifact(CampaignKind::Fuzz);
    for date in [
        "1969-12-31",
        "2026-00-01",
        "2026-13-01",
        "2026-02-29",
        "2024-02-30",
        "2026-04-31",
        "2026/07/22",
        "2026-07/22",
        "202x-07-22",
        "2026-0x-22",
        "2026-07-2x",
    ] {
        assert_document_error(
            &policy,
            &artifact,
            &source,
            &configuration,
            date,
            CampaignError::ArtifactSchema,
        );
    }
    for date in ["2024-02-29", "2000-02-29", "2026-01-01", "2026-02-01"] {
        let dated = set_toml(
            &set_toml(
                &artifact,
                "generated_on",
                toml::Value::String(date.to_owned()),
            ),
            "expires_on",
            toml::Value::String("2026-08-21".to_owned()),
        );
        let result = validate_campaign_documents(&policy, &dated, &source, &configuration, date);
        assert!(matches!(result, Ok(()) | Err(CampaignError::Expired)));
    }
    assert_eq!(
        platform_effect_diagnostics("other"),
        Err(CampaignError::Platform)
    );
}

fn write_fuzz_policy_root(root: &Path, policy: &str) {
    let directory = root.join("assurance/adversarial/v1");
    fs::create_dir_all(&directory).expect("campaign control directory");
    fs::write(directory.join("fuzz-policy.toml"), policy).expect("campaign control policy");
}

fn bounded_fuzz_policy(source_paths: Vec<&str>, configuration_paths: Vec<&str>) -> String {
    let (policy, _, _, _) = policy_and_artifact(CampaignKind::Fuzz);
    set_toml(
        &set_toml(
            &policy,
            "source_paths",
            toml::Value::Array(
                source_paths
                    .into_iter()
                    .map(|path| toml::Value::String(path.to_owned()))
                    .collect(),
            ),
        ),
        "configuration_paths",
        toml::Value::Array(
            configuration_paths
                .into_iter()
                .map(|path| toml::Value::String(path.to_owned()))
                .collect(),
        ),
    )
}

#[test]
#[allow(clippy::too_many_lines)]
fn bounded_campaign_filesystem_failures_are_deterministic() {
    let missing = tempfile::tempdir().expect("missing-policy control");
    assert_eq!(
        campaign_bindings(missing.path(), CampaignKind::Fuzz),
        Err(CampaignError::PolicySyntax)
    );
    assert_eq!(
        validate_repository_campaign(missing.path(), CampaignKind::Fuzz, CAMPAIGN_AS_OF),
        Err(CampaignError::PolicySyntax)
    );

    let policy_only = tempfile::tempdir().expect("missing-artifact control");
    let (fuzz_policy, fuzz_artifact, _, _) = policy_and_artifact(CampaignKind::Fuzz);
    write_fuzz_policy_root(policy_only.path(), &fuzz_policy);
    assert_eq!(
        validate_repository_campaign(policy_only.path(), CampaignKind::Fuzz, CAMPAIGN_AS_OF),
        Err(CampaignError::ArtifactSyntax)
    );

    let malformed_repository = tempfile::tempdir().expect("repository policy control");
    write_fuzz_policy_root(malformed_repository.path(), "not TOML");
    fs::write(
        malformed_repository
            .path()
            .join("assurance/adversarial/v1/fuzz-campaign.toml"),
        fuzz_artifact,
    )
    .expect("repository artifact control");
    assert_eq!(
        validate_repository_campaign(
            malformed_repository.path(),
            CampaignKind::Fuzz,
            CAMPAIGN_AS_OF,
        ),
        Err(CampaignError::PolicySyntax)
    );

    let binding_failure = tempfile::tempdir().expect("repository binding control");
    write_fuzz_policy_root(
        binding_failure.path(),
        &bounded_fuzz_policy(vec!["missing-source"], vec!["missing-configuration"]),
    );
    let (_, artifact, _, _) = policy_and_artifact(CampaignKind::Fuzz);
    fs::write(
        binding_failure
            .path()
            .join("assurance/adversarial/v1/fuzz-campaign.toml"),
        artifact,
    )
    .expect("repository binding artifact");
    assert_eq!(
        validate_repository_campaign(binding_failure.path(), CampaignKind::Fuzz, CAMPAIGN_AS_OF),
        Err(CampaignError::BoundInput)
    );

    let mismatched_kind = tempfile::tempdir().expect("mismatched-kind control");
    let (mutation_policy, _, _, _) = policy_and_artifact(CampaignKind::Mutation);
    write_fuzz_policy_root(mismatched_kind.path(), &mutation_policy);
    assert_eq!(
        campaign_bindings(mismatched_kind.path(), CampaignKind::Fuzz),
        Err(CampaignError::PolicySchema)
    );

    let configuration_failure = tempfile::tempdir().expect("configuration-path control");
    fs::write(configuration_failure.path().join("source.txt"), "source")
        .expect("configuration-path source");
    write_fuzz_policy_root(
        configuration_failure.path(),
        &bounded_fuzz_policy(vec!["source.txt"], vec!["missing-configuration"]),
    );
    assert_eq!(
        campaign_bindings(configuration_failure.path(), CampaignKind::Fuzz),
        Err(CampaignError::BoundInput)
    );

    let malformed = tempfile::tempdir().expect("malformed-policy control");
    write_fuzz_policy_root(malformed.path(), "not TOML");
    assert_eq!(
        campaign_bindings(malformed.path(), CampaignKind::Fuzz),
        Err(CampaignError::PolicySyntax)
    );

    for paths in [vec!["../escape"], vec!["missing"], vec!["empty"]] {
        let fixture = tempfile::tempdir().expect("bounded-path control");
        if paths[0] == "empty" {
            fs::create_dir_all(fixture.path().join("empty")).expect("empty directory control");
        }
        fs::write(fixture.path().join("configuration.txt"), "configuration")
            .expect("bounded configuration");
        write_fuzz_policy_root(
            fixture.path(),
            &bounded_fuzz_policy(paths, vec!["configuration.txt"]),
        );
        assert_eq!(
            campaign_bindings(fixture.path(), CampaignKind::Fuzz),
            Err(CampaignError::BoundInput)
        );
    }

    let oversized = tempfile::tempdir().expect("oversized-path control");
    fs::write(oversized.path().join("configuration.txt"), "configuration")
        .expect("bounded configuration");
    let oversized_file =
        fs::File::create(oversized.path().join("oversized.bin")).expect("oversized sparse file");
    oversized_file
        .set_len(268_435_457)
        .expect("oversized sparse length");
    write_fuzz_policy_root(
        oversized.path(),
        &bounded_fuzz_policy(vec!["oversized.bin"], vec!["configuration.txt"]),
    );
    assert_eq!(
        campaign_bindings(oversized.path(), CampaignKind::Fuzz),
        Err(CampaignError::BoundInput)
    );

    #[cfg(feature = "coverage-controls")]
    {
        let count_bound = tempfile::tempdir().expect("file-count bound control");
        fs::create_dir_all(count_bound.path().join("source")).expect("count-bound source");
        fs::write(count_bound.path().join("source/one"), "one").expect("first bound file");
        fs::write(count_bound.path().join("source/two"), "two").expect("second bound file");
        assert_eq!(
            campaign_bound_control(count_bound.path(), &["source".to_owned()], 1, 268_435_456,),
            Err(CampaignError::BoundInput)
        );
        for fault in [
            "read-file",
            "read-directory",
            "read-entry",
            "file-digest",
            "set-digest",
        ] {
            assert_eq!(
                campaign_input_fault_control(count_bound.path(), &["source".to_owned()], fault),
                Err(CampaignError::BoundInput)
            );
        }
    }

    let overlapping = tempfile::tempdir().expect("overlapping-path control");
    fs::create_dir_all(overlapping.path().join("tree")).expect("tree directory");
    fs::write(overlapping.path().join("tree/file"), "source").expect("tree file");
    fs::write(
        overlapping.path().join("configuration.txt"),
        "configuration",
    )
    .expect("bounded configuration");
    write_fuzz_policy_root(
        overlapping.path(),
        &bounded_fuzz_policy(vec!["tree", "tree/file"], vec!["configuration.txt"]),
    );
    assert_eq!(
        campaign_bindings(overlapping.path(), CampaignKind::Fuzz),
        Err(CampaignError::BoundInput)
    );

    #[cfg(all(unix, feature = "coverage-controls"))]
    {
        use std::ffi::OsString;
        use std::os::unix::ffi::OsStringExt;
        use std::os::unix::fs::symlink;
        use std::os::unix::net::UnixListener;

        assert_eq!(
            repository_path_text_control(Path::new(&OsString::from_vec(vec![0xff]))),
            Err(CampaignError::BoundInput)
        );

        let symlinked = tempfile::tempdir().expect("symlink control");
        fs::write(symlinked.path().join("source.txt"), "source").expect("symlink source");
        symlink(
            symlinked.path().join("source.txt"),
            symlinked.path().join("source-link"),
        )
        .expect("source symlink");
        fs::write(symlinked.path().join("configuration.txt"), "configuration")
            .expect("bounded configuration");
        write_fuzz_policy_root(
            symlinked.path(),
            &bounded_fuzz_policy(vec!["source-link"], vec!["configuration.txt"]),
        );
        assert_eq!(
            campaign_bindings(symlinked.path(), CampaignKind::Fuzz),
            Err(CampaignError::BoundInput)
        );

        let recursive_symlink = tempfile::tempdir().expect("recursive symlink control");
        fs::create_dir_all(recursive_symlink.path().join("source"))
            .expect("recursive source directory");
        fs::write(recursive_symlink.path().join("target"), "source")
            .expect("recursive symlink target");
        symlink(
            recursive_symlink.path().join("target"),
            recursive_symlink.path().join("source/link"),
        )
        .expect("recursive source symlink");
        fs::write(
            recursive_symlink.path().join("configuration.txt"),
            "configuration",
        )
        .expect("bounded configuration");
        write_fuzz_policy_root(
            recursive_symlink.path(),
            &bounded_fuzz_policy(vec!["source"], vec!["configuration.txt"]),
        );
        assert_eq!(
            campaign_bindings(recursive_symlink.path(), CampaignKind::Fuzz),
            Err(CampaignError::BoundInput)
        );

        let nonportable_name = tempfile::tempdir().expect("nonportable-name control");
        fs::create_dir_all(nonportable_name.path().join("source"))
            .expect("nonportable source directory");
        fs::write(nonportable_name.path().join("source/bad\\name"), "source")
            .expect("nonportable source name");
        fs::write(
            nonportable_name.path().join("configuration.txt"),
            "configuration",
        )
        .expect("bounded configuration");
        write_fuzz_policy_root(
            nonportable_name.path(),
            &bounded_fuzz_policy(vec!["source"], vec!["configuration.txt"]),
        );
        assert_eq!(
            campaign_bindings(nonportable_name.path(), CampaignKind::Fuzz),
            Err(CampaignError::BoundInput)
        );

        let special = tempfile::tempdir().expect("special-file control");
        fs::write(special.path().join("configuration.txt"), "configuration")
            .expect("bounded configuration");
        let socket_path = special.path().join("source.sock");
        let _listener = UnixListener::bind(&socket_path).expect("unix socket control");
        write_fuzz_policy_root(
            special.path(),
            &bounded_fuzz_policy(vec!["source.sock"], vec!["configuration.txt"]),
        );
        assert_eq!(
            campaign_bindings(special.path(), CampaignKind::Fuzz),
            Err(CampaignError::BoundInput)
        );
    }
}

fn write_bound_campaign(root: &Path, kind: CampaignKind, policy: &str) {
    let stem = match kind {
        CampaignKind::Fuzz => "fuzz",
        CampaignKind::Mutation => "mutation",
    };
    let directory = root.join("assurance/adversarial/v1");
    fs::create_dir_all(&directory).expect("bounded campaign directory");
    fs::create_dir_all(root.join("bound")).expect("bounded input directory");
    fs::write(root.join("bound/source.txt"), "source").expect("bounded source");
    fs::write(root.join("bound/configuration.txt"), "configuration")
        .expect("bounded configuration");
    fs::write(directory.join(format!("{stem}-policy.toml")), policy)
        .expect("bounded campaign policy");
    let bindings = campaign_bindings(root, kind).expect("bounded campaign bindings");
    let (_, artifact, _, _) = policy_and_artifact(kind);
    let artifact = set_toml(
        &set_toml(
            &artifact,
            "source_digest",
            toml::Value::String(bindings.source_digest),
        ),
        "configuration_digest",
        toml::Value::String(bindings.configuration_digest),
    );
    fs::write(directory.join(format!("{stem}-campaign.toml")), artifact)
        .expect("bounded campaign artifact");
}

fn fuzz_layout(root: &Path, include_manifest: bool, include_targets: bool, corpus_cases: usize) {
    if !include_manifest {
        return;
    }
    let (policy, _, _, _) = policy_and_artifact(CampaignKind::Fuzz);
    let policy = toml::from_str::<toml::Value>(&policy).expect("fuzz layout policy");
    let units = policy["required_units"]
        .as_array()
        .expect("required fuzz units");
    fs::create_dir_all(root.join("fuzz/fuzz_targets")).expect("fuzz target directory");
    let mut manifest = String::new();
    for unit in units {
        let unit = unit.as_str().expect("required fuzz unit");
        writeln!(
            manifest,
            "[[bin]]\nname = \"{unit}\"\npath = \"fuzz_targets/{unit}.rs\""
        )
        .expect("string-backed fuzz manifest");
        if include_targets {
            fs::write(
                root.join(format!("fuzz/fuzz_targets/{unit}.rs")),
                "fn main() {}",
            )
            .expect("fuzz target");
            let corpus = root.join(format!("fuzz/corpus/{unit}"));
            fs::create_dir_all(&corpus).expect("fuzz corpus directory");
            for index in 0..corpus_cases {
                fs::write(corpus.join(format!("case-{index}")), index.to_string())
                    .expect("fuzz corpus case");
            }
        }
    }
    fs::write(root.join("fuzz/Cargo.toml"), manifest).expect("fuzz manifest");
}

fn materialize_named_catalog(root: &Path, catalog: &str) {
    let directory = root.join("assurance/adversarial/v1");
    fs::create_dir_all(&directory).expect("named catalog directory");
    fs::write(directory.join("named-mutants.tsv"), catalog).expect("named catalog fixture");
    for line in catalog.lines().skip(1) {
        let fields = line.split('\t').collect::<Vec<_>>();
        if fields.len() != 8 {
            continue;
        }
        let source_path = root.join(fields[3]);
        if let Some(parent) = source_path.parent() {
            fs::create_dir_all(parent).expect("named catalog source directory");
        }
        let mut source = fs::read_to_string(&source_path).unwrap_or_default();
        source.push_str(fields[4]);
        source.push('\n');
        fs::write(source_path, source).expect("named catalog source fixture");
    }
}

#[test]
#[allow(clippy::too_many_lines)]
fn repository_campaign_layout_checks_reach_each_failure_stage() {
    for (include_manifest, include_targets, corpus_cases, expected) in [
        (false, false, 0, Err(CampaignError::PolicySchema)),
        (true, false, 0, Err(CampaignError::PolicySchema)),
        (true, true, 0, Err(CampaignError::PolicySchema)),
        (true, true, 1, Err(CampaignError::PolicySchema)),
        (true, true, 2, Ok(())),
    ] {
        let fixture = tempfile::tempdir().expect("fuzz layout control");
        fuzz_layout(
            fixture.path(),
            include_manifest,
            include_targets,
            corpus_cases,
        );
        let policy = bounded_fuzz_policy(vec!["bound/source.txt"], vec!["bound/configuration.txt"]);
        write_bound_campaign(fixture.path(), CampaignKind::Fuzz, &policy);
        assert_eq!(
            validate_repository_campaign(fixture.path(), CampaignKind::Fuzz, CAMPAIGN_AS_OF),
            expected
        );
    }

    let missing_corpus = tempfile::tempdir().expect("fuzz corpus-read control");
    fuzz_layout(missing_corpus.path(), true, true, 2);
    fs::remove_dir_all(missing_corpus.path().join("fuzz/corpus/canonical_values"))
        .expect("remove fuzz corpus control");
    let policy = bounded_fuzz_policy(vec!["bound/source.txt"], vec!["bound/configuration.txt"]);
    write_bound_campaign(missing_corpus.path(), CampaignKind::Fuzz, &policy);
    assert_eq!(
        validate_repository_campaign(missing_corpus.path(), CampaignKind::Fuzz, CAMPAIGN_AS_OF),
        Err(CampaignError::PolicySchema)
    );

    let missing_name = tempfile::tempdir().expect("fuzz manifest-name control");
    fuzz_layout(missing_name.path(), true, true, 2);
    let manifest_path = missing_name.path().join("fuzz/Cargo.toml");
    let manifest = fs::read_to_string(&manifest_path).expect("fuzz manifest control");
    fs::write(
        &manifest_path,
        manifest.replacen("name = \"canonical_values\"", "name = \"other\"", 1),
    )
    .expect("fuzz manifest-name control");
    let policy = bounded_fuzz_policy(vec!["bound/source.txt"], vec!["bound/configuration.txt"]);
    write_bound_campaign(missing_name.path(), CampaignKind::Fuzz, &policy);
    assert_eq!(
        validate_repository_campaign(missing_name.path(), CampaignKind::Fuzz, CAMPAIGN_AS_OF),
        Err(CampaignError::PolicySchema)
    );

    let stale = tempfile::tempdir().expect("repository stale control");
    fuzz_layout(stale.path(), true, true, 2);
    let policy = bounded_fuzz_policy(vec!["bound/source.txt"], vec!["bound/configuration.txt"]);
    write_bound_campaign(stale.path(), CampaignKind::Fuzz, &policy);
    let artifact_path = stale
        .path()
        .join("assurance/adversarial/v1/fuzz-campaign.toml");
    let artifact = fs::read_to_string(&artifact_path).expect("bounded campaign artifact");
    fs::write(
        artifact_path,
        set_toml(
            &artifact,
            "source_digest",
            toml::Value::String("xxh3-128:00000000000000000000000000000000".to_owned()),
        ),
    )
    .expect("stale campaign artifact");
    assert_eq!(
        validate_repository_campaign(stale.path(), CampaignKind::Fuzz, CAMPAIGN_AS_OF),
        Err(CampaignError::SourceStale)
    );

    let (mutation_policy, _, _, _) = policy_and_artifact(CampaignKind::Mutation);
    let mutation_policy = set_toml(
        &set_toml(
            &mutation_policy,
            "source_paths",
            toml::Value::Array(vec![toml::Value::String("bound/source.txt".to_owned())]),
        ),
        "configuration_paths",
        toml::Value::Array(vec![toml::Value::String(
            "bound/configuration.txt".to_owned(),
        )]),
    );
    for (configuration, expected) in [
        (None, Err(CampaignError::PolicySchema)),
        (Some("paths = []\n"), Err(CampaignError::PolicySchema)),
        (
            Some(
                "paths = [\n  \"crates/buoy-core/src/value.rs\",\n  \"crates/buoy-core/src/algorithms.rs\",\n  \"crates/buoy-core/src/report.rs\",\n  \"src/repository.rs\",\n  \"src/oracle.rs\",\n]\nexclude_re = [\"skip\"]\n",
            ),
            Err(CampaignError::PolicySchema),
        ),
        (
            Some(
                "paths = [\n  \"crates/buoy-core/src/value.rs\",\n  \"crates/buoy-core/src/algorithms.rs\",\n  \"crates/buoy-core/src/report.rs\",\n  \"src/repository.rs\",\n  \"src/oracle.rs\",\n]\nexclude_re = []\nexclude_globs = [\n  \"crates/buoy-core/src/value.rs\",\n]\n",
            ),
            Err(CampaignError::PolicySchema),
        ),
        (
            Some(
                "paths = [\n  \"crates/buoy-core/src/value.rs\",\n  \"crates/buoy-core/src/algorithms.rs\",\n  \"crates/buoy-core/src/report.rs\",\n  \"src/repository.rs\",\n  \"src/oracle.rs\",\n]\nexclude_re = []\n",
            ),
            Ok(()),
        ),
    ] {
        let fixture = tempfile::tempdir().expect("mutation layout control");
        if let Some(configuration) = configuration {
            fs::create_dir_all(fixture.path().join(".cargo"))
                .expect("mutation configuration directory");
            fs::write(fixture.path().join(".cargo/mutants.toml"), configuration)
                .expect("mutation configuration");
        }
        let catalog = fs::read_to_string(root().join("assurance/adversarial/v1/named-mutants.tsv"))
            .expect("named catalog");
        materialize_named_catalog(fixture.path(), &catalog);
        write_bound_campaign(fixture.path(), CampaignKind::Mutation, &mutation_policy);
        assert_eq!(
            validate_repository_campaign(fixture.path(), CampaignKind::Mutation, CAMPAIGN_AS_OF),
            expected
        );
    }

    let named_failure = tempfile::tempdir().expect("repository named-catalog control");
    fs::create_dir_all(named_failure.path().join(".cargo"))
        .expect("mutation configuration directory");
    fs::write(
        named_failure.path().join(".cargo/mutants.toml"),
        "paths = [\n  \"crates/buoy-core/src/value.rs\",\n  \"crates/buoy-core/src/algorithms.rs\",\n  \"crates/buoy-core/src/report.rs\",\n  \"src/repository.rs\",\n  \"src/oracle.rs\",\n]\nexclude_re = []\n",
    )
    .expect("mutation configuration");
    write_bound_campaign(
        named_failure.path(),
        CampaignKind::Mutation,
        &mutation_policy,
    );
    fs::write(
        named_failure
            .path()
            .join("assurance/adversarial/v1/named-mutants.tsv"),
        "wrong\n",
    )
    .expect("invalid named catalog");
    assert_eq!(
        validate_repository_campaign(named_failure.path(), CampaignKind::Mutation, CAMPAIGN_AS_OF,),
        Err(CampaignError::NamedMutants)
    );
}

#[test]
fn named_mutant_catalog_rejects_each_closed_shape() {
    let (_, _, _, _) = policy_and_artifact(CampaignKind::Mutation);
    let catalog = fs::read_to_string(root().join("assurance/adversarial/v1/named-mutants.tsv"))
        .expect("named-mutant catalog");
    let header = catalog.lines().next().expect("catalog header");
    let first = catalog.lines().nth(1).expect("catalog row");

    let missing = tempfile::tempdir().expect("missing catalog control");
    assert_eq!(
        validate_named_mutant_catalog(missing.path()),
        Err(CampaignError::NamedMutants)
    );

    let fields = first.split('\t').collect::<Vec<_>>();
    let missing_required = catalog
        .lines()
        .filter(|line| !line.contains("\tartifact-freshness\t"))
        .collect::<Vec<_>>()
        .join("\n");
    for content in [
        "wrong\n".to_owned(),
        format!("{header}\nshort\trow\n"),
        format!(
            "{header}\n{}\n",
            first.replacen("buoy-named-mutant/v1", "wrong/v1", 1)
        ),
        format!("{header}\n{}\n", first.replacen(fields[4], "", 1)),
        format!(
            "{header}\n{}\n",
            first.replacen("\tfalse\tblocking", "\ttrue\tblocking", 1)
        ),
        format!(
            "{header}\n{}\n",
            first.replacen("\tblocking", "\tadvisory", 1)
        ),
        format!("{header}\n{}\n", first.replacen("cargo test ", "just ", 1)),
        format!("{catalog}{first}\n"),
        format!(
            "{catalog}{}\n",
            first.replacen("NAMED-ARTIFACT-FRESHNESS", "NAMED-DUPLICATE-CATEGORY", 1)
        ),
        missing_required,
    ] {
        let fixture = tempfile::tempdir().expect("catalog control");
        materialize_named_catalog(fixture.path(), &content);
        assert_eq!(
            validate_named_mutant_catalog(fixture.path()),
            Err(CampaignError::NamedMutants)
        );
    }

    for remove_source in [true, false] {
        let fixture = tempfile::tempdir().expect("catalog source control");
        materialize_named_catalog(fixture.path(), &catalog);
        let source_path = fixture.path().join(fields[3]);
        if remove_source {
            fs::remove_file(source_path).expect("remove named source");
        } else {
            fs::write(source_path, "different symbol").expect("replace named source");
        }
        assert_eq!(
            validate_named_mutant_catalog(fixture.path()),
            Err(CampaignError::NamedMutants)
        );
    }
}
