#![doc = "Mounted end-to-end containment controls for the publication pipeline (round-3 findings)."]
#![allow(missing_docs, clippy::expect_used, clippy::panic)]
#![cfg(feature = "blake3-256")]

use buoy::core::LogicalMountId;
use buoy::repository::{MountMappings, RepositoryConfig, snapshot_mounted_repository};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use tempfile::TempDir;

const CONFIG: &str = "fixtures/logical-mount-artifact-bindings/v1/configs/artifact-binding.toml";
const REQUIREMENT_SPEC: &str = "# Demo\n\n## Requirements\n\n### Requirement: OSA-900 Preserve bytes\nThe project MUST preserve exact bytes.\n\n#### Scenario: Exact bytes\n\n- **GIVEN** one artifact\n- **WHEN** validation runs\n- **THEN** the bytes remain exact\n";

/// A real two-mount repository (`project`, `authority`) with a genuinely
/// bound requirement: `openspec/specs/demo/spec.md` under `authority`
/// declares `OSA-900`, `artifact.bin` under `project` is the bound
/// implementation, and the binding record itself is produced by one real
/// `buoy bind` CLI invocation -- not fabricated bytes.
struct MountedBindingFixture {
    project: TempDir,
    authority: TempDir,
    config_path: PathBuf,
    /// Every selected logical (mount-prefixed) path this snapshot resolves,
    /// in the exact form `buoy publication`'s own retained snapshot uses.
    source_set: Vec<String>,
}

impl MountedBindingFixture {
    fn build() -> Self {
        let project = TempDir::new().expect("project mount root");
        let authority = TempDir::new().expect("authority mount root");
        let specification = authority.path().join("openspec/specs/demo/spec.md");
        fs::create_dir_all(specification.parent().expect("specification parent"))
            .expect("specification directory");
        fs::write(&specification, REQUIREMENT_SPEC).expect("standing specification");
        fs::write(
            project.path().join("artifact.bin"),
            b"mounted-artifact-bytes",
        )
        .expect("bound artifact bytes");
        fs::create_dir_all(project.path().join("buoy")).expect("binding store root");

        let config_path = PathBuf::from(CONFIG);
        let config = RepositoryConfig::parse(&fs::read(&config_path).expect("config bytes"))
            .expect("valid mounted configuration");
        let mappings = MountMappings::new(vec![
            (
                LogicalMountId::new("project").expect("project mount ID"),
                project.path().to_path_buf(),
            ),
            (
                LogicalMountId::new("authority").expect("authority mount ID"),
                authority.path().to_path_buf(),
            ),
        ])
        .expect("mount mappings");
        let snapshot =
            snapshot_mounted_repository(&mappings, &config).expect("pre-bind mounted snapshot");
        let checked =
            buoy::check::check_snapshot(&snapshot, &config).expect("pre-bind mounted check");
        let requirement = checked
            .source
            .graph()
            .nodes()
            .iter()
            .find(|node| node.item().kind().as_str() == "openspec/requirement/v1")
            .expect("OSA-900 requirement node")
            .reference()
            .canonical_text();

        let bind_output = Command::new(env!("CARGO_BIN_EXE_buoy"))
            .args([
                "bind",
                "--mount",
                &format!("project={}", project.path().display()),
                "--mount",
                &format!("authority={}", authority.path().display()),
                "--config",
                config_path.to_str().expect("UTF-8 config path"),
                "--requirement",
                &requirement,
                "--artifact",
                "project/artifact.bin",
                "--relation",
                "PUBLICATION-MOUNT-CONTROL",
                "--format",
                "machine",
            ])
            .output()
            .expect("real mounted bind command");
        assert!(
            bind_output.status.success(),
            "stdout={} stderr={}",
            String::from_utf8_lossy(&bind_output.stdout),
            String::from_utf8_lossy(&bind_output.stderr)
        );

        let mut source_set = vec![
            String::from("authority/openspec/specs/demo/spec.md"),
            String::from("project/artifact.bin"),
        ];
        for path in binding_toml_files(&project.path().join("buoy")) {
            source_set.push(format!(
                "project/{}",
                path.strip_prefix(project.path())
                    .expect("binding file under project root")
                    .to_str()
                    .expect("UTF-8 binding path")
            ));
        }
        source_set.sort();

        Self {
            project,
            authority,
            config_path,
            source_set,
        }
    }

