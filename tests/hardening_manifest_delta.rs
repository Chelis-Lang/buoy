#![doc = "Change-local hardening-manifest completeness contract for REH-012 and REH-013."]
#![allow(
    missing_docs,
    clippy::expect_used,
    clippy::panic,
    clippy::too_many_lines
)]

//! The repository-wide `fixtures/hardening/v2/hardening-manifest.tsv` checker
//! derives its requirement inventory from `openspec/specs/**`.
//! REH-012 and REH-013 now exist in those standing specifications.
//! This archived-delta contract verifies that every archived scenario still
//! has one manifest row and one real, passing focused oracle.
//!
//! Nothing here is a hand-maintained requirement list. The requirement and
//! scenario sets come from the archived delta specifications. The test also
//! verifies that every archived requirement exists in the standing specs.
//! The recognized independent-oracle vocabulary comes from the repository
//! manifest. The standing checker supplies the required oracle and command
//! mappings, so the archived delta cannot drift from repository policy.
//!
//! `manifest_problems` reports rather than panics, so the omission and
//! mutation self-tests can prove the checker is not vacuous by running it
//! over deliberately damaged in-memory copies of the real committed manifest.
//!
//! Static agreement is necessary but not sufficient, so this file also
//! EXECUTES the declared command and the derived scenario oracles as real
//! subprocesses and requires a nonzero executed test count parsed from real
//! `cargo test` output. A command that compiles zero tests still exits
//! successfully, so counting is the only way to tell a passing oracle from an
//! absent one.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, PoisonError};

static FOCUSED_COMMAND_LOCK: Mutex<()> = Mutex::new(());

const CHANGE_ROOT: &str =
    "openspec/changes/archive/2026-08-25-add-deterministic-trace-publication-tooling";
const DELTA_MANIFEST: &str = "openspec/changes/archive/2026-08-25-add-deterministic-trace-publication-tooling/hardening-manifest-delta.tsv";
/// The standing specifications must contain each archived REH requirement.
const STANDING_SPEC_ROOT: &str = "openspec/specs";
/// The real, repository-wide hardening manifest. Its column 6 is the only
/// place the recognized independent-oracle vocabulary is actually written
/// down as committed data, and its column 9 is the acceptance command every
/// real hardening row already answers to.
const REPOSITORY_MANIFEST: &str = "fixtures/hardening/v2/hardening-manifest.tsv";
/// The real repository hardening checker. Its `independent_oracle` and
/// `focused_command` authority mappings decide which oracle and which command
/// the REH requirements must name, and its `FINAL_ORACLE` constant decides the
/// acceptance command.
const REPOSITORY_CHECKER: &str = "tools/check_hardening_manifest.rs";
/// The real devenv workflow policy, which names the acceptance lifecycle the
/// final oracle has to be a real invocation of.
const WORKFLOW_POLICY: &str = "nix/devenv/workflow-policy.toml";

/// The requirement family this change extends.
const FAMILY: &str = "REH";
const SCHEMA: &str = "buoy-hardening-fixture-manifest/v1";
const HEADER: &str = "schema\trequirement\tscenario\tpolarity\tfixture_id\tindependent_oracle\texpected\tfocused_command\tfinal_oracle";
/// The wrapper a declared focused command carries so it runs inside the
/// pinned devenv toolchain. These controls already run inside that toolchain,
/// so they drop exactly this prefix and execute the remainder verbatim.
const DEVENV_SHELL: &str = "devenv shell -- ";

/// Mirrors `tools/check_hardening_manifest.rs::slugify` exactly so delta
/// manifest slugs stay comparable with the real checker's convention.
fn slugify(input: &str) -> String {
    let mut result = String::new();
    let mut separator = false;
    for character in input.chars().flat_map(char::to_lowercase) {
        if character.is_alphanumeric() {
            if separator && !result.is_empty() {
                result.push('-');
            }
            result.push(character);
            separator = false;
        } else {
            separator = true;
        }
    }
    result
}

// ---------------------------------------------------------------------------
// Derived requirement and scenario inventory.
// ---------------------------------------------------------------------------

/// Every `spec.md` beneath `root`, sorted.
fn spec_paths(root: &Path) -> Vec<PathBuf> {
    fn visit(directory: &Path, found: &mut Vec<PathBuf>) {
        let Ok(entries) = fs::read_dir(directory) else {
            return;
        };
        for entry in entries {
            let entry = entry.expect("directory entry");
            let path = entry.path();
            if entry.file_type().expect("file type").is_dir() {
                visit(&path, found);
            } else if path.file_name().and_then(|name| name.to_str()) == Some("spec.md") {
                found.push(path);
            }
        }
    }
    let mut found = Vec::new();
    visit(root, &mut found);
    found.sort();
    found
}

