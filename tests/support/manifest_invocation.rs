//! Real, isolated command invocations for every scenario row in
//! `fixtures/trace-publication/v1/manifest.tsv`.
//!
//! Round-5 closure audit: the manifest's `command`, `exit_class`, and
//! `diagnostic` columns used to be decorative. The execution harness
//! "checked" them by SCANNING RUST SOURCE TEXT for the literal `Some(2)` and
//! for the diagnostic string inside the named focused test's body, so a row
//! could declare `publication check`, exit class `2`, and a stable code and
//! be satisfied by a test that merely CONTAINED those characters. No command
//! ever ran and no process exit status was ever observed.
//!
//! This module is the repair. For every `(requirement, scenario)` key it
//! builds ONE FRESH ISOLATED TEMPORARY FIXTURE and returns the exact argv of
//! the REAL executable that demonstrates that scenario:
//!
//! * `env!("CARGO_BIN_EXE_buoy")` for `static`, `trace`, `package-identity`,
//!   and every `publication check|generate|evidence` row;
//! * `env!("CARGO_BIN_EXE_check-package-contract")` for the packaging gate;
//! * `env!("CARGO_BIN_EXE_check-devenv-task-graph")` for the task-graph gate
//!   the `devenv test` acceptance graph itself invokes; and
//! * `bash nix/tools/check-derivation-contract.sh` for the real derivation
//!   contract.
//!
//! The harness then asserts the row's declared exit class against the REAL
//! process exit status, and the row's declared `diagnostic` against the REAL
//! bytes the process wrote to stdout or stderr. Nothing is read from Rust
//! source text.
//!
//! Column meaning, as enforced by the harness:
//!
//! * `command` -- the real command family this row's invocation executes;
//! * `exit_class` -- the exact process exit status that invocation returns;
//! * `diagnostic` -- one stable, root-independent token the invocation MUST
//!   emit on stdout or stderr (a `TPT-*`/`CAB-*`/`DEVENV-*` code, a gate
//!   diagnostic, a versioned schema ID, or a machine-report field), or empty
//!   when the real command emits no stable token for that scenario.
//!
//! Completeness is fail-closed: [`invocation_for`] returns `None` only for a
//! key it does not know, and the harness turns every such key into a
//! FAILURE. A scenario added to the delta specs therefore cannot be
//! satisfied by a manifest row alone.

#![allow(dead_code)]

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use tempfile::TempDir;

/// Frozen self-consistent BLAKE3 binding repository.
const BINDING_FIXTURE: &str = "fixtures/content-addressed-bindings/v1/positive";
/// Frozen repository whose trailing normative anchor absorbs an appended
/// publication slot, so a real link edit keeps its pinned binding current.
const SEQUENCE_FIXTURE: &str = "fixtures/content-addressed-bindings/v1/publication-sequence";
/// Frozen reviewed trace snapshot used by the readable projection commands.
const TRACE_FIXTURE: &str = "fixtures/readable-trace-projections/v1/repository";
/// The real committed consumer package expression the packaging gate reads.
const PACKAGE_FILE: &str = "nix/consumer-package-v1.nix";
/// The real committed reviewed source-content lock.
const SOURCE_CONTENT_LOCK: &str = "nix/consumer-package-v1.source-content.json";
/// The real committed derivation-contract script.
const DERIVATION_CONTRACT: &str = "nix/tools/check-derivation-contract.sh";
/// The resolved acceptance task graph fixture the real gate validates.
const VALID_TASKS: &str = "fixtures/devenv/v1/valid-tasks.json";

const BINDING_OBJECT: &str = "buoy/objects/v1/blake3-256/0e857e33e387325fbf457e426bc42286d9d99926b19f3efa3c203795cfeeb6c4.toml";
const SEQUENCE_OBJECT: &str = "buoy/objects/v1/blake3-256/5be031013feeaccb77f755d96d8f04dd9d131a4a493f7842483a734b6a38088f.toml";

/// The real committed compact implementation attribute.
const REAL_ATTRIBUTE: &str = "#[buoy::implements(\"blake3-256:0e857e33e387325fbf457e426bc42286d9d99926b19f3efa3c203795cfeeb6c4\")]";
const ATTRIBUTE_PAYLOAD: &str = "0e857e33e387325fbf457e426bc42286d9d99926b19f3efa3c203795cfeeb6c4";
/// The current semantic atom revision of `BGLC-003` in both fixtures.
const CURRENT_ATOM_REVISION: &str =
    "blake3-256:a352b07c143fff7a0aa8e2a57d65fba2ee2d59635fdb20191f68fe93ef21c87f";
/// A well-formed digest that is not any real object in the fixtures.
const STALE_ATOM_REVISION: &str =
    "blake3-256:1111111111111111111111111111111111111111111111111111111111111111";

/// One real command invocation bound to a live isolated fixture.
pub struct Invocation {
    program: PathBuf,
    args: Vec<String>,
    env: Vec<(String, String)>,
    env_remove: Vec<String>,
    /// Retained so the temporary fixture outlives the invocation.
    fixtures: Vec<TempDir>,
}

impl Invocation {
    /// The exact argv this invocation runs, for diagnostics.
    #[must_use]
    pub fn rendered(&self) -> String {
        format!("{} {}", self.program.display(), self.args.join(" "))
    }

    /// Returns the command-family label that the manifest declares.
    #[must_use]
    pub fn command_family(&self) -> String {
        let program = self
            .program
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or_default();
        if program == "check-package-contract" {
            return String::from("check-package-contract");
        }
        if program == "check-devenv-task-graph" {
            return String::from("check-devenv-task-graph");
        }
        if program == "check-derivation-contract.sh"
            || self
                .args
                .iter()
                .any(|arg| arg.ends_with("check-derivation-contract.sh"))
        {
            return String::from("check-derivation-contract.sh");
        }
        if program == "check_trace_projection" {
            return String::from("check-trace-projection");
        }
        if self.args.first().map(String::as_str) == Some("publication") {
            return format!(
                "publication {}",
                self.args.get(1).map_or("", String::as_str)
            );
        }
        self.args.first().cloned().unwrap_or_default()
    }

    /// Runs the real command and returns its real output.
    ///
    /// # Panics
    ///
    /// Panics when the real executable cannot be spawned at all.
    #[must_use]
    pub fn run(&self) -> Output {
        let mut command = Command::new(&self.program);
        command.args(&self.args);
        for key in &self.env_remove {
            command.env_remove(key);
        }
        for (key, value) in &self.env {
            command.env(key, value);
        }
        command
            .output()
            .unwrap_or_else(|error| panic!("spawn `{}`: {error}", self.rendered()))
    }
}

fn buoy() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_buoy"))
}

fn package_gate() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_check-package-contract"))
}

fn task_graph_gate() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_check-devenv-task-graph"))
}

fn text(path: &Path) -> String {
    fs::read_to_string(path).unwrap_or_else(|error| panic!("read {}: {error}", path.display()))
}

fn put(root: &Path, relative: &str, contents: &str) {
    let path = root.join(relative);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("fixture directory");
    }
    fs::write(&path, contents).unwrap_or_else(|error| panic!("write {}: {error}", path.display()));
}

fn append(root: &Path, relative: &str, contents: &str) {
    let path = root.join(relative);
    let mut existing = text(&path);
    existing.push_str(contents);
    fs::write(&path, existing).expect("appended fixture bytes");
}

/// Copies one committed fixture tree into a fresh temporary root.
fn stage(fixture: &str) -> TempDir {
    let target = TempDir::new().expect("temporary fixture root");
    let source = Path::new(env!("CARGO_MANIFEST_DIR")).join(fixture);
    let mut directories = Vec::from([source.clone()]);
    let mut copied = 0_usize;
    while let Some(directory) = directories.pop() {
        for entry in fs::read_dir(&directory).expect("fixture directory") {
            let path = entry.expect("fixture entry").path();
            if path.is_dir() {
                directories.push(path);
                continue;
            }
            let relative = path.strip_prefix(&source).expect("fixture-relative path");
            let destination = target.path().join(relative);
            fs::create_dir_all(destination.parent().expect("fixture parent"))
                .expect("staged directory");
            fs::copy(&path, &destination).expect("staged fixture file");
            copied += 1;
        }
    }
    assert!(copied >= 4, "{fixture} must stage real committed files");
    target
}

fn binding_repository() -> TempDir {
    stage(BINDING_FIXTURE)
}

