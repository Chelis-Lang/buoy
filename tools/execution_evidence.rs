//! Deterministic per-case execution-evidence records shared by local authorities.

#![allow(missing_docs)]

use super::content_digest;
use std::fs;
use std::path::Path;

pub const SCHEMA: &str = "buoy-local-execution-evidence/v1";
pub const HEADER: &str = "schema\tcategory\tid\tsource_paths\tsource_digest\ttool_paths\ttool_digest\tconfiguration_paths\tconfiguration_digest\tresult\tresult_digest\texecution\n";

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct Record {
    pub category: String,
    pub id: String,
    pub source_paths: String,
    pub source_digest: String,
    pub tool_paths: String,
    pub tool_digest: String,
    pub configuration_paths: String,
    pub configuration_digest: String,
    pub result: String,
    pub result_digest: String,
    pub execution: String,
}

pub fn bytes_binding(bytes: &[u8]) -> String {
    content_digest::bytes_binding(bytes)
}

pub fn path_set_binding(paths: &[&str]) -> Result<String, String> {
    let mut bytes = Vec::new();
    for path in paths {
        let content = fs::read(path).map_err(|error| format!("read {path}: {error}"))?;
        bytes.extend_from_slice(path.len().to_string().as_bytes());
        bytes.push(b':');
        bytes.extend_from_slice(path.as_bytes());
        bytes.extend_from_slice(content.len().to_string().as_bytes());
        bytes.push(b':');
        bytes.extend_from_slice(&content);
    }
    Ok(bytes_binding(&bytes))
}

#[allow(dead_code)]
pub fn passing(
    category: &str,
    id: impl Into<String>,
    source_paths: impl Into<String>,
    source_bytes: &[u8],
    tool_paths: &[&str],
    configuration_paths: &[&str],
    result: impl Into<String>,
) -> Result<Record, String> {
    passing_bound(
        category,
        id.into(),
        source_paths.into(),
        source_bytes,
        tool_paths,
        configuration_paths,
        result.into(),
    )
}

/// Monomorphization-stable core of [`passing`]: every caller of the generic
/// wrapper above shares this single non-generic body and its coverage regions.
fn passing_bound(
    category: &str,
    id: String,
    source_paths: String,
    source_bytes: &[u8],
    tool_paths: &[&str],
    configuration_paths: &[&str],
    result: String,
) -> Result<Record, String> {
    passing_with_bindings(
        category,
        id,
        source_paths,
        source_bytes,
        &tool_paths.join(";"),
        &path_set_binding(tool_paths)?,
        &configuration_paths.join(";"),
        &path_set_binding(configuration_paths)?,
        result,
    )
}

#[allow(clippy::too_many_arguments)]
pub fn passing_with_bindings(
    category: &str,
    id: impl Into<String>,
    source_paths: impl Into<String>,
    source_bytes: &[u8],
    tool_paths: &str,
    tool_digest: &str,
    configuration_paths: &str,
    configuration_digest: &str,
    result: impl Into<String>,
) -> Result<Record, String> {
    build_record(
        category,
        id.into(),
        source_paths.into(),
        source_bytes,
        tool_paths,
        tool_digest,
        configuration_paths,
        configuration_digest,
        result.into(),
    )
}

/// Monomorphization-stable core of [`passing_with_bindings`]: every caller of
/// the generic wrapper above shares this single non-generic body and its
/// coverage regions.
#[allow(clippy::too_many_arguments)]
fn build_record(
    category: &str,
    id: String,
    source_paths: String,
    source_bytes: &[u8],
    tool_paths: &str,
    tool_digest: &str,
    configuration_paths: &str,
    configuration_digest: &str,
    result: String,
) -> Result<Record, String> {
    validate_field(category, "category")?;
    validate_field(&id, "id")?;
    validate_paths(&source_paths, "source_paths")?;
    validate_paths(tool_paths, "tool_paths")?;
    validate_field(tool_digest, "tool_digest")?;
    validate_paths(configuration_paths, "configuration_paths")?;
    validate_field(configuration_digest, "configuration_digest")?;
    validate_field(&result, "result")?;
    Ok(Record {
        category: category.to_owned(),
        id,
        source_paths,
        source_digest: bytes_binding(source_bytes),
        tool_paths: tool_paths.to_owned(),
        tool_digest: tool_digest.to_owned(),
        configuration_paths: configuration_paths.to_owned(),
        configuration_digest: configuration_digest.to_owned(),
        result_digest: bytes_binding(result.as_bytes()),
        result,
        execution: "pass".to_owned(),
    })
}

