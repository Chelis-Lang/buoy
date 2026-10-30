//! Explicit Chelis snapshot and configuration construction.
//!
//! Canonical inputs contain only saved repository paths, saved bytes, and
//! explicit configuration. Construction rejects symlinks, submodule and
//! nested-repository boundaries, overlapping selectors, and portable path
//! collisions. No Git metadata, provider state, clock, or ambient
//! environment value enters a snapshot.

use crate::records::ChelisDialect;
use buoy::core::RepositoryPath;
use serde::Deserialize;
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

/// One explicit path selector bound to one Chelis dialect adapter.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ChelisSelector {
    /// Repository-relative selector root.
    pub root: String,
    /// Selected dialect adapter.
    pub dialect: ChelisDialect,
}

/// Parsed explicit Chelis adapter configuration.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ChelisConfiguration {
    /// Explicit selectors in configuration order.
    pub selectors: Vec<ChelisSelector>,
    /// Content-addressed binding-store mount prefix, when configured.
    ///
    /// Objects resolve at `<binding-store>/buoy/objects/v1/<algorithm>/<hex>.toml`,
    /// the frozen upstream layout, so the store must lie under one selector
    /// root for its bytes to enter a saved snapshot.
    pub binding_store: Option<RepositoryPath>,
    /// Exact configuration bytes for adapter input digests.
    pub bytes: Vec<u8>,
}