fn sequence_repository() -> TempDir {
    stage(SEQUENCE_FIXTURE)
}

fn trace_repository() -> TempDir {
    stage(TRACE_FIXTURE)
}

fn quoted(paths: &[&str]) -> String {
    paths
        .iter()
        .map(|path| format!("\"{path}\""))
        .collect::<Vec<_>>()
        .join(",")
}

fn binding_source_set() -> String {
    quoted(&["openspec/specs/demo/spec.md", "src/lib.rs", BINDING_OBJECT])
}

fn sequence_source_set() -> String {
    quoted(&["openspec/specs/demo/spec.md", "src/lib.rs", SEQUENCE_OBJECT])
}

/// Writes one publication plan and returns its path.
fn plan(
    root: &Path,
    id: &str,
    selector: &str,
    rows: usize,
    sources: &str,
    outputs: &str,
) -> PathBuf {
    let path = root.join("plan.json");
    fs::write(
        &path,
        format!(
            "{{\"schema\":\"buoy/trace-publication-plan/v1\",\"publication_id\":\"{id}\",\
             \"selector\":{selector},\"expected_rows\":{rows},\"source_set\":[{sources}],\
             \"outputs\":[{outputs}]}}"
        ),
    )
    .expect("publication plan");
    path
}

/// The ordinary `requirement_id` selector both fixtures resolve.
fn requirement_selector() -> &'static str {
    "{\"requirement_id\":\"BGLC-003\"}"
}

fn page_output(path: &str, lifecycle: &str) -> String {
    format!(
        "{{\"id\":\"page\",\"mount\":\"repository\",\"path\":\"{path}\",\"kind\":\"page\",\
         \"lifecycle\":\"{lifecycle}\"}}"
    )
}

fn based_page_output(path: &str, lifecycle: &str) -> String {
    format!(
        "{{\"id\":\"page\",\"mount\":\"repository\",\"path\":\"{path}\",\"kind\":\"page\",\
         \"lifecycle\":\"{lifecycle}\",\"target_base\":\"https://example.invalid/trace/\"}}"
    )
}

fn link_output() -> &'static str {
    "{\"id\":\"link\",\"mount\":\"repository\",\"path\":\"openspec/specs/demo/spec.md\",\
     \"kind\":\"external-link\",\"lifecycle\":\"replaceable\",\"relation\":\"documents\",\
     \"page\":\"page\"}"
}

fn evidence_output(lifecycle: &str) -> String {
    format!(
        "{{\"id\":\"evidence\",\"mount\":\"repository\",\"path\":\"docs/evidence.md\",\
         \"kind\":\"evidence\",\"lifecycle\":\"{lifecycle}\"}}"
    )
}

fn edit_output() -> &'static str {
    "{\"id\":\"edit\",\"mount\":\"repository\",\"path\":\"openspec/specs/demo/spec.md\",\
     \"kind\":\"source-edit\",\"lifecycle\":\"replaceable\"}"
}

/// The immutable basis-keyed page plus its owning external link.
fn basis_page_and_link() -> String {
    format!(
        "{},{}",
        based_page_output("out/{publication_basis_path}.md", "immutable"),
        link_output()
    )
}

/// Argv for one `buoy publication` invocation against one isolated root.
fn publication_args(root: &Path, plan_path: &Path, operation: &str, extra: &[&str]) -> Vec<String> {
    let mut args = Vec::from([
        String::from("publication"),
        operation.to_owned(),
        String::from("--plan"),
        plan_path.display().to_string(),
        String::from("--config"),
        root.join("buoy.toml").display().to_string(),
        String::from("--root"),
        root.display().to_string(),
    ]);
    args.extend(extra.iter().map(|value| (*value).to_owned()));
    args
}

fn strings(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| (*value).to_owned()).collect()
}

fn invocation(program: PathBuf, args: Vec<String>, fixtures: Vec<TempDir>) -> Invocation {
    Invocation {
        program,
        args,
        env: Vec::new(),
        env_remove: Vec::new(),
        fixtures,
    }
}

/// Runs one preparatory command inside the builder, so a scenario that only
/// exists after a real earlier operation (a generated link, a created
/// immutable page) is set up by that real operation rather than by a
/// hand-written stand-in.
///
/// Under the acceptance feature set a failed preparation is a loud panic
/// naming the exact command. Under a build that omits a fixture's digest
/// member the frozen fixtures cannot load at all, and the exit-class
/// assertions are compiled out with them, so the builder stays constructible
/// for the ungated completeness control rather than panicking.
fn prepare(program: &Path, args: &[String]) -> Output {
    let output = Command::new(program)
        .args(args)
        .output()
        .expect("preparatory command");
    #[cfg(all(feature = "blake3-256", feature = "xxh3-128"))]
    assert!(
        output.status.success(),
        "preparatory `{} {}` failed with {:?}: {} {}",
        program.display(),
        args.join(" "),
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    output
}

/// A `buoy publication` invocation whose fixture is already staged.
fn publication(root: &TempDir, plan_path: &Path, operation: &str, extra: &[&str]) -> Vec<String> {
    publication_args(root.path(), plan_path, operation, extra)
}

// ---------------------------------------------------------------------------
// Devenv acceptance task graph
// ---------------------------------------------------------------------------

/// Stages the resolved acceptance task graph into a fresh temporary file with
/// every task `cwd` rewritten to this repository's real root, then applies one
/// textual mutation. The REAL `check-devenv-task-graph` gate -- the binary the
/// `devenv test` acceptance graph itself runs -- then validates that isolated
/// graph, so a `devenv test` row is bound to the real blocking gate without
/// spending thirteen minutes running the acceptance suite per row.
fn task_graph_fixture(mutate: impl Fn(String) -> String) -> (TempDir, PathBuf) {
    let root = std::env::current_dir()
        .expect("package root")
        .canonicalize()
        .expect("canonical package root");
    let source = text(&root.join(VALID_TASKS));
    let rewritten = source.replace("\"/repo\"", &format!("\"{}\"", root.display()));
    assert_ne!(rewritten, source, "the task fixture must declare a cwd");
    let mutated = mutate(rewritten);
    let fixture = TempDir::new().expect("task graph fixture root");
    let path = fixture.path().join("tasks.json");
    fs::write(&path, mutated).expect("staged task graph");
    (fixture, path)
}

fn task_graph_invocation(mutate: impl Fn(String) -> String) -> Invocation {
    let (fixture, path) = task_graph_fixture(mutate);
    let mut built = invocation(
        task_graph_gate(),
        strings(&["--check", "--tasks", &path.display().to_string()]),
        Vec::from([fixture]),
    );
    built.env_remove.push(String::from("DEVENV_TASK_FILE"));
    built
}

// ---------------------------------------------------------------------------
// Packaging gate
// ---------------------------------------------------------------------------

/// Stages the REAL committed consumer package expression into a fresh
/// temporary file and applies one mutation, then returns the real gate
/// invocation for it.
fn package_gate_invocation(mutate: impl Fn(&str) -> String) -> Invocation {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let committed = text(&root.join(PACKAGE_FILE));
    let mutated = mutate(&committed);
    let fixture = TempDir::new().expect("package fixture root");
    let path = fixture.path().join("consumer-package-v1.nix");
    fs::write(&path, &mutated).expect("staged package file");
    invocation(
        package_gate(),
        strings(&[
            "--package-file",
            &path.display().to_string(),
            "--lock-file",
            &root.join(SOURCE_CONTENT_LOCK).display().to_string(),
        ]),
        Vec::from([fixture]),
    )
}

// ---------------------------------------------------------------------------
// Repository mutations used by the CAB rows
// ---------------------------------------------------------------------------

fn implementation_source(attribute: &str) -> String {
    format!(
        "{attribute}\npub fn render_catalog(libraries: &[&str]) -> String {{\n    \
         libraries.join(\"\\n\")\n}}\n"
    )
}

/// Rewrites the staged configuration to a change-detection member and drops
/// the compact-binding selector, so the repository configuration itself
/// loads and the compact attribute really reaches the digest-class check.
fn change_detection_repository() -> TempDir {
    let root = trace_repository();
    let config = text(&root.path().join("buoy.toml"))
        .replace("\"blake3-256\"]", "\"xxh3-128\"]")
        .replace(
            "[[selectors]]\nroot = \"buoy/objects\"\nadapter = \"binding/v1\"\n\
             include = [\"**/*.toml\"]\nexclude = []\nauthority_expectation = \"forbidden\"\n\n",
            "",
        );
    assert!(
        !config.contains("binding/v1"),
        "the compact-binding selector must be removed"
    );
    put(root.path(), "buoy.toml", &config);
    fs::remove_dir_all(root.path().join("buoy")).expect("removed binding store");
    root
}

/// The canonical publication slot bytes for one atom revision.
fn slot(link_id: &str, revision: &str, target: &str) -> String {
    format!(
        "\n## Publication: {link_id}\n\n```buoy-external-trace-v1\nid = \"{link_id}\"\n\
         atoms = [\"BGLC-003@{revision}\"]\nrelation = \"documents\"\ntarget = \"{target}\"\n```\n"
    )
}

/// Resolves the real basis-keyed physical page path a plan renders, by
/// asking the real `check` operation for its own reported basis.
fn basis_page_path(root: &Path, plan_path: &Path) -> PathBuf {
    let output = Command::new(buoy())
        .args(publication_args(
            root,
            plan_path,
            "check",
            &["--artifact", "page"],
        ))
        .output()
        .expect("basis discovery check");
    let report: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("machine report for basis discovery");
    let basis = report["basis"].as_str().expect("reported basis").to_owned();
    let (algorithm, payload) = basis.split_once(':').expect("algorithm:payload basis");
    root.join("out")
        .join(algorithm)
        .join(format!("{payload}.md"))
}

/// A directory holding an executable named `buoy` that writes a marker and
/// exits successfully. Prepending it to `PATH` proves the publication
/// command never resolves its own name from the environment.
fn substitute_executable_directory() -> (TempDir, PathBuf) {
    let directory = TempDir::new().expect("substitute executable directory");
    let marker = directory.path().join("substitute-ran");
    let script = directory.path().join("buoy");
    fs::write(
        &script,
        format!(
            "#!/bin/sh\necho substitute > \"{}\"\nexit 0\n",
            marker.display()
        ),
    )
    .expect("substitute executable");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        fs::set_permissions(&script, fs::Permissions::from_mode(0o755))
            .expect("executable substitute");
    }
    (directory, marker)
}

