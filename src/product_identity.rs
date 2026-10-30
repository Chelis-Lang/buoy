//! Repository-owned Buoy product-identity migration checks.
//!
//! This module is shell-side assurance tooling. It uses Git only to enumerate
//! the reviewed working-tree file set; Git state is never a canonical product
//! input and never contributes to graph or report identity.

#[path = "../tools/digest_binding.rs"]
mod content_digest;
#[path = "../tools/execution_evidence.rs"]
mod execution_evidence;

use buoy_core::{CanonicalValue, DecodeBudget, semantic_digest_v2};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::{Display, Formatter, Write as _};
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::process::Command;

/// Machine schema for the frozen pre-rename corpus.
pub const CORPUS_SCHEMA: &str = "buoy-pre-rename-corpus/v1";
/// Machine schema for old-name occurrence inventories.
pub const INVENTORY_SCHEMA: &str = "buoy-naming-inventory/v1";
/// Machine schema for product-identity fixture cases.
pub const FIXTURE_SCHEMA: &str = "buoy-product-identity-fixtures/v1";

const PRE_RENAME_CORPUS: &str = "assurance/product-identity/v1/pre-rename-corpus.tsv";
const PRE_RENAME_SNAPSHOTS: &str = "assurance/product-identity/v1/pre-rename-snapshots.tsv";
const SNAPSHOT_SCHEMA: &str = "buoy-pre-rename-snapshot/v1";
const PRE_RENAME_OCCURRENCES: &str = "assurance/product-identity/v1/pre-rename-occurrences.tsv";
const CURRENT_INVENTORY: &str = "assurance/product-identity/v1/naming-inventory.tsv";
const FIXTURE_MANIFEST: &str = "fixtures/product-identity/v1/manifest.tsv";
const FROZEN_MUTATIONS: &str = "fixtures/product-identity/v1/frozen-identifier-mutations.tsv";
const FIXTURE_EXECUTIONS: &str = "assurance/v1/product-identity-fixture-executions.tsv";
const EXECUTION_TOOL_PATHS: [&str; 4] = [
    "src/product_identity.rs",
    "src/bin/buoy-product-identity.rs",
    "tools/execution_evidence.rs",
    "tools/digest_binding.rs",
];
const EXECUTION_CONFIGURATION_PATHS: [&str; 2] = [FIXTURE_MANIFEST, FROZEN_MUTATIONS];

const OLD_PATTERNS: [(&str, &[u8]); 4] = [
    ("display", b"spec provenance"),
    ("kebab", b"spec-provenance"),
    ("snake", b"spec_provenance"),
    ("compact", b"specprovenance"),
];

const CORPUS_ENTRIES: [(&str, &str, &str); 10] = [
    (
        "canonical-values-v2",
        "canonical-value",
        "fixtures/hardening/v2/canonical-vectors.tsv",
    ),
    (
        "canonical-graph-v2",
        "canonical-graph",
        "fixtures/hardening/v2/canonical-vectors.tsv",
    ),
    (
        "canonical-static-report-v2",
        "static-report",
        "fixtures/hardening/v2/canonical-report-goldens.tsv",
    ),
    (
        "canonical-execution-report-v2",
        "execution-report",
        "fixtures/hardening/v2/canonical-report-goldens.tsv",
    ),
    (
        "canonical-impact-report-v2",
        "impact-report",
        "fixtures/hardening/v2/canonical-report-goldens.tsv",
    ),
    (
        "protocol-inventory-v2",
        "protocol",
        "schema/v2/inventory.tsv",
    ),
    (
        "diagnostic-corpus-v2",
        "diagnostic",
        "fixtures/v2/baselines/invalid-diagnostics.toml",
    ),
    (
        "historical-values-v1",
        "historical-reader",
        "fixtures/hardening/v1/canonical-vectors.tsv",
    ),
    (
        "historical-reports-v1",
        "historical-reader",
        "fixtures/hardening/v1/canonical-report-goldens.tsv",
    ),
    (
        "historical-migration-input-v1",
        "historical-reader",
        "fixtures/xxh3/v2/migration/complete-v1-input.toml",
    ),
];

/// One stable product-identity diagnostic.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProductIdentityError {
    code: &'static str,
    detail: String,
}

impl ProductIdentityError {
    fn new(code: &'static str, detail: impl Into<String>) -> Self {
        Self {
            code,
            detail: detail.into(),
        }
    }

    /// Returns the stable diagnostic code.
    #[must_use]
    pub const fn code(&self) -> &'static str {
        self.code
    }
}

impl Display for ProductIdentityError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}: {}", self.code, self.detail)
    }
}

impl std::error::Error for ProductIdentityError {}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
enum Disposition {
    RenameProduct,
    CompatibilityAlias,
    FrozenIdentifier,
    HistoricalReference,
    GeneratedOrExternal,
}

impl Disposition {
    const fn name(self) -> &'static str {
        match self {
            Self::RenameProduct => "rename-product",
            Self::CompatibilityAlias => "compatibility-alias",
            Self::FrozenIdentifier => "frozen-v1-identifier",
            Self::HistoricalReference => "historical-reference",
            Self::GeneratedOrExternal => "generated-or-external",
        }
    }

    fn parse(value: &str) -> Result<Self, ProductIdentityError> {
        match value {
            "rename-product" => Ok(Self::RenameProduct),
            "compatibility-alias" => Ok(Self::CompatibilityAlias),
            "frozen-v1-identifier" => Ok(Self::FrozenIdentifier),
            "historical-reference" => Ok(Self::HistoricalReference),
            "generated-or-external" => Ok(Self::GeneratedOrExternal),
            _ => Err(ProductIdentityError::new(
                "PID-INVENTORY-DISPOSITION",
                value,
            )),
        }
    }
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct Occurrence {
    path: String,
    line: usize,
    column: usize,
    form: String,
    lexeme: String,
    context_digest: String,
    context: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct InventoryRow {
    id: String,
    occurrence: Occurrence,
    disposition: Disposition,
    rationale: String,
    owner: String,
    rule: String,
}

impl InventoryRow {
    fn render(&self) -> String {
        format!(
            "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
            INVENTORY_SCHEMA,
            self.id,
            self.occurrence.path,
            self.occurrence.line,
            self.occurrence.column,
            self.occurrence.form,
            self.occurrence.lexeme,
            self.occurrence.context_digest,
            self.disposition.name(),
            self.rationale,
            self.owner,
            self.rule,
        )
    }
}

fn digest_text(bytes: &[u8]) -> String {
    content_digest::bytes_binding(bytes)
}

fn read_text(root: &Path, path: &str, code: &'static str) -> Result<String, ProductIdentityError> {
    fs::read_to_string(root.join(path))
        .map_err(|error| ProductIdentityError::new(code, format!("{path}: {error}")))
}

fn tracked_paths(root: &Path) -> Result<Vec<PathBuf>, ProductIdentityError> {
    tracked_paths_with_program(root, "git")
}

fn tracked_paths_with_program(
    root: &Path,
    program: &str,
) -> Result<Vec<PathBuf>, ProductIdentityError> {
    let output = Command::new(program)
        .args([
            "ls-files",
            "-z",
            "--cached",
            "--others",
            "--exclude-standard",
        ])
        .current_dir(root)
        .output()
        .map_err(|error| ProductIdentityError::new("PID-SCAN-GIT", error.to_string()))?;
    if !output.status.success() {
        return Err(ProductIdentityError::new(
            "PID-SCAN-GIT",
            String::from_utf8_lossy(&output.stderr),
        ));
    }
    let mut paths = output
        .stdout
        .split(|byte| *byte == 0)
        .filter(|bytes| !bytes.is_empty())
        .filter_map(|bytes| core::str::from_utf8(bytes).ok())
        .filter(|path| *path != PRE_RENAME_OCCURRENCES && *path != CURRENT_INVENTORY)
        .filter(|path| root.join(path).is_file())
        .map(PathBuf::from)
        .collect::<Vec<_>>();
    paths.sort_unstable();
    paths.dedup();
    Ok(paths)
}

fn scan_line(path: &str, line_number: usize, line: &str) -> Vec<Occurrence> {
    let bytes = line.as_bytes();
    let lower = bytes.iter().map(u8::to_ascii_lowercase).collect::<Vec<_>>();
    let mut result = Vec::new();
    let mut offset = 0_usize;
    while offset < lower.len() {
        let matched = OLD_PATTERNS
            .iter()
            .find(|(_, pattern)| lower[offset..].starts_with(pattern));
        let Some((form, pattern)) = matched else {
            offset += 1;
            continue;
        };
        let end = offset + pattern.len();
        let lexeme = bytes[offset..end]
            .iter()
            .map(|byte| char::from(*byte))
            .collect();
        result.push(Occurrence {
            path: path.to_owned(),
            line: line_number,
            column: offset + 1,
            form: (*form).to_owned(),
            lexeme,
            context_digest: digest_text(bytes),
            context: line.to_owned(),
        });
        offset = end;
    }
    result
}

fn scan_paths(root: &Path, paths: &[PathBuf]) -> Result<Vec<Occurrence>, ProductIdentityError> {
    let mut result = Vec::new();
    for relative in paths {
        let path_text = relative.to_string_lossy().replace('\\', "/");
        let bytes = fs::read(root.join(relative)).map_err(|error| {
            ProductIdentityError::new("PID-SCAN-READ", format!("{path_text}: {error}"))
        })?;
        let Ok(text) = core::str::from_utf8(&bytes) else {
            continue;
        };
        for (line_index, line) in text.lines().enumerate() {
            result.extend(scan_line(&path_text, line_index + 1, line));
        }
    }
    result.sort_unstable();
    Ok(result)
}

fn scan_repository(root: &Path) -> Result<Vec<Occurrence>, ProductIdentityError> {
    scan_paths(root, &tracked_paths(root)?)
}

fn surrounding_token(line: &str, column: usize, lexeme_length: usize) -> String {
    let bytes = line.as_bytes();
    let mut start = column.saturating_sub(1);
    let mut end = start.saturating_add(lexeme_length).min(bytes.len());
    let token_byte = |byte: u8| {
        byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'/' | b':' | b'*')
    };
    while start > 0 && token_byte(bytes[start - 1]) {
        start -= 1;
    }
    while end < bytes.len() && token_byte(bytes[end]) {
        end += 1;
    }
    String::from_utf8_lossy(&bytes[start..end]).to_ascii_lowercase()
}

/// Runs deterministic scanner failures without changing process search paths.
#[cfg(feature = "coverage-controls")]
#[doc(hidden)]
pub fn scan_control(
    root: &Path,
    paths: &[PathBuf],
    program: &str,
) -> Result<usize, ProductIdentityError> {
    if paths.is_empty() {
        if program == "repository" {
            scan_repository(root).map(|occurrences| occurrences.len())
        } else {
            let tracked = tracked_paths_with_program(root, program)?;
            Ok(tracked.len())
        }
    } else {
        scan_paths(root, paths).map(|occurrences| occurrences.len())
    }
}

/// Classifies one synthetic old-name occurrence for closed-rule controls.
#[cfg(feature = "coverage-controls")]
#[doc(hidden)]
#[must_use]
pub fn classify_control(path: &str, context: &str, column: usize, lexeme: &str) -> &'static str {
    classify(&Occurrence {
        path: path.to_owned(),
        line: 1,
        column,
        form: String::from("control"),
        lexeme: lexeme.to_owned(),
        context_digest: digest_text(context.as_bytes()),
        context: context.to_owned(),
    })
    .0
    .name()
}

fn is_compatibility_authority(path: &str) -> bool {
    matches!(
        path,
        "nix/devenv/scripts/core.nix" | "nix/devenv/tasks.nix" | "tools/check_devenv_task_graph.rs"
    )
}