/// Deterministic snapshot or configuration construction failure.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SnapshotError {
    /// The configuration bytes are not a valid explicit configuration.
    Configuration(String),
    /// Two selectors overlap.
    SelectorOverlap(String),
    /// A selected entry is a symlink.
    Symlink(String),
    /// A selected tree contains a submodule or nested repository boundary.
    RepositoryBoundary(String),
    /// Two selected paths collide under portable case folding.
    PathCollision(String),
    /// A selected path is not a canonical repository path.
    Path(String),
    /// A filesystem read failed.
    Read(String),
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawConfiguration {
    schema: String,
    selector: Vec<RawSelector>,
    #[serde(default)]
    tool: Vec<RawTool>,
    #[serde(default, rename = "binding-store")]
    binding_store: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawTool {
    id: String,
    version: String,
    digest: String,
    #[serde(rename = "configuration-digest")]
    configuration_digest: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawSelector {
    root: String,
    dialect: String,
}

impl ChelisConfiguration {
    /// Parses explicit configuration bytes at the boundary.
    ///
    /// # Errors
    ///
    /// Rejects malformed configuration and overlapping selectors.
    pub fn parse(bytes: &[u8]) -> Result<Self, SnapshotError> {
        let text = core::str::from_utf8(bytes)
            .map_err(|_| SnapshotError::Configuration(String::from("invalid-utf8")))?;
        let raw = toml::from_str::<RawConfiguration>(text)
            .map_err(|_| SnapshotError::Configuration(String::from("invalid-toml")))?;
        if raw.schema != crate::adapter::CONFIGURATION_SCHEMA_ID {
            return Err(SnapshotError::Configuration(String::from("invalid-schema")));
        }
        if raw.selector.is_empty() {
            return Err(SnapshotError::Configuration(String::from(
                "empty-selectors",
            )));
        }
        for tool in &raw.tool {
            if tool.id.is_empty()
                || tool.version.is_empty()
                || tool.digest.is_empty()
                || tool.configuration_digest.is_empty()
            {
                return Err(SnapshotError::Configuration(String::from("invalid-tool")));
            }
        }
        let mut selectors = Vec::new();
        for selector in raw.selector {
            let dialect = match selector.dialect.as_str() {
                "surf" => ChelisDialect::Surf,
                "deep" => ChelisDialect::Deep,
                other => {
                    return Err(SnapshotError::Configuration(format!(
                        "invalid-dialect:{other}"
                    )));
                }
            };
            if selector.root.starts_with('/') || selector.root.split('/').any(|part| part == "..") {
                return Err(SnapshotError::Configuration(format!(
                    "invalid-root:{}",
                    selector.root
                )));
            }
            selectors.push(ChelisSelector {
                root: selector.root,
                dialect,
            });
        }
        for (index, left) in selectors.iter().enumerate() {
            for right in selectors.iter().skip(index + 1) {
                let overlap = left.root == right.root
                    || left.root.starts_with(&format!("{}/", right.root))
                    || right.root.starts_with(&format!("{}/", left.root))
                    || left.root.is_empty()
                    || right.root.is_empty();
                if overlap {
                    return Err(SnapshotError::SelectorOverlap(format!(
                        "{}|{}",
                        left.root, right.root
                    )));
                }
            }
        }
        let binding_store = match raw.binding_store {
            None => None,
            Some(store) => Some(validated_binding_store(&store, &selectors)?),
        };
        Ok(Self {
            selectors,
            binding_store,
            bytes: bytes.to_vec(),
        })
    }
}

/// Validates one configured binding-store prefix at the configuration boundary.
///
/// The store must be a canonical repository-relative path and must lie under
/// one selector root, because only selected bytes enter a saved snapshot; a
/// store outside every root could never resolve and is rejected here rather
/// than surfacing later as a missing object.
fn validated_binding_store(
    store: &str,
    selectors: &[ChelisSelector],
) -> Result<RepositoryPath, SnapshotError> {
    let canonical = RepositoryPath::new(store)
        .map_err(|_| SnapshotError::Configuration(format!("invalid-binding-store:{store}")))?;
    if canonical.as_str() != store {
        return Err(SnapshotError::Configuration(format!(
            "invalid-binding-store:{store}"
        )));
    }
    let contained = selectors.iter().any(|selector| {
        store == selector.root || store.starts_with(&format!("{}/", selector.root))
    });
    if !contained {
        return Err(SnapshotError::Configuration(format!(
            "binding-store-outside-selectors:{store}"
        )));
    }
    Ok(canonical)
}

/// One explicit saved Chelis snapshot.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ChelisSnapshot {
    files: BTreeMap<RepositoryPath, (ChelisDialect, Vec<u8>)>,
    artifacts: BTreeMap<String, Vec<u8>>,
}

impl ChelisSnapshot {
    /// Constructs a snapshot from explicit saved paths and bytes.
    ///
    /// # Errors
    ///
    /// Rejects noncanonical paths and portable path collisions.
    pub fn from_files(files: Vec<(String, ChelisDialect, Vec<u8>)>) -> Result<Self, SnapshotError> {
        Self::from_files_with_artifacts(files, Vec::new())
    }

    /// Constructs a snapshot from explicit dialect sources and raw artifacts.
    ///
    /// # Errors
    ///
    /// Rejects noncanonical paths and portable path collisions.
    pub fn from_files_with_artifacts(
        files: Vec<(String, ChelisDialect, Vec<u8>)>,
        artifacts: Vec<(String, Vec<u8>)>,
    ) -> Result<Self, SnapshotError> {
        let mut canonical = BTreeMap::new();
        let mut folded = BTreeMap::new();
        for (path, dialect, bytes) in files {
            let repository_path =
                RepositoryPath::new(&path).map_err(|_| SnapshotError::Path(path.clone()))?;
            let fold = path.to_lowercase();
            if let Some(previous) = folded.insert(fold, path.clone()) {
                return Err(SnapshotError::PathCollision(format!("{previous}|{path}")));
            }
            if canonical
                .insert(repository_path, (dialect, bytes))
                .is_some()
            {
                return Err(SnapshotError::PathCollision(path));
            }
        }
        let mut raw = BTreeMap::new();
        for (path, bytes) in artifacts {
            let fold = path.to_lowercase();
            if let Some(previous) = folded.insert(fold, path.clone()) {
                return Err(SnapshotError::PathCollision(format!("{previous}|{path}")));
            }
            if raw.insert(path.clone(), bytes).is_some() {
                return Err(SnapshotError::PathCollision(path));
            }
        }
        Ok(Self {
            files: canonical,
            artifacts: raw,
        })
    }

    /// Constructs a snapshot from one saved repository root and configuration.
    ///
    /// The walk is deterministic, does not follow symlinks, and rejects
    /// submodule and nested-repository boundaries below the root.
    ///
    /// # Errors
    ///
    /// Rejects symlinks, repository boundaries, collisions, and read failures.
    pub fn from_directory(
        root: &Path,
        configuration: &ChelisConfiguration,
    ) -> Result<Self, SnapshotError> {
        let mut files = Vec::new();
        let mut artifacts = Vec::new();
        for selector in &configuration.selectors {
            let selector_root = root.join(&selector.root);
            if !selector_root.exists() {
                continue;
            }
            walk(root, &selector_root, selector, &mut files, &mut artifacts)?;
        }
        files.sort_by(|left, right| left.0.cmp(&right.0));
        artifacts.sort_by(|left, right| left.0.cmp(&right.0));
        Self::from_files_with_artifacts(files, artifacts)
    }

    /// Selected files in canonical path order.
    pub fn files(&self) -> impl Iterator<Item = (&RepositoryPath, ChelisDialect, &[u8])> {
        self.files
            .iter()
            .map(|(path, (dialect, bytes))| (path, *dialect, bytes.as_slice()))
    }

    /// Selected raw artifacts in canonical path order.
    pub fn artifacts(&self) -> impl Iterator<Item = (&str, &[u8])> {
        self.artifacts
            .iter()
            .map(|(path, bytes)| (path.as_str(), bytes.as_slice()))
    }

    /// Selected raw artifact bytes at exactly one saved path.
    ///
    /// Content-addressed resolution must consult the one path an address
    /// derives and no other, so this lookup is exact and never searches.
    #[must_use]
    pub fn artifact(&self, path: &str) -> Option<&[u8]> {
        self.artifacts.get(path).map(Vec::as_slice)
    }

    /// Number of selected files.
    #[must_use]
    pub fn len(&self) -> usize {
        self.files.len()
    }

    /// Whether the snapshot selected no files.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.files.is_empty()
    }
}