// ---------------------------------------------------------------------------
// Registry
// ---------------------------------------------------------------------------

/// Every `(requirement, scenario)` key this registry builds a real command
/// invocation for.
///
/// This cheap membership list exists so the fail-closed completeness control
/// can run WITHOUT staging a fixture or spawning a process, and therefore
/// without depending on any digest feature. Drift is caught in both
/// directions: a key listed here with no match arm below makes
/// [`invocation_for`] return `None` and the execution harness fail loudly,
/// and a key handled below but omitted here is rejected by the same
/// completeness control.
pub const REGISTERED: &[&str] = &[
    "CAB-002/positive-algorithm-qualified-compact-attribute-resolves",
    "CAB-002/negative-attribute-contains-raw-payload-bytes",
    "CAB-002/negative-attribute-contains-extra-metadata",
    "CAB-007/positive-blake3-binding-is-selected",
    "CAB-007/negative-xxh3-binding-is-selected",
    "CAB-007/negative-locator-and-active-member-differ",
    "DEV-010/positive-maintainer-selects-publication-check",
    "DEV-010/negative-consumer-test-depends-on-publication-check",
    "ETL-008/positive-current-atom-revision-replaces-a-stale-reference",
    "ETL-008/negative-generator-uses-the-atom-object-digest",
    "ETL-009/positive-basis-keyed-page-preserves-an-old-target",
    "ETL-009/negative-exact-atom-uses-one-mutable-branch-page",
    "ETL-009/negative-target-depends-on-generated-page-commit",
    "OBW-008/positive-completed-binding-receives-generated-evidence",
    "OBW-008/positive-no-completed-relation-keeps-an-empty-snapshot",
    "OBW-008/negative-human-transcribes-a-conflicting-digest",
    "OBW-008/negative-archived-snapshot-claims-current-authority",
    "OBW-009/positive-plan-declares-a-current-atom-invariant",
    "OBW-009/negative-planning-row-pins-an-ungenerated-atom-digest",
    "OBW-010/positive-downstream-task-invokes-pinned-buoy-package",
    "OBW-010/negative-consumer-shell-validates-json-fragments",
    "PBD-001/positive-consumer-realizes-pinned-buoy-package",
    "PBD-001/negative-consumer-wrapper-fetches-a-live-branch",
    "PBD-002/positive-collision-resistant-consumer-selects-blake3",
    "PBD-002/negative-package-feature-set-differs-from-configuration",
    "PBD-003/positive-consumer-checks-the-approved-package",
    "PBD-003/negative-package-report-omits-its-feature-set",
    "PBD-004/positive-realized-package-runs-offline",
    "PBD-004/negative-packaged-launcher-clones-buoy-at-runtime",
    "PBD-005/positive-two-clean-package-realizations-agree",
    "PBD-005/positive-private-source-integration-remains-advisory",
    "PBD-005/negative-package-contract-is-absent-from-final-acceptance",
    "PBD-005/negative-package-succeeds-only-with-developer-cargo-state",
    "RTP-007/positive-published-page-identifies-the-complete-chain",
    "RTP-007/negative-graph-domain-is-ambiguous",
    "RTP-007/negative-page-omits-the-atom-revision",
    "RTP-007/negative-renderer-bytes-change-without-a-new-schema",
    "REH-012/positive-regular-sibling-replaces-a-tracked-page",
    "REH-012/negative-parent-directory-is-a-symlink",
    "REH-012/positive-controlled-edit-targets-selected-source",
    "REH-012/negative-derived-output-becomes-selected-source",
    "REH-013/positive-two-equal-page-generations-overlap",
    "REH-013/negative-first-operation-stops-before-replacement",
    "REH-013/negative-concurrent-initializer-removes-another-cache",
    "TPT-001/positive-complete-publication-plan-is-accepted",
    "TPT-001/negative-immutable-page-has-no-link-owner",
    "TPT-001/negative-plan-has-an-unknown-field",
    "TPT-002/positive-page-check-uses-the-current-buoy-process",
    "TPT-002/negative-environment-supplies-another-buoy-executable",
    "TPT-003/positive-one-exact-current-row-satisfies-the-plan",
    "TPT-003/positive-stale-declared-link-can-receive-a-candidate",
    "TPT-003/negative-expected-fragments-occur-on-different-rows",
    "TPT-004/positive-equal-roots-produce-equal-candidates",
    "TPT-004/negative-recursive-selector-gains-another-file",
    "TPT-004/negative-environment-redirects-an-authority-mount",
    "TPT-004/negative-page-precedes-its-source-edit",
    "TPT-004/negative-selected-source-changes-before-replacement",
    "TPT-005/positive-generated-current-alias-replaces-stale-bytes",
    "TPT-005/positive-immutable-basis-page-is-created-once",
    "TPT-005/negative-immutable-page-has-different-bytes",
    "TPT-005/negative-trace-relation-is-stale",
    "TPT-006/positive-stale-atom-reference-receives-a-current-link",
    "TPT-006/negative-legacy-section-contains-unrelated-text",
    "TPT-006/negative-proposed-edit-changes-requirement-bytes",
    "TPT-007/positive-immutable-target-uses-the-publication-basis",
    "TPT-007/positive-first-link-generation-preserves-the-basis",
    "TPT-007/negative-atom-only-key-permits-unequal-immutable-pages",
    "TPT-007/negative-immutable-basis-uses-a-change-detection-member",
    "TPT-007/negative-target-uses-the-page-commit",
    "TPT-008/positive-completion-evidence-derives-from-one-snapshot",
    "TPT-008/negative-page-drift-blocks-new-evidence",
    "TPT-008/negative-evidence-claims-current-authority-after-drift",
    "TPT-009/positive-equal-stale-pages-exist-under-separate-roots",
    "TPT-009/negative-diff-includes-a-temporary-path",
    "TPT-010/positive-consumer-supplies-all-repository-policy",
    "TPT-010/negative-generated-evidence-claims-correctness",
    "TPT-011/positive-page-generation-emits-one-completed-replacement",
    "TPT-011/negative-report-claims-both-repositories-changed",
];

/// Whether one scenario key has a registered real command invocation.
#[must_use]
pub fn is_registered(requirement: &str, scenario: &str) -> bool {
    REGISTERED.contains(&format!("{requirement}/{scenario}").as_str())
}

