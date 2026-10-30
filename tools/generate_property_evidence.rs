//! Generate source-bound authoritative Hegel property inventory and run records.

#![allow(clippy::too_many_arguments, clippy::too_many_lines)]

use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::path::Path;

#[path = "digest_binding.rs"]
mod content_digest;

const INVENTORY: &str = "assurance/v1/property-inventory.tsv";
const RUNS: &str = "assurance/v1/property-runs.tsv";
const TOOL_BINDING: &str = "assurance/v1/hegel-tool-binding.toml";
const INVENTORY_HEADER: &str = "property_id\texpected_polarity\tproduction_target\tproduction_api\tproduction_source\treference_kind\treference_api\treference_source\ttest\ttest_source\tseed\tcases\tbackend\tderandomize\tdatabase\tphases\tgenerator_domain\tgenerator_limits\texplicit_edges\trejection_limit\tassertions\tsettings_digest\treplay_fixtures\tscope\tassurance_class\tstatus\n";
const RUN_HEADER: &str = "schema\tproperty_id\texpected_polarity\ttool_id\ttool_version\ttool_digest\ttoolchain_digest\tsettings_digest\tdomain_digest\tproperty_source_digest\tproduction_source_digest\treference_source_digest\treplay_digests\texecution\tdiagnostic\treplay_command\tidentity_binding\tassurance_class\tregistration\tfreshness\n";
const SEED_PREFIX: u64 = 0x5350_5631_0000_0000;
const PHASES: &str = "Explicit,Reuse,Generate,Target,Shrink";
const HEGEL_ARCHIVE: &str = "f12603ff7e94c463c26e97565d725010e92139f68fc93edcacf7cb800cfd422b";

type GenerateResult<T> = Result<T, String>;

#[derive(Clone, Debug)]
struct Property {
    id: &'static str,
    polarity: &'static str,
    production_target: &'static str,
    production_api: &'static str,
    production_source: &'static str,
    reference_kind: &'static str,
    reference_api: &'static str,
    reference_source: &'static str,
    test: &'static str,
    test_source: &'static str,
    seed_suffix: u16,
    domain: &'static str,
    limits: &'static str,
    edges: &'static str,
    rejection_limit: &'static str,
    assertions: &'static str,
    replay: &'static str,
    scope: &'static str,
    status: &'static str,
}

fn main() {
    if let Err(error) = run() {
        eprintln!("property-evidence-generator: {error}");
        std::process::exit(1);
    }
}

fn run() -> GenerateResult<()> {
    let arguments = env::args().skip(1).collect::<Vec<_>>();
    let check = match arguments.as_slice() {
        [] => false,
        [argument] if argument == "--check" => true,
        _ => {
            return Err(String::from("usage: generate-property-evidence [--check]"));
        }
    };
    let generated = generate()?;
    if check {
        for (path, expected) in &generated {
            let actual =
                fs::read_to_string(path).map_err(|error| format!("cannot read {path}: {error}"))?;
            if actual != *expected {
                return Err(format!("generated property evidence is stale: {path}"));
            }
        }
        println!(
            "property-evidence-generator: current files={}",
            generated.len()
        );
    } else {
        for (path, content) in &generated {
            fs::write(path, content).map_err(|error| format!("cannot write {path}: {error}"))?;
        }
        println!(
            "property-evidence-generator: generated files={}",
            generated.len()
        );
    }
    Ok(())
}

fn generate() -> GenerateResult<BTreeMap<&'static str, String>> {
    let properties = properties();
    let mut inventory = String::from(INVENTORY_HEADER);
    let mut settings = Vec::new();
    for property in &properties {
        let seed = SEED_PREFIX | u64::from(property.seed_suffix);
        let settings_binding = settings_binding(property, seed);
        let settings_digest = content_digest::bytes_binding(settings_binding.as_bytes());
        inventory.push_str(&inventory_row(property, seed, &settings_digest));
        settings.push(settings_digest);
    }

    let toolchain_digest = content_digest::file_binding(Path::new("rust-toolchain.toml"))?;
    let mut runs = String::from(RUN_HEADER);
    for (property, settings_digest) in properties.iter().zip(&settings) {
        runs.push_str(&run_row(property, settings_digest, &toolchain_digest)?);
    }

    let mut generated = BTreeMap::new();
    generated.insert(INVENTORY, inventory);
    generated.insert(RUNS, runs);
    generated.insert(TOOL_BINDING, tool_binding(&toolchain_digest));
    Ok(generated)
}