fn classify(occurrence: &Occurrence) -> (Disposition, &'static str, &'static str) {
    let path = occurrence.path.as_str();
    let token = surrounding_token(
        &occurrence.context,
        occurrence.column,
        occurrence.lexeme.len(),
    );
    let frozen_path = path.starts_with("docs/schema/")
        || path.starts_with("fixtures/v1/")
        || path.starts_with("fixtures/v2/")
        || path.starts_with("fixtures/xxh3/v2/migration")
        || path.starts_with("fixtures/adapters/rust/")
        || (path == "tests/adapters.rs" && occurrence.context.contains("spec_provenance::"))
        || ((path == "src/adapter_sdk.rs") & occurrence.context.contains("spec_provenance::"))
        || path.starts_with("src/adapters/")
        || (path.starts_with("crates/buoy-")
            && occurrence.lexeme == occurrence.lexeme.to_ascii_uppercase())
        || (path.starts_with("tests/xxh3_")
            && occurrence.lexeme == occurrence.lexeme.to_ascii_uppercase())
        || path == "tools/check_xxh3_vectors.rs";
    let frozen = frozen_path
        || (token.starts_with("spec-provenance")
            && (token.contains("/v1")
                || token.contains("/v2")
                || token.contains("/v3")
                || token.contains("-v1")
                || token.contains("-v2")
                || token.contains(".semantic/v")));
    if frozen {
        return (
            Disposition::FrozenIdentifier,
            "versioned protocol or canonical identifier is byte-frozen",
            "retain-exact-bytes",
        );
    }
    if path == "Cargo.lock"
        || path.ends_with("/Cargo.lock")
        || path.starts_with("proofs/spikes/aeneas/generated")
        || path == "proofs/spikes/aeneas/integrity.llbc"
        || path == "src/product_identity.rs"
        || path.starts_with("tests/product_identity.rs")
        || path == "tests/assurance_coverage_controls.rs"
        || path == "tests/migration.rs"
        || (path.starts_with("assurance/v1/") && path != "assurance/v1/red-baseline.toml")
        || path.starts_with("assurance/supply-chain/v1/")
        || path == "docs/assurance/quality-assurance-status.md"
        || path.starts_with("fixtures/chelis-provenance/")
        || path.starts_with("fixtures/quality/")
        || path == "tests/chelis_preimplementation.rs"
        || path.starts_with("tools/check_reference_independence.rs")
        || path == "tools/check_property_evidence.rs"
        || path == "tools/check_schema_inventory.rs"
        || path.starts_with("fixtures/product-identity/")
    {
        return (
            Disposition::GeneratedOrExternal,
            "generated, fixture, or checker-owned occurrence has an explicit authority",
            "regenerate-or-retain-by-authority",
        );
    }
    if path.starts_with("openspec/changes/archive/")
        || path.starts_with("openspec/changes/rename-spec-provenance-to-buoy/")
        || path.starts_with("docs/migration")
        || path == "openspec/specs/product-identity/spec.md"
        || path.starts_with("assurance/product-identity/")
        || path == "assurance/v1/red-baseline.toml"
        || occurrence
            .context
            .contains("establish-spec-provenance-system")
        || occurrence
            .context
            .contains("harden-spec-provenance-production-readiness")
        || occurrence.context.contains("openspec/changes/archive/")
    {
        return (
            Disposition::HistoricalReference,
            "reviewed migration or historical record names the former identity",
            "retain-as-history",
        );
    }
    if path.starts_with("crates/spec-provenance/")
        || path.starts_with("crates/spec-provenance-core/")
        || occurrence.context.contains("compatibility-alias")
        || occurrence.context.contains("compatibility-facade")
        || occurrence.context.contains("upgrade-fixture")
        || path.starts_with("docs/release/buoy-rename-checklist.md")
        || (is_compatibility_authority(path) && occurrence.context.contains("spec-provenance"))
        || (path == "devenv.nix" && occurrence.context.contains("spec-provenance"))
        || (path == "Cargo.toml" && occurrence.context.contains("crates/spec-provenance"))
    {
        return (
            Disposition::CompatibilityAlias,
            "time-bounded compatibility surface delegates to Buoy",
            "remove-at-0.3.0",
        );
    }
    (
        Disposition::RenameProduct,
        "current product, package, command, path, or source identity must migrate",
        "replace-with-buoy",
    )
}

fn inventory_rows(occurrences: Vec<Occurrence>) -> Vec<InventoryRow> {
    occurrences
        .into_iter()
        .enumerate()
        .map(|(index, occurrence)| {
            let (disposition, rationale, rule) = classify(&occurrence);
            InventoryRow {
                id: format!("OLD-NAME-{:06}", index + 1),
                occurrence,
                disposition,
                rationale: rationale.to_owned(),
                owner: String::from("buoy-maintainers"),
                rule: rule.to_owned(),
            }
        })
        .collect()
}

fn render_inventory(rows: &[InventoryRow]) -> String {
    let mut output = String::from(
        "schema\tid\tpath\tline\tcolumn\tform\tlexeme\tcontext_digest\tdisposition\trationale\towner\trule\n",
    );
    for row in rows {
        output.push_str(&row.render());
        output.push('\n');
    }
    output
}

fn parse_inventory(text: &str) -> Result<Vec<InventoryRow>, ProductIdentityError> {
    let mut lines = text.lines();
    if lines.next()
        != Some(
            "schema\tid\tpath\tline\tcolumn\tform\tlexeme\tcontext_digest\tdisposition\trationale\towner\trule",
        )
    {
        return Err(ProductIdentityError::new(
            "PID-INVENTORY-SCHEMA",
            "invalid header",
        ));
    }
    let mut rows = Vec::new();
    let mut ids = BTreeSet::new();
    let mut locations = BTreeSet::new();
    for line in lines {
        let fields = line.split('\t').collect::<Vec<_>>();
        if fields.len() != 12 || fields[0] != INVENTORY_SCHEMA {
            return Err(ProductIdentityError::new("PID-INVENTORY-SCHEMA", line));
        }
        let line_number = fields[3]
            .parse::<usize>()
            .map_err(|_| ProductIdentityError::new("PID-INVENTORY-LOCATION", fields[3]))?;
        let column = fields[4]
            .parse::<usize>()
            .map_err(|_| ProductIdentityError::new("PID-INVENTORY-LOCATION", fields[4]))?;
        if !ids.insert(fields[1].to_owned()) {
            return Err(ProductIdentityError::new(
                "PID-INVENTORY-DUPLICATE",
                fields[1],
            ));
        }
        let location = (
            fields[2].to_owned(),
            line_number,
            column,
            fields[6].to_owned(),
        );
        if !locations.insert(location) {
            return Err(ProductIdentityError::new(
                "PID-INVENTORY-DUPLICATE",
                fields[2],
            ));
        }
        if fields[2].contains('*') || fields[2].ends_with('/') {
            return Err(ProductIdentityError::new(
                "PID-INVENTORY-BROAD-EXCLUSION",
                fields[2],
            ));
        }
        rows.push(InventoryRow {
            id: fields[1].to_owned(),
            occurrence: Occurrence {
                path: fields[2].to_owned(),
                line: line_number,
                column,
                form: fields[5].to_owned(),
                lexeme: fields[6].to_owned(),
                context_digest: fields[7].to_owned(),
                context: String::new(),
            },
            disposition: Disposition::parse(fields[8])?,
            rationale: fields[9].to_owned(),
            owner: fields[10].to_owned(),
            rule: fields[11].to_owned(),
        });
    }
    Ok(rows)
}

fn render_corpus(root: &Path) -> Result<String, ProductIdentityError> {
    let snapshots = parse_snapshot_bindings(root)?;
    let mut output = String::from("schema\tid\tkind\tpath\tdigest\n");
    for (id, kind, path) in CORPUS_ENTRIES {
        let bytes = if let Some(binding) = snapshots.get(path) {
            snapshot_bytes(root, path, binding)?
        } else {
            fs::read(root.join(path)).map_err(|error| {
                ProductIdentityError::new("PID-CORPUS-READ", format!("{path}: {error}"))
            })?
        };
        let _ = writeln!(
            output,
            "{CORPUS_SCHEMA}\t{id}\t{kind}\t{path}\t{}",
            digest_text(&bytes)
        );
    }
    Ok(output)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum SnapshotReplay {
    CanonicalVectors,
    LineSubset,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct SnapshotBinding {
    snapshot_path: String,
    replay: SnapshotReplay,
}

fn parity_error(detail: impl Into<String>) -> ProductIdentityError {
    ProductIdentityError::new("PID-CANONICAL-PARITY", detail)
}

fn canonical_relative_path(path: &str) -> bool {
    !path.is_empty()
        && !Path::new(path).is_absolute()
        && Path::new(path)
            .components()
            .all(|component| matches!(component, Component::Normal(_)))
}

fn parse_snapshot_bindings(
    root: &Path,
) -> Result<BTreeMap<String, SnapshotBinding>, ProductIdentityError> {
    let manifest_path = root.join(PRE_RENAME_SNAPSHOTS);
    if !manifest_path.exists() {
        return Ok(BTreeMap::new());
    }
    let source = fs::read_to_string(&manifest_path).map_err(|error| {
        ProductIdentityError::new(
            "PID-CORPUS-READ",
            format!("{PRE_RENAME_SNAPSHOTS}: {error}"),
        )
    })?;
    let mut lines = source.lines();
    if lines.next() != Some("schema\tsource_path\tsnapshot_path\treplay") {
        return Err(parity_error(PRE_RENAME_SNAPSHOTS));
    }
    let corpus_paths = CORPUS_ENTRIES
        .iter()
        .map(|(_, _, path)| *path)
        .collect::<BTreeSet<_>>();
    let mut bindings = BTreeMap::new();
    for line in lines {
        let fields = line.split('\t').collect::<Vec<_>>();
        let [schema, source_path, snapshot_path, replay] = fields.as_slice() else {
            return Err(parity_error(PRE_RENAME_SNAPSHOTS));
        };
        if (
            *schema == SNAPSHOT_SCHEMA,
            corpus_paths.contains(source_path),
            canonical_relative_path(snapshot_path),
        ) != (true, true, true)
        {
            return Err(parity_error(PRE_RENAME_SNAPSHOTS));
        }
        let replay = match *replay {
            "canonical-vectors" => SnapshotReplay::CanonicalVectors,
            "line-subset" => SnapshotReplay::LineSubset,
            _ => return Err(parity_error(PRE_RENAME_SNAPSHOTS)),
        };
        if bindings
            .insert(
                (*source_path).to_owned(),
                SnapshotBinding {
                    snapshot_path: (*snapshot_path).to_owned(),
                    replay,
                },
            )
            .is_some()
        {
            return Err(parity_error(PRE_RENAME_SNAPSHOTS));
        }
    }
    if bindings.is_empty() {
        return Err(parity_error(PRE_RENAME_SNAPSHOTS));
    }
    Ok(bindings)
}

const fn hex_nibble(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        _ => None,
    }
}

fn decode_hex(bytes: &[u8], allow_whitespace: bool) -> Option<Vec<u8>> {
    if !allow_whitespace && bytes.iter().any(u8::is_ascii_whitespace) {
        return None;
    }
    let digits = bytes
        .iter()
        .copied()
        .filter(|byte| !byte.is_ascii_whitespace())
        .collect::<Vec<_>>();
    if digits.is_empty() || digits.len() % 2 != 0 {
        return None;
    }
    digits
        .chunks_exact(2)
        .map(|pair| Some((hex_nibble(pair[0])? << 4) | hex_nibble(pair[1])?))
        .collect()
}

fn validate_frozen_canonical_vectors(bytes: &[u8]) -> Result<(), ProductIdentityError> {
    let source = std::str::from_utf8(bytes).map_err(|_| parity_error(PRE_RENAME_SNAPSHOTS))?;
    let mut lines = source.lines();
    if lines.next() != Some("schema\tkind\tid\tcanonical_hex\tsemantic_digest") {
        return Err(parity_error(PRE_RENAME_SNAPSHOTS));
    }
    let mut identities = BTreeSet::new();
    let mut count = 0_usize;
    for line in lines {
        let fields = line.split('\t').collect::<Vec<_>>();
        if fields.len() != 5 {
            return Err(parity_error(PRE_RENAME_SNAPSHOTS));
        }
        if (
            fields[0] == "spec-provenance-canonical-vector/v2",
            identities.insert((fields[1], fields[2])),
        ) != (true, true)
        {
            return Err(parity_error(PRE_RENAME_SNAPSHOTS));
        }
        let encoded = decode_hex(fields[3].as_bytes(), false)
            .ok_or_else(|| parity_error(PRE_RENAME_SNAPSHOTS))?;
        let value = CanonicalValue::decode(&encoded, DecodeBudget::v2_default())
            .map_err(|_| parity_error(PRE_RENAME_SNAPSHOTS))?;
        match fields[1] {
            "type-tag" | "nested" | "report-fact" if fields[4] == "-" => {}
            "domain" => {
                let CanonicalValue::Record(record) = value else {
                    return Err(parity_error(PRE_RENAME_SNAPSHOTS));
                };
                let digest = semantic_digest_v2(fields[2], record)
                    .map_err(|_| parity_error(PRE_RENAME_SNAPSHOTS))?;
                if digest.canonical_text() != fields[4] {
                    return Err(parity_error(PRE_RENAME_SNAPSHOTS));
                }
            }
            _ => return Err(parity_error(PRE_RENAME_SNAPSHOTS)),
        }
        count += 1;
    }
    if count == 0 {
        return Err(parity_error(PRE_RENAME_SNAPSHOTS));
    }
    Ok(())
}

fn validate_frozen_line_subset(
    root: &Path,
    source_path: &str,
    frozen_bytes: &[u8],
) -> Result<(), ProductIdentityError> {
    let frozen =
        std::str::from_utf8(frozen_bytes).map_err(|_| parity_error(PRE_RENAME_SNAPSHOTS))?;
    let current = read_text(root, source_path, "PID-CORPUS-READ")?;
    let mut frozen_lines = frozen.lines();
    let mut current_lines = current.lines();
    if frozen_lines.next() != current_lines.next() {
        return Err(parity_error(source_path));
    }
    let current_rows = current_lines.collect::<BTreeSet<_>>();
    if frozen_lines.all(|line| current_rows.contains(line)) {
        Ok(())
    } else {
        Err(parity_error(source_path))
    }
}

fn snapshot_bytes(
    root: &Path,
    source_path: &str,
    binding: &SnapshotBinding,
) -> Result<Vec<u8>, ProductIdentityError> {
    let encoded = fs::read(root.join(&binding.snapshot_path)).map_err(|error| {
        ProductIdentityError::new(
            "PID-CORPUS-READ",
            format!("{}: {error}", binding.snapshot_path),
        )
    })?;
    let bytes = decode_hex(&encoded, true).ok_or_else(|| parity_error(&binding.snapshot_path))?;
    match binding.replay {
        SnapshotReplay::CanonicalVectors => validate_frozen_canonical_vectors(&bytes)?,
        SnapshotReplay::LineSubset => validate_frozen_line_subset(root, source_path, &bytes)?,
    }
    Ok(bytes)
}

fn validate_corpus(root: &Path, text: &str) -> Result<(), ProductIdentityError> {
    let snapshots = parse_snapshot_bindings(root)?;
    let mut lines = text.lines();
    if lines.next() != Some("schema\tid\tkind\tpath\tdigest") {
        return Err(parity_error(PRE_RENAME_CORPUS));
    }
    let rows = lines.collect::<Vec<_>>();
    if rows.len() != CORPUS_ENTRIES.len() {
        return Err(parity_error(PRE_RENAME_CORPUS));
    }
    let mut preserved = BTreeMap::<&str, Vec<u8>>::new();
    for (line, (expected_id, expected_kind, expected_path)) in rows.iter().zip(CORPUS_ENTRIES) {
        let fields = line.split('\t').collect::<Vec<_>>();
        if fields.len() != 5 {
            return Err(parity_error(PRE_RENAME_CORPUS));
        }
        if (fields[0], fields[1], fields[2], fields[3])
            != (CORPUS_SCHEMA, expected_id, expected_kind, expected_path)
        {
            return Err(parity_error(PRE_RENAME_CORPUS));
        }
        let bytes = match preserved.entry(expected_path) {
            std::collections::btree_map::Entry::Occupied(entry) => entry.into_mut(),
            std::collections::btree_map::Entry::Vacant(entry) => {
                let bytes = if let Some(binding) = snapshots.get(expected_path) {
                    snapshot_bytes(root, expected_path, binding)?
                } else {
                    fs::read(root.join(expected_path)).map_err(|error| {
                        ProductIdentityError::new(
                            "PID-CORPUS-READ",
                            format!("{expected_path}: {error}"),
                        )
                    })?
                };
                entry.insert(bytes)
            }
        };
        if digest_text(bytes) != fields[4] {
            return Err(parity_error(expected_path));
        }
    }
    Ok(())
}

/// Exercises closed snapshot-manifest parsing from an external coverage harness.
#[cfg(feature = "coverage-controls")]
#[doc(hidden)]
pub fn validate_snapshot_manifest_control(root: &Path) -> Result<(), ProductIdentityError> {
    parse_snapshot_bindings(root).map(drop)
}

/// Exercises closed frozen-corpus parsing from an external coverage harness.
#[cfg(feature = "coverage-controls")]
#[doc(hidden)]
pub fn validate_corpus_control(root: &Path, text: &str) -> Result<(), ProductIdentityError> {
    validate_corpus(root, text)
}

fn ensure_parent(path: &Path) -> Result<(), ProductIdentityError> {
    let Some(parent) = path.parent() else {
        return Err(ProductIdentityError::new(
            "PID-WRITE-PATH",
            path.display().to_string(),
        ));
    };
    fs::create_dir_all(parent)
        .map_err(|error| ProductIdentityError::new("PID-WRITE-PATH", error.to_string()))
}

/// Freezes the exact pre-rename corpus and every old-name occurrence.
///
/// # Errors
///
/// Returns a stable diagnostic when corpus input, repository scanning, or output writing fails.
pub fn freeze_pre_rename(root: &Path) -> Result<(), ProductIdentityError> {
    let corpus_path = root.join(PRE_RENAME_CORPUS);
    let occurrences_path = root.join(PRE_RENAME_OCCURRENCES);
    ensure_parent(&corpus_path)?;
    fs::write(&corpus_path, render_corpus(root)?)
        .map_err(|error| ProductIdentityError::new("PID-WRITE-CORPUS", error.to_string()))?;
    let inventory = render_inventory(&inventory_rows(scan_repository(root)?));
    fs::write(&occurrences_path, inventory)
        .map_err(|error| ProductIdentityError::new("PID-WRITE-INVENTORY", error.to_string()))
}

/// Regenerates the inventory for all old-name occurrences retained after rename.
///
/// # Errors
///
/// Returns a stable diagnostic when repository scanning or output writing fails.
pub fn generate_current_inventory(root: &Path) -> Result<(), ProductIdentityError> {
    let inventory_path = root.join(CURRENT_INVENTORY);
    ensure_parent(&inventory_path)?;
    let inventory = render_inventory(&inventory_rows(scan_repository(root)?));
    fs::write(inventory_path, inventory)
        .map_err(|error| ProductIdentityError::new("PID-WRITE-INVENTORY", error.to_string()))
}

#[derive(Clone, Copy)]
struct IdentityFixture<'a> {
    product: &'a str,
    package: &'a str,
    core_package: &'a str,
    binary: &'a str,
    canonical: &'a [u8],
    compatibility_expires: &'a str,
    dependency_edges: &'a [(&'a str, &'a str)],
    evidence_bindings: &'a [(&'a str, &'a str)],
}