fn validate_paths(paths: &str, field: &str) -> Result<(), String> {
    validate_field(paths, field)?;
    if paths.split(';').all(|path| Path::new(path).is_file()) {
        Ok(())
    } else {
        Err(format!(
            "EXECUTION-EVIDENCE-PATH: {field} contains a missing file"
        ))
    }
}

fn validate_field(value: &str, field: &str) -> Result<(), String> {
    if value.is_empty() || value.contains(['\t', '\r', '\n']) {
        Err(format!(
            "EXECUTION-EVIDENCE-FIELD: {field} must be nonempty single-line UTF-8"
        ))
    } else {
        Ok(())
    }
}

pub fn render(records: &[Record]) -> Result<String, String> {
    let mut sorted = records.to_vec();
    sorted.sort();
    for pair in sorted.windows(2) {
        if pair[0].category == pair[1].category && pair[0].id == pair[1].id {
            return Err(format!(
                "EXECUTION-EVIDENCE-DUPLICATE: {}:{}",
                pair[0].category, pair[0].id
            ));
        }
    }
    let mut output = String::from(HEADER);
    for record in sorted {
        let fields = [
            SCHEMA,
            &record.category,
            &record.id,
            &record.source_paths,
            &record.source_digest,
            &record.tool_paths,
            &record.tool_digest,
            &record.configuration_paths,
            &record.configuration_digest,
            &record.result,
            &record.result_digest,
            &record.execution,
        ];
        for (index, field) in fields.iter().enumerate() {
            validate_field(field, &format!("column {index}"))?;
        }
        output.push_str(&fields.join("\t"));
        output.push('\n');
    }
    Ok(output)
}

pub fn validate_materialized(path: &str, expected: &str, write: bool) -> Result<(), String> {
    if write {
        fs::write(path, expected).map_err(|error| format!("write {path}: {error}"))?;
    }
    let actual = fs::read_to_string(path).map_err(|error| format!("read {path}: {error}"))?;
    if actual == expected {
        Ok(())
    } else {
        Err(format!(
            "EXECUTION-EVIDENCE-STALE: {path}; rerun the exact authority with its reviewed write flag"
        ))
    }
}

