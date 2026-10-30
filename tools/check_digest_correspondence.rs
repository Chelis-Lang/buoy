//! Deterministic cross-algorithm correspondence index generator and verifier.

#![allow(clippy::expect_used)]

use buoy_core::{
    ActiveAlgorithm, CanonicalValue, DigestAlgorithm, SemanticDigest, semantic_digest_with,
};
use std::collections::BTreeSet;
use std::env;
use std::fs;
use std::path::Path;

const INVENTORY: &str = "assurance/v1/digest-artifact-inventory.tsv";
const INDEX: &str = "assurance/v1/digest-correspondence-index.tsv";
const DERIVATION: &str = "assurance/v1/digest-correspondence-derivation.md";
const HEADER: &str =
    "stable_id\tobject_kind\tsource_path\tsource_owner\txxh3_128\tblake3_256\tsha_256\n";

fn active(algorithm: DigestAlgorithm) -> ActiveAlgorithm {
    ActiveAlgorithm::new(algorithm).expect("the correspondence checker requires every member")
}

fn binding(
    algorithm: DigestAlgorithm,
    stable_id: &str,
    path: &str,
    bytes: &[u8],
) -> SemanticDigest {
    semantic_digest_with(
        active(algorithm),
        "digest-correspondence-source",
        vec![
            (1, CanonicalValue::Text(stable_id.to_owned())),
            (2, CanonicalValue::Text(String::from("repository-artifact"))),
            (3, CanonicalValue::Text(path.to_owned())),
            (4, CanonicalValue::Bytes(bytes.to_vec())),
        ],
    )
    .expect("correspondence source fields are canonical")
}

fn artifact_binding(algorithm: DigestAlgorithm, media_type: &str, bytes: &[u8]) -> SemanticDigest {
    semantic_digest_with(
        active(algorithm),
        "raw-artifact",
        vec![
            (1, CanonicalValue::Text(media_type.to_owned())),
            (2, CanonicalValue::Bytes(bytes.to_vec())),
        ],
    )
    .expect("artifact binding fields are canonical")
}

fn read_string(path: &Path, code: &str) -> Result<String, String> {
    fs::read_to_string(path).map_err(|error| format!("{code}: {}: {error}", path.display()))
}

fn read_bytes(path: &Path, code: &str) -> Result<Vec<u8>, String> {
    fs::read(path).map_err(|error| format!("{code}: {}: {error}", path.display()))
}

fn write_bytes(path: &Path, bytes: impl AsRef<[u8]>, code: &str) -> Result<(), String> {
    fs::write(path, bytes).map_err(|error| format!("{code}: {}: {error}", path.display()))
}

fn create_parent(path: &Path) -> Result<(), String> {
    path.parent().map_or(Ok(()), |parent| {
        fs::create_dir_all(parent).map_err(|error| format!("DIG-INDEX-CONTROL: {error}"))
    })
}

fn contains_canonical_digest(source: &str) -> bool {
    [
        ("xxh3-128:", 32_usize),
        ("blake3-256:", 64),
        ("sha-256:", 64),
    ]
    .into_iter()
    .any(|(prefix, length)| {
        let mut remaining = source;
        while let Some(offset) = remaining.find(prefix) {
            let payload = &remaining[offset + prefix.len()..];
            if payload.len() >= length {
                let candidate = &payload.as_bytes()[..length];
                let canonical = candidate
                    .iter()
                    .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase());
                let bounded = payload
                    .as_bytes()
                    .get(length)
                    .is_none_or(|byte| !byte.is_ascii_hexdigit());
                if canonical && bounded {
                    return true;
                }
            }
            remaining = &payload[payload.len().min(1)..];
        }
        false
    })
}

fn collect_digest_paths(
    root: &Path,
    current: &Path,
    output: &mut BTreeSet<String>,
) -> Result<(), String> {
    collect_digest_paths_internal(root, current, output, false, false, false)
}