/// Every requirement identifier in the `FAMILY` declared by any `spec.md`
/// beneath `root`.
fn family_requirements(root: &Path) -> BTreeSet<String> {
    let mut requirements = BTreeSet::new();
    for path in spec_paths(root) {
        let text = fs::read_to_string(&path).expect("spec file");
        for line in text.lines() {
            if let Some(heading) = line.strip_prefix("### Requirement: ") {
                if let Some(id) = heading.split_whitespace().next() {
                    if id.starts_with(&format!("{FAMILY}-")) {
                        requirements.insert(id.to_owned());
                    }
                }
            }
        }
    }
    requirements
}

/// The archived REH requirements, all of which must exist in the standing specs.
fn delta_requirements() -> BTreeSet<String> {
    let declared = family_requirements(&PathBuf::from(CHANGE_ROOT).join("specs"));
    let standing = family_requirements(Path::new(STANDING_SPEC_ROOT));
    let missing = declared
        .difference(&standing)
        .cloned()
        .collect::<BTreeSet<_>>();
    assert!(
        !declared.is_empty(),
        "the archived change must declare at least one {FAMILY} requirement"
    );
    assert!(
        missing.is_empty(),
        "the standing specs must contain every archived {FAMILY} requirement: {missing:?}"
    );
    declared
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct ScenarioKey {
    requirement: String,
    polarity: String,
    slug: String,
}

impl ScenarioKey {
    fn label(&self) -> String {
        format!("{}/{}/{}", self.requirement, self.polarity, self.slug)
    }

    /// Mirrors `tools/check_hardening_manifest.rs::fixture_id`.
    fn fixture_id(&self) -> String {
        format!(
            "hardening-{}-{}-{}",
            self.requirement.to_ascii_lowercase(),
            self.polarity,
            self.slug
        )
    }

    /// Mirrors `tools/check_hardening_manifest.rs::expected`.
    fn expected(&self) -> String {
        if self.polarity == "positive" {
            String::from("pass")
        } else {
            format!(
                "{}-{}",
                self.requirement,
                self.slug.to_ascii_uppercase().replace('-', "_")
            )
        }
    }

    /// The one Rust test that is this scenario's own mechanism oracle. The
    /// binding is mechanical, so a row cannot silently borrow some other
    /// real, passing test that happens to exist.
    fn oracle_test_name(&self) -> String {
        format!(
            "{}_{}_{}",
            self.requirement.to_ascii_lowercase().replace('-', "_"),
            self.polarity,
            self.slug.replace('-', "_")
        )
    }
}

/// Every declared scenario for `requirements`, parsed out of the real delta
/// spec files.
fn declared_scenarios(requirements: &BTreeSet<String>) -> BTreeSet<ScenarioKey> {
    let mut scenarios = BTreeSet::new();
    for path in spec_paths(&PathBuf::from(CHANGE_ROOT).join("specs")) {
        let text = fs::read_to_string(&path).expect("delta spec file");
        let mut requirement = None;
        for line in text.lines() {
            if let Some(heading) = line.strip_prefix("### Requirement: ") {
                requirement = heading.split_whitespace().next().map(str::to_owned);
            } else if let Some(heading) = line.strip_prefix("#### Scenario: ") {
                let Some(current) = requirement.as_deref() else {
                    continue;
                };
                if !requirements.contains(current) {
                    continue;
                }
                let (polarity, title) = heading
                    .split_once(" - ")
                    .expect("every scenario heading declares Positive/Negative - title");
                scenarios.insert(ScenarioKey {
                    requirement: current.to_owned(),
                    polarity: polarity.to_ascii_lowercase(),
                    slug: slugify(title),
                });
            }
        }
    }
    scenarios
}

// ---------------------------------------------------------------------------
// Manifest parsing.
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
struct ManifestRow {
    line: usize,
    schema: String,
    key: ScenarioKey,
    fixture_id: String,
    independent_oracle: String,
    expected: String,
    focused_command: String,
    final_oracle: String,
}

/// Parses the manifest body, reporting malformed rows instead of panicking.
fn rows_from_manifest(text: &str) -> (Vec<ManifestRow>, Vec<String>) {
    let mut rows = Vec::new();
    let mut problems = Vec::new();
    for (index, line) in text.lines().skip(1).enumerate() {
        if line.is_empty() {
            continue;
        }
        let fields = line.split('\t').collect::<Vec<_>>();
        if fields.len() != 9 {
            problems.push(format!(
                "manifest line {} is malformed ({} columns, expected 9)",
                index + 2,
                fields.len()
            ));
            continue;
        }
        rows.push(ManifestRow {
            line: index + 2,
            schema: fields[0].to_owned(),
            key: ScenarioKey {
                requirement: fields[1].to_owned(),
                polarity: fields[3].to_owned(),
                slug: fields[2].to_owned(),
            },
            fixture_id: fields[4].to_owned(),
            independent_oracle: fields[5].to_owned(),
            expected: fields[6].to_owned(),
            focused_command: fields[7].to_owned(),
            final_oracle: fields[8].to_owned(),
        });
    }
    (rows, problems)
}

// ---------------------------------------------------------------------------
// Real repository authorities.
// ---------------------------------------------------------------------------

/// The oracle authorities this repository really declares, read out of
/// committed repository artifacts rather than restated as literals here.
#[derive(Debug)]
struct RepositoryOracles {
    /// Every independent-oracle name the real committed hardening manifest
    /// uses. An oracle outside this vocabulary is not a recognized authority.
    recognized: BTreeSet<String>,
    /// The independent oracle the real checker's authority mapping assigns
    /// to the REH requirement family.
    family_oracle: String,
    /// The focused command the real checker's `focused_command` authority
    /// mapping assigns to each delta requirement. This is the whole of
    /// finding "focused-command ownership": the standing checker owns this
    /// column, so the delta manifest must carry exactly what it will emit.
    focused_commands: BTreeMap<String, String>,
    /// The acceptance command the real checker's `FINAL_ORACLE` names, which
    /// is also the single final oracle every real manifest row carries.
    acceptance_command: String,
}

/// Extracts the string literal that follows `needle` in `text`.
fn literal_after(text: &str, needle: &str, source: &str) -> String {
    let start = text
        .find(needle)
        .unwrap_or_else(|| panic!("{source} no longer contains {needle:?}"))
        + needle.len();
    let rest = &text[start..];
    let end = rest
        .find('"')
        .unwrap_or_else(|| panic!("{source} has an unterminated literal after {needle:?}"));
    rest[..end].to_owned()
}

/// Reads the focused command the real checker's `focused_command` mapping
/// assigns `requirement`: the arm that mentions the requirement literal, then
/// the first string literal after that arm's `=>`.
fn checker_focused_command(checker: &str, requirement: &str) -> String {
    let mapping = checker
        .find("fn focused_command(")
        .expect("the repository checker declares a focused_command authority mapping");
    let region = &checker[mapping..];
    let arm = region
        .find(&format!("\"{requirement}\""))
        .unwrap_or_else(|| panic!("the checker's focused_command mapping omits {requirement}"));
    let after = region[arm..]
        .find("=> ")
        .unwrap_or_else(|| panic!("the checker's {requirement} arm has no body"));
    literal_after(&region[arm + after..], "\"", REPOSITORY_CHECKER)
}

fn repository_oracles(requirements: &BTreeSet<String>) -> RepositoryOracles {
    let manifest = fs::read_to_string(REPOSITORY_MANIFEST).expect("repository hardening manifest");
    let mut recognized = BTreeSet::new();
    let mut final_oracles = BTreeSet::new();
    for line in manifest.lines().skip(1).filter(|line| !line.is_empty()) {
        let fields = line.split('\t').collect::<Vec<_>>();
        assert_eq!(
            fields.len(),
            9,
            "the repository hardening manifest changed shape: {line}"
        );
        recognized.insert(fields[5].to_owned());
        final_oracles.insert(fields[8].to_owned());
    }
    assert!(
        !recognized.is_empty(),
        "the repository hardening manifest declares no independent oracles"
    );
    assert_eq!(
        final_oracles.len(),
        1,
        "the repository hardening manifest must carry exactly one acceptance command, found {final_oracles:?}"
    );

    let checker = fs::read_to_string(REPOSITORY_CHECKER).expect("repository hardening checker");
    // Scope the family lookup to the checker's `independent_oracle` authority
    // mapping: `focused_command` right below it matches the same family arm
    // shape but answers a different question.
    let mapping_start = checker
        .find("fn independent_oracle(")
        .expect("the repository checker declares an independent_oracle authority mapping");
    let family_oracle = literal_after(
        &checker[mapping_start..],
        &format!("\"{FAMILY}\" => \""),
        REPOSITORY_CHECKER,
    );
    let acceptance_command = literal_after(
        &checker,
        "const FINAL_ORACLE: &str = \"",
        REPOSITORY_CHECKER,
    );
    let focused_commands = requirements
        .iter()
        .map(|requirement| {
            (
                requirement.clone(),
                checker_focused_command(&checker, requirement),
            )
        })
        .collect::<BTreeMap<_, _>>();

    assert!(
        recognized.contains(&family_oracle),
        "the checker's {FAMILY}-family oracle {family_oracle:?} is not used by any real manifest row"
    );
    assert!(
        final_oracles.contains(&acceptance_command),
        "the checker's FINAL_ORACLE {acceptance_command:?} is not the manifest's acceptance command {final_oracles:?}"
    );

    RepositoryOracles {
        recognized,
        family_oracle,
        focused_commands,
        acceptance_command,
    }
}

// ---------------------------------------------------------------------------
// The contract.
// ---------------------------------------------------------------------------

/// Every way the delta manifest can fail its contract, named.
///
/// Reporting rather than panicking is what lets the omission and mutation
/// self-tests below run this same function over damaged copies of the real
/// committed manifest and require that each damage is caught.
fn manifest_problems(
    requirements: &BTreeSet<String>,
    manifest: &str,
    oracles: &RepositoryOracles,
) -> Vec<String> {
    let mut problems = Vec::new();

    match manifest.lines().next() {
        Some(header) if header == HEADER => {}
        header => problems.push(format!(
            "delta manifest header differs from the closed schema: {header:?}"
        )),
    }

    let declared = declared_scenarios(requirements);
    let (rows, parse_problems) = rows_from_manifest(manifest);
    problems.extend(parse_problems);

    let mut seen: BTreeMap<ScenarioKey, usize> = BTreeMap::new();
    let mut fixture_ids: BTreeSet<String> = BTreeSet::new();
    for row in &rows {
        let label = row.key.label();
        if let Some(first) = seen.insert(row.key.clone(), row.line) {
            problems.push(format!(
                "delta manifest line {} duplicates scenario {label} already declared on line {first}",
                row.line
            ));
        }
        if !fixture_ids.insert(row.fixture_id.clone()) {
            problems.push(format!(
                "delta manifest line {} duplicates fixture {}",
                row.line, row.fixture_id
            ));
        }
        if row.schema != SCHEMA {
            problems.push(format!(
                "delta manifest line {} for {label} declares schema {} rather than {SCHEMA}",
                row.line, row.schema
            ));
        }
        if !matches!(row.key.polarity.as_str(), "positive" | "negative") {
            problems.push(format!(
                "delta manifest line {} for {label} declares an unknown polarity {}",
                row.line, row.key.polarity
            ));
        }
        if row.fixture_id != row.key.fixture_id() {
            problems.push(format!(
                "delta manifest line {} for {label} substitutes fixture identifier {} for {}",
                row.line,
                row.fixture_id,
                row.key.fixture_id()
            ));
        }
        // The `expected` column is this scenario's exit class: `pass` for a
        // positive oracle, the requirement's own stable diagnostic for a
        // negative one. Swapping the two families is an exit-class error.
        if row.expected != row.key.expected() {
            problems.push(format!(
                "delta manifest line {} for {label} substitutes expectation {} for {}",
                row.line,
                row.expected,
                row.key.expected()
            ));
        }
        if !oracles.recognized.contains(&row.independent_oracle) {
            problems.push(format!(
                "delta manifest line {} for {label} names independent oracle {}, which no real \
                 repository hardening row recognizes",
                row.line, row.independent_oracle
            ));
        } else if row.independent_oracle != oracles.family_oracle {
            problems.push(format!(
                "delta manifest line {} for {label} names independent oracle {}, but the real \
                 checker's authority mapping assigns the {FAMILY} family {}",
                row.line, row.independent_oracle, oracles.family_oracle
            ));
        }
        if row.final_oracle != oracles.acceptance_command {
            problems.push(format!(
                "delta manifest line {} for {label} names final oracle {}, not the repository \
                 acceptance command {}",
                row.line, row.final_oracle, oracles.acceptance_command
            ));
        }
        problems.extend(focused_command_problems(row, oracles));
    }

    for key in &declared {
        if !seen.contains_key(key) {
            problems.push(format!("delta manifest omits scenario {}", key.label()));
        }
    }
    for key in seen.keys() {
        if !declared.contains(key) {
            problems.push(format!(
                "delta manifest declares scenario {}, which the delta spec does not contain",
                key.label()
            ));
        }
    }
    for requirement in requirements {
        for polarity in ["positive", "negative"] {
            if !declared
                .iter()
                .any(|key| &key.requirement == requirement && key.polarity == polarity)
            {
                problems.push(format!("{requirement} declares no {polarity} oracle"));
            }
        }
    }

    problems
}

/// The focused command must be exactly the command the standing checker's
/// `focused_command` authority mapping owns for this requirement -- including
/// every feature flag -- and the test target it names must really carry this
/// scenario's own mechanism oracle.
fn focused_command_problems(row: &ManifestRow, oracles: &RepositoryOracles) -> Vec<String> {
    let label = row.key.label();
    let Some(owned) = oracles.focused_commands.get(&row.key.requirement) else {
        return vec![format!(
            "delta manifest line {} for {label} names requirement {}, for which the real checker \
             declares no focused command",
            row.line, row.key.requirement
        )];
    };
    if &row.focused_command != owned {
        return vec![format!(
            "delta manifest line {} for {label} declares focused command {}, not the command the \
             real checker owns for {}: {owned}",
            row.line, row.focused_command, row.key.requirement
        )];
    }
    let Some(binary) = owned
        .split_whitespace()
        .skip_while(|token| *token != "--test")
        .nth(1)
    else {
        return vec![format!(
            "delta manifest line {} for {label} names no focused test target: {owned}",
            row.line
        )];
    };
    let path = format!("tests/{binary}.rs");
    let Ok(source) = fs::read_to_string(&path) else {
        return vec![format!(
            "delta manifest line {} for {label} names nonexistent test binary {binary}",
            row.line
        )];
    };
    if !source.contains(&format!("fn {}(", row.key.oracle_test_name())) {
        return vec![format!(
            "delta manifest line {} for {label} binds to {binary}, which declares no oracle named \
             {}",
            row.line,
            row.key.oracle_test_name()
        )];
    }
    Vec::new()
}

fn delta_manifest() -> String {
    fs::read_to_string(DELTA_MANIFEST).expect("delta hardening manifest")
}

// ---------------------------------------------------------------------------
// Static contract tests.
// ---------------------------------------------------------------------------

/// The real committed delta manifest satisfies the whole contract: every
/// declared REH-012/REH-013 scenario has exactly one row, and every column
/// of every row re-derives correctly.
#[test]
fn the_committed_delta_manifest_reports_no_problems() {
    let requirements = delta_requirements();
    let oracles = repository_oracles(&requirements);
    let problems = manifest_problems(&requirements, &delta_manifest(), &oracles);
    assert!(
        problems.is_empty(),
        "the committed delta manifest must satisfy its contract, found:\n{}",
        problems.join("\n")
    );
    let declared = declared_scenarios(&requirements);
    let (rows, parse_problems) = rows_from_manifest(&delta_manifest());
    assert!(parse_problems.is_empty());
    assert_eq!(rows.len(), declared.len(), "one row per declared scenario");
    // Non-vacuity: the derived inventory really covers both delta
    // requirements, with a positive and a negative oracle each.
    assert!(
        requirements.len() >= 2,
        "derived requirements {requirements:?}"
    );
    for requirement in &requirements {
        for polarity in ["positive", "negative"] {
            assert!(
                declared
                    .iter()
                    .any(|key| &key.requirement == requirement && key.polarity == polarity),
                "{requirement} must declare an independent {polarity} oracle"
            );
        }
    }
}

/// The independent-oracle, focused-command, and final-oracle columns are
/// checked against real repository authorities, not against literals restated
/// in this test. The focused-command half is the ownership contract: whatever
/// the standing checker will emit for these requirements at sync time is what
/// the delta manifest carries today.
#[test]
fn delta_manifest_oracle_columns_name_real_repository_authorities() {
    let requirements = delta_requirements();
    let oracles = repository_oracles(&requirements);
    let (rows, _) = rows_from_manifest(&delta_manifest());
    assert!(!rows.is_empty());

    for row in &rows {
        assert!(
            oracles.recognized.contains(&row.independent_oracle),
            "{} names an unrecognized independent oracle",
            row.key.label()
        );
        assert_eq!(
            row.independent_oracle,
            oracles.family_oracle,
            "{} must name the {FAMILY}-family oracle the real checker assigns",
            row.key.label()
        );
        assert_eq!(
            Some(&row.focused_command),
            oracles.focused_commands.get(&row.key.requirement),
            "{} must name the focused command the standing checker owns",
            row.key.label()
        );
        assert_eq!(row.final_oracle, oracles.acceptance_command);
    }

    // The owned command is a complete invocation: it names a target and it
    // requests every feature, because these requirements own scenarios behind
    // `coverage-controls` and `blake3-256`.
    for (requirement, command) in &oracles.focused_commands {
        let invocation = command
            .strip_prefix(DEVENV_SHELL)
            .unwrap_or_else(|| panic!("{requirement} command must run in the pinned toolchain"));
        assert!(invocation.starts_with("cargo test "), "{invocation}");
        assert!(
            invocation.contains(" --all-features "),
            "{requirement} command {command} omits a required feature flag"
        );
        assert!(invocation.contains(" --test "), "{command}");
    }

    // The acceptance command is a real invocation of the acceptance
    // lifecycle the devenv workflow policy declares: `devenv test` runs
    // `devenv:enterTest`.
    let policy = fs::read_to_string(WORKFLOW_POLICY).expect("devenv workflow policy");
    let lifecycle = literal_after(&policy, "lifecycle = \"", WORKFLOW_POLICY);
    let subcommand = oracles
        .acceptance_command
        .strip_prefix("devenv ")
        .expect("the acceptance command invokes devenv");
    let lifecycle_subcommand = lifecycle
        .strip_prefix("devenv:enter")
        .expect("the declared lifecycle is a devenv enter phase")
        .to_ascii_lowercase();
    assert_eq!(
        subcommand, lifecycle_subcommand,
        "the final oracle {} must invoke the declared acceptance lifecycle {lifecycle}",
        oracles.acceptance_command
    );
}

/// Omission self-test: removing any single scenario row from a real copy of
/// the committed manifest is detected, and the report names exactly the
/// scenario that went missing -- and nothing else.
#[test]
fn omitting_any_scenario_row_is_detected_and_named() {
    let requirements = delta_requirements();
    let manifest = delta_manifest();
    let oracles = repository_oracles(&requirements);
    let (rows, _) = rows_from_manifest(&manifest);
    assert_eq!(
        rows.len(),
        declared_scenarios(&requirements).len(),
        "the committed manifest must already be complete before it is damaged"
    );
    assert!(
        manifest_problems(&requirements, &manifest, &oracles).is_empty(),
        "the undamaged manifest must report zero faults, or these controls prove nothing"
    );

    let lines: Vec<&str> = manifest.lines().collect();
    for row in &rows {
        let damaged = lines
            .iter()
            .enumerate()
            .filter(|(index, _)| index + 1 != row.line)
            .map(|(_, line)| *line)
            .collect::<Vec<_>>()
            .join("\n")
            + "\n";
        let problems = manifest_problems(&requirements, &damaged, &oracles);
        let wanted = format!("delta manifest omits scenario {}", row.key.label());
        assert!(
            problems.contains(&wanted),
            "omitting {} must be detected and named, found:\n{}",
            row.key.label(),
            problems.join("\n")
        );
        // Nothing else is falsely reported: exactly the one omission.
        assert_eq!(
            problems.len(),
            1,
            "omitting one row must report exactly one problem, found:\n{}",
            problems.join("\n")
        );
    }
}

/// Mutation self-test for the single-field substitutions a reviewer would
/// most plausibly miss. Each mutation changes exactly one column to a value
/// that is superficially plausible -- a real oracle name from another
/// requirement family, the other polarity's exit class, the family command
/// the standing checker used before these requirements got their own, a real
/// but unrelated test target, a command that merely drops a feature flag --
/// and each must be reported as EXACTLY ONE named fault.
#[test]
fn mutating_a_single_row_field_reports_exactly_one_named_fault() {
    let requirements = delta_requirements();
    let manifest = delta_manifest();
    let oracles = repository_oracles(&requirements);
    let (rows, _) = rows_from_manifest(&manifest);
    assert!(
        manifest_problems(&requirements, &manifest, &oracles).is_empty(),
        "the undamaged manifest must report zero faults, or these controls prove nothing"
    );

    // A recognized oracle that is NOT the REH-family authority: proves the
    // independent-oracle column is checked for authority, not just spelling.
    let foreign_oracle = oracles
        .recognized
        .iter()
        .find(|oracle| **oracle != oracles.family_oracle)
        .expect("the repository declares more than one independent oracle")
        .clone();
    // A real, existing test target that is not where these oracles live.
    let unrelated_target = "hardening_manifest_delta";
    assert!(Path::new(&format!("tests/{unrelated_target}.rs")).exists());

    let mut checked = 0_usize;
    for row in &rows {
        let original = manifest
            .lines()
            .nth(row.line - 1)
            .expect("the row line")
            .to_owned();
        let fields: Vec<&str> = original.split('\t').collect();
        let owned = oracles
            .focused_commands
            .get(&row.key.requirement)
            .expect("the owned focused command");
        // The other polarity's exit class: `pass` on a negative row, a real
        // stable diagnostic on a positive row.
        let wrong_exit_class = if row.key.polarity == "positive" {
            format!(
                "{}-{}",
                row.key.requirement,
                row.key.slug.to_ascii_uppercase().replace('-', "_")
            )
        } else {
            String::from("pass")
        };

        let mutations = vec![
            (
                "schema",
                replace_field(&fields, 0, "buoy-hardening-fixture-manifest/v2"),
            ),
            (
                "fixture identifier",
                replace_field(&fields, 4, "hardening-something-else"),
            ),
            (
                "independent oracle (unrecognized)",
                replace_field(&fields, 5, "production-substitution"),
            ),
            (
                "independent oracle (another family's real oracle)",
                replace_field(&fields, 5, &foreign_oracle),
            ),
            (
                "expected diagnostic",
                replace_field(&fields, 6, "REH-000-NOT_THE_DECLARED_DIAGNOSTIC"),
            ),
            (
                "exit class (the other polarity's)",
                replace_field(&fields, 6, &wrong_exit_class),
            ),
            (
                "focused command (feature flag dropped)",
                replace_field(&fields, 7, &owned.replacen(" --all-features", "", 1)),
            ),
            (
                "focused command (the superseded family command)",
                replace_field(&fields, 7, "devenv tasks run buoy:integration"),
            ),
            (
                "focused command (a real but unrelated test target)",
                replace_field(
                    &fields,
                    7,
                    &owned.replacen("publication_filesystem", unrelated_target, 1),
                ),
            ),
            ("final oracle", replace_field(&fields, 8, "historical pass")),
        ];

        for (name, mutated_line) in mutations {
            let damaged = manifest.replace(&original, &mutated_line);
            assert_ne!(
                damaged, manifest,
                "the {name} mutation must change the manifest"
            );
            let problems = manifest_problems(&requirements, &damaged, &oracles);
            assert_eq!(
                problems.len(),
                1,
                "mutating the {name} of {} must report exactly one fault, found:\n{}",
                row.key.label(),
                problems.join("\n")
            );
            assert!(
                problems[0].contains(&row.key.label()),
                "the report for a mutated {name} must name {}, found: {}",
                row.key.label(),
                problems[0]
            );
            checked += 1;
        }
    }
    assert!(
        checked >= 40,
        "the mutation matrix must be real, ran {checked}"
    );
}

/// Mutation self-test for the structural columns. Changing a row's
/// requirement, polarity, or scenario slug detaches it from the scenario it
/// claims to answer for; each must be detected and must name the scenario
/// that has lost its oracle.
#[test]
fn mutating_a_rows_scenario_identity_is_detected() {
    let requirements = delta_requirements();
    let manifest = delta_manifest();
    let oracles = repository_oracles(&requirements);
    let (rows, _) = rows_from_manifest(&manifest);
    let other_requirement = |current: &str| {
        requirements
            .iter()
            .find(|requirement| requirement.as_str() != current)
            .expect("the change declares more than one requirement")
            .clone()
    };

    for row in &rows {
        let original = manifest
            .lines()
            .nth(row.line - 1)
            .expect("the row line")
            .to_owned();
        let fields: Vec<&str> = original.split('\t').collect();
        let flipped = if row.key.polarity == "positive" {
            "negative"
        } else {
            "positive"
        };
        for (name, mutated_line) in [
            (
                "requirement",
                replace_field(&fields, 1, &other_requirement(&row.key.requirement)),
            ),
            (
                "scenario slug",
                replace_field(&fields, 2, "some-other-mechanism"),
            ),
            ("polarity", replace_field(&fields, 3, flipped)),
        ] {
            let damaged = manifest.replace(&original, &mutated_line);
            assert_ne!(damaged, manifest);
            let problems = manifest_problems(&requirements, &damaged, &oracles);
            assert!(
                problems.iter().any(|problem| problem.contains(&format!(
                    "delta manifest omits scenario {}",
                    row.key.label()
                ))),
                "mutating the {name} of {} must report that scenario as unanswered, found:\n{}",
                row.key.label(),
                problems.join("\n")
            );
        }
    }
}

fn replace_field(fields: &[&str], index: usize, value: &str) -> String {
    let mut mutated: Vec<String> = fields.iter().map(|field| (*field).to_owned()).collect();
    value.clone_into(&mut mutated[index]);
    mutated.join("\t")
}

// ---------------------------------------------------------------------------
// Execution controls: the declared command must really run tests.
// ---------------------------------------------------------------------------

/// The number of tests a real `cargo test` run actually executed, summed over
/// every `test result:` line it printed. `cargo test` exits successfully when
/// a filter matches nothing, so the exit status alone cannot tell a passing
/// oracle from an absent one -- only this count can.
fn executed_test_count(stdout: &str) -> usize {
    let mut total = 0;
    let mut saw_result = false;
    for line in stdout.lines() {
        let Some(rest) = line.trim().strip_prefix("test result: ") else {
            continue;
        };
        saw_result = true;
        assert!(
            rest.starts_with("ok."),
            "a focused command must not report a failing result: {line}"
        );
        let passed = rest
            .trim_start_matches("ok.")
            .split(';')
            .next()
            .expect("the passed clause")
            .trim()
            .trim_end_matches(" passed")
            .parse::<usize>()
            .unwrap_or_else(|error| panic!("unparsable test result line {line:?}: {error}"));
        total += passed;
    }
    assert!(saw_result, "cargo test printed no result line:\n{stdout}");
    total
}

/// Runs a declared focused command verbatim, minus only the `devenv shell --`
/// wrapper (this control already runs inside the pinned toolchain), plus any
/// extra arguments, and returns its real standard output.
fn run_focused_output(command: &str, extra: &[&str]) -> String {
    // Cargo feature variants share the CLI output path.
    // Hold the lock until the child test process exits.
    let _guard = FOCUSED_COMMAND_LOCK
        .lock()
        .unwrap_or_else(PoisonError::into_inner);
    let invocation = command
        .strip_prefix(DEVENV_SHELL)
        .unwrap_or_else(|| panic!("focused command must run in the pinned toolchain: {command}"));
    let mut tokens = invocation.split_whitespace();
    assert_eq!(
        tokens.next(),
        Some("cargo"),
        "focused command must be a cargo invocation: {command}"
    );
    let output = std::process::Command::new(env!("CARGO"))
        .args(tokens)
        .args(extra)
        // A stale ambient devenv task file breaks nested cargo invocations.
        .env_remove("DEVENV_TASK_FILE")
        .output()
        .unwrap_or_else(|error| panic!("running {command:?}: {error}"));
    assert!(
        output.status.success(),
        "focused command {command:?} {extra:?} failed:\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).into_owned()
}

/// The number of tests a declared focused command really executed.
fn run_focused(command: &str, extra: &[&str]) -> usize {
    executed_test_count(&run_focused_output(command, extra))
}

/// Every test name a declared focused command would actually compile, taken
/// from the real `--list` output of that exact invocation.
fn run_focused_listing(command: &str) -> BTreeSet<String> {
    let stdout = run_focused_output(command, &["--", "--list"]);
    let names = stdout
        .lines()
        .filter_map(|line| line.strip_suffix(": test"))
        .map(str::to_owned)
        .collect::<BTreeSet<_>>();
    assert!(
        !names.is_empty(),
        "cargo test --list printed no tests for {command:?}:\n{stdout}"
    );
    names
}

/// Executes the declared focused command as a real subprocess and requires a
/// NONZERO executed test count, then proves the requirement is not vacuous by
/// running the same command with its feature flag dropped and requiring that
/// it really does lose oracles.
#[test]
fn the_declared_focused_command_executes_a_nonzero_number_of_tests() {
    let requirements = delta_requirements();
    let oracles = repository_oracles(&requirements);
    let manifest = delta_manifest();
    let (rows, parse_problems) = rows_from_manifest(&manifest);
    assert!(parse_problems.is_empty(), "{parse_problems:?}");

    let commands: BTreeSet<String> = rows.iter().map(|row| row.focused_command.clone()).collect();
    assert!(!commands.is_empty());

    for command in &commands {
        let executed = run_focused(command, &[]);
        assert!(
            executed > 0,
            "declared command {command:?} executed zero tests, so it can never fail"
        );
        let bound = rows
            .iter()
            .filter(|row| &row.focused_command == command)
            .count();
        assert!(
            executed >= bound,
            "declared command {command:?} executed {executed} tests but carries {bound} scenario \
             oracles"
        );

        // Discrimination control: the feature flag is load-bearing. Dropping
        // it from the very same command silently removes oracles while still
        // exiting successfully -- which is exactly what makes an incomplete
        // declared command a vacuous oracle.
        let without_features = command.replacen(" --all-features", "", 1);
        assert_ne!(&without_features, command);
        let degraded = run_focused(&without_features, &[]);
        assert!(
            degraded < executed,
            "dropping --all-features must really remove oracles ({degraded} vs {executed}); if it \
             does not, this control cannot detect an incomplete declared command"
        );
    }
    // The commands the manifest declares are the ones the standing checker
    // owns, so this execution proof transfers to the standing manifest.
    for command in oracles.focused_commands.values() {
        assert!(commands.contains(command));
    }
}

/// Every scenario's own mechanism oracle really exists and really passes when
/// run alone through the declared command. Running each oracle with `--exact`
/// and requiring EXACTLY ONE executed test is what forbids a row from being
/// answered by a renamed, deleted, or feature-erased test.
#[test]
fn every_scenario_oracle_runs_alone_and_passes() {
    let requirements = delta_requirements();
    let oracles = repository_oracles(&requirements);
    let manifest = delta_manifest();
    let (rows, parse_problems) = rows_from_manifest(&manifest);
    assert!(parse_problems.is_empty(), "{parse_problems:?}");
    assert_eq!(rows.len(), declared_scenarios(&requirements).len());

    let mut names = BTreeSet::new();
    for row in &rows {
        assert!(
            names.insert(row.key.oracle_test_name()),
            "each scenario must have its own distinct mechanism oracle"
        );
    }
    assert_eq!(names.len(), rows.len());

    let mut degraded_listings: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let mut erased_without_features = BTreeSet::new();
    for row in &rows {
        let command = oracles
            .focused_commands
            .get(&row.key.requirement)
            .expect("the owned focused command");
        let name = row.key.oracle_test_name();
        let executed = run_focused(command, &[&name, "--", "--exact"]);
        assert_eq!(
            executed,
            1,
            "the oracle for {} must be exactly one real, passing test",
            row.key.label()
        );
        // The same command with the feature flag dropped, listed once per
        // distinct command: an oracle missing from that listing is one the
        // incomplete command silently erases.
        let degraded = command.replacen(" --all-features", "", 1);
        assert_ne!(&degraded, command);
        let listing = degraded_listings
            .entry(degraded.clone())
            .or_insert_with(|| run_focused_listing(&degraded));
        if !listing.contains(&name) {
            erased_without_features.insert(row.key.label());
        }
    }
    assert!(
        !erased_without_features.is_empty(),
        "at least one scenario oracle must be feature-gated, or the feature-completeness \
         requirement on the declared command would be untestable"
    );
}