/// Exercise every success and negative branch of this shared module.
///
/// Panics on the first control that unexpectedly disagreed with its expected outcome.
#[allow(
    dead_code,
    clippy::too_many_lines,
    clippy::too_many_arguments,
    clippy::expect_used
)]
pub fn self_test() {
    let sandbox = std::env::temp_dir().join(format!(
        "execution-evidence-self-test-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&sandbox);
    fs::create_dir_all(&sandbox).expect("create execution-evidence self-test sandbox");
    let tool = sandbox.join("tool.rs");
    let configuration = sandbox.join("configuration.toml");
    fs::write(&tool, b"tool").expect("write execution-evidence self-test tool file");
    fs::write(&configuration, b"configuration")
        .expect("write execution-evidence self-test configuration file");
    let tool = tool.to_string_lossy().into_owned();
    let configuration = configuration.to_string_lossy().into_owned();

    require(
        bytes_binding(b"") == content_digest::bytes_binding(b""),
        "bytes_binding",
    )
    .expect("execution-evidence self-test bytes_binding invariant");
    require(false, "self-test-invariant-control")
        .expect_err("self-test invariant control must fail");
    path_set_binding(&["/nonexistent/execution-evidence-self-test"])
        .expect_err("missing path_set_binding input must fail");
    validate_paths("", "self-test-paths").expect_err("empty validate_paths input must fail");
    let _ = path_set_binding(&[tool.as_str(), configuration.as_str()])
        .expect("execution-evidence self-test tool/configuration binding");

    let record = passing(
        "self-test-category",
        "self-test-id",
        &tool,
        b"source",
        &[tool.as_str()],
        &[configuration.as_str()],
        "result=pass",
    )
    .expect("execution-evidence self-test passing record");
    require(record.execution == "pass", "execution")
        .expect("execution-evidence self-test execution invariant");
    passing(
        "self-test-category",
        "self-test-id",
        &tool,
        b"source",
        &["/nonexistent/execution-evidence-self-test-tool"],
        &[configuration.as_str()],
        "result=pass",
    )
    .expect_err("missing tool_paths passing control must fail");
    passing(
        "self-test-category",
        "self-test-id",
        &tool,
        b"source",
        &[tool.as_str()],
        &["/nonexistent/execution-evidence-self-test-configuration"],
        "result=pass",
    )
    .expect_err("missing configuration_paths passing control must fail");

    for (
        category,
        id,
        source_paths,
        tool_paths,
        tool_digest,
        configuration_paths,
        configuration_digest,
        result,
    ) in [
        (
            "",
            "id",
            tool.as_str(),
            tool.as_str(),
            "digest",
            configuration.as_str(),
            "digest",
            "result",
        ),
        (
            "category",
            "",
            tool.as_str(),
            tool.as_str(),
            "digest",
            configuration.as_str(),
            "digest",
            "result",
        ),
        (
            "category",
            "id",
            "/nonexistent/self-test",
            tool.as_str(),
            "digest",
            configuration.as_str(),
            "digest",
            "result",
        ),
        (
            "category",
            "id",
            tool.as_str(),
            "/nonexistent/self-test",
            "digest",
            configuration.as_str(),
            "digest",
            "result",
        ),
        (
            "category",
            "id",
            tool.as_str(),
            tool.as_str(),
            "",
            configuration.as_str(),
            "digest",
            "result",
        ),
        (
            "category",
            "id",
            tool.as_str(),
            tool.as_str(),
            "digest",
            "/nonexistent/self-test",
            "digest",
            "result",
        ),
        (
            "category",
            "id",
            tool.as_str(),
            tool.as_str(),
            "digest",
            configuration.as_str(),
            "",
            "result",
        ),
        (
            "category",
            "id",
            tool.as_str(),
            tool.as_str(),
            "digest",
            configuration.as_str(),
            "digest",
            "",
        ),
    ] {
        passing_with_bindings(
            category,
            id,
            source_paths,
            b"source",
            tool_paths,
            tool_digest,
            configuration_paths,
            configuration_digest,
            result,
        )
        .expect_err("invalid execution-evidence field control must fail");
    }

    let first = record.clone();
    let mut second = record.clone();
    second.category = String::from("self-test-category-2");
    let rendered = render(&[first.clone(), second]).expect("execution-evidence self-test render");
    require(rendered.starts_with(HEADER), "header")
        .expect("execution-evidence self-test header invariant");
    render(&[first.clone(), first.clone()]).expect_err("duplicate render control must fail");
    let mut malformed = first.clone();
    malformed.result = String::from("bad\tresult");
    render(&[malformed]).expect_err("malformed render field control must fail");

    let materialized = sandbox.join("materialized.tsv");
    let materialized = materialized.to_string_lossy().into_owned();
    validate_materialized(&materialized, "current", true)
        .expect("execution-evidence self-test materialized write");
    validate_materialized(&materialized, "current", false)
        .expect("execution-evidence self-test materialized read");
    validate_materialized(&materialized, "stale", false)
        .expect_err("stale materialized control must fail");
    validate_materialized(
        "/nonexistent/execution-evidence-self-test.tsv",
        "current",
        false,
    )
    .expect_err("missing materialized control must fail");
    validate_materialized(
        "/nonexistent/execution-evidence-self-test/materialized.tsv",
        "current",
        true,
    )
    .expect_err("unwritable materialized control must fail");

    fs::remove_dir_all(&sandbox).expect("remove execution-evidence self-test sandbox");
}

fn require(condition: bool, code: &str) -> Result<(), String> {
    if condition {
        Ok(())
    } else {
        Err(format!("EXECUTION-EVIDENCE-SELF-TEST: {code}"))
    }
}
