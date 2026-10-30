//! Generate and stale-check the focused LLVM coverage fixture suite.

#![allow(clippy::format_push_string, clippy::too_many_lines)]

use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

#[path = "digest_binding.rs"]
mod content_digest;

const ROOT: &str = "fixtures/coverage/v1";
const INVENTORY_HEADER: &str =
    "target_id\tpath\tkind\tplatform\tcoverage_class\tdisposition\trationale\n";
const RESULT_HEADER: &str = "schema\ttarget_id\tpath\tsource_digest\tplatform\tlines_covered\tlines_count\tregions_covered\tregions_count\tfunctions_covered\tfunctions_count\tbranches_covered\tbranches_count\ttool_digest\tconfiguration_digest\tinventory_digest\texport_digest\tregistration\tfreshness\texecution\tassurance_class\n";
const DIGEST_A: &str = "sha-256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const DIGEST_B: &str = "sha-256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
const DIGEST_C: &str = "sha-256:cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc";
const DIGEST_D: &str = "sha-256:dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd";

type GenerateResult<T> = Result<T, String>;

fn main() {
    if let Err(error) = run() {
        eprintln!("coverage-fixtures: {error}");
        std::process::exit(1);
    }
}

fn run() -> GenerateResult<()> {
    let arguments = env::args().skip(1).collect::<Vec<_>>();
    let check = match arguments.as_slice() {
        [] => false,
        [argument] if argument == "--check" => true,
        _ => {
            return Err(String::from("usage: generate-coverage-fixtures [--check]"));
        }
    };
    let files = generated_files();
    if check {
        check_files(&files)?;
        println!("coverage-fixtures: current files={}", files.len());
    } else {
        write_files(&files)?;
        println!("coverage-fixtures: generated files={}", files.len());
    }
    Ok(())
}

fn generated_files() -> BTreeMap<String, String> {
    let mut files = BTreeMap::new();
    add_sources(&mut files);
    add_inventories(&mut files);
    add_exports(&mut files);
    add_metadata(&mut files);
    add_results(&mut files);
    add_manifest(&mut files);
    files
}

fn add_sources(files: &mut BTreeMap<String, String>) {
    for (path, source) in [
        (
            "repository/src/covered.rs",
            "pub fn covered(value: bool) -> u8 { if value { 1 } else { 2 } }\n",
        ),
        (
            "repository/tools/validator.rs",
            "fn main() { println!(\"validator\"); }\n",
        ),
        (
            "repository/dylint/src/lib.rs",
            "pub fn register_lints() { let _registered = true; }\n",
        ),
        (
            "repository/generated.rs",
            "pub fn generated_lookup(value: u8) -> bool { value == 1 }\n",
        ),
        (
            "repository/platform.rs",
            "pub fn platform_value(value: bool) -> u8 { if value { 3 } else { 4 } }\n",
        ),
        (
            "repository/declarations.rs",
            "pub struct DeclarationOnly;\n",
        ),
        (
            "repository/duplicate.rs",
            "pub fn duplicate_fixture() -> bool { true }\n",
        ),
        (
            "repository/windows.rs",
            "pub fn windows_value() -> u8 { 5 }\n",
        ),
    ] {
        files.insert(path.to_owned(), source.to_owned());
    }
}

fn inventory_row(
    id: &str,
    path: &str,
    kind: &str,
    platform: &str,
    class: &str,
    disposition: &str,
    rationale: &str,
) -> String {
    format!("{id}\t{path}\t{kind}\t{platform}\t{class}\t{disposition}\t{rationale}\n")
}

fn positive_inventory() -> String {
    let mut inventory = String::from(INVENTORY_HEADER);
    for row in [
        inventory_row(
            "covered-source",
            "src/covered.rs",
            "production-library",
            "portable",
            "cargo-llvm-cov",
            "included",
            "workspace source",
        ),
        inventory_row(
            "dylint-source",
            "dylint/src/lib.rs",
            "local-dylint-detector",
            "portable",
            "separate-manifest-cargo-llvm-cov",
            "included-assurance",
            "separate manifest source",
        ),
        inventory_row(
            "generated-source",
            "generated.rs",
            "generated-executable-source",
            "portable",
            "cargo-llvm-cov",
            "included-assurance",
            "generated executable source",
        ),
        inventory_row(
            "platform-source",
            "platform.rs",
            "production-platform-backend",
            "macos",
            "aggregate-platform-cargo-llvm-cov",
            "included",
            "required platform source",
        ),
        inventory_row(
            "standalone-validator",
            "tools/validator.rs",
            "standalone-validator",
            "portable",
            "cargo-harness-llvm-cov",
            "included-assurance",
            "Cargo reachable validator",
        ),
    ] {
        inventory.push_str(&row);
    }
    inventory
}

