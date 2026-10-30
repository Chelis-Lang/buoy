#![doc = "Fail-closed completeness contract for `fixtures/trace-publication/v1/manifest.tsv`."]
#![allow(missing_docs, clippy::expect_used)]

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

#[path = "support/manifest_execution.rs"]
mod manifest_execution;

use manifest_execution::{MANIFEST_HEADER, MANIFEST_PATH, MANIFEST_SCHEMA as SCHEMA};

const REQUIRED_FAMILIES: [&str; 8] = ["TPT", "PBD", "RTP", "ETL", "REH", "OBW", "CAB", "DEV"];
const DELTA_SPEC_ROOT: &str =
    "openspec/changes/archive/2026-08-25-add-deterministic-trace-publication-tooling/specs";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ManifestError {
    EmptyManifest,
    HeaderMismatch,
    MalformedRow(usize),
    UnknownFamily(usize),
    MissingFamily(&'static str),
    DuplicateScenario(usize),
}

/// Validates the manifest text against its fail-closed completeness
/// contract: a stable header, eight well-formed fields on every row
/// (including a nonempty scenario slug and a nonempty `focused_test`
/// binding this row to one real, executable test), a requirement ID whose
/// family prefix is one of the eight declared families, one row per
/// `(requirement, scenario)` key, and at least one row present for every
/// declared family.
///
/// # Errors
///
/// Returns the first [`ManifestError`] encountered.
fn validate_manifest(text: &str) -> Result<usize, ManifestError> {
    let mut lines = text.lines();
    let header = lines.next().ok_or(ManifestError::EmptyManifest)?;
    if header != MANIFEST_HEADER {
        return Err(ManifestError::HeaderMismatch);
    }
    let mut families = BTreeSet::new();
    let mut keys = BTreeSet::new();
    let mut count = 0_usize;
    for (index, line) in lines.enumerate().filter(|(_, line)| !line.is_empty()) {
        let fields = line.split('\t').collect::<Vec<_>>();
        if fields.len() != 8
            || fields[0] != SCHEMA
            || fields[1].is_empty()
            || fields[2].is_empty()
            || fields[4].is_empty()
            || fields[7].is_empty()
        {
            return Err(ManifestError::MalformedRow(index + 2));
        }
        if !matches!(fields[3], "positive" | "negative") {
            return Err(ManifestError::MalformedRow(index + 2));
        }
        // A scenario slug is derived from its heading, so it can only ever
        // contain lowercase alphanumerics separated by single dashes.
        if !fields[2].chars().all(|character| {
            character.is_ascii_lowercase() || character.is_ascii_digit() || character == '-'
        }) || fields[2].starts_with('-')
            || fields[2].ends_with('-')
            || fields[2].contains("--")
        {
            return Err(ManifestError::MalformedRow(index + 2));
        }
        // A scenario's declared polarity must agree with the polarity its
        // own slug records, so the two can never drift.
        let slug_polarity = if fields[2].starts_with("positive-") {
            "positive"
        } else if fields[2].starts_with("negative-") {
            "negative"
        } else {
            return Err(ManifestError::MalformedRow(index + 2));
        };
        if slug_polarity != fields[3] {
            return Err(ManifestError::MalformedRow(index + 2));
        }
        // An exit class is a real process exit code, never free text.
        if fields[5].parse::<u8>().is_err() {
            return Err(ManifestError::MalformedRow(index + 2));
        }
        let family = fields[1]
            .split_once('-')
            .map_or(fields[1], |(family, _)| family);
        if !REQUIRED_FAMILIES.contains(&family) {
            return Err(ManifestError::UnknownFamily(index + 2));
        }
        if !keys.insert(format!("{}/{}", fields[1], fields[2])) {
            return Err(ManifestError::DuplicateScenario(index + 2));
        }
        families.insert(family);
        count += 1;
    }
    for family in REQUIRED_FAMILIES {
        if !families.contains(family) {
            return Err(ManifestError::MissingFamily(family));
        }
    }
    Ok(count)
}

/// One malformed-row probe text built from the real header.
fn one_row(row: &str) -> String {
    format!("{MANIFEST_HEADER}\n{row}")
}

#[test]
fn committed_manifest_covers_every_required_family() {
    let text = std::fs::read_to_string(MANIFEST_PATH).expect("trace-publication manifest");
    let count = validate_manifest(&text).expect("manifest must satisfy its completeness contract");
    assert!(count >= REQUIRED_FAMILIES.len());
}

#[test]
fn checker_fails_closed_on_an_omitted_family() {
    let text = std::fs::read_to_string(MANIFEST_PATH).expect("trace-publication manifest");
    for family in REQUIRED_FAMILIES {
        let without_family = text
            .lines()
            .filter(|line| {
                line.starts_with("schema\t")
                    || !line.split('\t').nth(1).is_some_and(|requirement| {
                        requirement
                            .split_once('-')
                            .map_or(requirement, |(prefix, _)| prefix)
                            == family
                    })
            })
            .collect::<Vec<_>>()
            .join("\n");
        assert_eq!(
            validate_manifest(&without_family),
            Err(ManifestError::MissingFamily(family)),
            "omitting every {family} row must be rejected"
        );
    }
}

#[test]
fn checker_fails_closed_on_a_malformed_row() {
    assert_eq!(
        validate_manifest(&one_row("not-enough-fields")),
        Err(ManifestError::MalformedRow(2))
    );
    assert_eq!(
        validate_manifest("wrong header"),
        Err(ManifestError::HeaderMismatch)
    );
    assert_eq!(validate_manifest(""), Err(ManifestError::EmptyManifest));
    // The v2 header (no `scenario` column) must never be accepted again.
    assert_eq!(
        validate_manifest(
            "schema\trequirement\tpolarity\tcommand\texit_class\tdiagnostic\tfocused_test\n"
        ),
        Err(ManifestError::HeaderMismatch)
    );
    // A nonnumeric exit class is not a real process exit code.
    assert_eq!(
        validate_manifest(&one_row(&format!(
            "{SCHEMA}\tTPT-001\tpositive-x\tpositive\tpublication check\tblocked\t\tsome::test"
        ))),
        Err(ManifestError::MalformedRow(2))
    );
    // A slug whose recorded polarity contradicts the polarity column.
    assert_eq!(
        validate_manifest(&one_row(&format!(
            "{SCHEMA}\tTPT-001\tnegative-x\tpositive\tpublication check\t0\t\tsome::test"
        ))),
        Err(ManifestError::MalformedRow(2))
    );
    // A slug that is not a derived heading slug.
    assert_eq!(
        validate_manifest(&one_row(&format!(
            "{SCHEMA}\tTPT-001\tPositive - Raw Heading\tpositive\tpublication check\t0\t\tsome::test"
        ))),
        Err(ManifestError::MalformedRow(2))
    );
}

#[test]
fn checker_fails_closed_on_an_unknown_family() {
    assert_eq!(
        validate_manifest(&one_row(&format!(
            "{SCHEMA}\tXYZ-001\tpositive-x\tpositive\tsomething\t0\t\tsome::test"
        ))),
        Err(ManifestError::UnknownFamily(2))
    );
}

#[test]
fn checker_fails_closed_on_a_missing_focused_test() {
    assert_eq!(
        validate_manifest(&one_row(&format!(
            "{SCHEMA}\tTPT-001\tpositive-x\tpositive\tsomething\t0\t\t"
        ))),
        Err(ManifestError::MalformedRow(2))
    );
}

#[test]
fn checker_fails_closed_on_a_duplicated_scenario_row() {
    let row = format!("{SCHEMA}\tTPT-001\tpositive-x\tpositive\tsomething\t0\t\tsome::test");
    assert_eq!(
        validate_manifest(&format!("{MANIFEST_HEADER}\n{row}\n{row}")),
        Err(ManifestError::DuplicateScenario(3))
    );
}

/// One scenario the delta specs declare: its owning requirement ID and the
/// deterministic slug derived from its heading text.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct DeclaredScenario {
    requirement: String,
    scenario: String,
}

