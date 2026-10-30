//! Syntax-aware repository hashing-policy analysis with deterministic findings.

use proc_macro2::{TokenStream, TokenTree};
use quote::ToTokens as _;
use serde_json::Value as JsonValue;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

/// One normalized, totally ordered hashing-policy finding.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct HashingPolicyFinding {
    /// Stable machine code.
    pub code: &'static str,
    /// Normalized repository-relative path.
    pub path: String,
    /// One-based source line, or zero for structured metadata without a source line.
    pub line: usize,
    /// One-based source column, or zero for structured metadata without a source column.
    pub column: usize,
    /// Semantic scanner role.
    pub role: String,
    /// Stable offending syntax/category.
    pub detail: String,
}

fn location(source: &str, marker: &str) -> (usize, usize) {
    source.find(marker).map_or((0, 0), |offset| {
        let prefix = &source[..offset];
        let line = prefix.bytes().filter(|byte| *byte == b'\n').count() + 1;
        let column = prefix
            .rsplit_once('\n')
            .map_or(offset + 1, |(_, tail)| tail.len() + 1);
        (line, column)
    })
}

fn finding(
    code: &'static str,
    path: &str,
    source: &str,
    marker: &str,
    role: &str,
) -> HashingPolicyFinding {
    let (line, column) = location(source, marker);
    HashingPolicyFinding {
        code,
        path: path.replace('\\', "/"),
        line,
        column,
        role: role.to_owned(),
        detail: marker.to_owned(),
    }
}

fn collect_identifiers(tokens: TokenStream, output: &mut BTreeSet<String>) {
    for token in tokens {
        match token {
            TokenTree::Ident(identifier) => {
                output.insert(identifier.to_string());
            }
            TokenTree::Group(group) => collect_identifiers(group.stream(), output),
            TokenTree::Punct(_) | TokenTree::Literal(_) => {}
        }
    }
}

fn collect_path_roots(tokens: TokenStream, output: &mut BTreeSet<String>) {
    let tokens = tokens.into_iter().collect::<Vec<_>>();
    for window in tokens.windows(3) {
        if let [
            TokenTree::Ident(identifier),
            TokenTree::Punct(first),
            TokenTree::Punct(second),
        ] = window
            && first.as_char() == ':'
            && second.as_char() == ':'
        {
            output.insert(identifier.to_string());
        }
    }
    for token in tokens {
        if let TokenTree::Group(group) = token {
            collect_path_roots(group.stream(), output);
        }
    }
}

fn adapter_member(path: &str) -> Option<&'static str> {
    match path.replace('\\', "/").as_str() {
        "crates/buoy-core/src/value/hash/xxh3_128.rs" => Some("xxhash_rust"),
        "crates/buoy-core/src/value/hash/blake3_256.rs" => Some("blake3"),
        "crates/buoy-core/src/value/hash/sha_256.rs" => Some("sha2"),
        _ => None,
    }
}

fn outside_registry_code(path: &str) -> &'static str {
    if path.starts_with("fixtures/digest/v3/") {
        "DIG-HASH-OUTSIDE-REGISTRY"
    } else {
        "XXH-POLICY-PROHIBITED-API"
    }
}

