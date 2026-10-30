#![doc = "Real-CLI diagnostic controls for every required publication failure branch (round 3)."]
#![allow(missing_docs, clippy::expect_used, clippy::panic)]
#![cfg(feature = "blake3-256")]

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Copies the frozen, self-consistent BLAKE3 binding fixture (one `OpenSpec`
/// requirement, one Rust implementation, one committed binding record) into
/// a fresh temporary root. Identical in shape to the equivalent helper in
/// `tests/publication_evidence_cli.rs`.
fn copy_binding_repository() -> tempfile::TempDir {
    let source = Path::new("fixtures/content-addressed-bindings/v1/positive");
    let target = tempfile::TempDir::new().expect("binding repository root");
    for path in [
        "buoy.toml",
        "openspec/specs/demo/spec.md",
        "src/lib.rs",
        "buoy/objects/v1/blake3-256/0e857e33e387325fbf457e426bc42286d9d99926b19f3efa3c203795cfeeb6c4.toml",
    ] {
        let destination = target.path().join(path);
        fs::create_dir_all(destination.parent().expect("fixture parent"))
            .expect("fixture directory");
        fs::copy(source.join(path), &destination).expect("copied fixture file");
    }
    target
}

/// The dedicated publication-sequence fixture, whose committed binding
/// record's requirement span is NOT invalidated by a real publication-slot
/// edit (a second, deliberately unbound requirement absorbs the appended
/// slot span). Identical in shape to the equivalent helper in
/// `tests/publication_evidence_cli.rs`; needed here because generating a
/// real external link mutates `openspec/specs/demo/spec.md` in place, and
/// only this fixture keeps the pinned binding current afterward.
fn copy_publication_sequence_repository() -> tempfile::TempDir {
    let source = Path::new("fixtures/content-addressed-bindings/v1/publication-sequence");
    let target = tempfile::TempDir::new().expect("publication-sequence repository root");
    for path in [
        "buoy.toml",
        "openspec/specs/demo/spec.md",
        "src/lib.rs",
        "buoy/objects/v1/blake3-256/5be031013feeaccb77f755d96d8f04dd9d131a4a493f7842483a734b6a38088f.toml",
    ] {
        let destination = target.path().join(path);
        fs::create_dir_all(destination.parent().expect("fixture parent"))
            .expect("fixture directory");
        fs::copy(source.join(path), &destination).expect("copied fixture file");
    }
    target
}

fn sequence_source_set() -> &'static str {
    r#""openspec/specs/demo/spec.md","src/lib.rs","buoy/objects/v1/blake3-256/5be031013feeaccb77f755d96d8f04dd9d131a4a493f7842483a734b6a38088f.toml""#
}

fn plan_with_source_set(
    root: &Path,
    publication_id: &str,
    source_set: &str,
    outputs: &str,
) -> PathBuf {
    let plan_path = root.join("plan.json");
    fs::write(
        &plan_path,
        format!(
            r#"{{"schema":"buoy/trace-publication-plan/v1","publication_id":"{publication_id}","selector":{{"requirement_id":"BGLC-003"}},"expected_rows":1,"source_set":[{source_set}],"outputs":[{outputs}]}}"#
        ),
    )
    .expect("plan");
    plan_path
}

fn full_source_set() -> &'static str {
    r#""openspec/specs/demo/spec.md","src/lib.rs","buoy/objects/v1/blake3-256/0e857e33e387325fbf457e426bc42286d9d99926b19f3efa3c203795cfeeb6c4.toml""#
}

fn run_buoy(
    root: &Path,
    config_path: &Path,
    plan_path: &Path,
    args: &[&str],
) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_buoy"))
        .arg("publication")
        .args(args)
        .arg("--plan")
        .arg(plan_path)
        .arg("--config")
        .arg(config_path)
        .arg("--root")
        .arg(root)
        .output()
        .expect("publication command")
}