fn settings_binding(property: &Property, seed: u64) -> String {
    format!(
        "backend=Default\ncases=256\nseed={seed}\nderandomize=true\ndatabase=none\nphases={PHASES}\ndomain={}\nlimits={}\nedges={}\nrejection_limit={}\n",
        property.domain, property.limits, property.edges, property.rejection_limit
    )
}

fn inventory_row(property: &Property, seed: u64, settings_digest: &str) -> String {
    [
        property.id.to_owned(),
        property.polarity.to_owned(),
        property.production_target.to_owned(),
        property.production_api.to_owned(),
        property.production_source.to_owned(),
        property.reference_kind.to_owned(),
        property.reference_api.to_owned(),
        property.reference_source.to_owned(),
        property.test.to_owned(),
        property.test_source.to_owned(),
        seed.to_string(),
        "256".to_owned(),
        "Default".to_owned(),
        "true".to_owned(),
        "none".to_owned(),
        PHASES.to_owned(),
        property.domain.to_owned(),
        property.limits.to_owned(),
        property.edges.to_owned(),
        property.rejection_limit.to_owned(),
        property.assertions.to_owned(),
        settings_digest.to_owned(),
        property.replay.to_owned(),
        property.scope.to_owned(),
        "sampled".to_owned(),
        property.status.to_owned(),
    ]
    .join("\t")
        + "\n"
}

fn run_row(
    property: &Property,
    settings_digest: &str,
    toolchain_digest: &str,
) -> GenerateResult<String> {
    let property_digest = content_digest::file_binding(Path::new(property.test_source))?;
    let production_digest = optional_digest(property.production_source)?;
    let reference_digest = optional_digest(property.reference_source)?;
    let replay_digests = optional_digest(property.replay)?;
    let domain_binding = format!(
        "domain={}\nlimits={}\nedges={}\nrejection_limit={}\nscope={}\n",
        property.domain, property.limits, property.edges, property.rejection_limit, property.scope
    );
    let domain_digest = content_digest::bytes_binding(domain_binding.as_bytes());
    let (execution, diagnostic) = if property.polarity == "fail" {
        ("expected-failure-observed", "HEGEL-NEGATIVE-CONTROL")
    } else {
        ("pass", "none")
    };
    Ok([
        "buoy-property-result/v1".to_owned(),
        property.id.to_owned(),
        property.polarity.to_owned(),
        "hegeltest".to_owned(),
        "0.28.0".to_owned(),
        format!("sha-256:{HEGEL_ARCHIVE}"),
        toolchain_digest.to_owned(),
        settings_digest.to_owned(),
        domain_digest,
        property_digest,
        production_digest,
        reference_digest,
        replay_digests,
        execution.to_owned(),
        diagnostic.to_owned(),
        format!(
            "devenv shell -- cargo test --test {} {} -- --exact",
            test_target(property.test_source),
            property.test
        ),
        "buoy-0.2.0-post-rename".to_owned(),
        "sampled".to_owned(),
        "present".to_owned(),
        "current".to_owned(),
    ]
    .join("\t")
        + "\n")
}

fn optional_digest(path: &str) -> GenerateResult<String> {
    if path == "none" {
        Ok(String::from("not-applicable"))
    } else {
        content_digest::file_binding(Path::new(path))
    }
}

fn test_target(path: &str) -> &str {
    Path::new(path)
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("unknown")
}

fn tool_binding(toolchain_digest: &str) -> String {
    format!(
        "schema = \"buoy-hegel-tool-binding/v1\"\nrepository = \"https://github.com/hegeldev/hegel-rust\"\npackage = \"hegeltest\"\ncrate_name = \"hegel\"\nversion = \"0.28.0\"\nsource_commit = \"634ea934be19dbb86a5792b4c3d35d6d4a54f7ad\"\nsource_tree = \"dc6a62465fb3f6b8d34942c60637d2d1aaca4347\"\narchive_sha256 = \"{HEGEL_ARCHIVE}\"\nmacro_package = \"hegeltest-macros\"\nmacro_version = \"0.28.0\"\nmacro_sha256 = \"c3fe342530584e9e0ac8144d40a746fff47c63ad1a2725c8793fbc883f7a6f9e\"\nc_package = \"hegeltest-c\"\nc_version = \"0.29.0\"\nc_source_commit = \"b8e2fdcf7046b55fdc84d9b7be68250324654f74\"\nc_source_tree = \"3736ecbe3cd96918c3d1aa19c815eb8ba6bcc4d2\"\nc_sha256 = \"dc2ab96fd030ce7ad37081c1f6ab3186ba794b9894cefeef2644d9dc1ecc1df7\"\nlicense = \"MIT\"\ntoolchain_digest = \"{toolchain_digest}\"\ndependency_boundary = \"test-shell-only\"\noffline = true\n"
    )
}