fn add_inventories(files: &mut BTreeMap<String, String>) {
    let positive = positive_inventory();
    files.insert("inventories/positive.tsv".into(), positive.clone());

    let mut declarations = String::from(INVENTORY_HEADER);
    declarations.push_str(&inventory_row(
        "declarations-only",
        "declarations.rs",
        "generated-production-data",
        "portable",
        "no-executable-regions",
        "not-applicable",
        "declarations only",
    ));
    files.insert(
        "inventories/positive-not-applicable.tsv".into(),
        declarations,
    );

    let first = positive.lines().nth(1).unwrap_or_default();
    let mut duplicate_id = positive.clone();
    duplicate_id.push_str(&inventory_row(
        "covered-source",
        "duplicate.rs",
        "production-library",
        "portable",
        "cargo-llvm-cov",
        "included",
        "duplicate stable ID",
    ));
    files.insert("inventories/negative-duplicate-id.tsv".into(), duplicate_id);

    let mut duplicate_path = positive.clone();
    duplicate_path.push_str(&format!(
        "{}\n",
        first.replacen("covered-source", "duplicate-path", 1)
    ));
    files.insert(
        "inventories/negative-duplicate-path.tsv".into(),
        duplicate_path,
    );

    files.insert(
        "inventories/negative-unknown-class.tsv".into(),
        positive.replacen("cargo-llvm-cov", "ambient-coverage", 1),
    );
    files.insert(
        "inventories/negative-broad-exclusion.tsv".into(),
        positive.replacen("cargo-llvm-cov\tincluded", "cargo-llvm-cov\texcluded", 1),
    );
    files.insert(
        "inventories/negative-missing-platform.tsv".into(),
        positive.replacen("\tportable\tcargo-llvm-cov", "\t\tcargo-llvm-cov", 1),
    );

    let mut invalid_disposition = String::from(INVENTORY_HEADER);
    invalid_disposition.push_str(&inventory_row(
        "invalid-disposition",
        "src/covered.rs",
        "production-library",
        "portable",
        "no-executable-regions",
        "not-applicable",
        "incorrect executable exclusion",
    ));
    files.insert(
        "inventories/negative-invalid-disposition.tsv".into(),
        invalid_disposition,
    );

    let mut missing_platform = positive;
    missing_platform.push_str(&inventory_row(
        "windows-required",
        "windows.rs",
        "production-platform-backend",
        "windows",
        "aggregate-platform-cargo-llvm-cov",
        "included",
        "required Windows artifact",
    ));
    files.insert(
        "inventories/negative-required-platform.tsv".into(),
        missing_platform,
    );
}

fn metric(count: u64, covered: u64) -> Value {
    json!({"count": count, "covered": covered, "percent": 100.0})
}

fn file(path: &str) -> Value {
    json!({
        "filename": path,
        "segments": [[1, 1, 1, true, true, false], [1, 20, 0, false, false, false]],
        "summary": {
            "lines": metric(2, 2),
            "regions": metric(2, 2),
            "functions": metric(1, 1),
            "branches": metric(2, 2)
        }
    })
}

fn base_export() -> Value {
    let paths = [
        "src/covered.rs",
        "dylint/src/lib.rs",
        "generated.rs",
        "platform.rs",
        "tools/validator.rs",
    ];
    json!({
        "data": [{
            "files": paths.into_iter().map(file).collect::<Vec<_>>(),
            "functions": [],
            "totals": {
                "lines": metric(10, 10),
                "regions": metric(10, 10),
                "functions": metric(5, 5),
                "branches": metric(10, 10)
            }
        }],
        "type": "llvm.coverage.json.export",
        "version": "2.0.1"
    })
}

fn pretty(value: &Value) -> String {
    format!("{value}\n")
}

fn metrics_without(omitted: &str, count: u64) -> Value {
    let mut result = serde_json::Map::new();
    for metric_name in ["lines", "regions", "functions", "branches"] {
        if metric_name != omitted {
            result.insert(metric_name.to_owned(), metric(count, count));
        }
    }
    Value::Object(result)
}