fn evaluate_fixture(operation: &str) -> Result<Option<&'static str>, ProductIdentityError> {
    const VALID_EDGES: &[(&str, &str)] = &[
        ("buoy", "buoy-core"),
        ("spec-provenance", "buoy"),
        ("spec-provenance-core", "buoy-core"),
        ("chelis-adapter", "buoy"),
    ];
    const VALID_EVIDENCE: &[(&str, &str)] = &[("LCOV-001", "P0-coverage")];
    let valid = IdentityFixture {
        product: "Buoy",
        package: "buoy",
        core_package: "buoy-core",
        binary: "buoy",
        canonical: b"spec-provenance-report/v2\0frozen",
        compatibility_expires: "0.3.0",
        dependency_edges: VALID_EDGES,
        evidence_bindings: VALID_EVIDENCE,
    };
    let diagnostic = match operation {
        "primary-identities-pass" => validate_primary(valid),
        "primary-identities-stale" => validate_primary(IdentityFixture {
            product: "Spec Provenance",
            ..valid
        }),
        "primary-package-stale" => validate_primary(IdentityFixture {
            package: "spec-provenance",
            ..valid
        }),
        "primary-core-package-stale" => validate_primary(IdentityFixture {
            core_package: "spec-provenance-core",
            ..valid
        }),
        "primary-binary-stale" => validate_primary(IdentityFixture {
            binary: "spec-provenance",
            ..valid
        }),
        "canonical-parity-pass" | "command-parity-pass" => {
            validate_canonical(valid, valid.canonical)
        }
        "canonical-global-replacement" => validate_canonical(valid, b"buoy-report/v2\0frozen"),
        "compatibility-alias-pass" => validate_compatibility(valid, "0.2.0"),
        "compatibility-alias-expired" => validate_compatibility(valid, "0.3.0"),
        "inventory-total-pass" => validate_synthetic_inventory(false, false, false),
        "inventory-missing" => validate_synthetic_inventory(true, false, false),
        "dependency-boundary-pass" | "chelis-boundary-pass" => validate_dependencies(valid),
        "dependency-core-coupling" => validate_dependencies(IdentityFixture {
            dependency_edges: &[("buoy-core", "spec-provenance")],
            ..valid
        }),
        "command-brand-in-canonical" => validate_canonical(valid, b"buoy\0frozen"),
        "evidence-replay-pass" | "focused-mapping-pass" => validate_evidence(valid),
        "evidence-stale-path" => validate_evidence(IdentityFixture {
            evidence_bindings: &[("LCOV-001", "spec-provenance:coverage")],
            ..valid
        }),
        "focused-mapping-duplicate" => validate_evidence(IdentityFixture {
            evidence_bindings: &[("LCOV-001", "P0-coverage"), ("LCOV-001", "P0-coverage")],
            ..valid
        }),
        "chelis-core-coupling" => validate_dependencies(IdentityFixture {
            dependency_edges: &[("chelis-core", "buoy-core")],
            ..valid
        }),
        "chelis-core-independent" => validate_dependencies(IdentityFixture {
            dependency_edges: &[("chelis-core", "chelis-model")],
            ..valid
        }),
        _ => {
            return Err(ProductIdentityError::new(
                "PID-FIXTURE-UNKNOWN-OPERATION",
                operation,
            ));
        }
    };
    Ok(diagnostic)
}

