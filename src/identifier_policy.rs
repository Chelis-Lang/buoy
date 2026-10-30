//! Syntax-aware stable-ID boundary analysis with deterministic findings.
//!
//! The gate proves that every identifier role uses its validated newtype, no
//! identifier role keeps an unchecked bypass constructor, and all grammars live
//! only in the identifier module.
//! Detection parses each source and inspects declared parameters, impl methods,
//! and resolved identifiers, so a name inside a string literal or comment is
//! never a finding.

use proc_macro2::{TokenStream, TokenTree};
use quote::ToTokens as _;
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

/// One normalized, totally ordered stable-ID boundary finding.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct IdentifierPolicyFinding {
    /// Stable machine code.
    pub code: &'static str,
    /// Normalized repository-relative path.
    pub path: String,
    /// One-based source line.
    pub line: usize,
    /// One-based source column.
    pub column: usize,
    /// Semantic scanner role.
    pub role: String,
    /// Stable offending declaration.
    pub detail: String,
}

/// The one module allowed to hold the frozen stable-ID grammar.
const GRAMMAR_HOME: &str = "crates/buoy-core/src/identifier.rs";

/// Free functions that implement the frozen stable-ID grammar check.
const GRAMMAR_HELPERS: [&str; 6] = [
    "require_stable_id",
    "stable_id_grammar",
    "require_scope_id",
    "require_schema_id",
    "scope_id_grammar",
    "schema_id_grammar",
];

/// The single bypass this phase allows. `PolicyRatchetImpact::from_validated`
/// carries `ratchet_id`, a scope ID that the second phase types and removes.
const ALLOWED_BYPASS: [(&str, &str); 1] = [("PolicyRatchetImpact", "from_validated")];

/// One exempt identifier-shaped parameter and the role it actually carries.
///
/// Each entry is `(type, function, parameter, role)`. The `scope-id` and
/// `schema-id` roles are the exact set that the second phase converts to
/// `ScopeId` and `SchemaId`; the `schema-impact` roles name the frozen
/// lowercase schema-graph vocabulary, which is not an object identifier.
/// Any identifier-shaped parameter outside this list must already be
/// `StableId`. The repository scan reports an entry that stops matching, so
/// the list cannot silently outlive the boundary it exempts.
const EXEMPT_ROLE_PARAMETERS: [(&str, &str, &str, &str); 3] = [
    ("<free>", "schema_impact_edge", "id", "schema-impact-edge"),
    (
        "SchemaImpactDependency",
        "new",
        "edge_id",
        "schema-impact-edge",
    ),
    ("SchemaImpactKey", "new", "id", "schema-impact-domain"),
];

fn normalize(path: &str) -> String {
    path.replace('\\', "/")
}

fn finding(
    code: &'static str,
    path: &str,
    span: proc_macro2::Span,
    role: &str,
    detail: String,
) -> IdentifierPolicyFinding {
    let start = span.start();
    IdentifierPolicyFinding {
        code,
        path: normalize(path),
        line: start.line,
        column: start.column + 1,
        role: role.to_owned(),
        detail,
    }
}

/// Returns whether a parameter name denotes an object-identifier role.
fn identifier_role(name: &str) -> bool {
    name == "id" || name.ends_with("_id")
}

/// Returns whether a declared type already proves stable-ID validation.
fn types_as_identifier(tokens: &TokenStream) -> bool {
    tokens.clone().into_iter().any(|token| match token {
        TokenTree::Ident(identifier) => matches!(
            identifier.to_string().as_str(),
            "StableId" | "ScopeId" | "SchemaId" | "NamespacePrefix" | "LogicalMountId"
        ),
        TokenTree::Group(group) => types_as_identifier(&group.stream()),
        TokenTree::Punct(_) | TokenTree::Literal(_) => false,
    })
}

fn exempt_role(owner: &str, function: &str, parameter: &str) -> Option<usize> {
    EXEMPT_ROLE_PARAMETERS.iter().position(
        |(exempt_owner, exempt_function, exempt_parameter, _)| {
            *exempt_owner == owner && *exempt_function == function && *exempt_parameter == parameter
        },
    )
}