fn add_exports(files: &mut BTreeMap<String, String>) {
    let base = base_export();
    files.insert("exports/positive.json".into(), pretty(&base));

    let mut missing = base.clone();
    missing["data"][0]["files"] = Value::Array(
        [
            "dylint/src/lib.rs",
            "generated.rs",
            "platform.rs",
            "tools/validator.rs",
        ]
        .into_iter()
        .map(file)
        .collect(),
    );
    files.insert(
        "exports/negative-missing-source.json".into(),
        pretty(&missing),
    );

    for metric_name in ["lines", "regions", "functions", "branches"] {
        let mut uncovered = base.clone();
        uncovered["data"][0]["totals"][metric_name]["covered"] = json!(9);
        uncovered["data"][0]["files"][0]["summary"][metric_name]["covered"] = json!(0);
        files.insert(
            format!("exports/negative-uncovered-{metric_name}.json"),
            pretty(&uncovered),
        );

        let mut zero = base.clone();
        zero["data"][0]["totals"][metric_name] = metric(0, 0);
        zero["data"][0]["files"][0]["summary"][metric_name] = metric(0, 0);
        files.insert(
            format!("exports/negative-zero-{metric_name}.json"),
            pretty(&zero),
        );

        let mut absent = base.clone();
        absent["data"][0]["totals"] = metrics_without(metric_name, 10);
        absent["data"][0]["files"][0]["summary"] = metrics_without(metric_name, 2);
        files.insert(
            format!("exports/negative-absent-{metric_name}.json"),
            pretty(&absent),
        );
    }

    let mut duplicate = base;
    duplicate["data"][0]["files"][0]["segments"] = json!([
        [1, 1, 1, true, true, false],
        [1, 20, 0, false, false, false],
        [1, 1, 1, true, true, false]
    ]);
    files.insert(
        "exports/negative-duplicate-region.json".into(),
        pretty(&duplicate),
    );
}

fn metadata(tool: &str, configuration: &str, platform: &str, inventory: &str) -> String {
    format!(
        "schema = \"buoy-coverage-export-binding/v1\"\nexport_id = \"synthetic-export\"\ntool_id = \"cargo-llvm-cov\"\ntool_version = \"0.8.5\"\ntool_digest = \"{tool}\"\nconfiguration_digest = \"{configuration}\"\nplatform = \"{platform}\"\ninventory_digest = \"{inventory}\"\nexecution = \"pass\"\n"
    )
}

fn add_metadata(files: &mut BTreeMap<String, String>) {
    let inventory_digest = content_digest::bytes_binding(
        files
            .get("inventories/positive.tsv")
            .map(String::as_bytes)
            .unwrap_or_default(),
    );
    files.insert(
        "metadata/positive.toml".into(),
        metadata(DIGEST_A, DIGEST_B, "portable;macos", &inventory_digest),
    );
    files.insert(
        "metadata/negative-tool.toml".into(),
        metadata(DIGEST_D, DIGEST_B, "portable;macos", &inventory_digest),
    );
    files.insert(
        "metadata/negative-configuration.toml".into(),
        metadata(DIGEST_A, DIGEST_D, "portable;macos", &inventory_digest),
    );
    files.insert(
        "metadata/negative-platform.toml".into(),
        metadata(DIGEST_A, DIGEST_B, "portable", &inventory_digest),
    );
    files.insert(
        "metadata/negative-stale.toml".into(),
        metadata(DIGEST_A, DIGEST_B, "portable;macos", DIGEST_D),
    );
}

fn result_row(
    target_id: &str,
    path: &str,
    source_digest: &str,
    platform: &str,
    registration: &str,
    freshness: &str,
    execution: &str,
) -> String {
    format!(
        "buoy-coverage-result/v1\t{target_id}\t{path}\t{source_digest}\t{platform}\t2\t2\t2\t2\t1\t1\t2\t2\t{DIGEST_A}\t{DIGEST_B}\t{DIGEST_C}\t{DIGEST_D}\t{registration}\t{freshness}\t{execution}\tstructural-source-execution\n"
    )
}

