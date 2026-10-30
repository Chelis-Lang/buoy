//! Generate and stale-check deterministic Hegel property evidence fixtures.

#![allow(clippy::format_push_string, clippy::too_many_lines)]

use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

const ROOT: &str = "fixtures/property/v1";
const INVENTORY_HEADER: &str = "property_id\texpected_polarity\tproduction_target\tproduction_api\tproduction_source\treference_kind\treference_api\treference_source\ttest\ttest_source\tseed\tcases\tbackend\tderandomize\tdatabase\tphases\tgenerator_domain\tgenerator_limits\texplicit_edges\trejection_limit\tassertions\tsettings_digest\treplay_fixtures\tscope\tassurance_class\tstatus\n";
const RUN_HEADER: &str = "schema\tproperty_id\texpected_polarity\ttool_id\ttool_version\ttool_digest\ttoolchain_digest\tsettings_digest\tdomain_digest\tproperty_source_digest\tproduction_source_digest\treference_source_digest\treplay_digests\texecution\tdiagnostic\treplay_command\tidentity_binding\tassurance_class\tregistration\tfreshness\n";
const DIGEST_B: &str = "sha-256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
const DIGEST_C: &str = "sha-256:cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc";
const DIGEST_D: &str = "sha-256:dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd";
const DIGEST_E: &str = "sha-256:eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee";
const DIGEST_F: &str = "sha-256:ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff";
const DIGEST_G: &str = "sha-256:1111111111111111111111111111111111111111111111111111111111111111";
const DIGEST_H: &str = "sha-256:2222222222222222222222222222222222222222222222222222222222222222";

type GenerateResult<T> = Result<T, String>;

fn main() {
    if let Err(error) = run() {
        eprintln!("property-fixtures: {error}");
        std::process::exit(1);
    }
}

fn run() -> GenerateResult<()> {
    let arguments = env::args().skip(1).collect::<Vec<_>>();
    let check = match arguments.as_slice() {
        [] => false,
        [argument] if argument == "--check" => true,
        _ => {
            return Err(String::from("usage: generate-property-fixtures [--check]"));
        }
    };
    let files = generated_files();
    if check {
        check_files(&files)?;
        println!("property-fixtures: current files={}", files.len());
    } else {
        write_files(&files)?;
        println!("property-fixtures: generated files={}", files.len());
    }
    Ok(())
}

fn generated_files() -> BTreeMap<String, String> {
    let mut files = BTreeMap::new();
    add_sources(&mut files);
    add_tool_bindings(&mut files);
    add_inventories(&mut files);
    add_results(&mut files);
    add_manifest(&mut files);
    files
}

fn add_sources(files: &mut BTreeMap<String, String>) {
    for (path, source) in [
        (
            "repository/production.rs",
            "pub fn semantic(input: &[u8]) -> Vec<u8> { input.to_vec() }\npub fn other(input: &[u8]) -> Vec<u8> { input.to_vec() }\n",
        ),
        (
            "repository/reference.rs",
            "pub fn semantic(input: &[u8]) -> Vec<u8> { input.iter().copied().collect() }\npub fn other(input: &[u8]) -> Vec<u8> { input.iter().copied().collect() }\n",
        ),
        (
            "repository/property.rs",
            "fn hegel_control_valid_property(input: &[u8]) { assert_eq!(production::semantic(input), reference::semantic(input)); }\nfn hegel_control_other_property(input: &[u8]) { assert_eq!(production::other(input), reference::other(input)); }\n",
        ),
        (
            "repository/helper.rs",
            "pub fn semantic(input: &[u8]) -> Vec<u8> { input.to_vec() }\n",
        ),
        (
            "repository/reference-imports-production.rs",
            "pub use buoy_core::semantic_digest as semantic;\n",
        ),
        (
            "replays/intentionally-false.tsv",
            "schema\tproperty_id\tseed\tinput\texpected_diagnostic\n\
             buoy-property-replay/v1\thegel-control-intentionally-false\t6003393071738257663\tfalse\tHEGEL-NEGATIVE-CONTROL\n",
        ),
    ] {
        files.insert(path.to_owned(), source.to_owned());
    }
}