    fn write_plan(&self, outputs_json: &str) -> PathBuf {
        let source_set_json = self
            .source_set
            .iter()
            .map(|path| format!("{path:?}"))
            .collect::<Vec<_>>()
            .join(",");
        let plan_path = self.project.path().join("plan.json");
        fs::write(
            &plan_path,
            format!(
                r#"{{"schema":"buoy/trace-publication-plan/v1","publication_id":"mounted-containment","selector":{{"requirement_id":"OSA-900"}},"expected_rows":1,"source_set":[{source_set_json}],"outputs":[{outputs_json}]}}"#
            ),
        )
        .expect("mounted publication plan");
        plan_path
    }

    fn run(&self, arguments: &[&str]) -> std::process::Output {
        Command::new(env!("CARGO_BIN_EXE_buoy"))
            .arg("publication")
            .args(arguments)
            .arg("--config")
            .arg(&self.config_path)
            .arg("--mount")
            .arg(format!("project={}", self.project.path().display()))
            .arg("--mount")
            .arg(format!("authority={}", self.authority.path().display()))
            .output()
            .expect("real mounted publication command")
    }

    /// Every file's complete bytes under both mount roots, keyed by
    /// mount-relative path, for a byte-identical before/after comparison.
    fn snapshot_bytes(&self) -> std::collections::BTreeMap<String, Vec<u8>> {
        let mut files = std::collections::BTreeMap::new();
        for (label, root) in [
            ("project", self.project.path()),
            ("authority", self.authority.path()),
        ] {
            walk_files(root, root, label, &mut files);
        }
        files
    }
}

fn walk_files(
    root: &Path,
    current: &Path,
    label: &str,
    out: &mut std::collections::BTreeMap<String, Vec<u8>>,
) {
    let Ok(entries) = fs::read_dir(current) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let file_type = entry.file_type().expect("entry file type");
        if file_type.is_dir() {
            walk_files(root, &path, label, out);
        } else if file_type.is_file() {
            let relative = path
                .strip_prefix(root)
                .expect("entry under mount root")
                .to_str()
                .expect("UTF-8 entry path");
            out.insert(
                format!("{label}/{relative}"),
                fs::read(&path).expect("entry bytes"),
            );
        }
    }
}

fn binding_toml_files(root: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    collect_toml_files(root, &mut found);
    found.sort();
    found
}

fn collect_toml_files(current: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(current) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let file_type = entry.file_type().expect("entry file type");
        if file_type.is_dir() {
            collect_toml_files(&path, out);
        } else if path
            .extension()
            .is_some_and(|extension| extension == "toml")
        {
            out.push(path);
        }
    }
}