/// The deterministic slug for one `#### Scenario:` heading: lowercase, with
/// every run of non-alphanumeric bytes collapsed to a single `-`.
fn slug(heading: &str) -> String {
    let mut out = String::new();
    for character in heading.trim().to_ascii_lowercase().chars() {
        if character.is_ascii_alphanumeric() {
            out.push(character);
        } else if !out.ends_with('-') {
            out.push('-');
        }
    }
    out.trim_matches('-').to_owned()
}

/// Round-3 finding H4 and round-4 finding H5: neither "at least one row per
/// declared family" nor "one positive and one negative row per requirement"
/// is the complete contract. The eight delta specs declare many scenarios
/// under one requirement ID, and every single one of them must have its own
/// executable manifest row.
///
/// This walks the real delta spec files under
/// `openspec/changes/.../specs/**/spec.md` (never a hand-maintained
/// duplicate list that could silently drift from them) and returns every
/// declared `(requirement, scenario)` pair in file order.
fn scenarios_from_delta_specs() -> Vec<DeclaredScenario> {
    let mut declared = Vec::new();
    let root = Path::new(DELTA_SPEC_ROOT);
    let mut capability_dirs: Vec<_> = fs::read_dir(root)
        .expect("delta spec capability directories")
        .map(|entry| entry.expect("delta spec directory entry").path())
        .collect();
    capability_dirs.sort();
    assert!(
        !capability_dirs.is_empty(),
        "the delta spec root must declare at least one capability directory"
    );
    for capability_dir in capability_dirs {
        let spec_path = capability_dir.join("spec.md");
        if !spec_path.is_file() {
            continue;
        }
        let text = fs::read_to_string(&spec_path).expect("delta spec text");
        let mut requirement: Option<String> = None;
        for line in text.lines() {
            if let Some(rest) = line.strip_prefix("### Requirement: ") {
                requirement = Some(
                    rest.split_whitespace()
                        .next()
                        .expect("requirement heading names its stable ID first")
                        .to_owned(),
                );
            } else if let Some(heading) = line.strip_prefix("#### Scenario: ") {
                let owner = requirement.clone().expect(
                    "every scenario heading in a delta spec follows its requirement heading",
                );
                declared.push(DeclaredScenario {
                    requirement: owner,
                    scenario: slug(heading),
                });
            }
        }
    }
    declared
}