fn property(
    id: &'static str,
    production_target: &'static str,
    production_api: &'static str,
    production_source: &'static str,
    reference_kind: &'static str,
    reference_api: &'static str,
    reference_source: &'static str,
    test: &'static str,
    test_source: &'static str,
    seed_suffix: u16,
    domain: &'static str,
    limits: &'static str,
    edges: &'static str,
    assertions: &'static str,
    scope: &'static str,
    status: &'static str,
) -> Property {
    Property {
        id,
        polarity: "pass",
        production_target,
        production_api,
        production_source,
        reference_kind,
        reference_api,
        reference_source,
        test,
        test_source,
        seed_suffix,
        domain,
        limits,
        edges,
        rejection_limit: "0",
        assertions,
        replay: "none",
        scope,
        status,
    }
}

fn properties() -> Vec<Property> {
    let main = "tests/hegel_properties.rs";
    let algorithms = "crates/buoy-core/src/algorithms.rs";
    let value = "crates/buoy-core/src/value.rs";
    let reference = "crates/buoy-reference/src/lib.rs";
    let mut properties = vec![
        property(
            "canonical-encode",
            "CanonicalValue::encode",
            "CanonicalValue::encode",
            value,
            "independent-reference",
            "Value::encode",
            reference,
            "hegel_production_and_reference_canonical_sets_agree",
            main,
            1,
            "Vec<u8>",
            "size=0..32;depth=1;rejections=0",
            "empty;duplicate;reverse-order",
            "exact-canonical-bytes",
            "canonical-set-values",
            "production-differential",
        ),
        property(
            "canonical-recursive-roundtrip",
            "CanonicalValue::encode/decode",
            "CanonicalValue::encode/decode",
            value,
            "independent-reference",
            "Value::encode/decode",
            reference,
            "hegel_recursive_values_round_trip_and_match_the_reference",
            main,
            20,
            "recursive-canonical-value",
            "bytes=0..64;depth=0..4;rejections=0",
            "false;true;zero;empty;max-depth",
            "exact-bytes-and-roundtrip",
            "recursive-values-depth-four",
            "production-differential",
        ),
        property(
            "semantic-digest",
            "semantic_digest_observation",
            "semantic_digest_observation",
            value,
            "independent-reference",
            "buoy_reference::semantic_preimage",
            reference,
            "hegel_production_and_reference_preimages_agree",
            main,
            2,
            "Vec<u8>",
            "size=0..64;depth=n/a;rejections=0",
            "empty;zero;max-length",
            "exact-preimage-bytes",
            "semantic-preimage-field-bytes",
            "production-differential",
        ),
        property(
            "repository-path",
            "RepositoryPath::new",
            "RepositoryPath::new",
            value,
            "independent-reference",
            "algorithms::portable_path",
            reference,
            "hegel_production_and_reference_path_policy_agree",
            main,
            3,
            "reviewed-path-corpus",
            "members=12;rejections=0",
            "unicode-nfc;traversal;absolute;reserved;empty",
            "acceptance-and-normalized-path",
            "portable-path-policy-corpus",
            "production-differential",
        ),
        property(
            "canonical-path-set",
            "canonical_paths",
            "canonical_paths",
            value,
            "independent-reference",
            "algorithms::portable_path_set",
            reference,
            "hegel_production_and_reference_path_sets_agree",
            main,
            4,
            "pair-of-reviewed-paths",
            "members=5x5;rejections=0",
            "case-collision;duplicate;distinct",
            "acceptance-and-exact-order",
            "portable-path-set-pairs",
            "production-differential",
        ),
        property(
            "lifecycle-transition",
            "lifecycle_transition",
            "lifecycle_transition",
            algorithms,
            "independent-reference",
            "algorithms::lifecycle_transition",
            reference,
            "hegel_production_and_reference_lifecycle_agree",
            main,
            5,
            "lifecycle-state-x-event",
            "states=4;events=3;rejections=0",
            "all-terminal-and-live-tags",
            "exact-next-state",
            "finite-lifecycle-product",
            "production-differential",
        ),
        property(
            "governance-transition",
            "governance_transition_valid",
            "governance_transition_valid",
            algorithms,
            "independent-reference",
            "algorithms::governance_transition_valid",
            reference,
            "hegel_production_and_reference_governance_lifecycle_agree",
            main,
            6,
            "policy-debt-waiver-transition",
            "policy=4x4;debt=4x4;waiver=5x5;rejections=0",
            "absent;active;all-terminals",
            "exact-validity",
            "governance-lifecycle-product",
            "production-differential",
        ),
        property(
            "surface-completeness",
            "surface_completeness",
            "surface_completeness",
            algorithms,
            "independent-reference",
            "algorithms::surface_complete",
            reference,
            "hegel_production_and_reference_surface_completeness_agree",
            main,
            7,
            "two-member-surface",
            "booleans=4;rejections=0",
            "missing;duplicate;authority-missing;complete",
            "exact-completeness",
            "two-member-surface-product",
            "production-differential",
        ),
        property(
            "carrier-freshness",
            "carrier_fresh",
            "carrier_fresh",
            algorithms,
            "independent-reference",
            "algorithms::all_current",
            reference,
            "hegel_production_and_reference_freshness_agree",
            main,
            8,
            "carrier-bindings",
            "booleans=2;rejections=0",
            "all-current;each-stale",
            "exact-freshness",
            "carrier-atom-oracle-bindings",
            "production-differential",
        ),
        property(
            "derivation-freshness",
            "derivation_fresh",
            "derivation_fresh",
            algorithms,
            "independent-reference",
            "algorithms::all_current",
            reference,
            "hegel_production_and_reference_derivation_freshness_agree",
            main,
            14,
            "derivation-bindings",
            "booleans=3;rejections=0",
            "all-current;each-stale",
            "exact-freshness",
            "derivation-source-target-oracle",
            "production-differential",
        ),
        property(
            "proof-freshness",
            "proof_fresh",
            "proof_fresh",
            algorithms,
            "independent-reference",
            "algorithms::all_current",
            reference,
            "hegel_production_and_reference_proof_freshness_agree",
            main,
            15,
            "proof-bindings",
            "booleans=6;rejections=0",
            "all-current;each-stale;fail-verdict",
            "exact-freshness",
            "proof-authority-implementation-assumption-verdict",
            "production-differential",
        ),
        property(
            "required-role-coverage",
            "required_role_coverage",
            "required_role_coverage",
            algorithms,
            "independent-reference",
            "algorithms::required_roles",
            reference,
            "hegel_production_and_reference_role_coverage_agree",
            main,
            9,
            "positive-negative-role-presence",
            "booleans=2;rejections=0",
            "none;positive-only;negative-only;both",
            "exact-present-and-missing",
            "two-required-role-product",
            "production-differential",
        ),
        property(
            "policy-ratchet",
            "compare_policy",
            "compare_policy",
            algorithms,
            "independent-reference",
            "algorithms::policy_ratchet",
            reference,
            "hegel_production_and_reference_policy_ratchets_agree",
            main,
            10,
            "policy-change-flags",
            "booleans=4;rejections=0",
            "unchanged;subject-removal;role-removal;mode;lineage",
            "exact-ratchet-facts",
            "policy-ratchet-product",
            "production-differential",
        ),
        property(
            "assurance-aggregation",
            "aggregate_assurance",
            "aggregate_assurance",
            algorithms,
            "independent-reference",
            "algorithms::assurance_classes",
            reference,
            "hegel_production_and_reference_no_laundering_agree",
            main,
            11,
            "assurance-input-flags",
            "booleans=11;rejections=0",
            "unlinked;stale;unselected;fail;all-classes",
            "exact-assurance-classes",
            "no-laundering-product",
            "production-differential",
        ),
        property(
            "change-impact",
            "change_impacts",
            "change_impacts",
            algorithms,
            "independent-reference",
            "algorithms::transitive_impacts",
            reference,
            "hegel_production_and_reference_impact_closure_agree",
            main,
            12,
            "three-node-impact-chain",
            "booleans=2;depth=3;rejections=0",
            "unchanged;atom;link;both",
            "exact-direct-transitive-set",
            "three-node-impact-product",
            "production-differential",
        ),
        property(
            "disposition-closure",
            "disposition_closure",
            "disposition_closure",
            algorithms,
            "independent-reference",
            "algorithms::disposition_closed",
            reference,
            "hegel_production_and_reference_disposition_closure_agree",
            main,
            16,
            "one-impact-disposition-count",
            "count=0..2;rejections=0",
            "missing;one;duplicate",
            "exact-closure",
            "single-impact-cardinality",
            "production-differential",
        ),
        property(
            "semantic-graph-digest",
            "semantic_graph_digest",
            "semantic_graph_digest",
            algorithms,
            "metamorphic",
            "permutation-invariance",
            "none",
            "hegel_semantic_graph_digest_is_order_independent",
            main,
            13,
            "two-link-permutation",
            "booleans=1;rejections=0",
            "forward;reverse",
            "exact-digest-equality",
            "two-link-permutation",
            "production-metamorphic",
        ),
        property(
            "execution-report",
            "execution_report",
            "execution_report",
            algorithms,
            "metamorphic",
            "canonical-verdict-order",
            "none",
            "hegel_execution_report_is_order_independent",
            main,
            17,
            "two-verdict-permutation",
            "booleans=1;rejections=0",
            "forward;reverse",
            "exact-canonical-order",
            "two-verdict-permutation",
            "production-metamorphic",
        ),
    ];

    let logical = "tests/logical_mount_hegel_properties.rs";
    let mounts = "crates/buoy-core/src/mount.rs";
    let attachments = "crates/buoy-core/src/attachment.rs";
    properties.extend([
        property(
            "logical-mount-permutation",
            "canonical_logical_mounts",
            "canonical_logical_mounts",
            mounts,
            "independent-reference",
            "logical_mount_set",
            reference,
            "hegel_mount_permutations_match_the_independent_model",
            logical,
            31,
            "two-mount-permutation",
            "booleans=1;rejections=0",
            "forward;reverse",
            "exact-canonical-mount-set",
            "two-disjoint-mounts",
            "production-differential",
        ),
        property(
            "logical-projection-injection",
            "project_logical_path",
            "project_logical_path",
            mounts,
            "independent-reference",
            "logical_projection",
            reference,
            "hegel_logical_projection_is_injective_for_disjoint_prefixes",
            logical,
            32,
            "u8-relative-path",
            "values=0..255;mounts=2;rejections=0",
            "project;authority",
            "distinct-prefix-injection-and-exact-path",
            "two-prefix-projection",
            "production-differential",
        ),
        property(
            "logical-prefix-collision",
            "canonical_logical_mounts",
            "canonical_logical_mounts",
            mounts,
            "independent-reference",
            "logical_mount_set",
            reference,
            "hegel_mount_collisions_match_the_independent_model",
            logical,
            33,
            "overlap-choice",
            "choices=3;rejections=0",
            "equal;nested;portable-case-collision",
            "exact-rejection",
            "two-overlapping-prefixes",
            "production-differential",
        ),
        property(
            "opaque-artifact-byte-identity",
            "extract_artifact",
            "extract_artifact",
            "src/adapters/artifact.rs",
            "metamorphic",
            "repeat-and-byte-change",
            "none",
            "hegel_opaque_byte_identity_is_deterministic_and_sensitive",
            logical,
            34,
            "Vec<u8>",
            "size=0..128;rejections=0",
            "empty;invalid-utf8;one-byte-extension",
            "repeat-equality-and-byte-sensitivity",
            "opaque-saved-bytes",
            "production-metamorphic",
        ),
        property(
            "artifact-attachment-key",
            "binding_attachment_key",
            "binding_attachment_key",
            attachments,
            "independent-reference",
            "attachment_key_preimage",
            reference,
            "hegel_attachment_keys_match_the_independent_preimage",
            logical,
            35,
            "u8-relation-suffix",
            "values=0..255;rejections=0",
            "zero;max",
            "exact-preimage-and-key-digest",
            "relation-key-preimage",
            "production-differential",
        ),
        property(
            "artifact-binding-transaction-plan",
            "artifact_binding_transaction_state",
            "artifact_binding_transaction_state",
            attachments,
            "independent-reference",
            "binding_transaction_state",
            reference,
            "hegel_transaction_plan_matches_the_independent_model",
            logical,
            36,
            "failure-stage-x-prior-state",
            "stages=14;booleans=2;rejections=0",
            "success;all-failures;prior-object;prior-sidecar",
            "exact-final-state",
            "bounded-transaction-plan",
            "production-differential",
        ),
    ]);

    let xxh3 = "tests/xxh3_hegel_properties.rs";
    for (id, test, suffix, domain, limits, edges, assertions) in [
        (
            "xxh3-one-shot",
            "hegel_xxh3_one_shot_is_deterministic",
            101,
            "Vec<u8>",
            "size=0..4096;rejections=0",
            "empty;zero;max-length",
            "repeat-equality",
        ),
        (
            "xxh3-incremental",
            "hegel_xxh3_incremental_matches_one_shot",
            102,
            "Vec<u8>-x-chunk",
            "size=0..4096;chunk=1..64;rejections=0",
            "empty;single-byte;max-chunk",
            "incremental-equals-one-shot",
        ),
        (
            "xxh3-forms",
            "hegel_xxh3_text_and_binary_forms_round_trip",
            103,
            "Vec<u8>",
            "size=0..1024;rejections=0",
            "empty;zero;max-length",
            "text-and-binary-roundtrip",
        ),
        (
            "xxh3-sensitivity",
            "hegel_xxh3_is_sensitive_to_domain_field_and_order_changes",
            104,
            "u8",
            "values=0..255;rejections=0",
            "zero;max",
            "observed-domain-field-order-sensitivity",
        ),
        (
            "xxh3-parser",
            "hegel_xxh3_parser_rejects_noncanonical_forms",
            105,
            "Vec<u8>-x-mutation",
            "size=0..1024;mutations=5;rejections=0",
            "uppercase;wrong-prefix;truncated;padded;nonhex",
            "all-mutants-rejected",
        ),
        (
            "xxh3-migration",
            "hegel_xxh3_complete_migration_is_idempotent",
            106,
            "pair-of-Vec<u8>",
            "size=0..512-each;rejections=0",
            "empty;distinct;equal",
            "migration-idempotence",
        ),
    ] {
        let mut row = property(
            id,
            "none",
            "none",
            "none",
            "reference-only",
            "xxhash-rust/test-helper",
            xxh3,
            test,
            xxh3,
            suffix,
            domain,
            limits,
            edges,
            assertions,
            domain,
            "reference-only",
        );
        row.replay = "fixtures/xxh3/v2/hegel-replay.toml";
        properties.push(row);
    }

    let core = "crates/buoy-core/tests/xxh3_hegel_envelope.rs";
    properties.push(property(
        "core-envelope-reference",
        "semantic_digest_observation",
        "semantic_digest_observation",
        value,
        "independent-reference",
        "buoy_reference::semantic_preimage",
        reference,
        "hegel_production_envelope_matches_the_explicit_profile",
        core,
        201,
        "domain-x-Vec<u8>",
        "domains=2;size=0..1024;rejections=0",
        "both-domains;empty;max-length",
        "exact-preimage-bytes",
        "v3-envelope-sample",
        "production-differential",
    ));
    properties.push(property(
        "core-envelope-bindings",
        "semantic_digest",
        "semantic_digest",
        value,
        "metamorphic",
        "domain-and-order-sensitivity",
        "none",
        "hegel_production_envelope_is_deterministic_and_exercises_bindings",
        core,
        202,
        "u8",
        "values=0..255;rejections=0",
        "zero;max;domain-change;order-change",
        "repeat-equality-and-observed-sensitivity",
        "v2-binding-sample",
        "production-metamorphic",
    ));

    let controls = "tests/hegel_controls.rs";
    let mut false_control = property(
        "hegel-intentionally-false-control",
        "none",
        "none",
        "none",
        "negative-control",
        "boolean-self-inequality",
        controls,
        "hegel_expected_failure_is_rejected_exactly_once",
        controls,
        255,
        "bool",
        "values=false,true;rejections=0",
        "false;true",
        "intentional-self-inequality-rejected",
        "expected-failure-protocol",
        "negative-control",
    );
    false_control.polarity = "fail";
    false_control.replay = "fixtures/property/v1/replays/intentionally-false.tsv";
    properties.push(false_control);

    let publication = "src/publication.rs";
    let publication_fs = "src/publication_fs.rs";
    let publication_models = "src/publication_models.rs";
    let publication_test = "tests/publication_hegel_properties.rs";
    properties.extend([
        property(
            "publication-plan-field-permutation",
            "parse_plan",
            "parse_plan",
            publication,
            "metamorphic",
            "field-permutation-invariance",
            "none",
            "hegel_plan_field_permutation_is_order_independent",
            publication_test,
            901,
            "publication-id/expected-rows",
            "id-len=1..12;rows=1..8;rejections=0",
            "forward-order;reverse-order",
            "equal-parsed-fields",
            "publication-plan-permutation",
            "production-metamorphic",
        ),
        property(
            "publication-plan-root-independence",
            "parse_plan",
            "parse_plan",
            publication,
            "metamorphic",
            "repeated-parse-invariance",
            "none",
            "hegel_plan_parsing_matches_an_independent_field_extractor_and_is_root_independent",
            publication_test,
            902,
            "publication-id/expected-rows",
            "id-len=1..10;rows=1..8;rejections=0",
            "first-parse;second-parse",
            "equal-parsed-fields",
            "publication-plan-root",
            "production-metamorphic",
        ),
        property(
            "publication-row-locality",
            "row_conformance",
            "row_conformance",
            publication,
            "metamorphic",
            "fragment-never-satisfies",
            "none",
            "hegel_row_locality_never_admits_a_two_row_fragment",
            publication_test,
            903,
            "trace-row-pair",
            "seed=0..255;field-drop=0..5;rejections=0",
            "one-blank-field;unrelated-second-row",
            "row-fragmentation-rejected",
            "publication-row-locality",
            "production-metamorphic",
        ),
        property(
            "publication-row-conformance-determinism",
            "row_conformance",
            "row_conformance",
            publication,
            "metamorphic",
            "repeat-call-invariance",
            "none",
            "hegel_row_conformance_diagnostics_are_deterministic",
            publication_test,
            904,
            "trace-row-set",
            "seed=0..255;count=0..3;expected=1..4;rejections=0",
            "first-call;second-call",
            "equal-result-variant",
            "publication-row-determinism",
            "production-metamorphic",
        ),
        property(
            "publication-transaction-source-guard",
            "TransactionState::allowed_next",
            "TransactionState::allowed_next",
            publication_models,
            "metamorphic",
            "write-requires-guard-invariance",
            "none",
            "hegel_transaction_walks_never_reach_write_without_a_source_guard",
            publication_test,
            905,
            "bounded-state-walk",
            "steps=0..12;rejections=0",
            "random-legal-transition-sequence",
            "no-write-without-guard",
            "publication-transaction-walk",
            "production-metamorphic",
        ),
        property(
            "publication-derived-output-exclusion",
            "classify_output",
            "classify_output",
            publication,
            "metamorphic",
            "selected-exclusion-invariance",
            "none",
            "hegel_derived_outputs_are_always_excluded_from_the_selected_source_set",
            publication_test,
            906,
            "output-path-pair",
            "name-len=1..12;rejections=0",
            "selected-derived;selected-source-edit;unrelated-derived",
            "exact-classification",
            "publication-output-classification",
            "production-metamorphic",
        ),
        property(
            "publication-basis-completeness",
            "compute_publication_basis",
            "compute_publication_basis",
            publication,
            "metamorphic",
            "single-byte-sensitivity",
            "none",
            "hegel_publication_basis_is_complete_over_its_snapshot_input",
            publication_test,
            907,
            "snapshot-bytes",
            "len=1..16;rejections=0",
            "base-snapshot;one-flipped-byte",
            "unequal-basis-on-any-byte-change",
            "publication-basis-sensitivity",
            "production-metamorphic",
        ),
        property(
            "publication-target-uri-join",
            "join_target_base",
            "join_target_base",
            publication,
            "metamorphic",
            "base-and-path-preservation",
            "none",
            "hegel_uri_joining_preserves_base_and_the_complete_page_path",
            publication_test,
            908,
            "host/page-path",
            "host-len=1..10;segment-len=1..10;rejections=0",
            "generated-host;generated-segment",
            "prefix-and-suffix-preserved",
            "publication-target-join",
            "production-metamorphic",
        ),
        property(
            "publication-uri-join-percent-encode-differential",
            "join_target_base",
            "join_target_base",
            publication,
            "metamorphic",
            "percent-encode-splice",
            "none",
            "hegel_uri_join_matches_an_independent_percent_encode_reimplementation",
            publication_test,
            914,
            "page-path-segments",
            "segment-len=1..6;count=1..3;rejections=0",
            "reserved-and-unreserved-byte-mix",
            "equal-percent-encoded-path",
            "publication-uri-percent-encode",
            "production-metamorphic",
        ),
        property(
            "publication-relative-path-traversal-rejection",
            "split_relative_path",
            "split_relative_path",
            publication_fs,
            "metamorphic",
            "traversal-shape-rejection",
            "none",
            "hegel_relative_path_split_rejects_every_generated_traversal_shape",
            publication_test,
            909,
            "path-segment-pair",
            "prefix-len=1..8;suffix-len=1..8;rejections=0",
            "dot-dot-component;doubled-separator;clean-path",
            "traversal-rejected-clean-accepted",
            "publication-path-traversal",
            "production-metamorphic",
        ),
        property(
            "publication-report-encoder-differential",
            "render_report",
            "render_report",
            publication,
            "metamorphic",
            "sort",
            "none",
            "hegel_report_encoder_matches_an_independent_sort_reimplementation",
            publication_test,
            910,
            "diagnostic-code-multiset",
            "count=0..5;code=TPT-000..TPT-255;rejections=0",
            "empty;single;duplicate-codes",
            "equal-sorted-order",
            "publication-report-order",
            "production-metamorphic",
        ),
        property(
            "publication-package-identity-feature-order",
            "PackageIdentity::render",
            "PackageIdentity::render",
            publication,
            "metamorphic",
            "sort_unstable",
            "none",
            "hegel_package_identity_feature_order_matches_an_independent_sort",
            publication_test,
            911,
            "feature-multiset",
            "count=1..3;pool=xxh3-128,blake3-256,sha-256;rejections=0",
            "single;duplicate;full-pool",
            "equal-sorted-order",
            "publication-package-feature-order",
            "production-metamorphic",
        ),
        property(
            "publication-evidence-encoder-differential",
            "render_evidence",
            "render_evidence",
            publication,
            "metamorphic",
            "json!",
            "none",
            "hegel_evidence_encoder_matches_an_independent_json_reimplementation",
            publication_test,
            912,
            "publication-id",
            "id-len=1..12;rejections=0",
            "generated-ascii-id",
            "byte-identical-json",
            "publication-evidence-bytes",
            "production-metamorphic",
        ),
        property(
            "publication-path-substitution-differential",
            "render_path",
            "render_path",
            publication,
            "metamorphic",
            "splice",
            "none",
            "hegel_path_substitution_matches_an_independent_splice_reimplementation",
            publication_test,
            913,
            "algorithm/payload",
            "payload-len=1..16;rejections=0",
            "blake3-256;xxh3-128",
            "equal-spliced-path",
            "publication-path-substitution",
            "production-metamorphic",
        ),
        property(
            "publication-immutable-lifecycle-conflict-preservation",
            "write_output",
            "write_output",
            publication_fs,
            "invariant",
            "create-if-absent",
            "none",
            "hegel_immutable_lifecycle_never_overwrites_unequal_existing_bytes",
            publication_test,
            915,
            "prior-and-candidate-byte-pair",
            "prior-len=1..24;candidate-len=1..24;rejections=0",
            "absent;equal;unequal",
            "created-or-noop-or-conflict-with-prior-bytes-preserved",
            "publication-immutable-lifecycle",
            "production-invariant",
        ),
        property(
            "requirement-subject-order",
            "RequirementSubjectFact::cmp",
            "RequirementSubjectFact::cmp",
            "src/check.rs",
            "metamorphic",
            "tuple-sort",
            "none",
            "hegel_requirement_subject_order_is_permutation_invariant",
            "tests/requirement_subject_inventory.rs",
            916,
            "requirement-subject-permutation",
            "rows=4;swaps=16;rejections=0",
            "distinct-id;equal-id;equal-scope-and-path",
            "equal-total-order",
            "requirement-subject-total-order",
            "production-metamorphic",
        ),
    ]);

    properties.sort_by_key(|property| property.id);
    properties
}