fn rust_findings(
    path: &str,
    source: &str,
    role: &str,
) -> Result<Vec<HashingPolicyFinding>, String> {
    let parsed = syn::parse_file(source).map_err(|_| format!("{path}: invalid Rust fixture"))?;
    let tokens = parsed.into_token_stream();
    let mut identifiers = BTreeSet::new();
    collect_identifiers(tokens.clone(), &mut identifiers);
    let mut path_roots = BTreeSet::new();
    collect_path_roots(tokens, &mut path_roots);
    let mut findings = BTreeSet::new();

    for (identifier, code) in [
        ("DefaultHasher", "XXH-POLICY-PROHIBITED-CACHE-HASHER"),
        ("RandomState", "XXH-POLICY-PROHIBITED-TABLE-HASHER"),
        ("HashMap", "XXH-POLICY-PROHIBITED-TABLE-HASHER"),
    ] {
        if identifiers.contains(identifier) {
            findings.insert(finding(code, path, source, identifier, role));
        }
    }

    let direct_crates = ["xxhash_rust", "blake3", "sha2"];
    if let Some(expected) = adapter_member(path) {
        for identifier in direct_crates {
            if identifier != expected && path_roots.contains(identifier) {
                findings.insert(finding(
                    "DIG-HASH-WRONG-ADAPTER",
                    path,
                    source,
                    identifier,
                    role,
                ));
            }
        }
    } else {
        for identifier in direct_crates {
            if path_roots.contains(identifier) {
                findings.insert(finding(
                    outside_registry_code(path),
                    path,
                    source,
                    identifier,
                    role,
                ));
            }
        }
        if ["INITIAL", "ROUND", "wrapping_add"]
            .iter()
            .all(|identifier| identifiers.contains(*identifier))
        {
            findings.insert(finding(
                outside_registry_code(path),
                path,
                source,
                "INITIAL",
                role,
            ));
        }
        if source.contains("Command::new(\"sha256sum\")") {
            findings.insert(finding(
                outside_registry_code(path),
                path,
                source,
                "sha256sum",
                role,
            ));
        }
    }
    Ok(findings.into_iter().collect())
}

fn dependency_keys(value: &toml::Value, output: &mut BTreeSet<String>) {
    match value {
        toml::Value::Table(table) => {
            for (key, value) in table {
                let lowered = key.to_ascii_lowercase();
                if matches!(lowered.as_str(), "sha2" | "blake3") {
                    output.insert(key.clone());
                }
                dependency_keys(value, output);
            }
        }
        toml::Value::Array(values) => {
            for value in values {
                dependency_keys(value, output);
            }
        }
        _ => {}
    }
}

fn manifest_findings(path: &str, source: &str) -> Result<Vec<HashingPolicyFinding>, String> {
    let value = toml::from_str::<toml::Value>(source)
        .map_err(|error| format!("{path}: invalid TOML manifest: {error}"))?;
    let mut dependencies = BTreeSet::new();
    dependency_keys(&value, &mut dependencies);
    if path.replace('\\', "/") == "crates/buoy-core/Cargo.toml" {
        return Ok(Vec::new());
    }
    Ok(dependencies
        .into_iter()
        .map(|dependency| {
            finding(
                "XXH-POLICY-PROHIBITED-DEPENDENCY",
                path,
                source,
                &dependency,
                "manifest",
            )
        })
        .collect())
}

fn transitive_findings(path: &str, source: &str) -> Result<Vec<HashingPolicyFinding>, String> {
    let value = serde_json::from_str::<JsonValue>(source)
        .map_err(|_| format!("{path}: invalid dependency metadata JSON"))?;
    let packages = value
        .get("packages")
        .and_then(JsonValue::as_array)
        .ok_or_else(|| format!("{path}: dependency metadata has no package array"))?;
    let package_names = packages
        .iter()
        .filter_map(|package| {
            Some((
                package.get("id")?.as_str()?.to_owned(),
                package.get("name")?.as_str()?.to_owned(),
            ))
        })
        .collect::<BTreeMap<_, _>>();
    let selected = value
        .get("resolve")
        .and_then(|resolve| resolve.get("nodes"))
        .and_then(JsonValue::as_array)
        .map_or_else(
            || packages.iter().collect::<Vec<_>>(),
            |nodes| nodes.iter().collect::<Vec<_>>(),
        );
    let mut findings = BTreeSet::new();
    for entry in selected {
        let name = entry
            .get("id")
            .and_then(JsonValue::as_str)
            .and_then(|id| package_names.get(id))
            .map_or_else(
                || entry.get("name").and_then(JsonValue::as_str),
                |name| Some(name.as_str()),
            );
        if name.is_some_and(|name| matches!(name, "sha2" | "blake3")) {
            continue;
        }
        let Some(features) = entry.get("features").and_then(JsonValue::as_array) else {
            continue;
        };
        for feature in features.iter().filter_map(JsonValue::as_str) {
            let feature = feature.to_ascii_lowercase();
            if name.is_some_and(|name| matches!(name, "buoy" | "buoy-core"))
                && matches!(feature.as_str(), "xxh3-128" | "blake3-256" | "sha-256")
            {
                continue;
            }
            for prohibited in ["sha2", "blake3"] {
                if feature.contains(prohibited) {
                    findings.insert(finding(
                        "XXH-POLICY-PROHIBITED-TRANSITIVE-FEATURE",
                        path,
                        source,
                        prohibited,
                        "transitive-features",
                    ));
                }
            }
        }
    }
    Ok(findings.into_iter().collect())
}