/// Every requirement ID the delta specs declare.
fn requirement_ids_from_delta_specs() -> BTreeSet<String> {
    scenarios_from_delta_specs()
        .into_iter()
        .map(|declared| declared.requirement)
        .collect()
}

/// Every `(requirement, scenario)` key a well-formed manifest text declares,
/// mapped to that row's polarity. Assumes `validate_manifest` has already
/// accepted the text.
fn manifest_scenarios(text: &str) -> BTreeMap<(String, String), String> {
    let mut rows = BTreeMap::new();
    for line in text.lines().skip(1).filter(|line| !line.is_empty()) {
        let fields: Vec<&str> = line.split('\t').collect();
        rows.insert(
            (fields[1].to_owned(), fields[2].to_owned()),
            fields[3].to_owned(),
        );
    }
    rows
}

/// Fails closed by returning EVERY missing scenario row -- never stopping at
/// the first -- when the manifest omits a row for any scenario the delta
/// specs declare.
fn missing_scenarios(manifest_text: &str, declared: &[DeclaredScenario]) -> Vec<String> {
    let rows = manifest_scenarios(manifest_text);
    declared
        .iter()
        .filter(|scenario| {
            !rows.contains_key(&(scenario.requirement.clone(), scenario.scenario.clone()))
        })
        .map(|scenario| format!("{}/{}", scenario.requirement, scenario.scenario))
        .collect()
}

/// Every manifest row that names a `(requirement, scenario)` key the delta
/// specs do not declare: a stale row is as much a drift fault as a missing
/// one.
fn undeclared_rows(manifest_text: &str, declared: &[DeclaredScenario]) -> Vec<String> {
    let expected: BTreeSet<(String, String)> = declared
        .iter()
        .map(|scenario| (scenario.requirement.clone(), scenario.scenario.clone()))
        .collect();
    manifest_scenarios(manifest_text)
        .into_keys()
        .filter(|key| !expected.contains(key))
        .map(|(requirement, scenario)| format!("{requirement}/{scenario}"))
        .collect()
}