fn add_results(files: &mut BTreeMap<String, String>) {
    let rows = [
        ("covered-source", "src/covered.rs", "portable", 'e'),
        ("dylint-source", "dylint/src/lib.rs", "portable", 'f'),
        ("generated-source", "generated.rs", "portable", '1'),
        ("platform-source", "platform.rs", "macos", '2'),
        (
            "standalone-validator",
            "tools/validator.rs",
            "portable",
            '3',
        ),
    ]
    .into_iter()
    .map(|(target_id, path, platform, digest_character)| {
        result_row(
            target_id,
            path,
            &format!("sha-256:{}", digest_character.to_string().repeat(64)),
            platform,
            "present",
            "current",
            "pass",
        )
    })
    .collect::<String>();
    let positive = format!("{RESULT_HEADER}{rows}");
    files.insert("results/positive.tsv".into(), positive.clone());
    files.insert(
        "results/negative-absolute-path.tsv".into(),
        positive.replacen("src/covered.rs", "/tmp/covered.rs", 1),
    );
    files.insert(
        "results/negative-store-path.tsv".into(),
        positive.replacen("src/covered.rs", "/nix/store/host/covered.rs", 1),
    );
    files.insert(
        "results/negative-timestamp.tsv".into(),
        format!(
            "{}\ttimestamp\n{}\t2026-07-19T00:00:00Z\n",
            RESULT_HEADER.trim_end(),
            positive.lines().nth(1).unwrap_or_default()
        ),
    );
    files.insert(
        "results/negative-wrong-digest.tsv".into(),
        positive.replacen(DIGEST_A, "sha-256:bad", 1),
    );
    let mut reversed_rows = positive.lines().skip(1).collect::<Vec<_>>();
    reversed_rows.reverse();
    let reversed_rows = reversed_rows.join("\n");
    files.insert(
        "results/negative-unsorted.tsv".into(),
        format!("{RESULT_HEADER}{reversed_rows}\n"),
    );
    files.insert(
        "results/negative-rounded-percent.tsv".into(),
        format!(
            "{}\tpercent\n{}\t100.0\n",
            RESULT_HEADER.trim_end(),
            positive.lines().nth(1).unwrap_or_default()
        ),
    );
    files.insert(
        "results/negative-historical-current.tsv".into(),
        positive.replacen("\tcurrent\tpass\t", "\tstale\tpass\t", 1),
    );
}