fn documentation_findings(path: &str, source: &str) -> Vec<HashingPolicyFinding> {
    let mut findings = BTreeSet::new();
    for claim in [
        "SHA-256 is the active Buoy identity",
        "BLAKE3 is the active Buoy identity",
    ] {
        if source.contains(claim) {
            findings.insert(finding(
                "XXH-POLICY-PROHIBITED-CLAIM",
                path,
                source,
                claim,
                "documentation",
            ));
        }
    }
    for claim in [
        "Digest equality establishes authenticity",
        "Digest equality establishes tamper evidence",
        "Digest equality establishes proof",
        "xxh3-128 is collision-resistant",
    ] {
        if source.contains(claim) {
            findings.insert(finding(
                "DIG-UNSUPPORTED-ASSURANCE-CLAIM",
                path,
                source,
                claim,
                "documentation-assurance",
            ));
        }
    }
    findings.into_iter().collect()
}

/// Evaluates one predeclared syntax fixture by semantic role.
///
/// # Errors
///
/// Returns an error if the fixture is not valid syntax for its declared role.
pub fn evaluate_hashing_policy_fixture(
    role: &str,
    path: &str,
    source: &str,
) -> Result<Vec<HashingPolicyFinding>, String> {
    match role {
        "core" | "reference" | "tool" | "generator" | "cache-fingerprint" | "hash-table" => {
            rust_findings(path, source, role)
        }
        "manifest" => manifest_findings(path, source),
        "transitive-features" => transitive_findings(path, source),
        "documentation" => Ok(documentation_findings(path, source)),
        _ => Err(format!("{path}: unknown policy role `{role}`")),
    }
}

#[allow(clippy::expect_used)]
fn collect_paths(
    root: &Path,
    current: &Path,
    output: &mut Vec<PathBuf>,
    inject_entry_error: bool,
    inject_nested_error: bool,
) -> Result<(), String> {
    let entries =
        fs::read_dir(current).map_err(|error| format!("{}: {error}", current.display()))?;
    let collected = if inject_entry_error {
        Err(std::io::Error::other("injected directory entry failure"))
    } else {
        entries.collect::<Result<Vec<_>, _>>()
    };
    let mut entries = collected.map_err(|error| format!("{}: {error}", current.display()))?;
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
                collect_paths(root, &path, output, inject_nested_error, false)?;
            }
        } else if path.is_file() {
            output.push(
                path.strip_prefix(root)
                    .expect("collected descendants retain their repository root")
                    .to_path_buf(),
            );
        }
    }
    Ok(())
}

fn documentation_claim_excluded(path: &str) -> bool {
    path.starts_with("docs/schema/v1/")
        || path.starts_with("openspec/changes/archive/")
        || path.starts_with("fixtures/xxh3/v2/policy/negative/")
        || path.starts_with("fixtures/digest/v3/policy/")
}