fn validate_primary(value: IdentityFixture<'_>) -> Option<&'static str> {
    if value.product == "Buoy"
        && value.package == "buoy"
        && value.core_package == "buoy-core"
        && value.binary == "buoy"
    {
        None
    } else {
        Some("PID-STALE-PRODUCT-IDENTITY")
    }
}

fn validate_canonical(value: IdentityFixture<'_>, observed: &[u8]) -> Option<&'static str> {
    if value.canonical == observed {
        None
    } else {
        Some("PID-CANONICAL-PARITY")
    }
}

fn version_at_least(value: &str, threshold: &str) -> bool {
    let parse = |input: &str| {
        input
            .split('.')
            .filter_map(|part| part.parse::<u64>().ok())
            .collect::<Vec<_>>()
    };
    parse(value) >= parse(threshold)
}

fn validate_compatibility(value: IdentityFixture<'_>, current: &str) -> Option<&'static str> {
    if version_at_least(current, value.compatibility_expires) {
        Some("PID-COMPATIBILITY-EXPIRED")
    } else {
        None
    }
}

fn validate_synthetic_inventory(
    missing: bool,
    duplicate: bool,
    broad: bool,
) -> Option<&'static str> {
    if broad {
        Some("PID-INVENTORY-BROAD-EXCLUSION")
    } else if duplicate {
        Some("PID-INVENTORY-DUPLICATE")
    } else if missing {
        Some("PID-INVENTORY-MISSING")
    } else {
        None
    }
}

fn validate_dependencies(value: IdentityFixture<'_>) -> Option<&'static str> {
    if value.dependency_edges.iter().any(|(source, target)| {
        *source == "buoy-core" || *source == "chelis-core" && *target == "buoy-core"
    }) {
        Some("PID-DEPENDENCY-BOUNDARY")
    } else {
        None
    }
}

fn validate_evidence(value: IdentityFixture<'_>) -> Option<&'static str> {
    if value
        .evidence_bindings
        .iter()
        .any(|(_, target)| target.contains("spec-provenance"))
    {
        return Some("PID-EVIDENCE-STALE");
    }
    let unique = value
        .evidence_bindings
        .iter()
        .copied()
        .collect::<BTreeSet<_>>();
    if unique.len() == value.evidence_bindings.len() {
        None
    } else {
        Some("PID-DUPLICATE-OBLIGATION")
    }
}

fn validate_frozen_mutations(root: &Path) -> Result<(), ProductIdentityError> {
    let text = read_text(root, FROZEN_MUTATIONS, "PID-FROZEN-READ")?;
    validate_frozen_mutation_sources(root, &text)
}

fn validate_frozen_mutation_sources(root: &Path, text: &str) -> Result<(), ProductIdentityError> {
    let mut lines = text.lines();
    if lines.next()
        != Some("schema\tid\tclass\tsource_path\tfrozen\treplacement\texpected_diagnostic")
    {
        return Err(ProductIdentityError::new(
            "PID-FROZEN-SCHEMA",
            "invalid header",
        ));
    }
    let mut ids = BTreeSet::new();
    let mut row_count = 0_usize;
    for line in lines {
        let fields = line.split('\t').collect::<Vec<_>>();
        if fields.len() != 7 || fields[0] != "buoy-frozen-identifier-mutations/v1" {
            return Err(ProductIdentityError::new("PID-FROZEN-SCHEMA", line));
        }
        if !ids.insert(fields[1]) {
            return Err(ProductIdentityError::new("PID-FROZEN-DUPLICATE", fields[1]));
        }
        if !fields[4].starts_with("spec-provenance")
            || !fields[5].starts_with("buoy")
            || fields[6] != "PID-FROZEN-IDENTITY"
        {
            return Err(ProductIdentityError::new("PID-FROZEN-VECTOR", fields[1]));
        }
        let source = read_text(root, fields[3], "PID-FROZEN-READ")?;
        if !source.contains(fields[4]) {
            return Err(ProductIdentityError::new(
                "PID-FROZEN-IDENTIFIER-MISSING",
                fields[1],
            ));
        }
        let _mutated = source.replacen(fields[4], fields[5], 1);
        row_count += 1;
    }
    if row_count < 10 {
        return Err(ProductIdentityError::new(
            "PID-FROZEN-VECTOR-MISSING",
            row_count.to_string(),
        ));
    }
    Ok(())
}

/// Validates injected frozen-identifier manifests against repository sources.
#[cfg(feature = "coverage-controls")]
#[doc(hidden)]
pub fn validate_frozen_mutation_control(
    root: &Path,
    text: &str,
) -> Result<(), ProductIdentityError> {
    validate_frozen_mutation_sources(root, text)
}

fn validate_evidence_rebinding_sources(
    source_inventory: &str,
    property_runs: &str,
    proof_record: &str,
    derivation_record: &str,
) -> Result<(), ProductIdentityError> {
    for required in [
        "crates/buoy-core/src/algorithms.rs",
        "crates/buoy-reference/src/lib.rs",
        "src/bin/buoy.rs",
        "src/product_identity.rs",
    ] {
        if !source_inventory.contains(required) {
            return Err(ProductIdentityError::new(
                "PID-EVIDENCE-SOURCE-MISSING",
                required,
            ));
        }
    }
    for line in source_inventory.lines().skip(1) {
        let fields = line.split('\t').collect::<Vec<_>>();
        if fields.len() == 7
            && (fields[1].starts_with("crates/spec-provenance-core/")
                || fields[1].starts_with("crates/spec-provenance/"))
            && fields[2] != "compatibility-facade"
        {
            return Err(ProductIdentityError::new(
                "PID-EVIDENCE-STALE-NAME",
                fields[1],
            ));
        }
    }
    let combined = format!("{property_runs}\n{proof_record}\n{derivation_record}");
    for stale in [
        "crates/spec-provenance-core/",
        "crates/spec-provenance-reference/",
        "spec_provenance_reference::",
        "spec-provenance:reference",
        "spec-provenance-core-proofs",
    ] {
        if combined.contains(stale) {
            return Err(ProductIdentityError::new("PID-EVIDENCE-STALE-NAME", stale));
        }
    }
    if !property_runs
        .lines()
        .skip(1)
        .all(|line| line.split('\t').nth(16) == Some("buoy-0.2.0-post-rename"))
    {
        return Err(ProductIdentityError::new(
            "PID-EVIDENCE-STALE-CAMPAIGN",
            "property run identity binding",
        ));
    }
    if !proof_record.contains("path = \"crates/buoy-core/src/algorithms.rs\"") {
        return Err(ProductIdentityError::new(
            "PID-EVIDENCE-PROOF-STALE",
            "proof binding",
        ));
    }
    if !derivation_record.contains("command = \"devenv shell -- buoy-core-proofs\"") {
        return Err(ProductIdentityError::new(
            "PID-EVIDENCE-PROOF-STALE",
            "derivation binding",
        ));
    }
    Ok(())
}

fn validate_evidence_ledger_sources(
    ledger: &str,
    coverage_replay: &str,
) -> Result<(), ProductIdentityError> {
    let mut lines = ledger.lines();
    if lines.next() != Some("schema\tid\tcategory\ttarget\tdisposition\tevidence\trationale") {
        return Err(ProductIdentityError::new(
            "PID-LEDGER-SCHEMA",
            "evidence ledger header",
        ));
    }
    let mut ids = BTreeSet::new();
    let mut coverage_blocked = false;
    for line in lines {
        let fields = line.split('\t').collect::<Vec<_>>();
        if fields.len() != 7
            || fields[0] != "buoy-product-identity-evidence/v1"
            || !ids.insert(fields[1])
        {
            return Err(ProductIdentityError::new("PID-LEDGER-SCHEMA", line));
        }
        if fields[1] == "EXACT-COVERAGE" {
            coverage_blocked = fields[4] == "blocked"
                && fields[5] == "assurance/product-identity/v1/coverage-replay.toml";
        }
    }
    for required in [
        "PID-001",
        "PID-002",
        "PID-003",
        "PID-004",
        "PID-005",
        "PID-006",
        "PID-007",
        "PID-008",
        "NAMING-ROWS",
        "COMPAT-ALIASES",
        "SOURCE-TARGETS",
        "PLATFORM-BLOCKERS",
        "FORMAL-PROOF",
        "HEGEL-CAMPAIGN",
        "FCIS-DYLINT",
        "EXACT-COVERAGE",
        "FUTURE-CAMPAIGNS",
    ] {
        if !ids.contains(required) {
            return Err(ProductIdentityError::new("PID-LEDGER-MISSING", required));
        }
    }
    if !coverage_blocked
        || !coverage_replay.contains("command = \"devenv tasks run buoy:coverage\"")
        || !coverage_replay.contains("identity_binding = \"buoy-0.2.0-post-rename\"")
        || !coverage_replay.contains("status = \"blocked\"")
        || !coverage_replay.contains("blocker = \"LCOV-P0-EXACT-CLOSURE\"")
        || !coverage_replay.contains("covered = 5042")
        || !coverage_replay.contains("count = 7833")
        || !coverage_replay.contains("count = 10692")
        || !coverage_replay.contains("count = 791")
        || !coverage_replay.contains("count = 1434")
    {
        return Err(ProductIdentityError::new(
            "PID-LEDGER-COVERAGE-DISPOSITION",
            "exact coverage replay",
        ));
    }
    Ok(())
}

fn validate_evidence_rebinding(root: &Path) -> Result<(), ProductIdentityError> {
    let read = |path: &str| read_text(root, path, "PID-EVIDENCE-READ");
    validate_evidence_rebinding_sources(
        &read("assurance/v1/source-inventory.tsv")?,
        &read("assurance/v1/property-runs.tsv")?,
        &read("proofs/core/proof-record.toml")?,
        &read("proofs/core/derivation-record.toml")?,
    )?;
    validate_evidence_ledger_sources(
        &read("assurance/product-identity/v1/evidence-ledger.tsv")?,
        &read("assurance/product-identity/v1/coverage-replay.toml")?,
    )
}

fn validate_planning_reconciliation(root: &Path) -> Result<(), ProductIdentityError> {
    let focused_design = read_text(
        root,
        "openspec/changes/archive/2026-07-19-add-cargo-llvm-cov-hegel-rs-testing-requirements/design.md",
        "PID-PLANNING-READ",
    )?;
    let focused_tasks = read_text(
        root,
        "openspec/changes/archive/2026-07-19-add-cargo-llvm-cov-hegel-rs-testing-requirements/tasks.md",
        "PID-PLANNING-READ",
    )?;
    let hardening_tasks = read_text(
        root,
        "openspec/changes/archive/2026-08-02-harden-spec-provenance-production-readiness/tasks.md",
        "PID-PLANNING-READ",
    )?;
    let chelis_design = read_text(
        root,
        "openspec/changes/archive/2026-08-28-integrate-spec-provenance-with-chelis/design.md",
        "PID-PLANNING-READ",
    )?;
    validate_planning_sources(
        &focused_design,
        &focused_tasks,
        &hardening_tasks,
        &chelis_design,
    )
}

fn validate_planning_sources(
    focused_design: &str,
    focused_tasks: &str,
    hardening_tasks: &str,
    chelis_design: &str,
) -> Result<(), ProductIdentityError> {
    for (name, text) in [
        ("focused-design", focused_design),
        ("focused-tasks", focused_tasks),
        ("hardening-tasks", hardening_tasks),
        ("chelis-design", chelis_design),
    ] {
        if [
            "Spec Provenance",
            "spec-provenance:coverage",
            "spec-provenance:properties",
            "spec-provenance-core",
            "spec_provenance_core",
        ]
        .iter()
        .any(|old_primary| text.contains(old_primary))
        {
            return Err(ProductIdentityError::new("PID-PLANNING-OLD-PRIMARY", name));
        }
    }
    if (
        focused_design.contains("map once to hardening tasks 2.10, 2.19, 2.21, and 2.22"),
        hardening_tasks.contains("maps to one P0 ledger entry"),
    ) != (true, true)
    {
        return Err(ProductIdentityError::new(
            "PID-PLANNING-MAPPING-MISSING",
            "focused P0 mapping",
        ));
    }
    let focused_all_done = !focused_tasks.lines().any(|line| line.starts_with("- [ ] "));
    for task in ["2.10", "2.19", "2.21", "2.22"] {
        let pending = hardening_tasks.contains(&format!("- [ ] {task} "));
        let complete = hardening_tasks.contains(&format!("- [x] {task} "));
        if !pending && !complete {
            return Err(ProductIdentityError::new("PID-PLANNING-TASK-MISSING", task));
        }
        if complete && !focused_all_done {
            return Err(ProductIdentityError::new(
                "PID-PLANNING-FALSE-COMPLETION",
                task,
            ));
        }
    }
    if (
        focused_tasks.contains("devenv tasks run buoy:coverage"),
        focused_tasks.contains("devenv tasks run buoy:properties"),
        chelis_design.contains("published Buoy shell/core interfaces"),
    ) != (true, true, true)
    {
        return Err(ProductIdentityError::new(
            "PID-PLANNING-BUOY-TARGET-MISSING",
            "focused or Chelis plan",
        ));
    }
    Ok(())
}

/// Validates injected planning sources without filesystem substitution.
#[cfg(feature = "coverage-controls")]
#[doc(hidden)]
pub fn validate_planning_control(
    focused_design: &str,
    focused_tasks: &str,
    hardening_tasks: &str,
    chelis_design: &str,
) -> Result<(), ProductIdentityError> {
    validate_planning_sources(
        focused_design,
        focused_tasks,
        hardening_tasks,
        chelis_design,
    )
}

fn validate_fixture_manifest(text: &str) -> Result<(), ProductIdentityError> {
    let mut lines = text.lines();
    if lines.next()
        != Some(
            "schema\tid\trequirement\tscenario\tpolarity\toperation\tmutation\texpected_diagnostic\texpected_count\tfocused_command\tfinal_oracle",
        )
    {
        return Err(ProductIdentityError::new(
            "PID-FIXTURE-SCHEMA",
            "invalid header",
        ));
    }
    let mut ids = BTreeSet::new();
    let mut scenarios = BTreeSet::new();
    for line in lines {
        let fields = line.split('\t').collect::<Vec<_>>();
        if fields.len() != 11 || fields[0] != FIXTURE_SCHEMA {
            return Err(ProductIdentityError::new("PID-FIXTURE-SCHEMA", line));
        }
        if !ids.insert(fields[1].to_owned()) || !scenarios.insert((fields[2], fields[3])) {
            return Err(ProductIdentityError::new(
                "PID-FIXTURE-DUPLICATE",
                fields[1],
            ));
        }
        let actual = evaluate_fixture(fields[5])?;
        let expected = if fields[7] == "-" {
            None
        } else {
            Some(fields[7])
        };
        let expected_count = fields[8]
            .parse::<usize>()
            .map_err(|_| ProductIdentityError::new("PID-FIXTURE-CARDINALITY", fields[8]))?;
        let actual_count = usize::from(actual.is_some());
        if actual != expected || actual_count != expected_count {
            return Err(ProductIdentityError::new(
                "PID-FIXTURE-STALE-EXPECTED",
                fields[1],
            ));
        }
        if fields[6] == "zero-target" || fields[6] == "multiple-target" || fields[6] == "no-op" {
            return Err(ProductIdentityError::new(
                "PID-FIXTURE-MUTATION-CARDINALITY",
                fields[1],
            ));
        }
        if !matches!(fields[6], "none" | "single-reviewed-target") {
            return Err(ProductIdentityError::new(
                "PID-FIXTURE-UNKNOWN-MUTATION",
                fields[6],
            ));
        }
        if fields[9] != "devenv tasks run buoy:product-identity"
            || fields[10] != "devenv test --no-tui"
        {
            return Err(ProductIdentityError::new(
                "PID-FIXTURE-SUBSTITUTED-AUTHORITY",
                fields[1],
            ));
        }
    }
    for requirement in 1..=8 {
        let id = format!("PID-{requirement:03}");
        let count = scenarios
            .iter()
            .filter(|(candidate, _)| **candidate == id)
            .count();
        if count < 2 {
            return Err(ProductIdentityError::new(
                "PID-FIXTURE-SCENARIO-MISSING",
                id,
            ));
        }
    }
    Ok(())
}

fn execution_error(error: String) -> ProductIdentityError {
    ProductIdentityError::new("PID-EXECUTION-EVIDENCE", error)
}

/// Bind a set of local authority paths for execution evidence, wrapping the
/// shared library's error into the stable product-identity diagnostic.
///
/// Extracted from [`render_fixture_execution_evidence_bound_to`] so the
/// binding failure path can be exercised directly against a genuinely
/// missing path without disturbing the real tool/configuration authorities
/// it is ordinarily bound against.
fn execution_tool_binding_digest(paths: &[&str]) -> Result<String, ProductIdentityError> {
    execution_evidence::path_set_binding(paths).map_err(execution_error)
}

/// Build one fixture-operation or fixture-mutation execution-evidence
/// record, wrapping the shared library's error into the stable
/// product-identity diagnostic.
///
/// Extracted from [`render_fixture_executions`] so the binding failure path
/// (an invalid tool/configuration digest) can be exercised directly with a
/// deliberately malformed digest.
#[allow(clippy::too_many_arguments)]
fn fixture_execution_record(
    category: &str,
    id: String,
    line: &str,
    tool_paths: &str,
    tool_digest: &str,
    configuration_paths: &str,
    configuration_digest: &str,
    result: String,
) -> Result<execution_evidence::Record, ProductIdentityError> {
    execution_evidence::passing_with_bindings(
        category,
        id,
        FIXTURE_MANIFEST,
        line.as_bytes(),
        tool_paths,
        tool_digest,
        configuration_paths,
        configuration_digest,
        result,
    )
    .map_err(execution_error)
}

/// Render the sorted, deterministic execution-evidence text for every row of
/// an already-parsed fixture manifest.
///
/// Extracted from [`render_fixture_execution_evidence_bound_to`] so the
/// duplicate-record rendering failure path (unreachable through the real
/// frozen manifest, which [`validate_fixture_manifest`] already rejects for
/// duplicate case identifiers before this function ever runs) can be
/// exercised directly and honestly with a constructed input.
fn render_fixture_executions(
    text: &str,
    tool_paths: &str,
    tool_digest: &str,
    configuration_paths: &str,
    configuration_digest: &str,
) -> Result<String, ProductIdentityError> {
    let mut records = Vec::new();
    for (index, line) in text.lines().skip(1).enumerate() {
        let fields = line.split('\t').collect::<Vec<_>>();
        let actual = evaluate_fixture(fields[5])?;
        let observed = actual.unwrap_or("-");
        let result = format!(
            "operation={};polarity={};diagnostic={observed};count={}",
            fields[5],
            fields[4],
            usize::from(actual.is_some())
        );
        records.push(fixture_execution_record(
            "fixture-operation",
            format!("{FIXTURE_MANIFEST}#{}#{}", fields[1], fields[5]),
            line,
            tool_paths,
            tool_digest,
            configuration_paths,
            configuration_digest,
            result,
        )?);
        if fields[6] != "none" {
            records.push(fixture_execution_record(
                "fixture-mutation",
                format!("{FIXTURE_MANIFEST}#{}#{index}:{}", fields[1], fields[6]),
                line,
                tool_paths,
                tool_digest,
                configuration_paths,
                configuration_digest,
                format!(
                    "mutation={};target-cardinality=1;diagnostic={observed};count={}",
                    fields[6],
                    usize::from(actual.is_some())
                ),
            )?);
        }
    }
    execution_evidence::render(&records).map_err(execution_error)
}

fn render_fixture_execution_evidence(root: &Path) -> Result<String, ProductIdentityError> {
    render_fixture_execution_evidence_bound_to(
        root,
        &EXECUTION_TOOL_PATHS,
        &EXECUTION_CONFIGURATION_PATHS,
    )
}

/// Core of [`render_fixture_execution_evidence`], parameterized over the
/// tool/configuration authority paths so the otherwise-unreachable binding
/// failure paths (the real constants always resolve against this repository
/// checkout's own real sources) can be exercised directly and honestly with
/// a deliberately invalid path, without disturbing any real repository file.
fn render_fixture_execution_evidence_bound_to(
    root: &Path,
    tool_authority_paths: &[&str],
    configuration_authority_paths: &[&str],
) -> Result<String, ProductIdentityError> {
    let text = read_text(root, FIXTURE_MANIFEST, "PID-FIXTURE-READ")?;
    validate_fixture_manifest(&text)?;
    let tool_paths = tool_authority_paths.join(";");
    let tool_digest = execution_tool_binding_digest(tool_authority_paths)?;
    let configuration_paths = configuration_authority_paths.join(";");
    let configuration_digest = execution_tool_binding_digest(configuration_authority_paths)?;
    render_fixture_executions(
        &text,
        &tool_paths,
        &tool_digest,
        &configuration_paths,
        &configuration_digest,
    )
}

/// Validate or rewrite the exact per-case product-identity execution artifact.
///
/// # Errors
///
/// Returns a stable diagnostic when a case fails or any source, tool,
/// configuration, or result binding is stale.
pub fn fixture_execution_evidence(root: &Path, write: bool) -> Result<(), ProductIdentityError> {
    let expected = render_fixture_execution_evidence(root)?;
    let path = root.join(FIXTURE_EXECUTIONS);
    execution_evidence::validate_materialized(&path.to_string_lossy(), &expected, write)
        .map_err(execution_error)
}

/// Exercise every read, parse, and binding failure that
/// [`render_fixture_execution_evidence`] can encounter, using an isolated
/// scratch root so the real committed fixtures are never disturbed.
///
/// `fixture_text` is the already-read, real committed fixture manifest (so
/// this control never needs a second, redundant read of `root`), and
/// `fault` injects one reviewed scratch filesystem failure, mirroring the
/// other scratch controls in [`self_test_inner`].
#[allow(clippy::too_many_lines)]
fn execution_evidence_coverage_controls(
    fixture_text: &str,
    fault: Option<&str>,
    assertions: &mut SelfTestAssertions,
) -> Result<(), ProductIdentityError> {
    let scratch = std::env::temp_dir().join(format!(
        "buoy-product-identity-execution-evidence-controls-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&scratch);
    let create_result = if fault == Some("execution-evidence-scratch-create") {
        Err(std::io::Error::other(
            "injected execution-evidence scratch creation failure",
        ))
    } else {
        fs::create_dir_all(&scratch)
    };
    self_test_io(create_result)?;

    // Neither the fixture manifest nor any authority exists yet; go through
    // the public checked/written wrapper so its own propagated read-failure
    // path is exercised too.
    assertions.record(
        fixture_execution_evidence(&scratch, false).is_err(),
        String::from("missing fixture manifest execution evidence unexpectedly passed"),
    );

    let fixture_directory_result = if fault == Some("execution-evidence-fixture-directory") {
        Err(std::io::Error::other(
            "injected execution-evidence fixture directory failure",
        ))
    } else {
        fs::create_dir_all(scratch.join("fixtures/product-identity/v1"))
    };
    self_test_io(fixture_directory_result)?;

    let invalid_manifest_result = if fault == Some("execution-evidence-invalid-manifest-write") {
        Err(std::io::Error::other(
            "injected execution-evidence invalid-manifest write failure",
        ))
    } else {
        fs::write(scratch.join(FIXTURE_MANIFEST), "invalid\n")
    };
    self_test_io(invalid_manifest_result)?;
    // The fixture manifest exists but does not parse.
    assertions.record(
        render_fixture_execution_evidence(&scratch).is_err(),
        String::from("unparsable fixture manifest execution evidence unexpectedly passed"),
    );

    let real_manifest_write_result = if fault == Some("execution-evidence-real-manifest-write") {
        Err(std::io::Error::other(
            "injected execution-evidence real-manifest write failure",
        ))
    } else {
        fs::write(scratch.join(FIXTURE_MANIFEST), fixture_text)
    };
    self_test_io(real_manifest_write_result)?;
    // Every authority the real tool/configuration constants name is a real,
    // checked-out repository path (not a scratch-relative one), so once the
    // scratch manifest itself parses, the full render succeeds.
    let rendered = if fault == Some("execution-evidence-full-render") {
        Err(execution_error(String::from(
            "injected execution-evidence full-render failure",
        )))
    } else {
        render_fixture_execution_evidence(&scratch)
    }?;
    assertions.record(
        rendered.starts_with(execution_evidence::HEADER),
        String::from("full scratch fixture execution evidence render lost its frozen header"),
    );

    assertions.record(
        render_fixture_execution_evidence_bound_to(
            &scratch,
            &["/nonexistent/execution-evidence-product-identity-coverage-tool"],
            &EXECUTION_CONFIGURATION_PATHS,
        )
        .is_err(),
        String::from("missing tool authority path execution evidence unexpectedly passed"),
    );
    assertions.record(
        render_fixture_execution_evidence_bound_to(
            &scratch,
            &EXECUTION_TOOL_PATHS,
            &["/nonexistent/execution-evidence-product-identity-coverage-configuration"],
        )
        .is_err(),
        String::from("missing configuration authority path execution evidence unexpectedly passed"),
    );

    assertions.record(
        fixture_execution_record(
            "fixture-operation",
            String::from("synthetic-record"),
            "synthetic-line",
            "Cargo.toml",
            "",
            "Cargo.toml",
            "sha-256:deadbeef",
            String::from("result=synthetic"),
        )
        .is_err(),
        String::from("malformed tool digest execution-evidence record unexpectedly passed"),
    );

    // Every row-rendering failure below is exercised through
    // `render_fixture_executions` directly (bypassing
    // `validate_fixture_manifest`) with synthetic, dummy-authority bindings:
    // `EXECUTION_TOOL_PATHS`/`EXECUTION_CONFIGURATION_PATHS` name real
    // repository files (so `validate_paths` accepts them), and the digest
    // fields are never independently checked against real content, only for
    // being nonempty single-line text.
    let real_tool_paths = EXECUTION_TOOL_PATHS.join(";");
    let real_configuration_paths = EXECUTION_CONFIGURATION_PATHS.join(";");
    let dummy_digest = "sha-256:deadbeef";

    let unknown_operation_row = String::from(
        "buoy-product-identity-fixtures/v1\tPID-EVIDENCE-CONTROL\tPID-EVIDENCE-CONTROL\t\
         synthetic-scenario\tpositive\ttotally-unknown-operation\tnone\t-\t0\t\
         devenv tasks run buoy:product-identity\tdevenv test --no-tui",
    );
    assertions.record(
        render_fixture_executions(
            &format!("header\n{unknown_operation_row}\n"),
            &real_tool_paths,
            dummy_digest,
            &real_configuration_paths,
            dummy_digest,
        )
        .is_err(),
        String::from("unknown-operation execution-evidence row unexpectedly rendered"),
    );

    let single_operation_row = String::from(
        "buoy-product-identity-fixtures/v1\tPID-EVIDENCE-CONTROL\tPID-EVIDENCE-CONTROL\t\
         synthetic-scenario\tpositive\tprimary-identities-pass\tnone\t-\t0\t\
         devenv tasks run buoy:product-identity\tdevenv test --no-tui",
    );
    assertions.record(
        render_fixture_executions(
            &format!("header\n{single_operation_row}\n"),
            &real_tool_paths,
            "",
            &real_configuration_paths,
            dummy_digest,
        )
        .is_err(),
        String::from(
            "malformed tool digest operation execution-evidence row unexpectedly rendered",
        ),
    );

    // A mutation name carrying an embedded, non-line-terminating carriage
    // return keeps this one line intact under `str::lines`, but makes the
    // rendered mutation record's id/result invalid, while the operation
    // record on the very same row (which never interpolates the mutation
    // field) still renders successfully.
    let carriage_return_mutation_row = String::from(
        "buoy-product-identity-fixtures/v1\tPID-EVIDENCE-CONTROL\tPID-EVIDENCE-CONTROL\t\
         synthetic-scenario\tpositive\tprimary-identities-pass\tsynthetic\rmutation\t-\t0\t\
         devenv tasks run buoy:product-identity\tdevenv test --no-tui",
    );
    assertions.record(
        render_fixture_executions(
            &format!("header\n{carriage_return_mutation_row}\n"),
            &real_tool_paths,
            dummy_digest,
            &real_configuration_paths,
            dummy_digest,
        )
        .is_err(),
        String::from("malformed mutation execution-evidence row unexpectedly rendered"),
    );

    let header = fixture_text.lines().next().unwrap_or_default();
    let first_row = fixture_text.lines().nth(1).unwrap_or_default();
    let duplicated = format!("{header}\n{first_row}\n{first_row}\n");
    assertions.record(
        render_fixture_executions(
            &duplicated,
            &real_tool_paths,
            dummy_digest,
            &real_configuration_paths,
            dummy_digest,
        )
        .is_err(),
        String::from("duplicate execution-evidence record unexpectedly rendered"),
    );

    let cleanup_result = if fault == Some("execution-evidence-scratch-cleanup") {
        Err(std::io::Error::other(
            "injected execution-evidence scratch cleanup failure",
        ))
    } else {
        fs::remove_dir_all(&scratch)
    };
    self_test_io(cleanup_result)
}

fn self_test_io(result: std::io::Result<()>) -> Result<(), ProductIdentityError> {
    result.map_err(|error| ProductIdentityError::new("PID-SELF-TEST", error.to_string()))
}

#[derive(Default)]
struct SelfTestAssertions {
    first_failure: Option<ProductIdentityError>,
}

impl SelfTestAssertions {
    fn record(&mut self, passed: bool, detail: String) {
        if !passed && self.first_failure.is_none() {
            self.first_failure = Some(ProductIdentityError::new("PID-SELF-TEST", detail));
        }
    }

    fn record_code(
        &mut self,
        actual: Option<&'static str>,
        expected: &'static str,
        detail: String,
    ) {
        self.record(actual == Some(expected), detail);
    }

    fn finish(self) -> Result<(), ProductIdentityError> {
        self.first_failure.map_or(Ok(()), Err)
    }
}

/// Runs deterministic positive and negative controls for the checker itself.
///
/// # Errors
///
/// Returns the first stable diagnostic when a positive fixture fails or a negative fixture passes.
#[allow(clippy::too_many_lines)]
pub fn self_test(root: &Path) -> Result<(), ProductIdentityError> {
    self_test_inner(root, None)
}

#[cfg(feature = "coverage-controls")]
#[doc(hidden)]
/// Injects one reviewed self-test filesystem failure.
pub fn self_test_fault_control(root: &Path, fault: &str) -> Result<(), ProductIdentityError> {
    self_test_inner(root, Some(fault))
}

#[allow(clippy::too_many_lines)]
fn self_test_inner(root: &Path, fault: Option<&str>) -> Result<(), ProductIdentityError> {
    let mut assertion_control = SelfTestAssertions::default();
    assertion_control.record_code(
        Some("PID-CONTROL"),
        "PID-CONTROL",
        String::from("ignored passing code control"),
    );
    assertion_control.record(true, String::from("ignored passing control"));
    assertion_control.record(false, String::from("injected assertion failure"));
    assertion_control.record_code(None, "PID-CONTROL", String::from("preserve first failure"));
    let _ = assertion_control.finish();
    let _ = self_test_io(Err(std::io::Error::other("injected I/O failure")));
    let mut assertions = SelfTestAssertions::default();
    assertions.record(
        decode_hex(b"00 ff\n", true) == Some(vec![0, 255]),
        String::from("whitespace-tolerant hex decoding failed"),
    );
    for malformed_hex in [b"".as_slice(), b"0", b"0g", b"g0"] {
        assertions.record(
            decode_hex(malformed_hex, false).is_none(),
            String::from("malformed hex decoding unexpectedly passed"),
        );
    }

    let fixture_text = read_text(root, FIXTURE_MANIFEST, "PID-FIXTURE-READ")?;
    validate_fixture_manifest(&fixture_text)?;
    let frozen_text = read_text(root, FROZEN_MUTATIONS, "PID-FROZEN-READ")?;
    validate_frozen_mutation_sources(root, &frozen_text)?;
    let frozen_header = frozen_text.lines().next().unwrap_or_default();
    let frozen_row = frozen_text.lines().nth(1).unwrap_or_default();
    let mutate_frozen_row = |index: usize, replacement: &str| {
        let mut fields = frozen_row.split('\t').collect::<Vec<_>>();
        fields[index] = replacement;
        format!("{frozen_header}\n{}\n", fields.join("\t"))
    };
    assertions.record_code(
        validate_corpus(root, "not-the-frozen-corpus")
            .err()
            .map(|error| error.code()),
        "PID-CANONICAL-PARITY",
        String::from("modified corpus was accepted"),
    );
    for (text, expected) in [
        (String::new(), "PID-FROZEN-SCHEMA"),
        (format!("{frozen_header}\ninvalid\n"), "PID-FROZEN-SCHEMA"),
        (
            mutate_frozen_row(0, "wrong-frozen-schema/v1"),
            "PID-FROZEN-SCHEMA",
        ),
        (
            format!("{frozen_header}\n{frozen_row}\n{frozen_row}\n"),
            "PID-FROZEN-DUPLICATE",
        ),
        (mutate_frozen_row(4, "buoy"), "PID-FROZEN-VECTOR"),
        (mutate_frozen_row(5, "spec-provenance"), "PID-FROZEN-VECTOR"),
        (mutate_frozen_row(6, "wrong"), "PID-FROZEN-VECTOR"),
        (
            mutate_frozen_row(3, "missing-frozen-source"),
            "PID-FROZEN-READ",
        ),
        (
            mutate_frozen_row(4, "spec-provenance-missing-control"),
            "PID-FROZEN-IDENTIFIER-MISSING",
        ),
        (
            format!("{frozen_header}\n{frozen_row}\n"),
            "PID-FROZEN-VECTOR-MISSING",
        ),
    ] {
        assertions.record_code(
            validate_frozen_mutation_sources(root, &text)
                .err()
                .map(|error| error.code()),
            expected,
            format!("frozen mutation control did not return {expected}"),
        );
    }

    let positive = read_text(
        root,
        "fixtures/product-identity/v1/positive/naming-inventory.tsv",
        "PID-FIXTURE-READ",
    )?;
    let positive_rows = parse_inventory(&positive)?;
    let rendered_positive = render_inventory(&positive_rows);
    assertions.record(
        parse_inventory(&rendered_positive)
            .is_ok_and(|rows| render_inventory(&rows) == rendered_positive),
        String::from("positive inventory did not round trip"),
    );
    let scanned_rows = inventory_rows(scan_repository(root)?);
    let rendered_scan = render_inventory(&scanned_rows);
    assertions.record(
        parse_inventory(&rendered_scan).is_ok_and(|rows| render_inventory(&rows) == rendered_scan),
        String::from("repository inventory did not round trip"),
    );
    let dispositions = positive_rows
        .iter()
        .map(|row| row.disposition)
        .collect::<BTreeSet<_>>();
    assertions.record(
        dispositions.len() == 5,
        String::from("positive inventory does not cover every disposition"),
    );
    let header = positive.lines().next().unwrap_or_default();
    let first_row = positive.lines().nth(1).unwrap_or_default();
    let mutate_row = |index: usize, replacement: &str| {
        let mut fields = first_row.split('\t').collect::<Vec<_>>();
        fields[index] = replacement;
        format!("{header}\n{}\n", fields.join("\t"))
    };
    for (text, expected) in [
        (String::new(), "PID-INVENTORY-SCHEMA"),
        (format!("{header}\ninvalid\n"), "PID-INVENTORY-SCHEMA"),
        (
            mutate_row(0, "wrong-inventory-schema/v1"),
            "PID-INVENTORY-SCHEMA",
        ),
        (mutate_row(2, "invalid/"), "PID-INVENTORY-BROAD-EXCLUSION"),
        (mutate_row(3, "not-a-line"), "PID-INVENTORY-LOCATION"),
        (mutate_row(4, "not-a-column"), "PID-INVENTORY-LOCATION"),
        (mutate_row(8, "unknown"), "PID-INVENTORY-DISPOSITION"),
    ] {
        assertions.record_code(
            parse_inventory(&text).err().map(|error| error.code()),
            expected,
            format!("inventory parser control did not return {expected}"),
        );
    }
    let mut duplicate_location_fields = first_row.split('\t').collect::<Vec<_>>();
    duplicate_location_fields[1] = "OLD-NAME-UNIQUE";
    let duplicate_location = format!(
        "{header}\n{first_row}\n{}\n",
        duplicate_location_fields.join("\t")
    );
    assertions.record_code(
        parse_inventory(&duplicate_location)
            .err()
            .map(|error| error.code()),
        "PID-INVENTORY-DUPLICATE",
        String::from("duplicate inventory location was accepted"),
    );

    for (path, expected) in [
        (
            "fixtures/product-identity/v1/negative/duplicate.tsv",
            "PID-INVENTORY-DUPLICATE",
        ),
        (
            "fixtures/product-identity/v1/negative/broad-exclusion.tsv",
            "PID-INVENTORY-BROAD-EXCLUSION",
        ),
    ] {
        let text = read_text(root, path, "PID-FIXTURE-READ")?;
        assertions.record_code(
            parse_inventory(&text).err().map(|error| error.code()),
            expected,
            format!("negative inventory unexpectedly accepted: {path}"),
        );
    }

    let wrong_class = read_text(
        root,
        "fixtures/product-identity/v1/negative/wrong-class.tsv",
        "PID-FIXTURE-READ",
    )?;
    let wrong_rows = parse_inventory(&wrong_class)?;
    assertions.record(
        !wrong_rows.iter().all(|row| {
            let mut occurrence = row.occurrence.clone();
            occurrence.context = occurrence.lexeme.clone();
            classify(&occurrence).0 == row.disposition
        }),
        String::from("wrong-class inventory unexpectedly matched classification"),
    );

    let modified = read_text(
        root,
        "fixtures/product-identity/v1/negative/modified-frozen.tsv",
        "PID-FIXTURE-READ",
    )?;
    let modified_rows = parse_inventory(&modified)?;
    assertions.record(
        !modified_rows.iter().all(|row| {
            row.occurrence
                .lexeme
                .to_ascii_lowercase()
                .starts_with("spec-provenance")
        }),
        String::from("modified frozen identifier unexpectedly retained old bytes"),
    );

    let missing = read_text(
        root,
        "fixtures/product-identity/v1/negative/missing.tsv",
        "PID-FIXTURE-READ",
    )?;
    assertions.record(
        parse_inventory(&missing)?.is_empty(),
        String::from("missing inventory fixture unexpectedly contains a row"),
    );

    let expired = read_text(
        root,
        "fixtures/product-identity/v1/negative/expired-alias.tsv",
        "PID-FIXTURE-READ",
    )?;
    let expired = format!("invalid\n{expired}");
    assertions.record(
        expired.lines().any(|line| {
            let fields = line.split('\t').collect::<Vec<_>>();
            fields.len() == 5
                && fields[0] == "buoy-compatibility-alias/v1"
                && version_at_least(fields[3], fields[4])
        }),
        String::from("expired alias fixture did not exercise expiry"),
    );

    assertions.record_code(
        evaluate_fixture("unknown-operation")
            .err()
            .map(|error| error.code()),
        "PID-FIXTURE-UNKNOWN-OPERATION",
        String::from("unknown operation was accepted"),
    );
    for operation in [
        "primary-package-stale",
        "primary-core-package-stale",
        "primary-binary-stale",
    ] {
        assertions.record(
            evaluate_fixture(operation) == Ok(Some("PID-STALE-PRODUCT-IDENTITY")),
            format!("{operation} was accepted"),
        );
    }
    assertions.record(
        evaluate_fixture("chelis-core-independent") == Ok(None),
        String::from("independent Chelis core was rejected"),
    );
    for mutation in ["zero-target", "multiple-target", "no-op"] {
        let mutated = fixture_text.replacen("\tnone\t", &format!("\t{mutation}\t"), 1);
        assertions.record_code(
            validate_fixture_manifest(&mutated)
                .err()
                .map(|error| error.code()),
            "PID-FIXTURE-MUTATION-CARDINALITY",
            format!("{mutation} mutation was accepted"),
        );
    }
    let fixture_header = fixture_text.lines().next().unwrap_or_default();
    let fixture_row = fixture_text.lines().nth(1).unwrap_or_default();
    for (text, expected) in [
        (String::new(), "PID-FIXTURE-SCHEMA"),
        (format!("{fixture_header}\ninvalid\n"), "PID-FIXTURE-SCHEMA"),
        (
            fixture_text.replacen(FIXTURE_SCHEMA, "wrong-fixture-schema/v1", 1),
            "PID-FIXTURE-SCHEMA",
        ),
        (
            format!("{fixture_header}\n{fixture_row}\n{fixture_row}\n"),
            "PID-FIXTURE-DUPLICATE",
        ),
        (
            format!(
                "{fixture_header}\n{fixture_row}\n{}\n",
                fixture_row.replacen("PID-001-POS", "PID-UNIQUE", 1)
            ),
            "PID-FIXTURE-DUPLICATE",
        ),
        (
            fixture_text.replacen("\tprimary-identities-pass\t", "\tunknown-operation\t", 1),
            "PID-FIXTURE-UNKNOWN-OPERATION",
        ),
        (
            fixture_text.replacen("\tnone\t", "\tunknown-mutation\t", 1),
            "PID-FIXTURE-UNKNOWN-MUTATION",
        ),
        (
            fixture_text.replacen("\t-\t0\t", "\t-\tnot-a-count\t", 1),
            "PID-FIXTURE-CARDINALITY",
        ),
    ] {
        assertions.record_code(
            validate_fixture_manifest(&text)
                .err()
                .map(|error| error.code()),
            expected,
            format!("fixture parser control did not return {expected}"),
        );
    }
    let stale_count = fixture_text.replacen("\t-\t0\t", "\t-\t1\t", 1);
    assertions.record_code(
        validate_fixture_manifest(&stale_count)
            .err()
            .map(|error| error.code()),
        "PID-FIXTURE-STALE-EXPECTED",
        String::from("stale expected count was accepted"),
    );
    let stale_expected = fixture_text.replacen("\t-\t0\t", "\tPID-WRONG\t1\t", 1);
    assertions.record_code(
        validate_fixture_manifest(&stale_expected)
            .err()
            .map(|error| error.code()),
        "PID-FIXTURE-STALE-EXPECTED",
        String::from("stale expected result was accepted"),
    );
    for substituted in [
        fixture_text.replacen(
            "devenv tasks run buoy:product-identity",
            "devenv tasks run substituted:authority",
            1,
        ),
        fixture_text.replacen(
            "devenv test --no-tui",
            "devenv test substituted:authority",
            1,
        ),
    ] {
        assertions.record_code(
            validate_fixture_manifest(&substituted)
                .err()
                .map(|error| error.code()),
            "PID-FIXTURE-SUBSTITUTED-AUTHORITY",
            String::from("substituted fixture authority was accepted"),
        );
    }
    assertions.record_code(
        validate_fixture_manifest(&format!("{fixture_header}\n{fixture_row}\n"))
            .err()
            .map(|error| error.code()),
        "PID-FIXTURE-SCENARIO-MISSING",
        String::from("incomplete fixture scenarios were accepted"),
    );
    assertions.record(
        validate_synthetic_inventory(false, true, false) == Some("PID-INVENTORY-DUPLICATE"),
        String::from("duplicate inventory control returned the wrong diagnostic"),
    );
    assertions.record(
        validate_synthetic_inventory(false, false, true) == Some("PID-INVENTORY-BROAD-EXCLUSION"),
        String::from("broad inventory control returned the wrong diagnostic"),
    );

    let source_inventory = read_text(
        root,
        "assurance/v1/source-inventory.tsv",
        "PID-EVIDENCE-READ",
    )?;
    let property_runs = read_text(root, "assurance/v1/property-runs.tsv", "PID-EVIDENCE-READ")?;
    let proof_record = read_text(root, "proofs/core/proof-record.toml", "PID-EVIDENCE-READ")?;
    let derivation_record = read_text(
        root,
        "proofs/core/derivation-record.toml",
        "PID-EVIDENCE-READ",
    )?;
    let stale_source =
        source_inventory.replacen("crates/buoy-core/", "crates/spec-provenance-core/", 1);
    let stale_campaign = property_runs.replacen(
        "buoy-0.2.0-post-rename",
        "spec-provenance-0.1.0-pre-rename",
        1,
    );
    let stale_proof = proof_record.replacen("crates/buoy-core/", "crates/spec-provenance-core/", 1);
    let stale_command =
        derivation_record.replacen("buoy-core-proofs", "spec-provenance-core-proofs", 1);
    for result in [
        validate_evidence_rebinding_sources(
            &source_inventory,
            &property_runs,
            &proof_record.replace(
                "path = \"crates/buoy-core/src/algorithms.rs\"",
                "path = \"missing-proof-source\"",
            ),
            &derivation_record,
        ),
        validate_evidence_rebinding_sources(
            &source_inventory,
            &property_runs,
            &proof_record,
            &derivation_record.replace(
                "command = \"devenv shell -- buoy-core-proofs\"",
                "command = \"missing-proof-command\"",
            ),
        ),
    ] {
        assertions.record(
            result.is_err(),
            String::from("missing proof binding was accepted"),
        );
    }
    for required in [
        "crates/buoy-core/src/algorithms.rs",
        "crates/buoy-reference/src/lib.rs",
        "src/bin/buoy.rs",
        "src/product_identity.rs",
    ] {
        let missing_required = source_inventory.replace(required, "missing-required-source");
        assertions.record(
            validate_evidence_rebinding_sources(
                &missing_required,
                &property_runs,
                &proof_record,
                &derivation_record,
            )
            .is_err(),
            format!("missing evidence source {required} was accepted"),
        );
    }
    for stale in [
        "crates/spec-provenance-core/",
        "crates/spec-provenance-reference/",
        "spec_provenance_reference::",
        "spec-provenance:reference",
        "spec-provenance-core-proofs",
    ] {
        assertions.record(
            validate_evidence_rebinding_sources(
                &source_inventory,
                &property_runs,
                &format!("{proof_record}\n{stale}"),
                &derivation_record,
            )
            .is_err(),
            format!("stale evidence binding {stale} was accepted"),
        );
    }
    let stale_source_package = format!(
        "{source_inventory}\nbuoy-source-inventory/v1\tcrates/spec-provenance/legacy.rs\tproduction-source\tworkspace-cargo-llvm-cov\tportable\troot-package\tcontrol\n"
    );
    assertions.record(
        validate_evidence_rebinding_sources(
            &stale_source_package,
            &property_runs,
            &proof_record,
            &derivation_record,
        )
        .is_err(),
        String::from("stale package source was accepted"),
    );
    let compatible_source = format!(
        "{source_inventory}\nbuoy-source-inventory/v1\tcrates/spec-provenance/legacy.rs\tcompatibility-facade\tworkspace-cargo-llvm-cov\tportable\troot-package\tcontrol\n"
    );
    let _ = validate_evidence_rebinding_sources(
        &compatible_source,
        &property_runs,
        &proof_record,
        &derivation_record,
    );
    let stale_property_identity = property_runs.replacen("buoy-0.2.0-post-rename", "wrong", 1);
    assertions.record(
        validate_evidence_rebinding_sources(
            &source_inventory,
            &stale_property_identity,
            &proof_record,
            &derivation_record,
        )
        .is_err(),
        String::from("stale property identity was accepted"),
    );

    for result in [
        validate_evidence_rebinding_sources(
            &stale_source,
            &property_runs,
            &proof_record,
            &derivation_record,
        ),
        validate_evidence_rebinding_sources(
            &source_inventory,
            &stale_campaign,
            &proof_record,
            &derivation_record,
        ),
        validate_evidence_rebinding_sources(
            &source_inventory,
            &property_runs,
            &stale_proof,
            &derivation_record,
        ),
        validate_evidence_rebinding_sources(
            &source_inventory,
            &property_runs,
            &proof_record,
            &stale_command,
        ),
    ] {
        assertions.record(
            result.is_err(),
            String::from("stale rename evidence was accepted"),
        );
    }
    let evidence_ledger = read_text(
        root,
        "assurance/product-identity/v1/evidence-ledger.tsv",
        "PID-EVIDENCE-READ",
    )?;
    let coverage_replay = read_text(
        root,
        "assurance/product-identity/v1/coverage-replay.toml",
        "PID-EVIDENCE-READ",
    )?;
    validate_evidence_ledger_sources(&evidence_ledger, &coverage_replay)?;
    let ledger_header = evidence_ledger.lines().next().unwrap_or_default();
    let ledger_row = evidence_ledger.lines().nth(1).unwrap_or_default();
    let missing_ledger_row = evidence_ledger
        .lines()
        .filter(|line| !line.contains("\tPID-001\t"))
        .collect::<Vec<_>>()
        .join("\n");
    for text in [
        String::new(),
        format!("{ledger_header}\ninvalid\n"),
        format!("{ledger_header}\n{ledger_row}\n{ledger_row}\n"),
        evidence_ledger.replacen(
            "buoy-product-identity-evidence/v1",
            "wrong-evidence-schema/v1",
            1,
        ),
        missing_ledger_row,
    ] {
        assertions.record(
            validate_evidence_ledger_sources(&text, &coverage_replay).is_err(),
            String::from("malformed evidence ledger was accepted"),
        );
    }
    for required in [
        "PID-001",
        "PID-002",
        "PID-003",
        "PID-004",
        "PID-005",
        "PID-006",
        "PID-007",
        "PID-008",
        "NAMING-ROWS",
        "COMPAT-ALIASES",
        "SOURCE-TARGETS",
        "PLATFORM-BLOCKERS",
        "FORMAL-PROOF",
        "HEGEL-CAMPAIGN",
        "FCIS-DYLINT",
        "EXACT-COVERAGE",
        "FUTURE-CAMPAIGNS",
    ] {
        let missing_required = evidence_ledger
            .lines()
            .filter(|line| !line.contains(&format!("\t{required}\t")))
            .collect::<Vec<_>>()
            .join("\n");
        assertions.record(
            validate_evidence_ledger_sources(&missing_required, &coverage_replay).is_err(),
            format!("missing ledger row {required} was accepted"),
        );
    }
    for ledger in [
        evidence_ledger.replacen("\tblocked\t", "\topen\t", 1),
        evidence_ledger.replacen(
            "assurance/product-identity/v1/coverage-replay.toml",
            "substituted-replay.toml",
            1,
        ),
    ] {
        assertions.record(
            validate_evidence_ledger_sources(&ledger, &coverage_replay).is_err(),
            String::from("invalid exact-coverage ledger row was accepted"),
        );
    }
    for replay in [
        coverage_replay.replacen("status = \"blocked\"", "status = \"pass\"", 1),
        coverage_replay.replacen("buoy:coverage", "substituted:coverage", 1),
        coverage_replay.replacen("buoy-0.2.0-post-rename", "stale-identity", 1),
        coverage_replay.replacen("LCOV-P0-EXACT-CLOSURE", "wrong-blocker", 1),
        coverage_replay.replacen("covered = 5042", "covered = 1", 1),
        coverage_replay.replacen("count = 7833", "count = 1", 1),
        coverage_replay.replacen("count = 10692", "count = 1", 1),
        coverage_replay.replacen("count = 791", "count = 1", 1),
        coverage_replay.replacen("count = 1434", "count = 1", 1),
    ] {
        assertions.record(
            validate_evidence_ledger_sources(&evidence_ledger, &replay).is_err(),
            String::from("unfinished exact coverage was laundered to pass"),
        );
    }

    assertions.record_code(
        ensure_parent(Path::new("/"))
            .err()
            .map(|error| error.code()),
        "PID-WRITE-PATH",
        String::from("parentless output path was accepted"),
    );
    let scratch = root.join(format!(
        "target/product-identity-coverage-controls-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&scratch);
    let create_result = if fault == Some("create-scratch") {
        Err(std::io::Error::other("injected scratch creation failure"))
    } else {
        fs::create_dir_all(&scratch)
    };
    self_test_io(create_result)?;
    let parent_file = scratch.join("parent-file");
    let write_result = if fault == Some("write-scratch") {
        Err(std::io::Error::other("injected scratch write failure"))
    } else {
        fs::write(&parent_file, b"ff\n")
    };
    self_test_io(write_result)?;
    assertions.record(
        snapshot_bytes(
            &scratch,
            "invalid-source",
            &SnapshotBinding {
                snapshot_path: String::from("parent-file"),
                replay: SnapshotReplay::CanonicalVectors,
            },
        )
        .is_err(),
        String::from("invalid canonical snapshot unexpectedly passed"),
    );
    assertions.record_code(
        ensure_parent(&parent_file.join("child"))
            .err()
            .map(|error| error.code()),
        "PID-WRITE-PATH",
        String::from("file parent was accepted as a directory"),
    );
    let generate_result = if fault == Some("generate-scratch") {
        Err(ProductIdentityError::new(
            "PID-SCAN-GIT",
            "injected scratch generation failure",
        ))
    } else {
        generate_current_inventory(&scratch)
    };
    generate_result?;
    assertions.record(
        scratch.join(CURRENT_INVENTORY).is_file(),
        String::from("generated inventory control did not write its output"),
    );
    for result in [
        freeze_pre_rename(&scratch),
        validate_evidence_rebinding(&scratch),
        check(&scratch),
    ] {
        assertions.record(
            result.is_err(),
            String::from("missing-input filesystem control unexpectedly passed"),
        );
    }
    let cleanup_result = if fault == Some("cleanup-scratch") {
        Err(std::io::Error::other("injected scratch cleanup failure"))
    } else {
        fs::remove_dir_all(&scratch)
    };
    self_test_io(cleanup_result)?;
    execution_evidence_coverage_controls(&fixture_text, fault, &mut assertions)?;
    assertions.finish()
}

fn check_corpus(root: &Path) -> Result<(), ProductIdentityError> {
    let corpus = read_text(root, PRE_RENAME_CORPUS, "PID-CORPUS-READ")?;
    validate_corpus(root, &corpus)?;
    validate_frozen_mutations(root)
}

/// Validates corpus parity, the current closed inventory, and fixture behavior.
///
/// # Errors
///
/// Returns the first stable parity, inventory, or fixture diagnostic.
pub fn check(root: &Path) -> Result<(), ProductIdentityError> {
    check_corpus(root)?;

    let pre_inventory = read_text(root, PRE_RENAME_OCCURRENCES, "PID-INVENTORY-READ")?;
    let pre_rows = parse_inventory(&pre_inventory)?;
    if pre_rows.is_empty() {
        return Err(ProductIdentityError::new(
            "PID-INVENTORY-MISSING",
            PRE_RENAME_OCCURRENCES,
        ));
    }

    let current = read_text(root, CURRENT_INVENTORY, "PID-INVENTORY-READ")?;
    let rows = parse_inventory(&current)?;
    if rows
        .iter()
        .any(|row| row.disposition == Disposition::RenameProduct)
    {
        return Err(ProductIdentityError::new(
            "PID-STALE-PRODUCT-IDENTITY",
            "rename-product rows remain",
        ));
    }
    let regenerated = render_inventory(&inventory_rows(scan_repository(root)?));
    if current != regenerated {
        return Err(ProductIdentityError::new(
            "PID-INVENTORY-STALE",
            CURRENT_INVENTORY,
        ));
    }

    let fixture_text = read_text(root, FIXTURE_MANIFEST, "PID-FIXTURE-READ")?;
    validate_fixture_manifest(&fixture_text)?;
    validate_planning_reconciliation(root)?;
    validate_evidence_rebinding(root)
}

/// Runs the product-identity command-line interface.
///
/// # Errors
///
/// Returns a stable usage or validation diagnostic.
pub fn run_cli(arguments: &[String], root: &Path) -> Result<(), ProductIdentityError> {
    match arguments.first().map(String::as_str) {
        Some("--freeze-pre-rename") => freeze_pre_rename(root),
        Some("--generate-inventory") => generate_current_inventory(root),
        Some("--self-test") => self_test(root),
        Some("--check-corpus") => check_corpus(root),
        Some("--check-planning") => validate_planning_reconciliation(root),
        Some("--write-execution-evidence") => fixture_execution_evidence(root, true),
        Some("--check-execution-evidence") => fixture_execution_evidence(root, false),
        Some("--check") | None => check(root),
        Some(other) => Err(ProductIdentityError::new("PID-USAGE", other)),
    }
}

#[cfg(all(test, feature = "coverage-controls"))]
#[allow(clippy::expect_used, clippy::format_collect, clippy::too_many_lines)]
mod coverage_tests {
    use super::*;

    fn parity(result: Result<(), ProductIdentityError>) {
        assert_eq!(
            result.expect_err("parity control must fail").code(),
            "PID-CANONICAL-PARITY"
        );
    }

    fn hex(bytes: &[u8]) -> String {
        bytes.iter().map(|byte| format!("{byte:02x}")).collect()
    }

    #[test]
    fn snapshot_manifest_and_frozen_parity_negative_paths_are_closed() {
        assert!(canonical_relative_path("schema/v2/inventory.tsv"));
        for invalid in ["", "/absolute", "parent/../escape", "./current"] {
            assert!(!canonical_relative_path(invalid), "{invalid}");
        }
        assert_eq!(decode_hex(b"00ff", false), Some(vec![0, 255]));
        assert_eq!(decode_hex(b"00 ff\n", true), Some(vec![0, 255]));
        for invalid in [b"".as_slice(), b"0", b"0g", b"g0", b"00 ff"] {
            assert!(decode_hex(invalid, false).is_none());
        }

        let actual = Path::new(".");
        assert!(
            !parse_snapshot_bindings(actual)
                .expect("actual snapshot bindings")
                .is_empty()
        );
        let scratch = tempfile::tempdir().expect("snapshot manifest scratch");
        assert!(
            parse_snapshot_bindings(scratch.path())
                .expect("absent snapshot manifest")
                .is_empty()
        );
        let manifest = scratch.path().join(PRE_RENAME_SNAPSHOTS);
        fs::create_dir_all(manifest.parent().expect("manifest parent"))
            .expect("manifest directory");
        let header = "schema\tsource_path\tsnapshot_path\treplay\n";
        let source_path = CORPUS_ENTRIES[0].2;
        let row = format!("{SNAPSHOT_SCHEMA}\t{source_path}\tsnapshots/control.hex\tline-subset\n");
        for malformed in [
            String::from("wrong\n"),
            String::from(header),
            format!("{header}malformed\n"),
            format!("{header}wrong\t{source_path}\tsnapshots/control.hex\tline-subset\n"),
            format!("{header}{SNAPSHOT_SCHEMA}\tunknown\tsnapshots/control.hex\tline-subset\n"),
            format!("{header}{SNAPSHOT_SCHEMA}\t{source_path}\t/absolute\tline-subset\n"),
            format!("{header}{SNAPSHOT_SCHEMA}\t{source_path}\tsnapshots/control.hex\tunknown\n"),
            format!("{header}{row}{row}"),
        ] {
            fs::write(&manifest, malformed).expect("malformed manifest");
            assert_eq!(
                validate_snapshot_manifest_control(scratch.path())
                    .expect_err("malformed manifest must fail")
                    .code(),
                "PID-CANONICAL-PARITY"
            );
        }
        fs::remove_file(&manifest).expect("remove manifest");
        fs::create_dir(&manifest).expect("manifest read-error directory");
        assert_eq!(
            parse_snapshot_bindings(scratch.path())
                .expect_err("manifest read error")
                .code(),
            "PID-CORPUS-READ"
        );

        let encoded_vectors =
            fs::read("assurance/product-identity/v1/snapshots/canonical-vectors-v2.hex")
                .expect("frozen vectors");
        let vector_bytes = decode_hex(&encoded_vectors, true).expect("decoded frozen vectors");
        validate_frozen_canonical_vectors(&vector_bytes).expect("valid frozen vectors");
        parity(validate_frozen_canonical_vectors(&[0xff]));
        parity(validate_frozen_canonical_vectors(b"wrong\n"));
        parity(validate_frozen_canonical_vectors(
            b"schema\tkind\tid\tcanonical_hex\tsemantic_digest\n",
        ));
        let vector_source = String::from_utf8(vector_bytes).expect("UTF-8 frozen vectors");
        let vector_header = vector_source.lines().next().expect("vector header");
        let first = vector_source.lines().nth(1).expect("first vector");
        parity(validate_frozen_canonical_vectors(
            format!("{vector_header}\n{first}\n{first}\n").as_bytes(),
        ));
        parity(validate_frozen_canonical_vectors(
            format!("{vector_header}\ninvalid\n").as_bytes(),
        ));
        let mutate_vector = |kind: Option<&str>, encoded: Option<String>, digest: Option<&str>| {
            let mut fields = first.split('\t').map(str::to_owned).collect::<Vec<_>>();
            if let Some(kind) = kind {
                fields[1] = kind.to_owned();
            }
            if let Some(encoded) = encoded {
                fields[3] = encoded;
            }
            if let Some(digest) = digest {
                fields[4] = digest.to_owned();
            }
            format!("{vector_header}\n{}\n", fields.join("\t"))
        };
        parity(validate_frozen_canonical_vectors(
            mutate_vector(None, Some(String::from("not-hex")), None).as_bytes(),
        ));
        parity(validate_frozen_canonical_vectors(
            mutate_vector(None, Some(hex(&[0xff])), None).as_bytes(),
        ));
        parity(validate_frozen_canonical_vectors(
            mutate_vector(Some("unknown"), None, None).as_bytes(),
        ));
        parity(validate_frozen_canonical_vectors(
            mutate_vector(None, None, Some("unexpected-digest")).as_bytes(),
        ));
        for kind in ["type-tag", "nested", "report-fact"] {
            let row = vector_source
                .lines()
                .skip(1)
                .find(|line| line.split('\t').nth(1) == Some(kind))
                .expect("guarded canonical vector");
            let mut fields = row.split('\t').map(str::to_owned).collect::<Vec<_>>();
            fields[4] = String::from("unexpected-digest");
            parity(validate_frozen_canonical_vectors(
                format!("{vector_header}\n{}\n", fields.join("\t")).as_bytes(),
            ));
        }
        let domain = vector_source
            .lines()
            .skip(1)
            .find(|line| line.split('\t').nth(1) == Some("domain"))
            .expect("domain vector");
        let mutate_domain = |encoded: Option<String>, digest: Option<&str>| {
            let mut fields = domain.split('\t').map(str::to_owned).collect::<Vec<_>>();
            if let Some(encoded) = encoded {
                fields[3] = encoded;
            }
            if let Some(digest) = digest {
                fields[4] = digest.to_owned();
            }
            format!("{vector_header}\n{}\n", fields.join("\t"))
        };
        parity(validate_frozen_canonical_vectors(
            mutate_domain(
                Some(hex(&CanonicalValue::True
                    .encode()
                    .expect("canonical true encoding"))),
                None,
            )
            .as_bytes(),
        ));
        let mut noncanonical_domain = domain.split('\t').map(str::to_owned).collect::<Vec<_>>();
        noncanonical_domain[2] = String::from("e\u{301}");
        parity(validate_frozen_canonical_vectors(
            format!("{vector_header}\n{}\n", noncanonical_domain.join("\t")).as_bytes(),
        ));
        parity(validate_frozen_canonical_vectors(
            mutate_domain(None, Some("xxh3-128:00000000000000000000000000000000")).as_bytes(),
        ));

        let encoded_inventory =
            fs::read("assurance/product-identity/v1/snapshots/protocol-inventory-v2.hex")
                .expect("frozen inventory");
        let inventory_bytes = decode_hex(&encoded_inventory, true).expect("decoded inventory");
        validate_frozen_line_subset(actual, "schema/v2/inventory.tsv", &inventory_bytes)
            .expect("valid frozen line subset");
        parity(validate_frozen_line_subset(
            actual,
            "schema/v2/inventory.tsv",
            &[0xff],
        ));
        assert_eq!(
            validate_frozen_line_subset(actual, "missing-source", b"header\n")
                .expect_err("missing current source")
                .code(),
            "PID-CORPUS-READ"
        );
        parity(validate_frozen_line_subset(
            actual,
            "schema/v2/inventory.tsv",
            b"wrong-header\n",
        ));
        let current_header = fs::read_to_string("schema/v2/inventory.tsv")
            .expect("current inventory")
            .lines()
            .next()
            .expect("current header")
            .to_owned();
        parity(validate_frozen_line_subset(
            actual,
            "schema/v2/inventory.tsv",
            format!("{current_header}\nmissing-row\n").as_bytes(),
        ));

        let canonical_binding = parse_snapshot_bindings(actual)
            .expect("snapshot bindings")
            .into_iter()
            .find(|(_, binding)| binding.replay == SnapshotReplay::CanonicalVectors)
            .expect("canonical snapshot binding");
        snapshot_bytes(actual, &canonical_binding.0, &canonical_binding.1)
            .expect("canonical snapshot replay");
        let invalid_snapshot = tempfile::tempdir().expect("invalid snapshot root");
        fs::write(invalid_snapshot.path().join("invalid.hex"), "ff\n").expect("invalid snapshot");
        parity(
            snapshot_bytes(
                invalid_snapshot.path(),
                "invalid-source",
                &SnapshotBinding {
                    snapshot_path: String::from("invalid.hex"),
                    replay: SnapshotReplay::CanonicalVectors,
                },
            )
            .map(|_| ()),
        );
        assert!(canonical_relative_path("snapshot.hex"));
        assert!(!canonical_relative_path(""));
        assert!(!canonical_relative_path("/absolute"));
        assert!(!canonical_relative_path("nested/../snapshot.hex"));

        let corpus = fs::read_to_string(PRE_RENAME_CORPUS).expect("frozen corpus");
        validate_corpus(actual, &corpus).expect("valid corpus");
        parity(validate_corpus(actual, "wrong\n"));
        parity(validate_corpus(
            actual,
            corpus.lines().next().expect("corpus header"),
        ));
        let corpus_row = corpus.lines().nth(1).expect("corpus row");
        parity(validate_corpus_control(
            actual,
            &corpus.replacen(corpus_row, "malformed", 1),
        ));
        let mut corpus_fields = corpus_row
            .split('\t')
            .map(str::to_owned)
            .collect::<Vec<_>>();
        corpus_fields[1] = String::from("wrong-id");
        parity(validate_corpus(
            actual,
            &corpus.replacen(corpus_row, &corpus_fields.join("\t"), 1),
        ));
        let bad_digest = corpus.replacen(
            corpus
                .lines()
                .nth(1)
                .expect("corpus row")
                .split('\t')
                .nth(4)
                .expect("digest"),
            "xxh3-128:00000000000000000000000000000000",
            1,
        );
        parity(validate_corpus(actual, &bad_digest));

        self_test(actual).expect("complete product identity self-test");
    }
}