fn collect_digest_paths_internal(
    root: &Path,
    current: &Path,
    output: &mut BTreeSet<String>,
    inject_entry_error: bool,
    inject_nested_error: bool,
    inject_file_error: bool,
) -> Result<(), String> {
    let entries = fs::read_dir(current)
        .map_err(|error| format!("DIG-INVENTORY-SCAN: {}: {error}", current.display()))?;
    let collected = if inject_entry_error {
        Err(std::io::Error::other("injected directory entry failure"))
    } else {
        entries.collect::<Result<Vec<_>, _>>()
    };
    let mut entries =
        collected.map_err(|error| format!("DIG-INVENTORY-SCAN: {}: {error}", current.display()))?;
    entries.sort_unstable_by_key(std::fs::DirEntry::file_name);
    for entry in entries {
        let path = entry.path();
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if path.is_dir() {
            if !matches!(
                name.as_ref(),
                ".git" | ".devenv" | ".worktrees" | "target" | "result" | "node_modules"
            ) {
                collect_digest_paths_internal(
                    root,
                    &path,
                    output,
                    inject_nested_error,
                    false,
                    inject_file_error,
                )?;
            }
        } else if path.is_file() {
            let read_path = if inject_file_error {
                root.join("missing-digest-source")
            } else {
                path.clone()
            };
            let bytes = read_bytes(&read_path, "DIG-INVENTORY-SCAN")?;
            if std::str::from_utf8(&bytes).is_ok_and(contains_canonical_digest) {
                output.insert(
                    path.strip_prefix(root)
                        .expect("collected descendants retain their repository root")
                        .to_string_lossy()
                        .replace('\\', "/"),
                );
            }
        }
    }
    Ok(())
}

fn validate_inventory_closure(root: &Path, inventory_path: &Path) -> Result<usize, String> {
    let source = read_string(inventory_path, "DIG-INVENTORY-READ")?;
    let mut inventoried = BTreeSet::new();
    for (index, line) in source.lines().enumerate().skip(1) {
        let fields = line.split('\t').collect::<Vec<_>>();
        if fields.len() != 4 || fields.iter().any(|field| field.is_empty()) {
            return Err(format!("DIG-INVENTORY-ROW: row {}", index + 1));
        }
        if !inventoried.insert(fields[0].to_owned()) {
            return Err(format!("DIG-INVENTORY-DUPLICATE: {}", fields[0]));
        }
        match fields[3] {
            "correspondence-output-excluded"
                if !matches!(
                    fields[0],
                    "assurance/v1/digest-correspondence-index.tsv"
                        | "assurance/v1/digest-correspondence-derivation.md"
                ) =>
            {
                return Err(format!("DIG-INVENTORY-EXCLUSION: {}", fields[0]));
            }
            "derived-cycle-excluded"
                if fields[0] != "assurance/product-identity/v1/naming-inventory.tsv" =>
            {
                return Err(format!("DIG-INVENTORY-EXCLUSION: {}", fields[0]));
            }
            _ => {}
        }
    }
    let mut discovered = BTreeSet::new();
    collect_digest_paths(root, root, &mut discovered)?;
    if let Some(path) = discovered.difference(&inventoried).next() {
        return Err(format!("DIG-INVENTORY-UNOWNED: {path}"));
    }
    if let Some(path) = inventoried.difference(&discovered).next() {
        return Err(format!("DIG-INVENTORY-STALE: {path}"));
    }
    Ok(inventoried.len())
}

fn copy_file(source: &Path, target: &Path) -> Result<(), String> {
    fs::copy(source, target)
        .map(|_| ())
        .map_err(|error| format!("DIG-INDEX-CONTROL: {error}"))
}

fn render(root: &Path, inventory_path: &Path) -> Result<String, String> {
    let source = read_string(inventory_path, "XXH-MIGRATION-MISSING-AUTHORITATIVE-BYTES")?;
    let mut rows = Vec::new();
    for (index, line) in source.lines().enumerate().skip(1) {
        let fields = line.split('\t').collect::<Vec<_>>();
        if fields.len() != 4 || fields.iter().any(|field| field.is_empty()) {
            return Err(format!(
                "DIG-INDEX-CORRESPONDENCE: inventory row {} is malformed",
                index + 1
            ));
        }
        if matches!(
            fields[3],
            "correspondence-output-excluded" | "derived-cycle-excluded"
        ) {
            continue;
        }
        let path = fields[0];
        let bytes = read_bytes(
            &root.join(path),
            "XXH-MIGRATION-MISSING-AUTHORITATIVE-BYTES",
        )?;
        let stable_id = format!("artifact:{path}");
        let xxh3 = binding(DigestAlgorithm::Xxh3_128, &stable_id, path, &bytes);
        let blake3_digest = binding(DigestAlgorithm::Blake3_256, &stable_id, path, &bytes);
        let sha = binding(DigestAlgorithm::Sha256, &stable_id, path, &bytes);
        rows.push(format!(
            "{stable_id}\trepository-artifact\t{path}\t{}\t{}\t{}\t{}",
            fields[1], xxh3, blake3_digest, sha
        ));
    }
    rows.sort_unstable();
    if rows.windows(2).any(|pair| pair[0] == pair[1]) {
        return Err(String::from("DIG-INDEX-ORDER: duplicate stable object ID"));
    }
    Ok(format!("{HEADER}{}\n", rows.join("\n")))
}