fn archival_or_negative_path(path: &str) -> bool {
    path.starts_with("docs/schema/v1/")
        || path.starts_with("schema/v1/")
        || path.starts_with("assurance/v1/")
        || path.starts_with("fixtures/hardening/v1/")
        || path.starts_with("openspec/changes/archive/")
        || path.starts_with("fixtures/xxh3/v2/policy/negative/")
        || path.starts_with("fixtures/xxh3/v2/external/")
        || path.ends_with("fixtures/xxh3/v2/malformed-digest-vectors.tsv")
        || path.ends_with("fixtures/xxh3/v2/migration/complete-v1-input.toml")
        || path.starts_with("fixtures/digest/v3/policy/")
}

/// Scans repository-owned source, manifests, documentation, and generated active values.
///
/// # Errors
///
/// Returns an error on unreadable paths or malformed Rust/manifests in scan scope.
pub fn scan_hashing_policy(root: &Path) -> Result<Vec<HashingPolicyFinding>, String> {
    scan_hashing_policy_internal(root, None)
}

/// Injects one reviewed repository scanner failure at its owning operation.
///
/// # Errors
///
/// Returns the injected or naturally encountered scanner failure.
#[cfg(feature = "coverage-controls")]
pub fn scan_hashing_policy_fault_control(
    root: &Path,
    fault: &str,
) -> Result<Vec<HashingPolicyFinding>, String> {
    scan_hashing_policy_internal(root, Some(fault))
}

fn read_policy_source(path: &Path, inject_error: bool) -> std::io::Result<String> {
    if inject_error {
        Err(std::io::Error::other("injected policy read failure"))
    } else {
        fs::read_to_string(path)
    }
}

fn scan_hashing_policy_internal(
    root: &Path,
    fault: Option<&str>,
) -> Result<Vec<HashingPolicyFinding>, String> {
    let mut paths = Vec::new();
    collect_paths(
        root,
        root,
        &mut paths,
        fault == Some("collect-entry"),
        fault == Some("nested-entry"),
    )?;
    paths.sort_unstable();
    let mut findings = BTreeSet::new();
    for relative in paths {
        let normalized = relative.to_string_lossy().replace('\\', "/");
        let extension = relative.extension().and_then(|value| value.to_str());
        if extension == Some("rs") {
            let source = read_policy_source(&root.join(&relative), fault == Some("rust-read"))
                .map_err(|error| format!("{normalized}: {error}"))?;
            findings.extend(rust_findings(&normalized, &source, "rust-source")?);
        } else if relative.file_name().and_then(|value| value.to_str()) == Some("Cargo.toml") {
            let source = read_policy_source(&root.join(&relative), fault == Some("manifest-read"))
                .map_err(|error| format!("{normalized}: {error}"))?;
            findings.extend(manifest_findings(&normalized, &source)?);
        } else if extension == Some("md") {
            let source = read_policy_source(&root.join(&relative), fault == Some("markdown-read"))
                .map_err(|error| format!("{normalized}: {error}"))?;
            if !documentation_claim_excluded(&normalized) {
                findings.extend(documentation_findings(&normalized, &source));
            }
        }
        if matches!(extension, Some("toml" | "tsv" | "json" | "txt"))
            && !archival_or_negative_path(&normalized)
        {
            let source =
                read_policy_source(&root.join(&relative), fault == Some("structured-read"))
                    .map_err(|error| format!("{normalized}: {error}"))?;
            if source.contains("sha256:") || source.contains("blake3:") {
                let marker = if source.contains("sha256:") {
                    "sha256:"
                } else {
                    "blake3:"
                };
                findings.insert(finding(
                    "XXH-POLICY-PROHIBITED-ACTIVE-VALUE",
                    &normalized,
                    &source,
                    marker,
                    "active-value",
                ));
            }
        }
    }
    Ok(findings.into_iter().collect())
}

/// Inspects resolved Cargo metadata for prohibited packages or selected features.
///
/// # Errors
///
/// Returns an error when metadata is malformed.
pub fn inspect_resolved_dependency_features(
    metadata_json: &str,
) -> Result<Vec<HashingPolicyFinding>, String> {
    transitive_findings("<cargo-metadata>", metadata_json)
}