#[test]
fn committed_manifest_covers_every_delta_spec_scenario() {
    let text = fs::read_to_string(MANIFEST_PATH).expect("trace-publication manifest");
    let count =
        validate_manifest(&text).expect("manifest must satisfy its base completeness contract");
    let declared = scenarios_from_delta_specs();
    assert!(
        declared.len() > requirement_ids_from_delta_specs().len(),
        "the delta specs must declare more scenarios than requirements; \
         otherwise the per-scenario contract is indistinguishable from the \
         per-requirement one it replaced"
    );
    let missing = missing_scenarios(&text, &declared);
    assert!(
        missing.is_empty(),
        "the manifest is missing a row for these {} scenarios:\n{}",
        missing.len(),
        missing.join("\n")
    );
    let stale = undeclared_rows(&text, &declared);
    assert!(
        stale.is_empty(),
        "the manifest declares these {} rows for scenarios the delta specs do not declare:\n{}",
        stale.len(),
        stale.join("\n")
    );
    assert_eq!(
        count,
        declared.len(),
        "the manifest must carry exactly one row per declared scenario"
    );
}

#[test]
fn committed_manifest_covers_every_delta_spec_requirement_in_both_polarities() {
    let text = fs::read_to_string(MANIFEST_PATH).expect("trace-publication manifest");
    validate_manifest(&text).expect("manifest must satisfy its base completeness contract");
    let required = requirement_ids_from_delta_specs();
    assert!(
        required.len() >= REQUIRED_FAMILIES.len(),
        "the delta specs must declare at least one requirement per family"
    );
    let rows = manifest_scenarios(&text);
    let mut missing = Vec::new();
    for requirement in &required {
        for polarity in ["positive", "negative"] {
            if !rows.iter().any(|((row_requirement, _), row_polarity)| {
                row_requirement == requirement && row_polarity == polarity
            }) {
                missing.push(format!("{requirement}/{polarity}"));
            }
        }
    }
    assert!(
        missing.is_empty(),
        "the manifest is missing a row for these (requirement, polarity) pairs: {missing:?}"
    );
}

#[test]
fn checker_fails_closed_on_a_missing_scenario_row_not_vacuously() {
    let text = fs::read_to_string(MANIFEST_PATH).expect("trace-publication manifest");
    let declared = scenarios_from_delta_specs();
    assert!(
        missing_scenarios(&text, &declared).is_empty(),
        "the positive case must not be vacuous"
    );

    // Removing EACH real committed scenario row, one at a time, must be
    // detected as exactly that missing scenario -- proving the per-scenario
    // completeness check is a real, fail-closed control for every row and
    // not a coincidence of one lucky sample.
    for scenario in &declared {
        let marker = format!("\t{}\t{}\t", scenario.requirement, scenario.scenario);
        let without_row = text
            .lines()
            .filter(|line| line.starts_with("schema\t") || !line.contains(&marker))
            .collect::<Vec<_>>()
            .join("\n");
        assert_ne!(
            without_row.lines().count(),
            text.lines().count(),
            "{}/{} must be a real committed row",
            scenario.requirement,
            scenario.scenario
        );
        assert_eq!(
            missing_scenarios(&without_row, &declared),
            vec![format!("{}/{}", scenario.requirement, scenario.scenario)],
            "removing one scenario row must be detected"
        );
    }
}

#[test]
fn checker_fails_closed_on_a_scenario_row_the_specs_never_declared() {
    let text = fs::read_to_string(MANIFEST_PATH).expect("trace-publication manifest");
    let declared = scenarios_from_delta_specs();
    assert!(undeclared_rows(&text, &declared).is_empty());
    let invented = format!(
        "{text}{SCHEMA}\tTPT-001\tpositive-a-scenario-no-spec-declares\tpositive\tpublication check\t0\t\tsome::test\n"
    );
    assert_eq!(
        undeclared_rows(&invented, &declared),
        vec![String::from("TPT-001/positive-a-scenario-no-spec-declares")]
    );
}

#[test]
fn scenario_slugs_are_derived_deterministically_from_their_real_headings() {
    assert_eq!(
        slug("Positive - complete publication plan is accepted"),
        "positive-complete-publication-plan-is-accepted"
    );
    assert_eq!(
        slug("Negative - XXH3 binding is selected"),
        "negative-xxh3-binding-is-selected"
    );
    assert_eq!(slug("  Positive -- spaced  "), "positive-spaced");
}