fn verify_source(root: &Path, source: &str) -> Result<usize, String> {
    if !source.ends_with('\n') {
        return Err(String::from("DIG-INDEX-ORDER: index must end with newline"));
    }
    let mut lines = source.lines();
    if lines.next() != Some(HEADER.trim_end()) {
        return Err(String::from("DIG-INDEX-CORRESPONDENCE: header differs"));
    }
    let mut prior = None;
    let mut count = 0;
    for (index, line) in lines.enumerate() {
        let fields = line.split('\t').collect::<Vec<_>>();
        if fields.len() != 7 {
            return Err(format!(
                "DIG-INDEX-CORRESPONDENCE: row {} width differs",
                index + 2
            ));
        }
        let stable_id = fields[0];
        let path = fields[2];
        if stable_id != format!("artifact:{path}") {
            return Err(format!("DIG-INDEX-DIGEST-KEY: row {}", index + 2));
        }
        if prior.is_some_and(|value: &str| value >= stable_id) {
            return Err(format!("DIG-INDEX-ORDER: row {}", index + 2));
        }
        let bytes = read_bytes(
            &root.join(path),
            "XXH-MIGRATION-MISSING-AUTHORITATIVE-BYTES",
        )?;
        for (field, algorithm) in [
            (4_usize, DigestAlgorithm::Xxh3_128),
            (5, DigestAlgorithm::Blake3_256),
            (6, DigestAlgorithm::Sha256),
        ] {
            let recorded = SemanticDigest::parse_canonical(fields[field])
                .map_err(|_| format!("DIG-INDEX-CORRESPONDENCE: row {}", index + 2))?;
            if recorded.algorithm() != algorithm
                || recorded != binding(algorithm, stable_id, path, &bytes)
            {
                return Err(format!("DIG-INDEX-CORRESPONDENCE: row {}", index + 2));
            }
        }
        prior = Some(stable_id);
        count += 1;
    }
    Ok(count)
}

fn media_type(path: &str) -> &'static str {
    match Path::new(path).extension().and_then(|value| value.to_str()) {
        Some("md") => "text/markdown",
        Some("rs") => "text/x-rust",
        Some("toml") => "application/toml",
        Some("tsv") => "text/tab-separated-values",
        Some("json") => "application/json",
        Some("txt") => "text/plain",
        _ => "application/octet-stream",
    }
}

fn render_derivation(root: &Path, index_source: &str) -> Result<String, String> {
    let inventory = read_bytes(
        &root.join(INVENTORY),
        "XXH-MIGRATION-MISSING-AUTHORITATIVE-BYTES",
    )?;
    let tool_path = "tools/check_digest_correspondence.rs";
    let tool = read_bytes(
        &root.join(tool_path),
        "XXH-MIGRATION-MISSING-AUTHORITATIVE-BYTES",
    )?;
    let algorithm = DigestAlgorithm::Xxh3_128;
    let inventory_digest = artifact_binding(algorithm, "text/tab-separated-values", &inventory);
    let tool_digest = artifact_binding(algorithm, "text/x-rust", &tool);
    let index_digest = artifact_binding(
        algorithm,
        "text/tab-separated-values",
        index_source.as_bytes(),
    );
    let mut sources = vec![format!(
        "{{ kind = \"repository-artifact\", path = \"{INVENTORY}\", raw_domain = \"raw-artifact\", media_type = \"text/tab-separated-values\", representation = \"saved-bytes/v1\", digest = \"{inventory_digest}\" }}"
    )];
    for line in index_source.lines().skip(1) {
        let fields = line.split('\t').collect::<Vec<_>>();
        if fields.len() != 7 {
            return Err(String::from(
                "DIG-INDEX-CORRESPONDENCE: source row width differs",
            ));
        }
        let path = fields[2];
        let bytes = read_bytes(
            &root.join(path),
            "XXH-MIGRATION-MISSING-AUTHORITATIVE-BYTES",
        )?;
        let digest = artifact_binding(algorithm, media_type(path), &bytes);
        sources.push(format!(
            "{{ kind = \"repository-artifact\", path = \"{path}\", raw_domain = \"raw-artifact\", media_type = \"{}\", representation = \"saved-bytes/v1\", digest = \"{digest}\" }}",
            media_type(path)
        ));
    }
    sources.sort_unstable();
    let sources = sources.join(", ");
    let oracle_digest = semantic_digest_with(
        active(algorithm),
        "digest-correspondence-oracle",
        vec![
            (1, CanonicalValue::Digest(inventory_digest)),
            (2, CanonicalValue::Digest(tool_digest)),
            (3, CanonicalValue::Digest(index_digest)),
        ],
    )
    .expect("derivation oracle fields are canonical");
    Ok(format!(
        "# Digest correspondence derivation\n\nThis derivation binds the source inventory and the checker to the generated correspondence index.\n\n```spec-provenance-derivation-v1\n\
         id = \"DERIV-DIGEST-CORRESPONDENCE-V3\"\n\
         sources = [{sources}]\n\
         targets = [{{ kind = \"repository-artifact\", path = \"{INDEX}\", raw_domain = \"raw-artifact\", media_type = \"text/tab-separated-values\", representation = \"generated-bytes/v1\", digest = \"{index_digest}\" }}]\n\
         tool = {{ id = \"check-digest-correspondence\", version = \"3.0.0\", executable = \"{tool_digest}\", configuration = \"{inventory_digest}\" }}\n\
         configuration = \"{inventory_digest}\"\n\
         oracle = {{ kind = \"oracle\", id = \"ORACLE-DIGEST-CORRESPONDENCE-V3\", digest = \"{oracle_digest}\" }}\n\
         relation = \"generated-from\"\n\
         relation_scope = {{ schema = \"digest-correspondence-scope/v1\", bytes_hex = \"7633\" }}\n\
         exclusions = [\"The correspondence index is not a semantic identity source.\"]\n\
         ```\n"
    ))
}

