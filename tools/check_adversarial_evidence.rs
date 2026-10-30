#![doc = "Checks current adversarial campaign bindings and emits portable platform corpora."]
#![forbid(unsafe_code)]

#[cfg(feature = "coverage-controls")]
use buoy::adversarial::portable_corpus_control;
use buoy::adversarial::{
    CampaignError, CampaignKind, campaign_bindings, platform_effect_diagnostics, portable_corpus,
    validate_campaign_documents, validate_repository_campaign,
};
use std::env;
use std::fs;
use std::path::Path;

const FUZZ_POLICY: &str = "assurance/adversarial/v1/fuzz-policy.toml";
const FUZZ_ARTIFACT: &str = "assurance/adversarial/v1/fuzz-campaign.toml";
const MUTATION_POLICY: &str = "assurance/adversarial/v1/mutation-policy.toml";
const MUTATION_ARTIFACT: &str = "assurance/adversarial/v1/mutation-campaign.toml";

type CampaignDocuments = (String, String, (String, String));

fn main() {
    let arguments = env::args().skip(1).collect::<Vec<_>>();
    let result = match arguments.as_slice() {
        [flag] if flag == "--self-test" => self_test(Path::new(".")),
        #[cfg(feature = "coverage-controls")]
        [flag, root] if flag == "--self-test-root" => self_test(Path::new(root)),
        #[cfg(feature = "coverage-controls")]
        [flag] if flag == "--expect-error-control" => self_test_control(Path::new("."), true),
        #[cfg(feature = "coverage-controls")]
        [flag, fault]
            if flag == "--self-test-platform-result-fault"
                && [
                    "platform",
                    "bound-input",
                    "os",
                    "read",
                    "remove-file",
                    "create-dir",
                    "cleanup",
                ]
                .contains(&fault.as_str()) =>
        {
            self_test_with_fault(Path::new("."), Some(fault.as_str()))
        }
        #[cfg(feature = "coverage-controls")]
        [flag] if flag == "--mutation-document-control" => require_document_pair(
            documents(Path::new("."), CampaignKind::Fuzz),
            Err(CampaignError::ArtifactSyntax),
        )
        .map(|_| ()),
        #[cfg(feature = "coverage-controls")]
        [flag, kind, root] if flag == "--bindings-root" => {
            parse_kind(kind).and_then(|kind| print_bindings(Path::new(root), kind))
        }
        #[cfg(feature = "coverage-controls")]
        [flag, fault, output] if flag == "--platform-fault" => {
            write_platform_corpus_with_fault("linux", Path::new(output), Some(fault))
        }
        [flag, kind] if flag == "--bindings" => {
            parse_kind(kind).and_then(|kind| print_bindings(Path::new("."), kind))
        }
        [flag, as_of] if flag == "--check" => check(Path::new("."), as_of),
        [flag, platform, output] if flag == "--platform-corpus" => {
            write_platform_corpus(platform, Path::new(output))
        }
        [flag, output] if flag == "--platform-result" => write_platform_result(Path::new(output)),
        _ => {
            eprintln!(
                "usage: check-adversarial-evidence --self-test | --bindings <fuzz|mutation> | --check <YYYY-MM-DD> | --platform-corpus <linux|macos|windows> <output-directory> | --platform-result <output-root>"
            );
            std::process::exit(2);
        }
    };
    if let Err(error) = result {
        eprintln!("{}", error.code());
        std::process::exit(1);
    }
}

fn parse_kind(value: &str) -> Result<CampaignKind, CampaignError> {
    match value {
        "fuzz" => Ok(CampaignKind::Fuzz),
        "mutation" => Ok(CampaignKind::Mutation),
        _ => Err(CampaignError::PolicySchema),
    }
}

fn print_bindings(root: &Path, kind: CampaignKind) -> Result<(), CampaignError> {
    let bindings = campaign_bindings(root, kind)?;
    println!("source_digest={}", bindings.source_digest);
    println!("configuration_digest={}", bindings.configuration_digest);
    Ok(())
}

fn check(root: &Path, as_of: &str) -> Result<(), CampaignError> {
    let fuzz = validate_repository_campaign(root, CampaignKind::Fuzz, as_of);
    let mutation = validate_repository_campaign(root, CampaignKind::Mutation, as_of);
    if let Some(error) = [fuzz, mutation].into_iter().find_map(Result::err) {
        Err(error)
    } else {
        println!("adversarial-campaigns: current exact-scopes=2/2 as-of={as_of}");
        Ok(())
    }
}

fn write_platform_corpus(platform: &str, output: &Path) -> Result<(), CampaignError> {
    write_platform_corpus_with_fault(platform, output, None)
}

fn platform_for_os(os: &str) -> Result<&'static str, CampaignError> {
    match os {
        "linux" => Ok("linux"),
        "macos" => Ok("macos"),
        "windows" => Ok("windows"),
        _ => Err(CampaignError::Platform),
    }
}

fn write_platform_result(output_root: &Path) -> Result<(), CampaignError> {
    write_platform_result_for_os(env::consts::OS, output_root)
}