/// Builds the real, isolated invocation that demonstrates one scenario.
///
/// Returns `None` only for an unregistered key; the harness treats that as a
/// completeness FAILURE, never as a skip.
#[must_use]
#[allow(clippy::too_many_lines)]
pub fn invocation_for(requirement: &str, scenario: &str) -> Option<Invocation> {
    let key = format!("{requirement}/{scenario}");
    if !REGISTERED.contains(&key.as_str()) {
        return None;
    }
    Some(match key.as_str() {
        // -------------------------------------------------------------- CAB
        "CAB-002/positive-algorithm-qualified-compact-attribute-resolves"
        | "CAB-007/positive-blake3-binding-is-selected" => {
            let root = trace_repository();
            let args = strings(&[
                "static",
                "--root",
                &root.path().display().to_string(),
                "--config",
                &root.path().join("buoy.toml").display().to_string(),
            ]);
            invocation(buoy(), args, Vec::from([root]))
        }
        "CAB-002/negative-attribute-contains-raw-payload-bytes" => {
            let root = trace_repository();
            put(
                root.path(),
                "src/lib.rs",
                &implementation_source(&format!("#[buoy::implements(\"{ATTRIBUTE_PAYLOAD}\")]")),
            );
            let args = strings(&[
                "static",
                "--root",
                &root.path().display().to_string(),
                "--config",
                &root.path().join("buoy.toml").display().to_string(),
            ]);
            invocation(buoy(), args, Vec::from([root]))
        }
        "CAB-002/negative-attribute-contains-extra-metadata" => {
            let root = trace_repository();
            put(
                root.path(),
                "src/lib.rs",
                &implementation_source(&format!(
                    "#[buoy::implements(\"blake3-256:{ATTRIBUTE_PAYLOAD}\", relation = \"implements\")]"
                )),
            );
            let args = strings(&[
                "static",
                "--root",
                &root.path().display().to_string(),
                "--config",
                &root.path().join("buoy.toml").display().to_string(),
            ]);
            invocation(buoy(), args, Vec::from([root]))
        }
        "CAB-007/negative-xxh3-binding-is-selected" => {
            let root = change_detection_repository();
            let args = strings(&[
                "static",
                "--root",
                &root.path().display().to_string(),
                "--config",
                &root.path().join("buoy.toml").display().to_string(),
            ]);
            invocation(buoy(), args, Vec::from([root]))
        }
        "CAB-007/negative-locator-and-active-member-differ" => {
            let root = trace_repository();
            put(
                root.path(),
                "src/lib.rs",
                &implementation_source(&format!(
                    "#[buoy::implements(\"sha-256:{ATTRIBUTE_PAYLOAD}\")]"
                )),
            );
            let args = strings(&[
                "static",
                "--root",
                &root.path().display().to_string(),
                "--config",
                &root.path().join("buoy.toml").display().to_string(),
            ]);
            invocation(buoy(), args, Vec::from([root]))
        }

        // -------------------------------------------------------------- DEV
        "DEV-010/positive-maintainer-selects-publication-check"
        | "PBD-005/positive-private-source-integration-remains-advisory" => {
            task_graph_invocation(|source| source)
        }
        "DEV-010/negative-consumer-test-depends-on-publication-check" => {
            task_graph_invocation(|source| {
                source.replace(
                    "\"after\":[\"buoy:package-contract\"]",
                    "\"after\":[\"buoy:package-contract\",\"buoy:publication-check\"]",
                )
            })
        }
        "PBD-005/negative-package-contract-is-absent-from-final-acceptance" => {
            task_graph_invocation(|source| {
                source.replace(
                    "\"after\":[\"buoy:package-contract\"]",
                    "\"after\":[\"buoy:specs\"]",
                )
            })
        }

        // -------------------------------------------------------------- ETL
        "ETL-008/positive-current-atom-revision-replaces-a-stale-reference"
        | "TPT-006/positive-stale-atom-reference-receives-a-current-link" => {
            let root = sequence_repository();
            append(
                root.path(),
                "openspec/specs/demo/spec.md",
                &slot(
                    "link",
                    STALE_ATOM_REVISION,
                    "https://example.invalid/trace/out/stale.md",
                ),
            );
            let plan_path = plan(
                root.path(),
                "stale-link-repair",
                requirement_selector(),
                1,
                &sequence_source_set(),
                &basis_page_and_link(),
            );
            let args = publication(&root, &plan_path, "generate", &["--artifact", "link"]);
            invocation(buoy(), args, Vec::from([root]))
        }
        "ETL-008/negative-generator-uses-the-atom-object-digest" => {
            // A slot bound to a digest other than the current atom REVISION
            // is not the candidate the generator derives, so the real
            // conformance check reports a static blocker rather than
            // accepting the wrong identity.
            let root = sequence_repository();
            append(
                root.path(),
                "openspec/specs/demo/spec.md",
                &slot(
                    "link",
                    STALE_ATOM_REVISION,
                    "https://example.invalid/trace/out/object-digest.md",
                ),
            );
            let plan_path = plan(
                root.path(),
                "atom-object-digest",
                requirement_selector(),
                1,
                &sequence_source_set(),
                &basis_page_and_link(),
            );
            let args = publication(&root, &plan_path, "check", &["--artifact", "link"]);
            invocation(buoy(), args, Vec::from([root]))
        }
        "ETL-009/positive-basis-keyed-page-preserves-an-old-target" => {
            // The immutable basis-keyed page is created once and a later
            // equal generation preserves it byte-for-byte.
            let root = sequence_repository();
            let plan_path = plan(
                root.path(),
                "basis-keyed-page",
                requirement_selector(),
                1,
                &sequence_source_set(),
                &basis_page_and_link(),
            );
            prepare(
                &buoy(),
                &publication(&root, &plan_path, "generate", &["--artifact", "link"]),
            );
            prepare(
                &buoy(),
                &publication(&root, &plan_path, "generate", &["--artifact", "page"]),
            );
            let args = publication(&root, &plan_path, "generate", &["--artifact", "page"]);
            invocation(buoy(), args, Vec::from([root]))
        }
        "ETL-009/negative-exact-atom-uses-one-mutable-branch-page" => {
            let root = binding_repository();
            let plan_path = plan(
                root.path(),
                "mutable-branch-page",
                requirement_selector(),
                1,
                &binding_source_set(),
                &format!(
                    "{},{}",
                    based_page_output("out/constant-branch-page.md", "immutable"),
                    link_output()
                ),
            );
            let args = publication(&root, &plan_path, "check", &["--artifact", "page"]);
            invocation(buoy(), args, Vec::from([root]))
        }
        "ETL-009/negative-target-depends-on-generated-page-commit"
        | "TPT-007/negative-target-uses-the-page-commit" => {
            let root = binding_repository();
            let plan_path = plan(
                root.path(),
                "page-commit-cycle",
                requirement_selector(),
                1,
                &binding_source_set(),
                &page_output("out/{page_commit}.md", "replaceable"),
            );
            let args = publication(&root, &plan_path, "check", &["--artifact", "page"]);
            invocation(buoy(), args, Vec::from([root]))
        }

        // -------------------------------------------------------------- OBW
        "OBW-008/positive-completed-binding-receives-generated-evidence" => {
            let root = binding_repository();
            let plan_path = plan(
                root.path(),
                "completion-evidence",
                requirement_selector(),
                1,
                &binding_source_set(),
                &evidence_output("replaceable"),
            );
            let args = publication(&root, &plan_path, "generate", &["--artifact", "evidence"]);
            invocation(buoy(), args, Vec::from([root]))
        }
        "OBW-008/positive-no-completed-relation-keeps-an-empty-snapshot" => {
            // No declared publication output at all: the evidence document
            // carries the resolved rows and no page, link, or artifact
            // identity it could only have invented.
            let root = binding_repository();
            let plan_path = plan(
                root.path(),
                "empty-completion-snapshot",
                requirement_selector(),
                1,
                &binding_source_set(),
                "",
            );
            let args = publication(&root, &plan_path, "evidence", &[]);
            invocation(buoy(), args, Vec::from([root]))
        }
        "OBW-008/negative-human-transcribes-a-conflicting-digest" => {
            let root = binding_repository();
            let plan_path = plan(
                root.path(),
                "transcribed-digest",
                requirement_selector(),
                1,
                &binding_source_set(),
                &evidence_output("replaceable"),
            );
            prepare(
                &buoy(),
                &publication(&root, &plan_path, "generate", &["--artifact", "evidence"]),
            );
            let generated = text(&root.path().join("docs/evidence.md"));
            put(
                root.path(),
                "docs/evidence.md",
                &generated.replace(
                    ATTRIBUTE_PAYLOAD,
                    "deadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeef",
                ),
            );
            let args = publication(&root, &plan_path, "check", &["--artifact", "evidence"]);
            invocation(buoy(), args, Vec::from([root]))
        }
        "OBW-008/negative-archived-snapshot-claims-current-authority" => {
            let root = binding_repository();
            let plan_path = plan(
                root.path(),
                "archived-authority-claim",
                requirement_selector(),
                1,
                &binding_source_set(),
                &evidence_output("replaceable"),
            );
            prepare(
                &buoy(),
                &publication(&root, &plan_path, "generate", &["--artifact", "evidence"]),
            );
            let generated = text(&root.path().join("docs/evidence.md"));
            put(
                root.path(),
                "docs/evidence.md",
                &generated.replace(
                    "Buoy remains the current-state authority.",
                    "This archived document currently represents the live state.",
                ),
            );
            let args = publication(&root, &plan_path, "check", &["--artifact", "evidence"]);
            invocation(buoy(), args, Vec::from([root]))
        }
        "OBW-009/positive-plan-declares-a-current-atom-invariant" => {
            // The plan names a requirement ID invariant, never a pinned
            // volatile digest, and the real operation resolves it.
            let root = binding_repository();
            let plan_path = plan(
                root.path(),
                "current-atom-invariant",
                requirement_selector(),
                1,
                &binding_source_set(),
                edit_output(),
            );
            let args = publication(&root, &plan_path, "check", &["--artifact", "edit"]);
            invocation(buoy(), args, Vec::from([root]))
        }
        "OBW-009/negative-planning-row-pins-an-ungenerated-atom-digest" => {
            let root = binding_repository();
            let plan_path = plan(
                root.path(),
                "premature-identity",
                "{\"record\":\"blake3-256:0000000000000000000000000000000000000000000000000000000000000000\"}",
                1,
                &binding_source_set(),
                &page_output("out/page.md", "replaceable"),
            );
            let args = publication(&root, &plan_path, "check", &["--artifact", "page"]);
            invocation(buoy(), args, Vec::from([root]))
        }
        "OBW-010/positive-downstream-task-invokes-pinned-buoy-package" => {
            // The consumer's focused task needs no repository-specific
            // logic: the pinned package answers with its own machine
            // identity document.
            invocation(buoy(), strings(&["package-identity"]), Vec::new())
        }
        "OBW-010/negative-consumer-shell-validates-json-fragments" => {
            // A shell-assembled plan carrying an untyped extra field is
            // rejected by the typed parser before any repository read.
            let root = binding_repository();
            let path = root.path().join("plan.json");
            fs::write(
                &path,
                format!(
                    "{{\"schema\":\"buoy/trace-publication-plan/v1\",\
                     \"publication_id\":\"shell-fragment\",\"selector\":{},\
                     \"expected_rows\":1,\"source_set\":[{}],\"outputs\":[{}],\
                     \"grep_expression\":\"state.*current\"}}",
                    requirement_selector(),
                    binding_source_set(),
                    page_output("out/page.md", "replaceable")
                ),
            )
            .expect("shell-assembled plan");
            let args = publication(&root, &path, "check", &["--artifact", "page"]);
            invocation(buoy(), args, Vec::from([root]))
        }

        // -------------------------------------------------------------- PBD
        "PBD-001/positive-consumer-realizes-pinned-buoy-package"
        | "PBD-005/positive-two-clean-package-realizations-agree" => {
            package_gate_invocation(std::borrow::ToOwned::to_owned)
        }
        "PBD-001/negative-consumer-wrapper-fetches-a-live-branch" => {
            package_gate_invocation(|text| {
                text.replacen(
                    "  inherit src;",
                    "  src = fetchFromGitHub { owner = \"buoy-project\"; repo = \"buoy\"; ref = \"main\"; };",
                    1,
                )
            })
        }
        "PBD-002/positive-collision-resistant-consumer-selects-blake3"
        | "PBD-003/positive-consumer-checks-the-approved-package" => {
            invocation(buoy(), strings(&["package-identity"]), Vec::new())
        }
        "PBD-002/negative-package-feature-set-differs-from-configuration" => {
            package_gate_invocation(|text| text.replace("\"sha-256\"", "\"sha-512\""))
        }
        "PBD-003/negative-package-report-omits-its-feature-set" => {
            package_gate_invocation(|text| {
                text.replacen("\"--no-default-features\"", "\"--offline\"", 1)
            })
        }
        "PBD-004/positive-realized-package-runs-offline" => {
            let root = Path::new(env!("CARGO_MANIFEST_DIR"));
            invocation(
                PathBuf::from("bash"),
                strings(&[&root.join(DERIVATION_CONTRACT).display().to_string()]),
                Vec::new(),
            )
        }
        "PBD-004/negative-packaged-launcher-clones-buoy-at-runtime" => {
            package_gate_invocation(|text| {
                text.replacen(
                    "  preBuild = ''",
                    "  postInstall = ''\n    echo 'git clone https://example.invalid/buoy' >> \"$out/bin/buoy\"\n  '';\n  preBuild = ''",
                    1,
                )
            })
        }
        "PBD-005/negative-package-succeeds-only-with-developer-cargo-state" => {
            package_gate_invocation(|text| {
                text.replacen("  src,\n", "  src,\n  rustPlatform,\n", 1)
            })
        }

        // -------------------------------------------------------------- RTP
        "RTP-007/positive-published-page-identifies-the-complete-chain" => {
            let root = trace_repository();
            let args = strings(&[
                "trace",
                "--root",
                &root.path().display().to_string(),
                "--config",
                &root.path().join("buoy.toml").display().to_string(),
                "--relation-id",
                "BGLC-RENDER-CATALOG",
                "--format",
                "markdown",
            ]);
            invocation(buoy(), args, Vec::from([root]))
        }
        "RTP-007/negative-renderer-bytes-change-without-a-new-schema" => {
            let root = trace_repository();
            let trace = prepare(
                &buoy(),
                &strings(&[
                    "trace",
                    "--root",
                    &root.path().display().to_string(),
                    "--config",
                    &root.path().join("buoy.toml").display().to_string(),
                    "--relation-id",
                    "BGLC-RENDER-CATALOG",
                    "--format",
                    "markdown",
                ]),
            );
            assert!(trace.status.success(), "renderer fixture must render");
            let mutated = String::from_utf8(trace.stdout)
                .expect("renderer fixture UTF-8")
                .replacen("attachment=", "dropped=", 1);
            let input = root.path().join("mutated-renderer.md");
            fs::write(&input, mutated).expect("mutated renderer fixture");
            invocation(
                PathBuf::from(env!("CARGO_BIN_EXE_check_trace_projection")),
                strings(&["--input", &input.display().to_string()]),
                Vec::from([root]),
            )
        }
        "RTP-007/negative-graph-domain-is-ambiguous" => {
            // A saved page whose graph label no longer names its domain is
            // not the projection the renderer derives, so the real page
            // check refuses to accept it.
            let root = binding_repository();
            let plan_path = plan(
                root.path(),
                "ambiguous-graph-label",
                requirement_selector(),
                1,
                &binding_source_set(),
                &page_output("out/page.md", "replaceable"),
            );
            prepare(
                &buoy(),
                &publication(&root, &plan_path, "generate", &["--artifact", "page"]),
            );
            let rendered = text(&root.path().join("out/page.md"));
            put(
                root.path(),
                "out/page.md",
                &rendered.replace("- Source graph: ", "- Graph: "),
            );
            let args = publication(&root, &plan_path, "check", &["--artifact", "page"]);
            invocation(buoy(), args, Vec::from([root]))
        }
        "RTP-007/negative-page-omits-the-atom-revision" => {
            let root = binding_repository();
            let plan_path = plan(
                root.path(),
                "page-without-atom-revision",
                requirement_selector(),
                1,
                &binding_source_set(),
                &page_output("out/page.md", "replaceable"),
            );
            prepare(
                &buoy(),
                &publication(&root, &plan_path, "generate", &["--artifact", "page"]),
            );
            let rendered = text(&root.path().join("out/page.md"));
            put(
                root.path(),
                "out/page.md",
                &rendered.replace(CURRENT_ATOM_REVISION, ""),
            );
            let args = publication(&root, &plan_path, "check", &["--artifact", "page"]);
            invocation(buoy(), args, Vec::from([root]))
        }

        // -------------------------------------------------------------- REH
        "REH-012/positive-regular-sibling-replaces-a-tracked-page" => {
            let root = binding_repository();
            let plan_path = plan(
                root.path(),
                "regular-sibling-replacement",
                requirement_selector(),
                1,
                &binding_source_set(),
                &page_output("out/page.md", "replaceable"),
            );
            put(root.path(), "out/page.md", "stale saved page\n");
            put(root.path(), "out/sibling.md", "an untouched sibling\n");
            let args = publication(&root, &plan_path, "generate", &["--artifact", "page"]);
            invocation(buoy(), args, Vec::from([root]))
        }
        "REH-012/negative-parent-directory-is-a-symlink" => {
            let root = binding_repository();
            let escape = TempDir::new().expect("escape target root");
            #[cfg(unix)]
            std::os::unix::fs::symlink(escape.path(), root.path().join("out"))
                .expect("symlinked output parent");
            let plan_path = plan(
                root.path(),
                "symlinked-output-parent",
                requirement_selector(),
                1,
                &binding_source_set(),
                &page_output("out/page.md", "replaceable"),
            );
            let args = publication(&root, &plan_path, "generate", &["--artifact", "page"]);
            invocation(buoy(), args, Vec::from([root, escape]))
        }
        "REH-012/positive-controlled-edit-targets-selected-source" => {
            let root = binding_repository();
            let plan_path = plan(
                root.path(),
                "controlled-source-edit",
                requirement_selector(),
                1,
                &binding_source_set(),
                edit_output(),
            );
            let args = publication(&root, &plan_path, "check", &["--artifact", "edit"]);
            invocation(buoy(), args, Vec::from([root]))
        }
        "REH-012/negative-derived-output-becomes-selected-source" => {
            // The declared derived output sits inside the Rust selector's
            // own recursive scope, so the operation would publish into the
            // set it reads.
            let root = binding_repository();
            put(root.path(), "src/page.rs", "// a real selected file\n");
            let plan_path = plan(
                root.path(),
                "derived-output-in-source-set",
                requirement_selector(),
                1,
                &quoted(&[
                    "openspec/specs/demo/spec.md",
                    "src/lib.rs",
                    "src/page.rs",
                    BINDING_OBJECT,
                ]),
                &page_output("src/page.rs", "replaceable"),
            );
            let args = publication(&root, &plan_path, "check", &["--artifact", "page"]);
            invocation(buoy(), args, Vec::from([root]))
        }
        "REH-013/positive-two-equal-page-generations-overlap" => {
            let root = binding_repository();
            let plan_path = plan(
                root.path(),
                "equal-overlapping-generations",
                requirement_selector(),
                1,
                &binding_source_set(),
                &page_output("out/page.md", "replaceable"),
            );
            prepare(
                &buoy(),
                &publication(&root, &plan_path, "generate", &["--artifact", "page"]),
            );
            let args = publication(&root, &plan_path, "generate", &["--artifact", "page"]);
            invocation(buoy(), args, Vec::from([root]))
        }
        "REH-013/negative-first-operation-stops-before-replacement" => {
            // A non-regular existing target stops the operation before any
            // atomic replacement, and preserves what was already there.
            let root = binding_repository();
            fs::create_dir_all(root.path().join("out/page.md")).expect("directory at output path");
            let plan_path = plan(
                root.path(),
                "stopped-before-replacement",
                requirement_selector(),
                1,
                &binding_source_set(),
                &page_output("out/page.md", "replaceable"),
            );
            let args = publication(&root, &plan_path, "generate", &["--artifact", "page"]);
            invocation(buoy(), args, Vec::from([root]))
        }
        "REH-013/negative-concurrent-initializer-removes-another-cache" => {
            // An orphaned temporary sibling left by an interrupted write
            // must neither block this operation nor be removed by it.
            let root = binding_repository();
            put(root.path(), "out/.orphan.tmp.0", "an interrupted write\n");
            let plan_path = plan(
                root.path(),
                "orphaned-temporary-sibling",
                requirement_selector(),
                1,
                &binding_source_set(),
                &page_output("out/page.md", "replaceable"),
            );
            let args = publication(&root, &plan_path, "generate", &["--artifact", "page"]);
            invocation(buoy(), args, Vec::from([root]))
        }

        // -------------------------------------------------------------- TPT
        "TPT-001/positive-complete-publication-plan-is-accepted" => {
            let root = sequence_repository();
            let plan_path = plan(
                root.path(),
                "complete-plan",
                requirement_selector(),
                1,
                &sequence_source_set(),
                &basis_page_and_link(),
            );
            prepare(
                &buoy(),
                &publication(&root, &plan_path, "generate", &["--artifact", "link"]),
            );
            let args = publication(&root, &plan_path, "check", &["--artifact", "link"]);
            invocation(buoy(), args, Vec::from([root]))
        }
        "TPT-001/negative-immutable-page-has-no-link-owner" => {
            let root = binding_repository();
            let plan_path = plan(
                root.path(),
                "orphan-immutable-page",
                requirement_selector(),
                1,
                &binding_source_set(),
                &based_page_output("out/{publication_basis_path}.md", "immutable"),
            );
            let args = publication(&root, &plan_path, "check", &["--artifact", "page"]);
            invocation(buoy(), args, Vec::from([root]))
        }
        "TPT-001/negative-plan-has-an-unknown-field" => {
            let root = binding_repository();
            let path = root.path().join("plan.json");
            fs::write(
                &path,
                format!(
                    "{{\"schema\":\"buoy/trace-publication-plan/v1\",\
                     \"publication_id\":\"unknown-field\",\"selector\":{},\
                     \"expected_rows\":1,\"source_set\":[{}],\"outputs\":[{}],\
                     \"undeclared\":true}}",
                    requirement_selector(),
                    binding_source_set(),
                    page_output("out/page.md", "replaceable")
                ),
            )
            .expect("plan with an undeclared field");
            let args = publication(&root, &path, "check", &["--artifact", "page"]);
            invocation(buoy(), args, Vec::from([root]))
        }
        "TPT-002/positive-page-check-uses-the-current-buoy-process" => {
            let root = binding_repository();
            let plan_path = plan(
                root.path(),
                "in-process-boundary",
                requirement_selector(),
                1,
                &binding_source_set(),
                edit_output(),
            );
            let args = publication(&root, &plan_path, "check", &["--artifact", "edit"]);
            invocation(buoy(), args, Vec::from([root]))
        }
        "TPT-002/negative-environment-supplies-another-buoy-executable" => {
            let root = binding_repository();
            let (substitute, _marker) = substitute_executable_directory();
            let plan_path = plan(
                root.path(),
                "hostile-executable-environment",
                requirement_selector(),
                1,
                &binding_source_set(),
                edit_output(),
            );
            let args = publication(&root, &plan_path, "check", &["--artifact", "edit"]);
            let mut built = invocation(buoy(), args, Vec::from([root, substitute]));
            let hostile = built.fixtures[1].path().display().to_string();
            built.env.push((String::from("PATH"), hostile.clone()));
            built
                .env
                .push((String::from("BUOY"), format!("{hostile}/buoy")));
            built
                .env
                .push((String::from("BUOY_EXECUTABLE"), format!("{hostile}/buoy")));
            built
        }
        "TPT-003/positive-one-exact-current-row-satisfies-the-plan" => {
            let root = binding_repository();
            let plan_path = plan(
                root.path(),
                "one-exact-current-row",
                requirement_selector(),
                1,
                &binding_source_set(),
                edit_output(),
            );
            let args = publication(&root, &plan_path, "check", &["--artifact", "edit"]);
            invocation(buoy(), args, Vec::from([root]))
        }
        "TPT-003/positive-stale-declared-link-can-receive-a-candidate" => {
            let root = sequence_repository();
            append(
                root.path(),
                "openspec/specs/demo/spec.md",
                &slot(
                    "link",
                    STALE_ATOM_REVISION,
                    "https://example.invalid/trace/out/prior.md",
                ),
            );
            let plan_path = plan(
                root.path(),
                "stale-link-receives-candidate",
                requirement_selector(),
                1,
                &sequence_source_set(),
                &basis_page_and_link(),
            );
            let args = publication(&root, &plan_path, "generate", &["--artifact", "link"]);
            invocation(buoy(), args, Vec::from([root]))
        }
        "TPT-003/negative-expected-fragments-occur-on-different-rows" => {
            // The plan expects two rows; the real selector resolves exactly
            // one, so no fragment-level match can satisfy it.
            let root = binding_repository();
            let plan_path = plan(
                root.path(),
                "fragment-level-match",
                requirement_selector(),
                2,
                &binding_source_set(),
                &page_output("out/page.md", "replaceable"),
            );
            let args = publication(&root, &plan_path, "check", &["--artifact", "page"]);
            invocation(buoy(), args, Vec::from([root]))
        }
        "TPT-004/positive-equal-roots-produce-equal-candidates" => {
            // Two independent physical roots, one command: the observed
            // invocation runs in the second root and must report the exact
            // candidate digest the first root already produced, which the
            // manifest pins as this row's stable token.
            let first = binding_repository();
            let first_plan = plan(
                first.path(),
                "equal-roots",
                requirement_selector(),
                1,
                &binding_source_set(),
                edit_output(),
            );
            prepare(
                &buoy(),
                &publication(&first, &first_plan, "check", &["--artifact", "edit"]),
            );
            let second = binding_repository();
            let second_plan = plan(
                second.path(),
                "equal-roots",
                requirement_selector(),
                1,
                &binding_source_set(),
                edit_output(),
            );
            let args = publication(&second, &second_plan, "check", &["--artifact", "edit"]);
            invocation(buoy(), args, Vec::from([second, first]))
        }
        "TPT-004/negative-recursive-selector-gains-another-file" => {
            // The recursive Rust selector now matches an additional nested
            // file the exact source-set policy never declared.
            let root = binding_repository();
            put(root.path(), "src/nested/extra.rs", "pub fn extra() {}\n");
            let plan_path = plan(
                root.path(),
                "scope-drift-gained-file",
                requirement_selector(),
                1,
                &binding_source_set(),
                &page_output("out/page.md", "replaceable"),
            );
            let args = publication(&root, &plan_path, "check", &["--artifact", "page"]);
            invocation(buoy(), args, Vec::from([root]))
        }
        "TPT-004/negative-environment-redirects-an-authority-mount" => {
            // A decoy worktree named by every plausible ambient root
            // variable must not supply one selected byte.
            let root = binding_repository();
            let decoy = binding_repository();
            put(
                decoy.path(),
                "openspec/specs/demo/spec.md",
                "# decoy\n\n## Requirements\n",
            );
            let plan_path = plan(
                root.path(),
                "ambient-root-redirection",
                requirement_selector(),
                1,
                &binding_source_set(),
                edit_output(),
            );
            let args = publication(&root, &plan_path, "check", &["--artifact", "edit"]);
            let mut built = invocation(buoy(), args, Vec::from([root, decoy]));
            let ambient = built.fixtures[1].path().display().to_string();
            for name in ["BUOY_ROOT", "BUOY_REPOSITORY_ROOT", "PWD"] {
                built.env.push((String::from(name), ambient.clone()));
            }
            built
        }
        "TPT-004/negative-page-precedes-its-source-edit" => {
            // The page's declared external link has never been generated,
            // so the page operation must stop before rendering.
            let root = sequence_repository();
            let plan_path = plan(
                root.path(),
                "page-before-its-edit",
                requirement_selector(),
                1,
                &sequence_source_set(),
                &basis_page_and_link(),
            );
            let args = publication(&root, &plan_path, "check", &["--artifact", "page"]);
            invocation(buoy(), args, Vec::from([root]))
        }
        "TPT-004/negative-selected-source-changes-before-replacement" => {
            // A selected source disappears after the plan declared it, so
            // the retained snapshot no longer equals the declared policy
            // and no replacement may happen.
            let root = binding_repository();
            let plan_path = plan(
                root.path(),
                "selected-source-changed",
                requirement_selector(),
                1,
                &quoted(&[
                    "openspec/specs/demo/spec.md",
                    "src/lib.rs",
                    "src/removed.rs",
                    BINDING_OBJECT,
                ]),
                &page_output("out/page.md", "replaceable"),
            );
            let args = publication(&root, &plan_path, "generate", &["--artifact", "page"]);
            invocation(buoy(), args, Vec::from([root]))
        }
        "TPT-005/positive-generated-current-alias-replaces-stale-bytes" => {
            let root = binding_repository();
            let plan_path = plan(
                root.path(),
                "stale-alias-replacement",
                requirement_selector(),
                1,
                &binding_source_set(),
                &page_output("out/page.md", "replaceable"),
            );
            put(
                root.path(),
                "out/page.md",
                "# Implementation trace\n\nstale\n",
            );
            let args = publication(&root, &plan_path, "generate", &["--artifact", "page"]);
            invocation(buoy(), args, Vec::from([root]))
        }
        "TPT-005/positive-immutable-basis-page-is-created-once" => {
            let root = sequence_repository();
            let plan_path = plan(
                root.path(),
                "immutable-created-once",
                requirement_selector(),
                1,
                &sequence_source_set(),
                &basis_page_and_link(),
            );
            prepare(
                &buoy(),
                &publication(&root, &plan_path, "generate", &["--artifact", "link"]),
            );
            let args = publication(&root, &plan_path, "generate", &["--artifact", "page"]);
            invocation(buoy(), args, Vec::from([root]))
        }
        "TPT-005/negative-immutable-page-has-different-bytes" => {
            let root = sequence_repository();
            let plan_path = plan(
                root.path(),
                "immutable-conflict",
                requirement_selector(),
                1,
                &sequence_source_set(),
                &basis_page_and_link(),
            );
            prepare(
                &buoy(),
                &publication(&root, &plan_path, "generate", &["--artifact", "link"]),
            );
            let conflicting = basis_page_path(root.path(), &plan_path);
            fs::create_dir_all(conflicting.parent().expect("basis directory"))
                .expect("basis directory");
            fs::write(&conflicting, b"a byte-different prior committed page\n")
                .expect("conflicting immutable target");
            let args = publication(&root, &plan_path, "generate", &["--artifact", "page"]);
            invocation(buoy(), args, Vec::from([root]))
        }
        "TPT-005/negative-trace-relation-is-stale" => {
            // The bound implementation body really changes, so the selected
            // relation becomes stale and the page must not be written.
            let root = binding_repository();
            let plan_path = plan(
                root.path(),
                "stale-trace-relation",
                requirement_selector(),
                1,
                &binding_source_set(),
                &page_output("out/page.md", "replaceable"),
            );
            put(
                root.path(),
                "src/lib.rs",
                &format!(
                    "{REAL_ATTRIBUTE}\npub fn render_catalog(libraries: &[&str]) -> String {{\n    \
                     libraries.join(\"\\n\") + \"a real later edit\"\n}}\n"
                ),
            );
            let args = publication(&root, &plan_path, "generate", &["--artifact", "page"]);
            invocation(buoy(), args, Vec::from([root]))
        }
        "TPT-006/negative-legacy-section-contains-unrelated-text" => {
            let root = sequence_repository();
            let mut section = slot(
                "link",
                CURRENT_ATOM_REVISION,
                "https://example.invalid/trace/out/legacy.md",
            );
            section.push_str("\nUnrelated prose a maintainer wrote inside this section.\n");
            append(root.path(), "openspec/specs/demo/spec.md", &section);
            let plan_path = plan(
                root.path(),
                "mixed-legacy-section",
                requirement_selector(),
                1,
                &sequence_source_set(),
                &basis_page_and_link(),
            );
            let args = publication(&root, &plan_path, "generate", &["--artifact", "link"]);
            invocation(buoy(), args, Vec::from([root]))
        }
        "TPT-006/negative-proposed-edit-changes-requirement-bytes" => {
            // The declared slot sits INSIDE the normative region, between
            // two requirements, so the candidate this link generation would
            // build relocates normative bytes. The real operation refuses
            // it and writes no candidate at all.
            let root = sequence_repository();
            let spec = text(&root.path().join("openspec/specs/demo/spec.md"));
            let interior = slot(
                "link",
                CURRENT_ATOM_REVISION,
                "https://example.invalid/trace/out/interior.md",
            );
            put(
                root.path(),
                "openspec/specs/demo/spec.md",
                &spec.replace(
                    "### Requirement: BGLC-004",
                    &format!("{interior}\n### Requirement: BGLC-004"),
                ),
            );
            let plan_path = plan(
                root.path(),
                "authority-span-relocation",
                requirement_selector(),
                1,
                &sequence_source_set(),
                &basis_page_and_link(),
            );
            let args = publication(&root, &plan_path, "generate", &["--artifact", "link"]);
            invocation(buoy(), args, Vec::from([root]))
        }
        "TPT-007/positive-immutable-target-uses-the-publication-basis" => {
            let root = sequence_repository();
            let plan_path = plan(
                root.path(),
                "immutable-basis-target",
                requirement_selector(),
                1,
                &sequence_source_set(),
                &basis_page_and_link(),
            );
            prepare(
                &buoy(),
                &publication(&root, &plan_path, "generate", &["--artifact", "link"]),
            );
            let args = publication(&root, &plan_path, "check", &["--artifact", "link"]);
            invocation(buoy(), args, Vec::from([root]))
        }
        "TPT-007/positive-first-link-generation-preserves-the-basis" => {
            // The first generation inserts the canonical slot; normalizing
            // it again must reach the same publication basis, which is what
            // makes the following check equal rather than drifted.
            let root = sequence_repository();
            let plan_path = plan(
                root.path(),
                "first-generation-basis",
                requirement_selector(),
                1,
                &sequence_source_set(),
                &basis_page_and_link(),
            );
            prepare(
                &buoy(),
                &publication(&root, &plan_path, "generate", &["--artifact", "link"]),
            );
            let args = publication(&root, &plan_path, "generate", &["--artifact", "link"]);
            invocation(buoy(), args, Vec::from([root]))
        }
        "TPT-007/negative-atom-only-key-permits-unequal-immutable-pages" => {
            let root = binding_repository();
            let plan_path = plan(
                root.path(),
                "atom-only-immutable-key",
                requirement_selector(),
                1,
                &binding_source_set(),
                &format!(
                    "{},{}",
                    based_page_output("out/{atom_revision}.md", "immutable"),
                    link_output()
                ),
            );
            let args = publication(&root, &plan_path, "check", &["--artifact", "page"]);
            invocation(buoy(), args, Vec::from([root]))
        }
        "TPT-007/negative-immutable-basis-uses-a-change-detection-member" => {
            let root = change_detection_repository();
            let plan_path = plan(
                root.path(),
                "change-detection-basis",
                requirement_selector(),
                1,
                &quoted(&["openspec/specs/demo/spec.md", "src/lib.rs"]),
                &basis_page_and_link(),
            );
            let args = publication(&root, &plan_path, "check", &["--artifact", "page"]);
            invocation(buoy(), args, Vec::from([root]))
        }
        "TPT-008/positive-completion-evidence-derives-from-one-snapshot" => {
            let root = binding_repository();
            let plan_path = plan(
                root.path(),
                "one-snapshot-evidence",
                requirement_selector(),
                1,
                &binding_source_set(),
                &evidence_output("replaceable"),
            );
            prepare(
                &buoy(),
                &publication(&root, &plan_path, "generate", &["--artifact", "evidence"]),
            );
            let args = publication(&root, &plan_path, "evidence", &[]);
            invocation(buoy(), args, Vec::from([root]))
        }
        "TPT-008/negative-page-drift-blocks-new-evidence" => {
            let root = binding_repository();
            let plan_path = plan(
                root.path(),
                "evidence-page-drift",
                requirement_selector(),
                1,
                &binding_source_set(),
                &page_output("out/page.md", "replaceable"),
            );
            put(root.path(), "out/page.md", "a drifted saved page\n");
            let args = publication(&root, &plan_path, "evidence", &[]);
            invocation(buoy(), args, Vec::from([root]))
        }
        "TPT-008/negative-evidence-claims-current-authority-after-drift" => {
            // The archived document is historical: after a real later
            // source change its recorded relation is stale, and a current
            // query says so instead of the document.
            let root = binding_repository();
            let plan_path = plan(
                root.path(),
                "archived-evidence-after-drift",
                requirement_selector(),
                1,
                &binding_source_set(),
                &evidence_output("replaceable"),
            );
            prepare(
                &buoy(),
                &publication(&root, &plan_path, "generate", &["--artifact", "evidence"]),
            );
            put(
                root.path(),
                "src/lib.rs",
                &format!(
                    "{REAL_ATTRIBUTE}\npub fn render_catalog(libraries: &[&str]) -> String {{\n    \
                     libraries.join(\"\\n\") + \"a real later edit\"\n}}\n"
                ),
            );
            let args = publication(&root, &plan_path, "evidence", &[]);
            invocation(buoy(), args, Vec::from([root]))
        }
        "TPT-009/positive-equal-stale-pages-exist-under-separate-roots" => {
            let root = binding_repository();
            let plan_path = plan(
                root.path(),
                "equal-stale-pages",
                requirement_selector(),
                1,
                &binding_source_set(),
                &page_output("out/page.md", "replaceable"),
            );
            put(
                root.path(),
                "out/page.md",
                "# Implementation trace\n\nstale saved content\n",
            );
            let args = publication(&root, &plan_path, "check", &["--artifact", "page"]);
            invocation(buoy(), args, Vec::from([root]))
        }
        "TPT-009/negative-diff-includes-a-temporary-path" => {
            // Every forbidden form lives in the PRIOR PAGE's own bytes; the
            // readable parity summary must still render deterministically.
            let root = binding_repository();
            let plan_path = plan(
                root.path(),
                "forbidden-parity-fields",
                requirement_selector(),
                1,
                &binding_source_set(),
                &page_output("out/page.md", "replaceable"),
            );
            put(
                root.path(),
                "out/page.md",
                "--- a/out/page.md\n2026-08-24 11:22:33\n/tmp/buoy.tmp.4213/page.md pid=4213\n",
            );
            let args = publication(
                &root,
                &plan_path,
                "check",
                &["--artifact", "page", "--format", "human"],
            );
            invocation(buoy(), args, Vec::from([root]))
        }
        "TPT-010/positive-consumer-supplies-all-repository-policy" => {
            // Every ID, path, mount label, and selector below is consumer
            // owned; generic Buoy code applies them with no branch of its
            // own.
            let root = binding_repository();
            let plan_path = plan(
                root.path(),
                "consumer-owned-policy",
                requirement_selector(),
                1,
                &binding_source_set(),
                "{\"id\":\"consumer-edit\",\"mount\":\"repository\",\
                 \"path\":\"openspec/specs/demo/spec.md\",\"kind\":\"source-edit\",\
                 \"lifecycle\":\"replaceable\"}",
            );
            let args = publication(&root, &plan_path, "check", &["--artifact", "consumer-edit"]);
            invocation(buoy(), args, Vec::from([root]))
        }
        "TPT-010/negative-generated-evidence-claims-correctness" => {
            // A saved evidence artifact rewritten to claim correctness,
            // coverage, and proof is not the document generation derives.
            let root = binding_repository();
            let plan_path = plan(
                root.path(),
                "claimed-correctness",
                requirement_selector(),
                1,
                &binding_source_set(),
                &evidence_output("replaceable"),
            );
            prepare(
                &buoy(),
                &publication(&root, &plan_path, "generate", &["--artifact", "evidence"]),
            );
            let generated = text(&root.path().join("docs/evidence.md"));
            put(
                root.path(),
                "docs/evidence.md",
                &generated.replace(
                    "It makes no assurance claim.",
                    "It proves correctness, coverage, execution, and approval.",
                ),
            );
            let args = publication(&root, &plan_path, "check", &["--artifact", "evidence"]);
            invocation(buoy(), args, Vec::from([root]))
        }
        "TPT-011/positive-page-generation-emits-one-completed-replacement" => {
            let root = binding_repository();
            let plan_path = plan(
                root.path(),
                "one-completed-replacement",
                requirement_selector(),
                1,
                &binding_source_set(),
                &page_output("out/page.md", "replaceable"),
            );
            put(root.path(), "out/page.md", "a prior saved page\n");
            let args = publication(&root, &plan_path, "generate", &["--artifact", "page"]);
            invocation(buoy(), args, Vec::from([root]))
        }
        "TPT-011/negative-report-claims-both-repositories-changed" => {
            // One invocation updates at most one declared artifact: an
            // operation with no selected artifact cannot report a
            // multi-repository completion, it is a usage fault.
            let root = binding_repository();
            let plan_path = plan(
                root.path(),
                "multi-repository-claim",
                requirement_selector(),
                1,
                &binding_source_set(),
                &format!(
                    "{},{}",
                    page_output("out/page.md", "replaceable"),
                    edit_output()
                ),
            );
            let args = publication(&root, &plan_path, "generate", &[]);
            invocation(buoy(), args, Vec::from([root]))
        }
        _ => return None,
    })
}