fn tool_binding(package: &str, version: &str, checksum: &str) -> String {
    format!(
        "schema = \"buoy-hegel-tool-binding/v1\"\nrepository = \"https://github.com/hegeldev/hegel-rust\"\npackage = \"{package}\"\ncrate_name = \"hegel\"\nversion = \"{version}\"\nsource_commit = \"634ea934be19dbb86a5792b4c3d35d6d4a54f7ad\"\nsource_tree = \"dc6a62465fb3f6b8d34942c60637d2d1aaca4347\"\narchive_sha256 = \"{checksum}\"\nmacro_package = \"hegeltest-macros\"\nmacro_version = \"0.28.0\"\nmacro_sha256 = \"c3fe342530584e9e0ac8144d40a746fff47c63ad1a2725c8793fbc883f7a6f9e\"\nc_package = \"hegeltest-c\"\nc_version = \"0.29.0\"\nc_source_commit = \"b8e2fdcf7046b55fdc84d9b7be68250324654f74\"\nc_source_tree = \"3736ecbe3cd96918c3d1aa19c815eb8ba6bcc4d2\"\nc_sha256 = \"dc2ab96fd030ce7ad37081c1f6ab3186ba794b9894cefeef2644d9dc1ecc1df7\"\nlicense = \"MIT\"\ntoolchain_digest = \"{DIGEST_B}\"\ndependency_boundary = \"test-shell-only\"\noffline = true\n"
    )
}

fn add_tool_bindings(files: &mut BTreeMap<String, String>) {
    files.insert(
        "tools/positive.toml".into(),
        tool_binding(
            "hegeltest",
            "0.28.0",
            "f12603ff7e94c463c26e97565d725010e92139f68fc93edcacf7cb800cfd422b",
        ),
    );
    files.insert(
        "tools/negative-substitution.toml".into(),
        tool_binding(
            "hegel",
            "0.1.1",
            "f12603ff7e94c463c26e97565d725010e92139f68fc93edcacf7cb800cfd422b",
        ),
    );
    files.insert(
        "tools/negative-version.toml".into(),
        tool_binding(
            "hegeltest",
            "0.29.0",
            "f12603ff7e94c463c26e97565d725010e92139f68fc93edcacf7cb800cfd422b",
        ),
    );
    files.insert(
        "tools/negative-checksum.toml".into(),
        tool_binding("hegeltest", "0.28.0", "unverified"),
    );
}

#[allow(clippy::too_many_arguments)]
fn inventory_row(
    id: &str,
    polarity: &str,
    production_target: &str,
    production_api: &str,
    production_source: &str,
    reference_kind: &str,
    reference_api: &str,
    reference_source: &str,
    test: &str,
    test_source: &str,
    seed: &str,
    cases: &str,
    backend: &str,
    derandomize: &str,
    database: &str,
    phases: &str,
    domain: &str,
    limits: &str,
    edges: &str,
    rejection_limit: &str,
    assertions: &str,
    settings_digest: &str,
    replay: &str,
    scope: &str,
    assurance: &str,
    status: &str,
) -> String {
    [
        id,
        polarity,
        production_target,
        production_api,
        production_source,
        reference_kind,
        reference_api,
        reference_source,
        test,
        test_source,
        seed,
        cases,
        backend,
        derandomize,
        database,
        phases,
        domain,
        limits,
        edges,
        rejection_limit,
        assertions,
        settings_digest,
        replay,
        scope,
        assurance,
        status,
    ]
    .join("\t")
        + "\n"
}

fn positive_inventory_row() -> String {
    inventory_row(
        "semantic-conformance",
        "pass",
        "semantic",
        "production::semantic",
        "production.rs",
        "independent-reference",
        "reference::semantic",
        "reference.rs",
        "hegel_control_valid_property",
        "property.rs",
        "6003393071738257409",
        "256",
        "Default",
        "true",
        "none",
        "Explicit,Reuse,Generate,Target,Shrink",
        "Vec<u8>",
        "size=0..64;depth=n/a;rejections=0",
        "empty;zero;max-length",
        "0",
        "exact-result-equality",
        DIGEST_C,
        "none",
        "bytes-length-0-through-64",
        "sampled",
        "production-differential",
    )
}