fn verify(root: &Path, inventory: &Path, index: &Path) -> Result<usize, String> {
    let expected = render(root, inventory)?;
    let actual = read_string(index, "DIG-INDEX-CORRESPONDENCE")?;
    let count = verify_source(root, &actual)?;
    if actual != expected {
        return Err(String::from(
            "DIG-INDEX-CORRESPONDENCE: committed index is stale",
        ));
    }
    let expected_derivation = render_derivation(root, &actual)?;
    let actual_derivation = read_string(&root.join(DERIVATION), "DIG-INDEX-CORRESPONDENCE")?;
    if actual_derivation != expected_derivation {
        return Err(String::from(
            "DIG-INDEX-CORRESPONDENCE: committed derivation is stale",
        ));
    }
    Ok(count)
}

fn copy_sources(target: &Path, inventory_path: &Path) -> Result<(), String> {
    let inventory = read_string(inventory_path, "DIG-INDEX-CONTROL")?;
    let target_inventory = target.join(INVENTORY);
    create_parent(&target_inventory)?;
    write_bytes(&target_inventory, &inventory, "DIG-INDEX-CONTROL")?;
    for line in inventory.lines().skip(1) {
        let fields = line.split('\t').collect::<Vec<_>>();
        if fields.len() != 4 {
            return Err(String::from("DIG-INDEX-CONTROL: malformed inventory"));
        }
        if matches!(
            fields[3],
            "correspondence-output-excluded" | "derived-cycle-excluded"
        ) {
            continue;
        }
        let path = fields[0];
        let target_path = target.join(path);
        create_parent(&target_path)?;
        copy_file(Path::new(path), &target_path)?;
    }
    Ok(())
}

fn copy_production_sources(target: &Path) -> Result<(), String> {
    let tool = Path::new("tools/check_digest_correspondence.rs");
    let target_tool = target.join(tool);
    copy_sources(target, Path::new(INVENTORY))
        .and_then(|()| create_parent(&target_tool))
        .and_then(|()| copy_file(tool, &target_tool))
}