fn add_manifest(files: &mut BTreeMap<String, String>) {
    let mut manifest = String::from(
        "fixture_id\tkind\tpolarity\tinventory\texport\tmetadata\tresult\texpected_diagnostic\texpected_count\n",
    );
    let positive_inventory = "inventories/positive.tsv";
    let positive_export = "exports/positive.json";
    let positive_metadata = "metadata/positive.toml";
    let mut row = |id: &str,
                   kind: &str,
                   polarity: &str,
                   inventory: &str,
                   export: &str,
                   metadata: &str,
                   result: &str,
                   diagnostic: &str,
                   count: u8| {
        manifest.push_str(&format!(
            "{id}\t{kind}\t{polarity}\t{inventory}\t{export}\t{metadata}\t{result}\t{diagnostic}\t{count}\n"
        ));
    };
    row(
        "coverage-positive",
        "export",
        "positive",
        positive_inventory,
        positive_export,
        positive_metadata,
        "none",
        "none",
        0,
    );
    row(
        "coverage-missing-source",
        "export",
        "negative",
        positive_inventory,
        "exports/negative-missing-source.json",
        positive_metadata,
        "none",
        "LCOV-SOURCE-MISSING",
        1,
    );
    row(
        "coverage-invalid-disposition",
        "export",
        "negative",
        "inventories/negative-invalid-disposition.tsv",
        positive_export,
        positive_metadata,
        "none",
        "LCOV-INVALID-DISPOSITION",
        1,
    );
    for metric_name in ["lines", "regions", "functions", "branches"] {
        row(
            &format!("coverage-uncovered-{metric_name}"),
            "export",
            "negative",
            positive_inventory,
            &format!("exports/negative-uncovered-{metric_name}.json"),
            positive_metadata,
            "none",
            "LCOV-METRIC-NOT-EXACT",
            2,
        );
        row(
            &format!("coverage-zero-{metric_name}"),
            "export",
            "negative",
            positive_inventory,
            &format!("exports/negative-zero-{metric_name}.json"),
            positive_metadata,
            "none",
            "LCOV-ZERO-DENOMINATOR",
            1,
        );
        row(
            &format!("coverage-absent-{metric_name}"),
            "export",
            "negative",
            positive_inventory,
            &format!("exports/negative-absent-{metric_name}.json"),
            positive_metadata,
            "none",
            "LCOV-METRIC-MISSING",
            2,
        );
    }
    for (id, metadata, diagnostic) in [
        (
            "coverage-tool-identity",
            "metadata/negative-tool.toml",
            "LCOV-TOOL-IDENTITY",
        ),
        (
            "coverage-configuration-identity",
            "metadata/negative-configuration.toml",
            "LCOV-CONFIGURATION-IDENTITY",
        ),
        (
            "coverage-platform-identity",
            "metadata/negative-platform.toml",
            "LCOV-PLATFORM-IDENTITY",
        ),
        (
            "coverage-stale-binding",
            "metadata/negative-stale.toml",
            "LCOV-EVIDENCE-STALE",
        ),
    ] {
        row(
            id,
            "export",
            "negative",
            positive_inventory,
            positive_export,
            metadata,
            "none",
            diagnostic,
            1,
        );
    }
    row(
        "coverage-duplicate-region",
        "export",
        "negative",
        positive_inventory,
        "exports/negative-duplicate-region.json",
        positive_metadata,
        "none",
        "LCOV-DUPLICATE-REGION",
        1,
    );

    for (id, path, diagnostic) in [
        (
            "inventory-positive-not-applicable",
            "inventories/positive-not-applicable.tsv",
            "none",
        ),
        (
            "inventory-duplicate-id",
            "inventories/negative-duplicate-id.tsv",
            "LCOV-INVENTORY-DUPLICATE-ID",
        ),
        (
            "inventory-duplicate-path",
            "inventories/negative-duplicate-path.tsv",
            "LCOV-INVENTORY-DUPLICATE-PATH",
        ),
        (
            "inventory-unknown-class",
            "inventories/negative-unknown-class.tsv",
            "LCOV-INSTRUMENTATION-CLASS",
        ),
        (
            "inventory-broad-exclusion",
            "inventories/negative-broad-exclusion.tsv",
            "LCOV-BROAD-EXCLUSION",
        ),
        (
            "inventory-missing-platform",
            "inventories/negative-missing-platform.tsv",
            "LCOV-PLATFORM-MISSING",
        ),
        (
            "inventory-required-platform",
            "inventories/negative-required-platform.tsv",
            "LCOV-PLATFORM-MISSING",
        ),
    ] {
        row(
            id,
            "inventory",
            if diagnostic == "none" {
                "positive"
            } else {
                "negative"
            },
            path,
            positive_export,
            positive_metadata,
            "none",
            diagnostic,
            u8::from(diagnostic != "none"),
        );
    }

    for (id, path, diagnostic) in [
        ("result-positive", "results/positive.tsv", "none"),
        (
            "result-absolute-path",
            "results/negative-absolute-path.tsv",
            "LCOV-RESULT-HOST-FIELD",
        ),
        (
            "result-store-path",
            "results/negative-store-path.tsv",
            "LCOV-RESULT-HOST-FIELD",
        ),
        (
            "result-timestamp",
            "results/negative-timestamp.tsv",
            "LCOV-RESULT-HOST-FIELD",
        ),
        (
            "result-wrong-digest",
            "results/negative-wrong-digest.tsv",
            "LCOV-RESULT-DIGEST",
        ),
        (
            "result-unsorted",
            "results/negative-unsorted.tsv",
            "LCOV-RESULT-ORDER",
        ),
        (
            "result-rounded-percent",
            "results/negative-rounded-percent.tsv",
            "LCOV-RESULT-ROUNDED",
        ),
        (
            "result-historical-current",
            "results/negative-historical-current.tsv",
            "LCOV-EVIDENCE-STALE",
        ),
    ] {
        row(
            id,
            "result",
            if diagnostic == "none" {
                "positive"
            } else {
                "negative"
            },
            positive_inventory,
            positive_export,
            positive_metadata,
            path,
            diagnostic,
            u8::from(diagnostic != "none"),
        );
    }
    files.insert("manifest.tsv".into(), manifest);
}

fn write_files(files: &BTreeMap<String, String>) -> GenerateResult<()> {
    for (relative, content) in files {
        let path = Path::new(ROOT).join(relative);
        let mut parent = PathBuf::from(&path);
        parent.pop();
        fs::create_dir_all(&parent)
            .map_err(|error| format!("cannot create {}: {error}", parent.display()))?;
        fs::write(&path, content)
            .map_err(|error| format!("cannot write {}: {error}", path.display()))?;
    }
    Ok(())
}

fn check_files(files: &BTreeMap<String, String>) -> GenerateResult<()> {
    for (relative, expected) in files {
        let path = Path::new(ROOT).join(relative);
        let actual = fs::read_to_string(&path)
            .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
        if actual != *expected {
            return Err(format!("generated fixture is stale: {}", path.display()));
        }
    }
    Ok(())
}