fn write_platform_result_for_os(os: &str, output_root: &Path) -> Result<(), CampaignError> {
    let platform = platform_for_os(os)?;
    let output = output_root.join(platform);
    write_platform_corpus(platform, &output)?;
    let result = format!(
        "schema = \"buoy-platform-result/v1\"\nplatform = \"{platform}\"\ncommand = \"devenv shell -- buoy-platform-result\"\nstatus = \"green\"\nprovenance = \"local-host-execution\"\nportable_corpus = \"portable-corpus.tsv\"\nplatform_effects = \"platform-effects.tsv\"\n"
    );
    fs::write(output.join("result.toml"), result).map_err(|_| CampaignError::BoundInput)?;
    println!("platform-result: platform={platform} status=green provenance=local-host-execution");
    Ok(())
}

fn write_platform_corpus_with_fault(
    platform: &str,
    output: &Path,
    fault: Option<&str>,
) -> Result<(), CampaignError> {
    #[cfg(feature = "coverage-controls")]
    let corpus_result = if fault == Some("corpus") {
        portable_corpus_control("canonical-decode")
    } else {
        portable_corpus()
    };
    #[cfg(not(feature = "coverage-controls"))]
    let corpus_result = portable_corpus();
    let corpus = corpus_result?;
    let effects_platform = if fault == Some("effects") {
        "invalid"
    } else {
        platform
    };
    let effects = platform_effect_diagnostics(effects_platform)?;
    fs::create_dir_all(output).map_err(|_| CampaignError::BoundInput)?;
    fs::write(output.join("portable-corpus.tsv"), corpus).map_err(|_| CampaignError::BoundInput)?;
    fs::write(output.join("platform-effects.tsv"), effects)
        .map_err(|_| CampaignError::BoundInput)?;
    println!("platform-corpus: platform={platform} portable=6 effects=2");
    Ok(())
}

fn self_test(root: &Path) -> Result<(), CampaignError> {
    self_test_with_fault(root, None)
}

#[allow(clippy::expect_used)]
fn self_test_with_fault(root: &Path, fault: Option<&str>) -> Result<(), CampaignError> {
    let output = root.join("target/adversarial-platform-result-self-test");
    let _ = fs::remove_dir_all(&output);
    let os = if fault == Some("os") {
        "invalid"
    } else {
        "macos"
    };
    write_platform_result_for_os(os, &output)?;
    let result_path = output.join("macos/result.toml");
    if fault == Some("read") {
        fs::remove_file(&result_path).expect("read fault control removes result file");
    }
    let result = fs::read_to_string(&result_path).map_err(|_| CampaignError::BoundInput)?;
    let platform_controls = [
        fault != Some("platform"),
        result.contains("platform = \"macos\""),
        result.contains("provenance = \"local-host-execution\""),
        platform_for_os("linux") == Ok("linux"),
        platform_for_os("macos") == Ok("macos"),
        platform_for_os("windows") == Ok("windows"),
        platform_for_os("unsupported") == Err(CampaignError::Platform),
    ];
    if !platform_controls.iter().all(|passed| *passed) {
        return Err(CampaignError::Platform);
    }
    if fault == Some("remove-file") {
        fs::remove_file(&result_path).expect("remove-file fault control removes result file");
    }
    fs::remove_file(&result_path).map_err(|_| CampaignError::BoundInput)?;
    if fault == Some("create-dir") {
        fs::create_dir(&result_path).expect("create-dir fault control creates result path");
    }
    fs::create_dir(&result_path).map_err(|_| CampaignError::BoundInput)?;
    let bound_input_controls = [
        fault != Some("bound-input"),
        write_platform_result_for_os("macos", &output) == Err(CampaignError::BoundInput),
    ];
    if !bound_input_controls.iter().all(|passed| *passed) {
        return Err(CampaignError::BoundInput);
    }
    if fault == Some("cleanup") {
        fs::remove_dir_all(&output).expect("cleanup fault control removes output directory");
    }
    fs::remove_dir_all(output).map_err(|_| CampaignError::BoundInput)?;
    self_test_control(root, false)
}

fn self_test_dates(artifact: &mut String) -> Result<(), CampaignError> {
    let mut document =
        toml::from_str::<toml::Value>(artifact).map_err(|_| CampaignError::ArtifactSyntax)?;
    let fields = document
        .as_table_mut()
        .ok_or(CampaignError::ArtifactSchema)?;
    // Fixed copied dates isolate controls from real campaign renewal.
    fields.insert(
        "generated_on".to_owned(),
        toml::Value::String("2026-08-31".to_owned()),
    );
    fields.insert(
        "expires_on".to_owned(),
        toml::Value::String("2026-09-27".to_owned()),
    );
    *artifact = toml::to_string(&document).map_err(|_| CampaignError::ArtifactSyntax)?;
    Ok(())
}