/// H1 (round 3): a plan output path can only be proven to collide with a
/// selected source once its declared mount is taken into account. The
/// declared output here is `mount: "project", path: "artifact.bin"`, which
/// collides with the separately selected `project/artifact.bin` source ONLY
/// once the mount prefix is applied -- the bare unqualified string
/// `"artifact.bin"` never appears anywhere in the real selected set at all.
#[test]
fn mounted_output_path_collision_is_rejected_only_once_mount_qualified() {
    let fixture = MountedBindingFixture::build();

    // First, prove directly that the OLD (pre-fix) code path -- classifying
    // the bare, un-mount-qualified rendered string against the selected set
    // -- would have MISSED this collision entirely: the literal string
    // "artifact.bin" is not a member of the logical (mount-prefixed)
    // selected set, so a check that never applies the mount prefix reports
    // this output as safely "derived".
    let bare_classification =
        buoy::publication::classify_output("artifact.bin", "derived", &fixture.source_set);
    assert!(
        bare_classification.is_ok(),
        "the unqualified bare string alone must NOT be sufficient to prove containment \
         (this documents the exact blind spot round-3 finding H1 identified): {bare_classification:?}"
    );

    // Now prove the real, fixed CLI pipeline -- which projects the declared
    // output's mount + path into the SAME logical namespace the retained
    // snapshot's own selected paths already use -- correctly rejects this
    // exact plan.
    let plan_path = fixture.write_plan(
        r#"{"id":"collide","mount":"project","path":"artifact.bin","kind":"page","lifecycle":"replaceable"}"#,
    );
    let before = fixture.snapshot_bytes();
    let output = fixture.run(&[
        "generate",
        "--plan",
        plan_path.to_str().expect("UTF-8 plan path"),
        "--artifact",
        "collide",
    ]);
    assert_eq!(
        output.status.code(),
        Some(2),
        "a plan output that collides with a selected source only once mount-qualified must be \
         rejected: stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    // Round-3 required repair: a failed mounted operation must leave both
    // mounts byte-identical and directory-identical to their prior state.
    let after = fixture.snapshot_bytes();
    assert_eq!(
        before, after,
        "a rejected mounted operation must never mutate either mount"
    );
}

/// H2 (round 3): mounted external-link generation must apply the mount
/// prefix exactly once -- physical descent happens under the resolved
/// `authority` mount's OWN root, using the mount-RELATIVE declared path, so
/// the write must land at `<authority-root>/openspec/specs/demo/spec.md`,
/// never at a doubly-prefixed `<authority-root>/authority/openspec/...`.
#[test]
fn mounted_external_link_generation_writes_the_single_declared_authority_path() {
    let fixture = MountedBindingFixture::build();
    let plan_path = fixture.write_plan(
        r#"{"id":"page","mount":"project","path":"pages/{publication_basis_path}.md","kind":"page","lifecycle":"immutable","target_base":"https://example.invalid/trace/"},{"id":"link","mount":"authority","path":"openspec/specs/demo/spec.md","kind":"external-link","lifecycle":"replaceable","relation":"documents","page":"page"}"#,
    );
    let output = fixture.run(&[
        "generate",
        "--plan",
        plan_path.to_str().expect("UTF-8 plan path"),
        "--artifact",
        "link",
    ]);
    assert_eq!(
        output.status.code(),
        Some(0),
        "stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    // The single, correctly mount-relative write location.
    let expected = fixture.authority.path().join("openspec/specs/demo/spec.md");
    assert!(
        expected.is_file(),
        "the declared authority document must be edited in place"
    );
    let edited = fs::read_to_string(&expected).expect("edited authority document");
    assert!(
        edited.contains("buoy-external-trace-v1"),
        "the real publication slot must be written into the single correct location"
    );

    // No doubly-prefixed stray directory: `<authority-root>/authority/...`
    // must never exist.
    assert!(
        !fixture.authority.path().join("authority").exists(),
        "mounted external-link generation must never double-apply its own mount prefix"
    );
    // No stray directory under the project mount either.
    assert!(!fixture.project.path().join("authority").exists());
}

/// Round-3 required repair: a failed mounted operation (here, an
/// output-path collision detected only once mount-qualified) must leave
/// BOTH mounts byte-identical and directory-identical to their prior state
/// -- no partial mutation, no stray temporary or lock artifact left behind
/// under either mount.
#[test]
fn failed_mounted_generate_leaves_both_mounts_directory_identical() {
    let fixture = MountedBindingFixture::build();
    let plan_path = fixture.write_plan(
        r#"{"id":"collide","mount":"project","path":"artifact.bin","kind":"page","lifecycle":"replaceable"}"#,
    );
    let before_project: Vec<_> = walk_names(fixture.project.path());
    let before_authority: Vec<_> = walk_names(fixture.authority.path());
    let before_bytes = fixture.snapshot_bytes();

    let output = fixture.run(&[
        "generate",
        "--plan",
        plan_path.to_str().expect("UTF-8 plan path"),
        "--artifact",
        "collide",
    ]);
    assert_eq!(output.status.code(), Some(2));

    let after_project: Vec<_> = walk_names(fixture.project.path());
    let after_authority: Vec<_> = walk_names(fixture.authority.path());
    let after_bytes = fixture.snapshot_bytes();
    assert_eq!(
        before_project, after_project,
        "the project mount's directory set must stay identical after a failed operation"
    );
    assert_eq!(
        before_authority, after_authority,
        "the authority mount's directory set must stay identical after a failed operation"
    );
    assert_eq!(before_bytes, after_bytes);
}

fn walk_names(root: &Path) -> Vec<String> {
    let mut names = Vec::new();
    collect_names(root, root, &mut names);
    names.sort();
    names
}

fn collect_names(root: &Path, current: &Path, out: &mut Vec<String>) {
    let Ok(entries) = fs::read_dir(current) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        out.push(
            path.strip_prefix(root)
                .expect("entry under root")
                .to_str()
                .expect("UTF-8 entry path")
                .to_owned(),
        );
        if entry.file_type().expect("entry file type").is_dir() {
            collect_names(root, &path, out);
        }
    }
}

/// Round-5 finding H5: TWO LOGICAL MOUNTS MUST NOT RESOLVE TO ONE PHYSICAL
/// LOCATION.
///
/// The mount model is what makes "this output lies outside every selected
/// source" a meaningful statement: `project/x` and `authority/x` are distinct
/// logical paths precisely because they live in distinct physical trees. When
/// two mounts alias onto one directory, that separation silently collapses --
/// a plan can declare an output under one mount, satisfy every containment
/// guard against the other mount's selected sources, and still write onto a
/// selected source file.
///
/// `MountMappings` only ever deduplicated by mount ID, so nothing rejected
/// this. Comparing the supplied path STRINGS would not have been sufficient
/// either, which is what the three spellings below demonstrate: each names
/// exactly one directory while comparing unequal as text. Identity is taken
/// from the retained descriptor (`fstat` device + inode) after the no-follow
/// open, so it observes the location that will actually be traversed.
#[test]
fn two_logical_mounts_resolving_to_one_physical_root_are_rejected() {
    let shared = TempDir::new().expect("shared physical root");
    fs::create_dir_all(shared.path().join("openspec/specs/demo")).expect("specification directory");
    fs::write(
        shared.path().join("openspec/specs/demo/spec.md"),
        REQUIREMENT_SPEC,
    )
    .expect("standing specification");
    fs::write(
        shared.path().join("artifact.bin"),
        b"mounted-artifact-bytes",
    )
    .expect("bound artifact bytes");
    fs::create_dir_all(shared.path().join("buoy")).expect("binding store root");

    let config = RepositoryConfig::parse(&fs::read(CONFIG).expect("config bytes"))
        .expect("valid mounted configuration");

    // Three distinct SPELLINGS of one physical directory. A string
    // comparison passes all three; descriptor identity rejects all three.
    let canonical = shared.path().to_path_buf();
    let trailing_slash = PathBuf::from(format!("{}/", canonical.display()));
    let dot_hop = canonical.join("openspec").join("..");
    let symlinked_root = {
        let link_parent = TempDir::new().expect("symlink parent");
        let link = link_parent.path().join("aliased-root");
        std::os::unix::fs::symlink(&canonical, &link).expect("symlink to the shared root");
        // Leak the parent so the symlink outlives this block.
        std::mem::forget(link_parent);
        link
    };

    // A SYMLINKED mount root is refused even earlier, by the no-follow open
    // itself (`REH-MOUNT-ROOT`), so it never reaches the identity check.
    // Both outcomes are rejections; asserting the exact code each spelling
    // produces records which guard actually caught it instead of blurring
    // two distinct protections into one loose "it failed somehow".
    for (label, aliased, expected) in [
        ("identical path", canonical.clone(), "LSM-MOUNT-ROOT-ALIAS"),
        ("trailing slash", trailing_slash, "LSM-MOUNT-ROOT-ALIAS"),
        ("dot-segment hop", dot_hop, "LSM-MOUNT-ROOT-ALIAS"),
        ("symlinked root", symlinked_root, "REH-MOUNT-ROOT"),
    ] {
        let mappings = MountMappings::new(vec![
            (
                LogicalMountId::new("project").expect("project mount ID"),
                canonical.clone(),
            ),
            (
                LogicalMountId::new("authority").expect("authority mount ID"),
                aliased.clone(),
            ),
        ])
        .expect("mount mappings are well-formed: aliasing is not an ID duplicate");

        let result = snapshot_mounted_repository(&mappings, &config);
        let error = result.err().unwrap_or_else(|| {
            panic!("{label}: two mounts on one physical root produced a snapshot")
        });
        assert_eq!(
            error.code(),
            expected,
            "{label}: wrong diagnostic for physical-root aliasing"
        );
    }
}

/// Non-vacuity for the control above: the identical configuration with two
/// genuinely distinct physical roots must still snapshot successfully.
///
/// Without this, an over-broad alias check that rejected every mounted
/// snapshot would satisfy the test above while breaking all real mounted use.
#[test]
fn distinct_physical_roots_still_snapshot_under_the_alias_check() {
    let project = TempDir::new().expect("project mount root");
    let authority = TempDir::new().expect("authority mount root");
    fs::create_dir_all(authority.path().join("openspec/specs/demo"))
        .expect("specification directory");
    fs::write(
        authority.path().join("openspec/specs/demo/spec.md"),
        REQUIREMENT_SPEC,
    )
    .expect("standing specification");
    fs::write(
        project.path().join("artifact.bin"),
        b"mounted-artifact-bytes",
    )
    .expect("bound artifact bytes");
    fs::create_dir_all(project.path().join("buoy")).expect("binding store root");

    let config = RepositoryConfig::parse(&fs::read(CONFIG).expect("config bytes"))
        .expect("valid mounted configuration");
    let mappings = MountMappings::new(vec![
        (
            LogicalMountId::new("project").expect("project mount ID"),
            project.path().to_path_buf(),
        ),
        (
            LogicalMountId::new("authority").expect("authority mount ID"),
            authority.path().to_path_buf(),
        ),
    ])
    .expect("mount mappings");
    assert!(
        snapshot_mounted_repository(&mappings, &config).is_ok(),
        "distinct physical roots must remain snapshotable"
    );
}