fn add_inventories(files: &mut BTreeMap<String, String>) {
    let row = positive_inventory_row();
    let positive = format!("{INVENTORY_HEADER}{row}");
    files.insert("inventories/positive.tsv".into(), positive.clone());

    let independent_second_row = row
        .replacen("semantic-conformance", "semantic-other-conformance", 1)
        .replacen(
            "\tsemantic\tproduction::semantic\t",
            "\tother\tproduction::other\t",
            1,
        )
        .replacen("reference::semantic", "reference::other", 1)
        .replacen(
            "hegel_control_valid_property",
            "hegel_control_other_property",
            1,
        )
        .replacen("6003393071738257409", "6003393071738257410", 1);
    files.insert(
        "inventories/negative-duplicate-id.tsv".into(),
        format!(
            "{positive}{}",
            independent_second_row.replacen(
                "semantic-other-conformance",
                "semantic-conformance",
                1
            )
        ),
    );
    files.insert(
        "inventories/negative-duplicate-seed.tsv".into(),
        format!(
            "{positive}{}",
            independent_second_row.replacen("6003393071738257410", "6003393071738257409", 1)
        ),
    );
    files.insert(
        "inventories/negative-domain-missing.tsv".into(),
        positive.replacen("\tVec<u8>\t", "\t\t", 1),
    );
    files.insert(
        "inventories/negative-settings-ambient.tsv".into(),
        positive
            .replacen("\t256\tDefault\t", "\tambient\tAutomatic\t", 1)
            .replacen("\ttrue\tnone\t", "\tenv\tlocal\t", 1),
    );
    files.insert(
        "inventories/negative-helper-only.tsv".into(),
        positive
            .replacen("production::semantic", "helper::semantic", 1)
            .replacen("production.rs", "helper.rs", 1)
            .replacen("production-differential", "reference-only", 1),
    );
    files.insert(
        "inventories/negative-reference-import.tsv".into(),
        positive.replacen("reference.rs", "reference-imports-production.rs", 1),
    );
    files.insert(
        "inventories/negative-production-missing.tsv".into(),
        positive.replacen("production::semantic", "production::missing", 1),
    );
    files.insert(
        "inventories/negative-duplicate-mapping.tsv".into(),
        format!(
            "{positive}{}",
            row.replacen("semantic-conformance", "semantic-conformance-two", 1)
                .replacen("6003393071738257409", "6003393071738257410", 1)
        ),
    );
}

#[allow(clippy::too_many_arguments)]
fn run_row(
    property_id: &str,
    settings_digest: &str,
    property_digest: &str,
    production_digest: &str,
    reference_digest: &str,
    replay_digests: &str,
    execution: &str,
    diagnostic: &str,
    registration: &str,
    freshness: &str,
) -> String {
    [
        "buoy-property-result/v1",
        property_id,
        "pass",
        "hegeltest",
        "0.28.0",
        "sha-256:f12603ff7e94c463c26e97565d725010e92139f68fc93edcacf7cb800cfd422b",
        DIGEST_B,
        settings_digest,
        DIGEST_D,
        property_digest,
        production_digest,
        reference_digest,
        replay_digests,
        execution,
        diagnostic,
        "devenv tasks run buoy:property-replay -- semantic-conformance",
        "buoy-0.2.0-post-rename",
        "sampled",
        registration,
        freshness,
    ]
    .join("\t")
        + "\n"
}

fn add_results(files: &mut BTreeMap<String, String>) {
    let positive_row = run_row(
        "semantic-conformance",
        DIGEST_C,
        DIGEST_E,
        DIGEST_F,
        DIGEST_G,
        "none",
        "pass",
        "none",
        "present",
        "current",
    );
    let positive = format!("{RUN_HEADER}{positive_row}");
    files.insert("results/positive.tsv".into(), positive.clone());

    files.insert(
        "results/negative-host-path.tsv".into(),
        positive.replacen(
            "devenv tasks run buoy:property-replay",
            "/tmp/host/target/debug/property-replay",
            1,
        ),
    );
    files.insert(
        "results/negative-timestamp.tsv".into(),
        format!(
            "{}\ttimestamp\n{}\t2026-07-19T00:00:00Z\n",
            RUN_HEADER.trim_end(),
            positive.lines().nth(1).unwrap_or_default()
        ),
    );
    files.insert(
        "results/negative-incomplete-binding.tsv".into(),
        positive.replacen(DIGEST_G, "none", 1),
    );
    files.insert(
        "results/negative-uncommitted-shrink.tsv".into(),
        positive.replacen("\tnone\tpass\tnone\t", "\t/tmp/shrink.log\tpass\tnone\t", 1),
    );
    files.insert(
        "results/negative-panic.tsv".into(),
        positive.replacen("\tpass\tnone\t", "\tpanic\tnone\t", 1),
    );
    files.insert(
        "results/negative-stale-settings.tsv".into(),
        positive.replacen(DIGEST_C, DIGEST_H, 1),
    );
    files.insert(
        "results/negative-stale-property.tsv".into(),
        positive.replacen(DIGEST_E, DIGEST_H, 1),
    );
    files.insert(
        "results/negative-stale-production.tsv".into(),
        positive.replacen(DIGEST_F, DIGEST_H, 1),
    );
    files.insert(
        "results/negative-stale-reference.tsv".into(),
        positive.replacen(DIGEST_G, DIGEST_H, 1),
    );
}