fn allowed_bypass(owner: &str, function: &str) -> bool {
    ALLOWED_BYPASS
        .iter()
        .any(|(allowed_owner, allowed_function)| {
            *allowed_owner == owner && *allowed_function == function
        })
}

#[allow(clippy::expect_used)]
fn owner_name(self_type: &syn::Type) -> String {
    match self_type {
        syn::Type::Path(path) => path
            .path
            .segments
            .last()
            .expect("a parsed type path has one segment")
            .ident
            .to_string(),
        other => other.to_token_stream().to_string(),
    }
}

fn signature_findings(
    path: &str,
    role: &str,
    owner: &str,
    signature: &syn::Signature,
    findings: &mut BTreeSet<IdentifierPolicyFinding>,
    exercised: &mut BTreeSet<usize>,
) {
    let function = signature.ident.to_string();
    if function == "from_validated" && !allowed_bypass(owner, &function) {
        findings.insert(finding(
            "SID-BOUNDARY-STRING-BYPASS",
            path,
            signature.ident.span(),
            role,
            format!("{owner}::{function}"),
        ));
    }
    for input in &signature.inputs {
        let syn::FnArg::Typed(typed) = input else {
            continue;
        };
        let syn::Pat::Ident(pattern) = typed.pat.as_ref() else {
            continue;
        };
        let parameter = pattern.ident.to_string();
        if !identifier_role(&parameter) || types_as_identifier(&typed.ty.to_token_stream()) {
            continue;
        }
        if let Some(index) = exempt_role(owner, &function, &parameter) {
            exercised.insert(index);
            continue;
        }
        findings.insert(finding(
            "SID-BOUNDARY-UNTYPED-PARAMETER",
            path,
            pattern.ident.span(),
            role,
            format!("{owner}::{function}({parameter})"),
        ));
    }
}

fn item_findings(
    path: &str,
    role: &str,
    items: &[syn::Item],
    findings: &mut BTreeSet<IdentifierPolicyFinding>,
    exercised: &mut BTreeSet<usize>,
) {
    for item in items {
        match item {
            syn::Item::Fn(function) => {
                signature_findings(path, role, "<free>", &function.sig, findings, exercised);
            }
            syn::Item::Impl(block) => {
                let owner = owner_name(block.self_ty.as_ref());
                for member in &block.items {
                    if let syn::ImplItem::Fn(function) = member {
                        signature_findings(path, role, &owner, &function.sig, findings, exercised);
                    }
                }
            }
            syn::Item::Trait(declaration) => {
                let owner = declaration.ident.to_string();
                for member in &declaration.items {
                    if let syn::TraitItem::Fn(function) = member {
                        signature_findings(path, role, &owner, &function.sig, findings, exercised);
                    }
                }
            }
            syn::Item::Mod(module) => {
                if let Some((_, nested)) = &module.content {
                    item_findings(path, role, nested, findings, exercised);
                }
            }
            _ => {}
        }
    }
}

fn grammar_findings(
    path: &str,
    role: &str,
    file: &syn::File,
    findings: &mut BTreeSet<IdentifierPolicyFinding>,
) {
    if normalize(path) == GRAMMAR_HOME {
        return;
    }
    collect_grammar_identifiers(path, role, file.to_token_stream(), findings);
}

fn collect_grammar_identifiers(
    path: &str,
    role: &str,
    tokens: TokenStream,
    findings: &mut BTreeSet<IdentifierPolicyFinding>,
) {
    for token in tokens {
        match token {
            TokenTree::Ident(identifier) => {
                let name = identifier.to_string();
                if GRAMMAR_HELPERS.contains(&name.as_str()) {
                    findings.insert(finding(
                        "SID-BOUNDARY-EXTERNAL-GRAMMAR",
                        path,
                        identifier.span(),
                        role,
                        name,
                    ));
                }
            }
            TokenTree::Group(group) => {
                collect_grammar_identifiers(path, role, group.stream(), findings);
            }
            TokenTree::Punct(_) | TokenTree::Literal(_) => {}
        }
    }
}