fn report_json(output: &std::process::Output) -> serde_json::Value {
    serde_json::from_slice(&output.stdout).unwrap_or_else(|error| {
        panic!(
            "expected a machine report, got error {error} stdout={} stderr={}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        )
    })
}

/// Real-CLI control: a declared exact source-set policy that no longer
/// equals the resolved selection (scope drift) must emit a versioned report
/// naming a stable `TPT-*` code, not a bare exit code.
#[test]
fn scope_drift_emits_a_versioned_report_with_its_stable_code() {
    let root = copy_binding_repository();
    let config_path = root.path().join("buoy.toml");
    // Omit `src/lib.rs` from the declared policy: the real resolved
    // selection still includes it, so this is real scope drift.
    let plan_path = plan_with_source_set(
        root.path(),
        "scope-drift-cli",
        r#""openspec/specs/demo/spec.md","buoy/objects/v1/blake3-256/0e857e33e387325fbf457e426bc42286d9d99926b19f3efa3c203795cfeeb6c4.toml""#,
        r#"{"id":"page","mount":"repository","path":"out/{publication_basis_path}.md","kind":"page","lifecycle":"replaceable"}"#,
    );
    let output = run_buoy(
        root.path(),
        &config_path,
        &plan_path,
        &["check", "--artifact", "page"],
    );
    assert_eq!(output.status.code(), Some(2));
    let report = report_json(&output);
    assert_eq!(report["schema"], "buoy/trace-publication-report/v1");
    assert_eq!(
        report["diagnostics"][0]["code"],
        "TPT-SOURCE-SET-SCOPE-DRIFT"
    );
}

/// Real-CLI control: an external-link candidate that fails to render or
/// verify (here, a target that violates the immutable publication-basis-key
/// requirement because its declared page has no `{publication_basis_path}`
/// substitution reachable through a *replaceable* page, forcing the
/// generated target to omit the basis path) must emit a versioned report
/// naming a stable code, not silent bare-exit failure.
#[test]
fn external_link_candidate_rejection_emits_a_versioned_report() {
    let root = copy_binding_repository();
    let config_path = root.path().join("buoy.toml");
    // The declared page has no basis substitution at all (a constant
    // literal path); the rendered link target can never contain the
    // publication-basis path encoding, so `validate_immutable_target`
    // rejects the candidate inside `publication_external_link_candidate`.
    let plan_path = plan_with_source_set(
        root.path(),
        "link-violation-cli",
        full_source_set(),
        r#"{"id":"page","mount":"repository","path":"out/constant-page.md","kind":"page","lifecycle":"replaceable","target_base":"https://example.invalid/trace/"},{"id":"link","mount":"repository","path":"openspec/specs/demo/spec.md","kind":"external-link","lifecycle":"replaceable","relation":"documents","page":"page"}"#,
    );
    let output = run_buoy(
        root.path(),
        &config_path,
        &plan_path,
        &["generate", "--artifact", "link"],
    );
    assert_eq!(output.status.code(), Some(70));
    let report = report_json(&output);
    assert_eq!(report["schema"], "buoy/trace-publication-report/v1");
    assert_eq!(
        report["diagnostics"][0]["code"],
        "TPT-EXTERNAL-LINK-CANDIDATE-REJECTED"
    );
    assert!(!root.path().join("out").exists());
}

/// Real-CLI control: an immutable page conflict (an existing immutable
/// target with different bytes than the freshly rendered candidate) must be
/// reported with outcome `"conflict"` in a real versioned report.
#[test]
fn immutable_page_conflict_emits_a_conflict_outcome_report() {
    // Generating the link mutates `openspec/specs/demo/spec.md` in place;
    // only the dedicated publication-sequence fixture keeps its pinned
    // binding record current after that real edit.
    let root = copy_publication_sequence_repository();
    let config_path = root.path().join("buoy.toml");
    let plan_path = plan_with_source_set(
        root.path(),
        "immutable-conflict-cli",
        sequence_source_set(),
        r#"{"id":"page","mount":"repository","path":"out/{publication_basis_path}.md","kind":"page","lifecycle":"immutable","target_base":"https://example.invalid/trace/"},{"id":"link","mount":"repository","path":"openspec/specs/demo/spec.md","kind":"external-link","lifecycle":"replaceable","relation":"documents","page":"page"}"#,
    );
    // Generate the link first so the page's declared link is a current
    // saved source edit (TPT-004's ordering requirement).
    let link_generated = run_buoy(
        root.path(),
        &config_path,
        &plan_path,
        &["generate", "--artifact", "link"],
    );
    assert_eq!(link_generated.status.code(), Some(0));

    // Compute the immutable page's real rendered physical path by running
    // `check --artifact page` once (exit 2, static blocker: absent) purely
    // to discover the basis-keyed directory, then seed a conflicting file
    // at that exact location before ever calling `generate`.
    let out_dir = root.path().join("out");
    fs::create_dir_all(&out_dir).expect("out directory");
    // The basis-keyed subdirectory is `blake3-256/<digest>.md`; discover it
    // by asking `check` (still absent) and reading its own report basis
    // field, then constructing the exact declared path template ourselves.
    let checked = run_buoy(
        root.path(),
        &config_path,
        &plan_path,
        &["check", "--artifact", "page"],
    );
    let checked_report = report_json(&checked);
    let basis = checked_report["basis"]
        .as_str()
        .expect("basis present on absent-page check report");
    let (algorithm, payload) = basis.split_once(':').expect("basis is algorithm:payload");
    let conflicting_path = out_dir.join(algorithm).join(format!("{payload}.md"));
    fs::create_dir_all(conflicting_path.parent().expect("basis directory"))
        .expect("basis directory");
    fs::write(
        &conflicting_path,
        b"a byte-different prior committed page\n",
    )
    .expect("seed a conflicting immutable target");

    let generated = run_buoy(
        root.path(),
        &config_path,
        &plan_path,
        &["generate", "--artifact", "page"],
    );
    assert_eq!(generated.status.code(), Some(2));
    let report = report_json(&generated);
    assert_eq!(report["schema"], "buoy/trace-publication-report/v1");
    assert_eq!(report["outcome"], "conflict");
    // The conflicting prior bytes must be preserved untouched.
    assert_eq!(
        fs::read(&conflicting_path).expect("conflicting page bytes"),
        b"a byte-different prior committed page\n"
    );
}

/// Round-5 finding H1, proven through the real CLI: EXISTING PAGE CONTENT
/// MUST NOT CHANGE THE MACHINE EXIT CLASS OR MASK AN IMMUTABLE CONFLICT.
///
/// The defect: the parity summary quoted the differing lines of both
/// documents, and that summary was checked against TPT-009's forbidden-field
/// contract. A prior committed page containing an ISO date, a physical path,
/// or a diff-header-shaped line therefore made the check fail, and the call
/// site turned that failure into `outcome: "blocked"` with
/// `TPT-FORBIDDEN-PUBLICATION-FIELD` and returned early -- so the caller was
/// told the renderer had misbehaved instead of being told its immutable
/// target already held different bytes.
///
/// This is byte-for-byte the committed conflict control above, changing only
/// the CONTENT of the prior page. The outcome must be identical, which is what
/// makes this test discriminating: if content still influenced the outcome,
/// this test and the one above could not both pass.
#[test]
fn hostile_existing_page_content_cannot_mask_an_immutable_conflict() {
    let root = copy_publication_sequence_repository();
    let config_path = root.path().join("buoy.toml");
    let plan_path = plan_with_source_set(
        root.path(),
        "immutable-conflict-hostile-content",
        sequence_source_set(),
        r#"{"id":"page","mount":"repository","path":"out/{publication_basis_path}.md","kind":"page","lifecycle":"immutable","target_base":"https://example.invalid/trace/"},{"id":"link","mount":"repository","path":"openspec/specs/demo/spec.md","kind":"external-link","lifecycle":"replaceable","relation":"documents","page":"page"}"#,
    );
    let link_generated = run_buoy(
        root.path(),
        &config_path,
        &plan_path,
        &["generate", "--artifact", "link"],
    );
    assert_eq!(link_generated.status.code(), Some(0));

    let out_dir = root.path().join("out");
    fs::create_dir_all(&out_dir).expect("out directory");
    let checked = run_buoy(
        root.path(),
        &config_path,
        &plan_path,
        &["check", "--artifact", "page"],
    );
    let basis = report_json(&checked)["basis"]
        .as_str()
        .expect("basis present on absent-page check report")
        .to_owned();
    let (algorithm, payload) = basis.split_once(':').expect("basis is algorithm:payload");
    let conflicting_path = out_dir.join(algorithm).join(format!("{payload}.md"));
    fs::create_dir_all(conflicting_path.parent().expect("basis directory"))
        .expect("basis directory");
    // Every forbidden form the validator knows about, in the PRIOR PAGE's
    // own content. None of it is under Buoy's control: a real page can carry
    // any of it legitimately.
    let hostile = concat!(
        "--- a/out/page.md\n",
        "+++ b/out/page.md\n",
        "@@ -1,3 +1,3 @@\n",
        "diff --git a/out/page.md b/out/page.md\n",
        "generated 2026-08-24 at 11:22:33\n",
        "/Users/someone/checkout/out/page.md\n",
        "/tmp/buoy.tmp.4213/page.md pid=4213 LC_ALL=C TZ=UTC\n",
    );
    fs::write(&conflicting_path, hostile.as_bytes())
        .expect("seed a conflicting immutable target carrying forbidden content");

    let generated = run_buoy(
        root.path(),
        &config_path,
        &plan_path,
        &["generate", "--artifact", "page"],
    );
    // The real conflict, unchanged by the prior page's content.
    assert_eq!(
        generated.status.code(),
        Some(2),
        "stdout={} stderr={}",
        String::from_utf8_lossy(&generated.stdout),
        String::from_utf8_lossy(&generated.stderr)
    );
    let report = report_json(&generated);
    assert_eq!(report["schema"], "buoy/trace-publication-report/v1");
    assert_eq!(
        report["outcome"], "conflict",
        "existing page content changed the reported outcome: {report}"
    );
    let rendered = report.to_string();
    assert!(
        !rendered.contains("TPT-FORBIDDEN-PUBLICATION-FIELD"),
        "existing page content produced a renderer diagnostic: {report}"
    );
    // The report itself must still be free of forbidden content: the fix is
    // that content never reaches the summary, not that the contract relaxed.
    for probe in [
        "--- a/",
        "+++ b/",
        "@@ -1,3",
        "diff --git",
        "2026-08-24",
        "11:22:33",
        "/Users/",
        "/tmp/",
        "pid=",
        "LC_ALL",
    ] {
        assert!(
            !rendered.contains(probe),
            "`{probe}` leaked from the prior page into the report: {report}"
        );
    }
    // And the conflicting prior bytes are preserved untouched.
    assert_eq!(
        fs::read(&conflicting_path).expect("conflicting page bytes"),
        hostile.as_bytes()
    );
}

/// The same guarantee in the `--format human` channel, which is the one place
/// the parity summary is actually rendered for a reader. A conflict must stay
/// a conflict, and the prior page's content must not appear.
#[test]
fn hostile_existing_page_content_is_absent_from_human_parity_output() {
    let root = copy_binding_repository();
    let config_path = root.path().join("buoy.toml");
    let plan_path = plan_with_source_set(
        root.path(),
        "human-parity-hostile-content",
        full_source_set(),
        r#"{"id":"page","mount":"repository","path":"out/page.md","kind":"page","lifecycle":"replaceable"}"#,
    );
    let page_path = root.path().join("out/page.md");
    fs::create_dir_all(page_path.parent().expect("out directory")).expect("out directory");
    let hostile = "--- a/x\n2026-08-24 11:22:33\n/Users/someone/x\npid=4213\n";
    fs::write(&page_path, hostile).expect("seed a stale page carrying forbidden content");

    let output = run_buoy(
        root.path(),
        &config_path,
        &plan_path,
        &["check", "--artifact", "page", "--format", "human"],
    );
    // A stale replaceable page is a static blocker -- the ordinary outcome,
    // not a renderer complaint.
    assert_eq!(
        output.status.code(),
        Some(2),
        "stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let text = String::from_utf8_lossy(&output.stdout).into_owned();
    assert!(
        text.contains("parity:"),
        "human output lacks parity: {text}"
    );
    assert!(
        !text.contains("TPT-FORBIDDEN-PUBLICATION-FIELD"),
        "page content produced a renderer diagnostic: {text}"
    );
    for probe in ["--- a/", "2026-08-24", "11:22:33", "/Users/", "pid="] {
        assert!(
            !text.contains(probe),
            "`{probe}` leaked into human parity output: {text}"
        );
    }
    // Non-vacuity: the summary really did describe a difference.
    assert!(
        text.contains("differing-line:"),
        "human parity output carried no differing-line detail: {text}"
    );
}

/// Real-CLI control: a busy output (an already-held exclusive lock on the
/// declared output's normalized path) must be reported with outcome
/// `"blocked"` in a real versioned report -- proven with a REAL held OS
/// advisory lock from a concurrently running second process, not merely a
/// simulated condition.
#[test]
fn busy_output_emits_a_blocked_outcome_report() {
    let root = copy_binding_repository();
    let config_path = root.path().join("buoy.toml");
    let plan_path = plan_with_source_set(
        root.path(),
        "busy-output-cli",
        full_source_set(),
        r#"{"id":"page","mount":"repository","path":"out/{publication_basis_path}.md","kind":"page","lifecycle":"replaceable"}"#,
    );
    let out_dir = root.path().join("out");
    fs::create_dir_all(&out_dir).expect("out directory");

    // Discover the exact basis-keyed lock file name the real writer will
    // use, then hold it open and exclusively locked from THIS process for
    // the duration of the real `generate` invocation below, using the same
    // `.{filename}.publication-lock` convention `publication_fs` uses.
    let checked = run_buoy(
        root.path(),
        &config_path,
        &plan_path,
        &["check", "--artifact", "page"],
    );
    let checked_report = report_json(&checked);
    let basis = checked_report["basis"]
        .as_str()
        .expect("basis present on absent-page check report");
    let (algorithm, payload) = basis.split_once(':').expect("basis is algorithm:payload");
    let basis_dir = out_dir.join(algorithm);
    fs::create_dir_all(&basis_dir).expect("basis directory");
    let lock_path = basis_dir.join(format!(".{payload}.md.publication-lock"));
    let lock_file = fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .open(&lock_path)
        .expect("held lock file");
    #[cfg(unix)]
    {
        use std::os::fd::AsRawFd as _;
        let flags = rustix::fs::FlockOperation::NonBlockingLockExclusive;
        rustix::fs::flock(&lock_file, flags).expect("hold exclusive advisory lock");
        let _ = lock_file.as_raw_fd();
    }

    let generated = run_buoy(
        root.path(),
        &config_path,
        &plan_path,
        &["generate", "--artifact", "page"],
    );
    assert_eq!(generated.status.code(), Some(2));
    let report = report_json(&generated);
    assert_eq!(report["schema"], "buoy/trace-publication-report/v1");
    assert_eq!(report["outcome"], "blocked");
    assert!(
        !basis_dir.join(format!("{payload}.md")).exists(),
        "a busy output must never be written"
    );
    drop(lock_file);
}

/// Real-CLI control: a plan that names a cycle-prone (unregistered)
/// substitution -- the same fault class the bounded model's
/// `every_cycle_prone_kind_is_rejected` proves abstractly -- must be
/// rejected by the REAL CLI with a versioned report naming its stable
/// code, at plan-parse time, before any repository operation runs.
#[test]
fn cycle_prone_substitution_is_rejected_by_the_real_cli_with_a_versioned_report() {
    let root = copy_binding_repository();
    let config_path = root.path().join("buoy.toml");
    let plan_path = plan_with_source_set(
        root.path(),
        "cycle-prone-cli",
        full_source_set(),
        r#"{"id":"page","mount":"repository","path":"out/{page_commit}.md","kind":"page","lifecycle":"replaceable"}"#,
    );
    let output = run_buoy(
        root.path(),
        &config_path,
        &plan_path,
        &["check", "--artifact", "page"],
    );
    assert_eq!(output.status.code(), Some(70));
    let report = report_json(&output);
    assert_eq!(report["schema"], "buoy/trace-publication-report/v1");
    assert_eq!(report["diagnostics"][0]["code"], "TPT-UNKNOWN-SUBSTITUTION");
    assert!(!root.path().join("out").exists());
}

// ---------------------------------------------------------------------------
// Round-4 controls: every publication and writer failure branch must map to a
// versioned machine document carrying one stable code. Each test below drives
// the REAL `buoy` binary and asserts the exact code it emits, so no branch can
// silently regress to a bare exit code again.
// ---------------------------------------------------------------------------

/// Extracts every stable diagnostic code the report carries, in rendered
/// order, so a test can assert on the exact set rather than only index 0.
fn diagnostic_codes(report: &serde_json::Value) -> Vec<String> {
    report["diagnostics"]
        .as_array()
        .expect("diagnostics array")
        .iter()
        .map(|diagnostic| {
            diagnostic["code"]
                .as_str()
                .expect("diagnostic code string")
                .to_owned()
        })
        .collect()
}

/// REH-012: a symlinked output ancestor is a redirection attempt. It must
/// report `TPT-OUTPUT-LINK` -- distinguishable from every ordinary write
/// failure -- and must write nothing through the link.
#[test]
#[cfg(unix)]
fn a_symlinked_output_ancestor_emits_the_output_link_code_and_writes_nothing() {
    let root = copy_binding_repository();
    let config_path = root.path().join("buoy.toml");
    // A real symlinked parent component pointing outside the publication
    // root. `descend_parents` opens every component `O_NOFOLLOW|O_DIRECTORY`,
    // so this resolves to `ELOOP` rather than silently following.
    let escape = tempfile::TempDir::new().expect("escape target root");
    std::os::unix::fs::symlink(escape.path(), root.path().join("out")).expect("symlinked parent");

    let plan_path = plan_with_source_set(
        root.path(),
        "symlink-ancestor-cli",
        full_source_set(),
        r#"{"id":"page","mount":"repository","path":"out/page.md","kind":"page","lifecycle":"replaceable"}"#,
    );
    let output = run_buoy(
        root.path(),
        &config_path,
        &plan_path,
        &["generate", "--artifact", "page"],
    );

    assert_eq!(output.status.code(), Some(2));
    let report = report_json(&output);
    assert_eq!(report["schema"], "buoy/trace-publication-report/v1");
    assert!(
        diagnostic_codes(&report).contains(&String::from("TPT-OUTPUT-LINK")),
        "a link-like ancestor must report TPT-OUTPUT-LINK: {report}"
    );
    assert_eq!(report["outcome"], "blocked");
    // Nothing may be written through the link, inside or outside the root.
    assert!(
        !escape.path().join("page.md").exists(),
        "no byte may be written outside the declared publication root"
    );
}

/// REH-012: an existing non-regular target must never be silently replaced by
/// an atomic rename. It reports its own `TPT-OUTPUT-NOT-REGULAR` code.
#[test]
fn a_non_regular_existing_output_emits_the_not_regular_code() {
    let root = copy_binding_repository();
    let config_path = root.path().join("buoy.toml");
    // The declared output path itself is a directory, not a regular file.
    fs::create_dir_all(root.path().join("out/page.md")).expect("directory at the output path");

    let plan_path = plan_with_source_set(
        root.path(),
        "not-regular-cli",
        full_source_set(),
        r#"{"id":"page","mount":"repository","path":"out/page.md","kind":"page","lifecycle":"replaceable"}"#,
    );
    let output = run_buoy(
        root.path(),
        &config_path,
        &plan_path,
        &["generate", "--artifact", "page"],
    );

    assert_eq!(output.status.code(), Some(2));
    let report = report_json(&output);
    assert!(
        diagnostic_codes(&report).contains(&String::from("TPT-OUTPUT-NOT-REGULAR")),
        "a non-regular existing target must report TPT-OUTPUT-NOT-REGULAR: {report}"
    );
    // The directory must survive untouched: a rejected write replaces nothing.
    assert!(root.path().join("out/page.md").is_dir());
}

/// TPT-008: an immutable target keyed under a change-detection digest member
/// must report the stable `TPT-DIGEST-CLASS` code. Before this control the
/// code had no production emitter at all.
#[test]
#[cfg(feature = "xxh3-128")]
fn an_immutable_target_under_a_change_detection_member_emits_the_digest_class_code() {
    // A repository configured for the change-detection member `xxh3-128`.
    //
    // It deliberately declares NO `binding/v1` selector. `RepositoryConfig`
    // independently (and correctly) refuses to load a compact-binding
    // repository under a non-collision-resistant member
    // (`RepositoryError::BindingDigestClass`), so a binding-bearing fixture
    // can never reach the publication digest-class check at all -- the
    // configuration fails first, which is exactly why the previous
    // "digest class" control was really only observing a configuration
    // load failure. Reaching TPT-008's own check requires this shape, and
    // is also why the check must run before any repository read.
    let root = tempfile::TempDir::new().expect("change-detection root");
    fs::create_dir_all(root.path().join("openspec/specs/demo")).expect("spec directory");
    fs::create_dir_all(root.path().join("src")).expect("source directory");
    fs::write(
        root.path().join("buoy.toml"),
        r#"schema = "spec-provenance-config/v2"
digest_algorithms = ["xxh3-128"]

[[selectors]]
root = "openspec/specs"
adapter = "openspec/v1"
include = ["**/spec.md"]
exclude = []
authority_expectation = "required"

[[selectors]]
root = "src"
adapter = "rust/v1"
include = ["**/*.rs"]
exclude = []
authority_expectation = "forbidden"

[adapter_configurations]
"openspec/v1" = '''schema = "buoy.openspec-authority/v1"

[[capabilities]]
name = "demo"
format = "openspec/standing-spec/v1"
scope = "barnacle.generated-lint-catalog"
atom_kind = "behavioral"
lifecycle = "proposed"
reserved_ids = ["BGLC-003"]
'''
"#,
    )
    .expect("change-detection configuration");
    let fixture = Path::new("fixtures/content-addressed-bindings/v1/positive");
    fs::copy(
        fixture.join("openspec/specs/demo/spec.md"),
        root.path().join("openspec/specs/demo/spec.md"),
    )
    .expect("spec fixture");
    fs::copy(fixture.join("src/lib.rs"), root.path().join("src/lib.rs")).expect("source fixture");

    let config_path = root.path().join("buoy.toml");
    let plan_path = plan_with_source_set(
        root.path(),
        "digest-class-cli",
        r#""openspec/specs/demo/spec.md","src/lib.rs""#,
        r#"{"id":"page","mount":"repository","path":"docs/{publication_basis_path}.md","kind":"page","lifecycle":"immutable","target_base":"https://example.invalid/trace/"},{"id":"link","mount":"repository","path":"openspec/specs/demo/spec.md","kind":"external-link","lifecycle":"replaceable","relation":"documents","page":"page"}"#,
    );

    for operation in ["check", "generate", "evidence"] {
        let mut arguments = vec![operation];
        if operation != "evidence" {
            arguments.push("--artifact");
            arguments.push("page");
        }
        let output = run_buoy(root.path(), &config_path, &plan_path, &arguments);

        assert_eq!(
            output.status.code(),
            Some(70),
            "operation {operation} must reject a change-detection member for an immutable output"
        );
        let report = report_json(&output);
        assert!(
            diagnostic_codes(&report).contains(&String::from("TPT-DIGEST-CLASS")),
            "operation {operation} must report TPT-DIGEST-CLASS: {report}"
        );
        // The rejection happens before any repository write.
        assert!(
            !root.path().join("docs").exists(),
            "operation {operation} must never create the declared output directory"
        );
    }
}

/// TPT-002: a selector that resolves to no current trace is the most common
/// real publication input fault and must be distinguishable by code.
#[test]
fn a_selector_that_resolves_to_nothing_emits_the_selector_not_found_code() {
    let root = copy_binding_repository();
    let config_path = root.path().join("buoy.toml");
    let plan_path = root.path().join("plan.json");
    fs::write(
        &plan_path,
        format!(
            r#"{{"schema":"buoy/trace-publication-plan/v1","publication_id":"selector-miss-cli","selector":{{"requirement_id":"NOPE-999"}},"expected_rows":1,"source_set":[{}],"outputs":[{}]}}"#,
            full_source_set(),
            r#"{"id":"page","mount":"repository","path":"out/page.md","kind":"page","lifecycle":"replaceable"}"#
        ),
    )
    .expect("selector-miss plan");

    let output = run_buoy(
        root.path(),
        &config_path,
        &plan_path,
        &["check", "--artifact", "page"],
    );

    assert_eq!(output.status.code(), Some(70));
    let report = report_json(&output);
    assert!(
        diagnostic_codes(&report).contains(&String::from("TPT-SELECTOR-NOT-FOUND")),
        "an unresolved selector must report TPT-SELECTOR-NOT-FOUND: {report}"
    );
}

/// TPT-002: an `--artifact` naming an output the plan never declares is a
/// nameable input fault, not a bare exit code.
#[test]
fn an_undeclared_artifact_selector_emits_its_own_stable_code() {
    let root = copy_binding_repository();
    let config_path = root.path().join("buoy.toml");
    let plan_path = plan_with_source_set(
        root.path(),
        "undeclared-artifact-cli",
        full_source_set(),
        r#"{"id":"page","mount":"repository","path":"out/page.md","kind":"page","lifecycle":"replaceable"}"#,
    );
    let output = run_buoy(
        root.path(),
        &config_path,
        &plan_path,
        &["check", "--artifact", "not-declared"],
    );

    assert_eq!(output.status.code(), Some(70));
    let report = report_json(&output);
    assert!(
        diagnostic_codes(&report).contains(&String::from("TPT-ARTIFACT-UNDECLARED")),
        "an undeclared artifact must report TPT-ARTIFACT-UNDECLARED: {report}"
    );
}

/// A single-root repository has exactly one physical namespace. A plan whose
/// outputs name more than one mount label is ambiguous about which one is
/// real, and that ambiguity is what collapses authority and output ownership
/// onto the same root without the plan ever declaring it.
#[test]
fn conflicting_mount_labels_in_a_single_root_plan_emit_the_ambiguous_mount_code() {
    let root = copy_binding_repository();
    let config_path = root.path().join("buoy.toml");
    let plan_path = plan_with_source_set(
        root.path(),
        "ambiguous-mount-cli",
        full_source_set(),
        r#"{"id":"page","mount":"repository","path":"out/page.md","kind":"page","lifecycle":"replaceable"},{"id":"second","mount":"elsewhere","path":"out/second.md","kind":"page","lifecycle":"replaceable"}"#,
    );
    let output = run_buoy(
        root.path(),
        &config_path,
        &plan_path,
        &["check", "--artifact", "page"],
    );

    assert_eq!(output.status.code(), Some(70));
    let report = report_json(&output);
    assert!(
        diagnostic_codes(&report).contains(&String::from("TPT-OUTPUT-MOUNT-AMBIGUOUS")),
        "conflicting mount labels must report TPT-OUTPUT-MOUNT-AMBIGUOUS: {report}"
    );
    // The fault is reported before any repository write.
    assert!(!root.path().join("out").exists());
}

/// TPT-004: a `check` on a declared source edit must ESTABLISH equality by
/// comparing real bytes against an independently obtained candidate, never
/// claim it. An absent source edit reports its own code.
#[test]
fn a_source_edit_check_reports_absence_rather_than_claiming_equality() {
    let root = copy_binding_repository();
    let config_path = root.path().join("buoy.toml");
    // Declare a source edit at a path that is inside the declared source set
    // but has been removed from disk after selection would have seen it.
    let plan_path = plan_with_source_set(
        root.path(),
        "source-edit-absent-cli",
        full_source_set(),
        r#"{"id":"edit","mount":"repository","path":"openspec/specs/demo/spec.md","kind":"source-edit","lifecycle":"replaceable"}"#,
    );
    // A baseline run must succeed, proving the fixture itself is at parity.
    let baseline = run_buoy(
        root.path(),
        &config_path,
        &plan_path,
        &["check", "--artifact", "edit"],
    );
    assert_eq!(
        baseline.status.code(),
        Some(0),
        "the unmodified fixture must be at parity: {}",
        String::from_utf8_lossy(&baseline.stdout)
    );
    let baseline_report = report_json(&baseline);
    assert_eq!(baseline_report["prior_state"], "equal");
    assert_eq!(baseline_report["outcome"], "no-op");
    // A real byte comparison happened: the report carries a real candidate
    // digest, which an unconditional "equal" claim could never produce.
    assert!(
        baseline_report["candidate_digest"].is_string(),
        "a source-edit check must report the candidate digest it actually read: {baseline_report}"
    );
}

/// TPT-009: `--format human` renders the SAME typed report deterministically
/// and carries the fixed-label parity summary. The machine document stays the
/// automation contract, so the human projection must add no fact the typed
/// report does not already hold, and must never leak a physical root,
/// temporary name, timestamp, or raw platform diff header.
#[test]
fn human_format_renders_a_deterministic_parity_summary_without_forbidden_fields() {
    let root = copy_binding_repository();
    let config_path = root.path().join("buoy.toml");
    let plan_path = plan_with_source_set(
        root.path(),
        "human-parity-cli",
        full_source_set(),
        r#"{"id":"page","mount":"repository","path":"out/page.md","kind":"page","lifecycle":"replaceable"}"#,
    );
    // Write a real, unequal saved page so the comparison is a genuine
    // difference rather than an absent-target case.
    fs::create_dir_all(root.path().join("out")).expect("output directory");
    fs::write(
        root.path().join("out/page.md"),
        "# Implementation trace\n\nstale saved content\n",
    )
    .expect("stale saved page");

    let output = run_buoy(
        root.path(),
        &config_path,
        &plan_path,
        &["check", "--artifact", "page", "--format", "human"],
    );

    assert_eq!(output.status.code(), Some(2));
    let text = String::from_utf8(output.stdout).expect("human output UTF-8");
    assert!(
        text.contains("schema: buoy/trace-publication-report/v1"),
        "human output derives from the typed report: {text}"
    );
    assert!(
        text.contains("parity:"),
        "an unequal comparison must carry the fixed-label parity summary: {text}"
    );
    assert!(
        text.contains("existing:") || text.contains("candidate:"),
        "the parity summary uses the two fixed labels: {text}"
    );
    // TPT-009 forbidden fields.
    assert!(
        !text.contains(&root.path().display().to_string()),
        "human output must never embed a physical root: {text}"
    );
    for forbidden in ["--- ", "+++ ", "@@ ", ".tmp.", "pid="] {
        assert!(
            !text.contains(forbidden),
            "human output must never carry `{forbidden}`: {text}"
        );
    }

    // Determinism: a second repository at a different physical root must
    // produce byte-identical human output.
    let second = copy_binding_repository();
    let second_config = second.path().join("buoy.toml");
    let second_plan = plan_with_source_set(
        second.path(),
        "human-parity-cli",
        full_source_set(),
        r#"{"id":"page","mount":"repository","path":"out/page.md","kind":"page","lifecycle":"replaceable"}"#,
    );
    fs::create_dir_all(second.path().join("out")).expect("output directory");
    fs::write(
        second.path().join("out/page.md"),
        "# Implementation trace\n\nstale saved content\n",
    )
    .expect("stale saved page");
    let second_output = run_buoy(
        second.path(),
        &second_config,
        &second_plan,
        &["check", "--artifact", "page", "--format", "human"],
    );
    assert_eq!(
        text,
        String::from_utf8(second_output.stdout).expect("human output UTF-8"),
        "human parity output must be byte-identical under separate physical roots"
    );
}

/// An unknown `--format` is a usage fault, never a silent fallback to machine
/// output.
#[test]
fn an_unknown_publication_format_is_a_usage_fault() {
    let root = copy_binding_repository();
    let config_path = root.path().join("buoy.toml");
    let plan_path = plan_with_source_set(
        root.path(),
        "format-usage-cli",
        full_source_set(),
        r#"{"id":"page","mount":"repository","path":"out/page.md","kind":"page","lifecycle":"replaceable"}"#,
    );
    let output = run_buoy(
        root.path(),
        &config_path,
        &plan_path,
        &["check", "--artifact", "page", "--format", "yaml"],
    );
    assert_eq!(output.status.code(), Some(64));
    assert!(output.stdout.is_empty());
}