fn add_manifest(files: &mut BTreeMap<String, String>) {
    let mut manifest = String::from(
        "fixture_id\tkind\tpolarity\ttool_binding\tinventory\tresult\texpected_diagnostic\texpected_count\n",
    );
    let mut row = |id: &str,
                   kind: &str,
                   polarity: &str,
                   tool: &str,
                   inventory: &str,
                   result: &str,
                   diagnostic: &str,
                   count: u8| {
        manifest.push_str(&format!(
            "{id}\t{kind}\t{polarity}\t{tool}\t{inventory}\t{result}\t{diagnostic}\t{count}\n"
        ));
    };
    let tool = "tools/positive.toml";
    let inventory = "inventories/positive.tsv";
    let result = "results/positive.tsv";
    row(
        "property-positive",
        "complete",
        "positive",
        tool,
        inventory,
        result,
        "none",
        0,
    );
    for (id, path) in [
        ("tool-substitution", "tools/negative-substitution.toml"),
        ("tool-version", "tools/negative-version.toml"),
        ("tool-checksum", "tools/negative-checksum.toml"),
    ] {
        row(
            id,
            "tool",
            "negative",
            path,
            inventory,
            result,
            "HEGEL-TOOL-IDENTITY",
            1,
        );
    }
    for (id, path, diagnostic) in [
        (
            "inventory-duplicate-id",
            "inventories/negative-duplicate-id.tsv",
            "HEGEL-PROPERTY-DUPLICATE-ID",
        ),
        (
            "inventory-duplicate-seed",
            "inventories/negative-duplicate-seed.tsv",
            "HEGEL-SEED-DUPLICATE",
        ),
        (
            "inventory-domain-missing",
            "inventories/negative-domain-missing.tsv",
            "HEGEL-DOMAIN-MISSING",
        ),
        (
            "inventory-settings-ambient",
            "inventories/negative-settings-ambient.tsv",
            "HEGEL-SETTINGS-AMBIENT",
        ),
        (
            "inventory-helper-only",
            "inventories/negative-helper-only.tsv",
            "HEGEL-PRODUCTION-NOT-CALLED",
        ),
        (
            "inventory-reference-import",
            "inventories/negative-reference-import.tsv",
            "HEGEL-REFERENCE-NOT-INDEPENDENT",
        ),
        (
            "inventory-production-missing",
            "inventories/negative-production-missing.tsv",
            "HEGEL-PROPERTY-MISSING",
        ),
        (
            "inventory-duplicate-mapping",
            "inventories/negative-duplicate-mapping.tsv",
            "HEGEL-PROPERTY-DUPLICATE-MAPPING",
        ),
    ] {
        row(
            id,
            "inventory",
            "negative",
            tool,
            path,
            result,
            diagnostic,
            1,
        );
    }
    for (id, path, diagnostic) in [
        (
            "result-host-path",
            "results/negative-host-path.tsv",
            "HEGEL-RESULT-HOST-FIELD",
        ),
        (
            "result-timestamp",
            "results/negative-timestamp.tsv",
            "HEGEL-RESULT-HOST-FIELD",
        ),
        (
            "result-incomplete-binding",
            "results/negative-incomplete-binding.tsv",
            "HEGEL-RESULT-BINDING-MISSING",
        ),
        (
            "result-uncommitted-shrink",
            "results/negative-uncommitted-shrink.tsv",
            "HEGEL-REPLAY-UNCOMMITTED",
        ),
        (
            "result-panic",
            "results/negative-panic.tsv",
            "HEGEL-EXECUTION-INCOMPLETE",
        ),
        (
            "result-stale-settings",
            "results/negative-stale-settings.tsv",
            "HEGEL-EVIDENCE-STALE",
        ),
        (
            "result-stale-property",
            "results/negative-stale-property.tsv",
            "HEGEL-EVIDENCE-STALE",
        ),
        (
            "result-stale-production",
            "results/negative-stale-production.tsv",
            "HEGEL-EVIDENCE-STALE",
        ),
        (
            "result-stale-reference",
            "results/negative-stale-reference.tsv",
            "HEGEL-EVIDENCE-STALE",
        ),
    ] {
        row(
            id, "result", "negative", tool, inventory, path, diagnostic, 1,
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