#[allow(clippy::too_many_lines, clippy::unnecessary_wraps)]
fn self_test() -> Result<(), String> {
    let root = env::temp_dir().join(format!("buoy-digest-correspondence-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(root.join("evidence")).expect("create correspondence control directory");
    fs::write(root.join("evidence/source.txt"), b"source")
        .expect("write correspondence control source");
    let inventory = root.join("inventory.tsv");
    let inventory_source = "path\towner\treproduced\tregeneration_disposition\nevidence/source.txt\tcontrol\tyes\tregenerate\n";
    fs::write(&inventory, inventory_source).expect("write correspondence control inventory");
    let valid = render(&root, &inventory).expect("render correspondence control");
    assert_eq!(
        verify_source(&root, &valid).expect("verify correspondence control"),
        1
    );

    let closure_root = root.join("closure");
    fs::create_dir_all(closure_root.join("evidence")).expect("create closure evidence root");
    fs::write(
        closure_root.join("evidence/digest.txt"),
        "xxh3-128:00000000000000000000000000000000\n",
    )
    .expect("write closure digest");
    let closure_inventory = closure_root.join("inventory.tsv");
    fs::write(
        &closure_inventory,
        "path\towner\treproduced\tregeneration_disposition\nevidence/digest.txt\tcontrol\tyes\tregenerate\n",
    )
    .expect("write closure inventory");
    fs::write(
        closure_root.join("evidence/plain.txt"),
        b"no digest content",
    )
    .expect("write plain closure file");
    fs::write(closure_root.join("evidence/binary.bin"), [0xff, 0xfe])
        .expect("write binary closure file");
    #[cfg(unix)]
    std::os::unix::fs::symlink("missing", closure_root.join("evidence/dangling"))
        .expect("write dangling closure link");
    assert_eq!(
        validate_inventory_closure(&closure_root, &closure_inventory)
            .expect("complete closure inventory"),
        1
    );
    fs::write(
        closure_root.join("evidence/unowned.txt"),
        "sha-256:0000000000000000000000000000000000000000000000000000000000000000\n",
    )
    .expect("write unowned digest");
    let unowned = validate_inventory_closure(&closure_root, &closure_inventory)
        .expect_err("unowned digest must fail");
    assert!(unowned.contains("DIG-INVENTORY-UNOWNED"));
    fs::remove_file(closure_root.join("evidence/unowned.txt")).expect("remove unowned digest");
    fs::remove_file(closure_root.join("evidence/digest.txt")).expect("remove owned digest");
    let stale = validate_inventory_closure(&closure_root, &closure_inventory)
        .expect_err("stale inventory row must fail");
    assert!(stale.contains("DIG-INVENTORY-STALE"));
    let malformed_closure = closure_root.join("malformed-closure.tsv");
    for (rows, expected) in [
        ("malformed\n", "DIG-INVENTORY-ROW"),
        ("a.txt\t\tyes\tregenerate\n", "DIG-INVENTORY-ROW"),
        (
            "a.txt\tcontrol\tyes\tregenerate\na.txt\tcontrol\tyes\tregenerate\n",
            "DIG-INVENTORY-DUPLICATE",
        ),
        (
            "a.txt\tcontrol\tyes\tcorrespondence-output-excluded\n",
            "DIG-INVENTORY-EXCLUSION",
        ),
        (
            "a.txt\tcontrol\tyes\tderived-cycle-excluded\n",
            "DIG-INVENTORY-EXCLUSION",
        ),
    ] {
        fs::write(
            &malformed_closure,
            format!("path\towner\treproduced\tregeneration_disposition\n{rows}"),
        )
        .expect("write malformed closure inventory");
        let error = validate_inventory_closure(&closure_root, &malformed_closure)
            .expect_err("malformed closure inventory must fail");
        assert!(error.contains(expected), "expected {expected}: {error}");
    }
    fs::remove_file(&malformed_closure).expect("remove malformed closure inventory");

    fs::create_dir_all(root.join("assurance/v1")).expect("create derivation control root");
    fs::write(root.join(INVENTORY), b"inventory control")
        .expect("write derivation control inventory");
    fs::create_dir_all(root.join("tools")).expect("create derivation control tools root");
    fs::write(
        root.join("tools/check_digest_correspondence.rs"),
        b"tool control",
    )
    .expect("write derivation control tool");
    let short_row = render_derivation(&root, "header\nshort\trow\n")
        .expect_err("short derivation source row must fail");
    assert!(short_row.contains("DIG-INDEX-CORRESPONDENCE"));
    let _ = render_derivation(&root, "header\na\tb\tmissing.txt\td\te\tf\tg\n")
        .expect_err("missing derivation source bytes must fail");
    let verification_index = root.join("verification-index.tsv");
    fs::write(&verification_index, &valid).expect("write derivation verification index");
    fs::remove_file(root.join(INVENTORY)).expect("remove derivation inventory control");
    let _ = verify(&root, &inventory, &verification_index)
        .expect_err("verification must propagate a derivation render failure");

    let missing_path = root.join("missing");
    let mut collected = BTreeSet::new();
    let _ = collect_digest_paths(&root, &missing_path, &mut collected)
        .expect_err("missing digest root must fail");
    for fault in [
        (true, false, false),
        (false, true, false),
        (false, false, true),
    ] {
        let _ =
            collect_digest_paths_internal(&root, &root, &mut collected, fault.0, fault.1, fault.2)
                .expect_err("digest collection fault must fail");
    }
    let _ = validate_inventory_closure(&root, &missing_path)
        .expect_err("missing closure inventory must fail");
    let _ = validate_inventory_closure(&missing_path, &inventory)
        .expect_err("missing closure root must fail");
    let _ = read_string(&missing_path, "CONTROL").expect_err("missing text must fail");
    let _ = read_bytes(&missing_path, "CONTROL").expect_err("missing bytes must fail");
    let _ = write_bytes(&root, b"x", "CONTROL").expect_err("directory write must fail");
    create_parent(Path::new("bare-control")).expect("a bare path has no parent work");
    let blocker = root.join("blocker");
    fs::write(&blocker, b"blocker").expect("write parent blocker");
    let _ = create_parent(&blocker.join("child")).expect_err("blocked parent must fail");
    let _ =
        copy_file(&missing_path, &root.join("copy")).expect_err("missing copy source must fail");
    let _ = render(&root, &missing_path).expect_err("missing inventory must fail");

    let malformed_inventory = root.join("malformed.tsv");
    fs::write(
        &malformed_inventory,
        "path\towner\treproduced\tregeneration_disposition\nmalformed\n",
    )
    .expect("write malformed inventory");
    let _ = render(&root, &malformed_inventory).expect_err("malformed inventory must fail");
    fs::write(
        &malformed_inventory,
        "path\towner\treproduced\tregeneration_disposition\nevidence/source.txt\t\tyes\tregenerate\n",
    )
    .expect("write empty-field inventory");
    let _ = render(&root, &malformed_inventory).expect_err("empty inventory field must fail");
    let duplicate_inventory = root.join("duplicate.tsv");
    fs::write(
        &duplicate_inventory,
        format!("{inventory_source}evidence/source.txt\tcontrol\tyes\tregenerate\n"),
    )
    .expect("write duplicate inventory");
    let _ = render(&root, &duplicate_inventory).expect_err("duplicate stable ID must fail");
    let absent_inventory = root.join("absent.tsv");
    fs::write(
        &absent_inventory,
        "path\towner\treproduced\tregeneration_disposition\nevidence/absent.txt\tcontrol\tyes\tregenerate\n",
    )
    .expect("write absent-source inventory");
    let _ = render(&root, &absent_inventory).expect_err("missing source bytes must fail");
    let excluded_inventory = root.join("excluded.tsv");
    fs::write(
        &excluded_inventory,
        "path\towner\treproduced\tregeneration_disposition\nevidence/absent.txt\tcontrol\tyes\tderived-cycle-excluded\n",
    )
    .expect("write excluded inventory");
    assert_eq!(
        render(&root, &excluded_inventory).expect("excluded output renders"),
        format!("{HEADER}\n")
    );

    let mut fields = valid
        .lines()
        .nth(1)
        .unwrap_or_default()
        .split('\t')
        .collect::<Vec<_>>();
    let row = fields.join("\t");
    let no_newline = valid.trim_end_matches('\n');
    let _ = verify_source(&root, no_newline).expect_err("missing newline must fail");
    let _ = verify_source(&root, &valid.replacen(HEADER.trim_end(), "wrong", 1))
        .expect_err("wrong header must fail");
    fields.pop();
    let _ = verify_source(&root, &format!("{HEADER}{}\n", fields.join("\t")))
        .expect_err("wrong row width must fail");
    let digest_keyed = valid.replacen("artifact:evidence/source.txt", "xxh3-128:00", 1);
    let key_error = verify_source(&root, &digest_keyed).expect_err("digest key must fail");
    assert!(key_error.contains("DIG-INDEX-DIGEST-KEY"));
    let duplicate_rows = format!("{HEADER}{row}\n{row}\n");
    let _ = verify_source(&root, &duplicate_rows).expect_err("duplicate row must fail");
    let contradicted = valid.replacen("xxh3-128:", "xxh3-128:00", 1);
    let contradiction = verify_source(&root, &contradicted).expect_err("contradiction must fail");
    assert!(contradiction.contains("DIG-INDEX-CORRESPONDENCE"));
    let row_fields = row.split('\t').map(str::to_owned).collect::<Vec<_>>();
    let mut wrong_algorithm = row_fields.clone();
    wrong_algorithm[4].clone_from(&row_fields[5]);
    let _ = verify_source(&root, &format!("{HEADER}{}\n", wrong_algorithm.join("\t")))
        .expect_err("wrong algorithm must fail");
    let mut wrong_digest = row_fields;
    wrong_digest[4] = String::from("xxh3-128:00000000000000000000000000000000");
    let _ = verify_source(&root, &format!("{HEADER}{}\n", wrong_digest.join("\t")))
        .expect_err("wrong digest must fail");

    let identity_before = render(&root, &inventory).expect("render identity before index change");
    fs::write(root.join("derived-index.tsv"), b"contradicted")
        .expect("write corrupted derived index");
    let identity_after_corruption = render(&root, &inventory).expect("render after corruption");
    fs::remove_file(root.join("derived-index.tsv")).expect("delete derived index");
    let identity_after_deletion = render(&root, &inventory).expect("render after deletion");
    assert_eq!(identity_before, identity_after_corruption);
    assert_eq!(identity_before, identity_after_deletion);
    fs::remove_file(root.join("evidence/source.txt")).expect("delete control source");
    let missing = verify_source(&root, &valid).expect_err("missing source must fail");
    assert!(missing.contains("XXH-MIGRATION-MISSING-AUTHORITATIVE-BYTES"));

    let derivation_root = root.join("derivation-missing-tool");
    let _ = render_derivation(&derivation_root, &valid)
        .expect_err("missing derivation inventory must fail");
    fs::create_dir_all(
        derivation_root.join(Path::new(INVENTORY).parent().unwrap_or(Path::new("."))),
    )
    .expect("create derivation inventory parent");
    fs::write(derivation_root.join(INVENTORY), inventory_source)
        .expect("write derivation inventory control");
    let _ = render_derivation(&derivation_root, &valid).expect_err("missing tool must fail");
    fs::remove_dir_all(&root).expect("remove correspondence control root");

    let mirror = env::temp_dir().join(format!(
        "buoy-digest-correspondence-mirror-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&mirror);
    copy_production_sources(&mirror).expect("copy production correspondence sources");
    let current = render(Path::new("."), Path::new(INVENTORY)).expect("render current index");
    let mirrored = render(&mirror, &mirror.join(INVENTORY)).expect("render mirrored index");
    assert_eq!(current, mirrored);
    let derivation = render_derivation(&mirror, &mirrored).expect("render mirrored derivation");
    create_parent(&mirror.join(INDEX)).expect("create mirrored index parent");
    write_bytes(&mirror.join(INDEX), &mirrored, "CONTROL").expect("write mirrored index");
    write_bytes(&mirror.join(DERIVATION), &derivation, "CONTROL")
        .expect("write mirrored derivation");
    verify(&mirror, &mirror.join(INVENTORY), &mirror.join(INDEX))
        .expect("verify mirrored correspondence");
    let _ = verify(
        &mirror,
        &mirror.join("missing-inventory"),
        &mirror.join(INDEX),
    )
    .expect_err("verify must reject a missing inventory");
    let _ = verify(
        &mirror,
        &mirror.join(INVENTORY),
        &mirror.join("missing-index"),
    )
    .expect_err("verify must reject a missing index");
    write_bytes(&mirror.join(INDEX), "invalid\n", "CONTROL").expect("write invalid mirrored index");
    let _ = verify(&mirror, &mirror.join(INVENTORY), &mirror.join(INDEX))
        .expect_err("verify must reject invalid index source");
    write_bytes(&mirror.join(INDEX), &mirrored, "CONTROL").expect("restore mirrored index");
    fs::remove_file(mirror.join(DERIVATION)).expect("remove mirrored derivation");
    let _ = verify(&mirror, &mirror.join(INVENTORY), &mirror.join(INDEX))
        .expect_err("verify must reject a missing derivation");
    write_bytes(&mirror.join(DERIVATION), &derivation, "CONTROL")
        .expect("restore mirrored derivation");
    fs::remove_file(mirror.join("tools/check_digest_correspondence.rs"))
        .expect("remove mirrored checker");
    let _ = verify(&mirror, &mirror.join(INVENTORY), &mirror.join(INDEX))
        .expect_err("verify must reject a missing checker source");
    write_bytes(
        &mirror.join("tools/check_digest_correspondence.rs"),
        fs::read("tools/check_digest_correspondence.rs").expect("read checker source"),
        "CONTROL",
    )
    .expect("restore mirrored checker");
    let mut stale_fields = mirrored
        .lines()
        .nth(1)
        .unwrap_or_default()
        .split('\t')
        .collect::<Vec<_>>();
    stale_fields[3] = "stale-owner";
    write_bytes(
        &mirror.join(INDEX),
        format!("{HEADER}{}\n", stale_fields.join("\t")),
        "CONTROL",
    )
    .expect("write stale index");
    let _ = verify(&mirror, &mirror.join(INVENTORY), &mirror.join(INDEX))
        .expect_err("stale index must fail");
    write_bytes(&mirror.join(INDEX), &mirrored, "CONTROL").expect("restore mirrored index");
    write_bytes(
        &mirror.join(DERIVATION),
        format!("{derivation}\n"),
        "CONTROL",
    )
    .expect("write stale derivation");
    let _ = verify(&mirror, &mirror.join(INVENTORY), &mirror.join(INDEX))
        .expect_err("stale derivation must fail");

    let malformed_copy_inventory = mirror.join("malformed-copy.tsv");
    fs::write(
        &malformed_copy_inventory,
        "path\towner\treproduced\tregeneration_disposition\nmalformed\n",
    )
    .expect("write malformed copy inventory");
    let _ = copy_sources(&mirror.join("copy-control"), &malformed_copy_inventory)
        .expect_err("malformed copy inventory must fail");
    let _ = copy_sources(
        &mirror.join("copy-missing"),
        &mirror.join("missing-copy.tsv"),
    )
    .expect_err("missing copy inventory must fail");
    let blocked_target = mirror.join("blocked-target");
    fs::write(&blocked_target, b"blocker").expect("write copy target blocker");
    let _ = copy_sources(&blocked_target, Path::new(INVENTORY))
        .expect_err("blocked inventory parent must fail");
    let directory_target = mirror.join("directory-target");
    fs::create_dir_all(directory_target.join(INVENTORY))
        .expect("create inventory directory blocker");
    let _ = copy_sources(&directory_target, Path::new(INVENTORY))
        .expect_err("inventory directory write must fail");
    let artifact_parent_inventory = mirror.join("artifact-parent.tsv");
    fs::write(
        &artifact_parent_inventory,
        "path\towner\treproduced\tregeneration_disposition\nblocked/child\tcontrol\tyes\tregenerate\n",
    )
    .expect("write artifact parent inventory");
    let artifact_parent_target = mirror.join("artifact-parent-target");
    fs::create_dir_all(&artifact_parent_target).expect("create artifact parent target");
    fs::write(artifact_parent_target.join("blocked"), b"blocker")
        .expect("write artifact parent blocker");
    let _ = copy_sources(&artifact_parent_target, &artifact_parent_inventory)
        .expect_err("blocked artifact parent must fail");
    let missing_artifact_inventory = mirror.join("missing-artifact.tsv");
    fs::write(
        &missing_artifact_inventory,
        "path\towner\treproduced\tregeneration_disposition\nmissing-copy-source\tcontrol\tyes\tregenerate\n",
    )
    .expect("write missing artifact inventory");
    let _ = copy_sources(
        &mirror.join("missing-artifact-target"),
        &missing_artifact_inventory,
    )
    .expect_err("missing artifact copy must fail");
    fs::remove_dir_all(&mirror).expect("remove correspondence mirror");
    println!("digest-correspondence: self-test ok controls=24");
    Ok(())
}

fn run(arguments: &[String]) -> Result<(), String> {
    match arguments {
        [] => validate_inventory_closure(Path::new("."), Path::new(INVENTORY)).and_then(|_| {
            verify(Path::new("."), Path::new(INVENTORY), Path::new(INDEX)).map(|count| {
                println!("digest-correspondence: ok rows={count} index={INDEX}");
            })
        }),
        [argument] if argument == "--write" => {
            validate_inventory_closure(Path::new("."), Path::new(INVENTORY))
                .and_then(|_| render(Path::new("."), Path::new(INVENTORY)))
                .and_then(|source| {
                    render_derivation(Path::new("."), &source)
                        .map(|derivation| (source, derivation))
                })
                .and_then(|(source, derivation)| {
                    write_bytes(Path::new(INDEX), source, "DIG-INDEX-WRITE").map(|()| derivation)
                })
                .and_then(|derivation| {
                    write_bytes(Path::new(DERIVATION), derivation, "DIG-INDEX-WRITE")
                })
                .and_then(|()| run(&[]))
        }
        [argument] if argument == "--self-test" => self_test(),
        _ => Err(String::from(
            "usage: check-digest-correspondence [--write|--self-test]",
        )),
    }
}

fn main() {
    if let Err(error) = run(&env::args().skip(1).collect::<Vec<_>>()) {
        eprintln!("digest-correspondence: {error}");
        std::process::exit(1);
    }
}