fn walk(
    repository_root: &Path,
    directory: &Path,
    selector: &ChelisSelector,
    files: &mut Vec<(String, ChelisDialect, Vec<u8>)>,
    artifacts: &mut Vec<(String, Vec<u8>)>,
) -> Result<(), SnapshotError> {
    let mut entries = fs::read_dir(directory)
        .map_err(|error| SnapshotError::Read(error.to_string()))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| SnapshotError::Read(error.to_string()))?;
    entries.sort_by_key(std::fs::DirEntry::file_name);
    for entry in entries {
        let path = entry.path();
        let name = entry.file_name();
        let display = path.display().to_string();
        let metadata =
            fs::symlink_metadata(&path).map_err(|error| SnapshotError::Read(error.to_string()))?;
        if metadata.file_type().is_symlink() {
            return Err(SnapshotError::Symlink(display));
        }
        if name == ".git" {
            return Err(SnapshotError::RepositoryBoundary(display));
        }
        if metadata.is_dir() {
            walk(repository_root, &path, selector, files, artifacts)?;
            continue;
        }
        let relative = path
            .strip_prefix(repository_root)
            .map_err(|_| SnapshotError::Path(display.clone()))?;
        let Some(relative) = relative.to_str() else {
            return Err(SnapshotError::Path(display));
        };
        let relative = relative.replace('\\', "/");
        let bytes = fs::read(&path).map_err(|error| SnapshotError::Read(error.to_string()))?;
        let matches_extension = path
            .extension()
            .is_some_and(|extension| extension == selector.dialect.extension());
        if matches_extension {
            files.push((relative, selector.dialect, bytes));
        } else {
            artifacts.push((relative, bytes));
        }
    }
    Ok(())
}