/// Evaluates one predeclared identifier-boundary fixture by semantic role.
///
/// # Errors
///
/// Returns an error if the fixture is not valid Rust or its role is unknown.
pub fn evaluate_identifier_policy_fixture(
    role: &str,
    path: &str,
    source: &str,
) -> Result<Vec<IdentifierPolicyFinding>, String> {
    if !matches!(role, "core-model" | "identifier-module") {
        return Err(format!("{path}: unknown identifier policy role `{role}`"));
    }
    let parsed = syn::parse_file(source).map_err(|_| format!("{path}: invalid Rust fixture"))?;
    let mut findings = BTreeSet::new();
    let mut exercised = BTreeSet::new();
    item_findings(path, role, &parsed.items, &mut findings, &mut exercised);
    if role == "core-model" {
        grammar_findings(path, role, &parsed, &mut findings);
    }
    Ok(findings.into_iter().collect())
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
                ".git" | ".devenv" | "target" | "result" | "node_modules" | ".worktrees"
            ) {
                collect_paths(root, &path, output, inject_nested_error, false)?;
            }
        } else if path.is_file() && path.extension().and_then(|value| value.to_str()) == Some("rs")
        {
            output.push(
                path.strip_prefix(root)
                    .expect("collected descendants retain their repository root")
                    .to_path_buf(),
            );
        }
    }
    Ok(())
}

/// Returns whether a path is core source subject to the parameter and bypass rules.
fn core_source(path: &str) -> bool {
    path.starts_with("crates/buoy-core/src/")
}

/// Scans repository Rust source for every stable-ID boundary regression.
///
/// # Errors
///
/// Returns an error on unreadable paths or malformed Rust in scan scope.
pub fn scan_identifier_policy(root: &Path) -> Result<Vec<IdentifierPolicyFinding>, String> {
    scan_identifier_policy_internal(root, None)
}

/// Injects one reviewed repository scanner failure at its owning operation.
///
/// # Errors
///
/// Returns the injected or naturally encountered scanner failure.
#[cfg(feature = "coverage-controls")]
pub fn scan_identifier_policy_fault_control(
    root: &Path,
    fault: &str,
) -> Result<Vec<IdentifierPolicyFinding>, String> {
    scan_identifier_policy_internal(root, Some(fault))
}

fn read_policy_source(path: &Path, inject_error: bool) -> std::io::Result<String> {
    if inject_error {
        Err(std::io::Error::other("injected policy read failure"))
    } else {
        fs::read_to_string(path)
    }
}

fn scan_identifier_policy_internal(
    root: &Path,
    fault: Option<&str>,
) -> Result<Vec<IdentifierPolicyFinding>, String> {
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
    let mut exercised = BTreeSet::new();
    for relative in paths {
        let normalized = normalize(&relative.to_string_lossy());
        let source = read_policy_source(&root.join(&relative), fault == Some("rust-read"))
            .map_err(|error| format!("{normalized}: {error}"))?;
        let source = if fault == Some("rust-syntax") {
            String::from("not valid Rust")
        } else {
            source
        };
        let parsed =
            syn::parse_file(&source).map_err(|_| format!("{normalized}: invalid Rust source"))?;
        if core_source(&normalized) {
            let role = if normalized == GRAMMAR_HOME {
                "identifier-module"
            } else {
                "core-model"
            };
            item_findings(
                &normalized,
                role,
                &parsed.items,
                &mut findings,
                &mut exercised,
            );
        }
        grammar_findings(&normalized, "rust-source", &parsed, &mut findings);
    }
    if fault == Some("stale-exemption") {
        exercised.clear();
    }
    for (index, (owner, function, parameter, exempt)) in EXEMPT_ROLE_PARAMETERS.iter().enumerate() {
        if exercised.contains(&index) {
            continue;
        }
        findings.insert(IdentifierPolicyFinding {
            code: "SID-BOUNDARY-STALE-EXEMPTION",
            path: String::from("src/identifier_policy.rs"),
            line: 0,
            column: 0,
            role: (*exempt).to_owned(),
            detail: format!("{owner}::{function}({parameter})"),
        });
    }
    Ok(findings.into_iter().collect())
}