#[allow(clippy::too_many_lines)]
fn self_test_control(root: &Path, force_failure: bool) -> Result<(), CampaignError> {
    let (
        (fuzz_policy, mut fuzz_artifact, fuzz_bindings),
        (mutation_policy, mut mutation_artifact, mutation_bindings),
    ) = require_document_pair(
        documents(root, CampaignKind::Fuzz),
        documents(root, CampaignKind::Mutation),
    )?;
    self_test_dates(&mut fuzz_artifact)?;
    self_test_dates(&mut mutation_artifact)?;
    let discharged = format!(
        "{fuzz_artifact}\n[[discharges]]\nid = \"SELF-TEST\"\nscope = \"all\"\nrationale = \"aggregate green\"\nreview = \"reviews/self-test.md\"\ncounts_as_satisfied = true\n"
    );
    let controls = [
        validate_campaign_documents(
            &fuzz_policy,
            &fuzz_artifact,
            &fuzz_bindings.0,
            &fuzz_bindings.1,
            "2026-08-31",
        )
        .is_ok(),
        expect_error(
            validate_campaign_documents(
                &fuzz_policy,
                &fuzz_artifact.replacen(
                    &fuzz_bindings.0,
                    "xxh3-128:00000000000000000000000000000000",
                    1,
                ),
                &fuzz_bindings.0,
                &fuzz_bindings.1,
                "2026-08-31",
            ),
            CampaignError::SourceStale,
        ),
        expect_error(
            validate_campaign_documents(
                &fuzz_policy,
                &fuzz_artifact.replacen(
                    &fuzz_bindings.1,
                    "xxh3-128:11111111111111111111111111111111",
                    1,
                ),
                &fuzz_bindings.0,
                &fuzz_bindings.1,
                "2026-08-31",
            ),
            CampaignError::ConfigurationStale,
        ),
        expect_error(
            validate_campaign_documents(
                &fuzz_policy,
                &fuzz_artifact.replacen(
                    "tool_version = \"0.13.2\"",
                    "tool_version = \"latest\"",
                    1,
                ),
                &fuzz_bindings.0,
                &fuzz_bindings.1,
                "2026-08-31",
            ),
            CampaignError::ToolBinding,
        ),
        expect_error(
            validate_campaign_documents(
                &fuzz_policy,
                &fuzz_artifact,
                &fuzz_bindings.0,
                &fuzz_bindings.1,
                "2026-10-01",
            ),
            CampaignError::Expired,
        ),
        expect_error(
            validate_campaign_documents(
                &fuzz_policy,
                &fuzz_artifact.replacen("result = \"pass\"", "result = \"fail\"", 1),
                &fuzz_bindings.0,
                &fuzz_bindings.1,
                "2026-08-31",
            ),
            CampaignError::CampaignFailed,
        ),
        expect_error(
            validate_campaign_documents(
                &fuzz_policy,
                &discharged,
                &fuzz_bindings.0,
                &fuzz_bindings.1,
                "2026-08-31",
            ),
            CampaignError::DischargeCannotSatisfy,
        ),
        validate_campaign_documents(
            &mutation_policy,
            &mutation_artifact,
            &mutation_bindings.0,
            &mutation_bindings.1,
            "2026-08-31",
        )
        .is_ok(),
        expect_error(
            validate_campaign_documents(
                &mutation_policy,
                &mutation_artifact.replacen(
                    "score_basis_points = 10000",
                    "score_basis_points = 8999",
                    1,
                ),
                &mutation_bindings.0,
                &mutation_bindings.1,
                "2026-08-31",
            ),
            CampaignError::MutationScore,
        ),
        !force_failure,
    ];
    require_controls(&controls)?;
    println!("adversarial-evidence-self-test: negative-controls=7/7");
    Ok(())
}

fn require_document_pair(
    fuzz: Result<CampaignDocuments, CampaignError>,
    mutation: Result<CampaignDocuments, CampaignError>,
) -> Result<(CampaignDocuments, CampaignDocuments), CampaignError> {
    match (fuzz, mutation) {
        (Ok(fuzz), Ok(mutation)) => Ok((fuzz, mutation)),
        (Err(error), _) | (Ok(_), Err(error)) => Err(error),
    }
}

fn documents(root: &Path, kind: CampaignKind) -> Result<CampaignDocuments, CampaignError> {
    let (policy_path, artifact_path) = match kind {
        CampaignKind::Fuzz => (FUZZ_POLICY, FUZZ_ARTIFACT),
        CampaignKind::Mutation => (MUTATION_POLICY, MUTATION_ARTIFACT),
    };
    let policy =
        fs::read_to_string(root.join(policy_path)).map_err(|_| CampaignError::PolicySyntax)?;
    let artifact =
        fs::read_to_string(root.join(artifact_path)).map_err(|_| CampaignError::ArtifactSyntax)?;
    let bindings = campaign_bindings(root, kind)?;
    Ok((
        policy,
        artifact,
        (bindings.source_digest, bindings.configuration_digest),
    ))
}

fn expect_error(result: Result<(), CampaignError>, expected: CampaignError) -> bool {
    result == Err(expected)
}

fn require_controls(controls: &[bool]) -> Result<(), CampaignError> {
    if controls.iter().all(|passed| *passed) {
        Ok(())
    } else {
        Err(CampaignError::ArtifactSchema)
    }
}
