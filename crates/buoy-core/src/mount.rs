//! Pure logical mount declarations and path projection.

use crate::{CanonicalValue, EncodingError, RepositoryPath, ScopeId};
use alloc::format;
use alloc::vec::Vec;

/// A validated logical mount identifier.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Hash)]
pub struct LogicalMountId(ScopeId);

impl LogicalMountId {
    /// Parses one lowercase mount identifier.
    ///
    /// # Errors
    ///
    /// Returns [`LogicalMountError::Id`] when the value violates the scope-ID grammar.
    pub fn new(value: &str) -> Result<Self, LogicalMountError> {
        ScopeId::new(value)
            .map(Self)
            .map_err(|_| LogicalMountError::Id)
    }

    /// Returns the canonical mount identifier text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

/// One canonical logical mount declaration.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct LogicalMount {
    id: LogicalMountId,
    prefix: RepositoryPath,
}

impl LogicalMount {
    /// Constructs one declaration from parsed values.
    #[must_use]
    pub const fn new(id: LogicalMountId, prefix: RepositoryPath) -> Self {
        Self { id, prefix }
    }

    /// Returns the mount identifier.
    #[must_use]
    pub const fn id(&self) -> &LogicalMountId {
        &self.id
    }

    /// Returns the canonical logical path prefix.
    #[must_use]
    pub const fn prefix(&self) -> &RepositoryPath {
        &self.prefix
    }

    /// Returns the canonical declaration value.
    #[must_use]
    pub fn canonical_value(&self) -> CanonicalValue {
        CanonicalValue::Record(alloc::vec![
            (1, CanonicalValue::Text(self.id.as_str().into())),
            (2, CanonicalValue::Text(self.prefix.as_str().into())),
        ])
    }

    /// Returns the complete canonical declaration bytes.
    ///
    /// # Errors
    ///
    /// Returns an encoding error only if the closed declaration fields stop encoding.
    pub fn canonical_bytes(&self) -> Result<Vec<u8>, EncodingError> {
        self.canonical_value().encode()
    }
}

/// A deterministic logical mount failure.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum LogicalMountError {
    /// A mount identifier violates the closed grammar.
    Id,
    /// Two declarations use one identifier.
    DuplicateId,
    /// Two canonical or portable prefixes overlap.
    PrefixOverlap,
    /// A projected path is not canonical.
    Projection,
}

impl LogicalMountError {
    /// Returns the stable machine diagnostic code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::Id => "LSM-MOUNT-ID",
            Self::DuplicateId => "LSM-MOUNT-ID-DUPLICATE",
            Self::PrefixOverlap => "LSM-MOUNT-PREFIX-OVERLAP",
            Self::Projection => "LSM-LOGICAL-PATH",
        }
    }
}

/// Canonicalizes declarations by mount ID and rejects duplicate or overlapping mounts.
///
/// # Errors
///
/// Returns a duplicate-ID or prefix-overlap error for an invalid mount set.
pub fn canonical_logical_mounts(
    mut mounts: Vec<LogicalMount>,
) -> Result<Vec<LogicalMount>, LogicalMountError> {
    mounts.sort_unstable_by(|left, right| left.id.cmp(&right.id));
    if mounts.windows(2).any(|pair| pair[0].id == pair[1].id) {
        return Err(LogicalMountError::DuplicateId);
    }
    for (index, left) in mounts.iter().enumerate() {
        for right in mounts.iter().skip(index + 1) {
            if prefixes_overlap(&left.prefix, &right.prefix) {
                return Err(LogicalMountError::PrefixOverlap);
            }
        }
    }
    Ok(mounts)
}

/// Projects one mount-relative path into the closed logical namespace.
///
/// # Errors
///
/// Returns a projection error if the joined path violates repository path policy.
pub fn project_logical_path(
    mount: &LogicalMount,
    relative: &RepositoryPath,
) -> Result<RepositoryPath, LogicalMountError> {
    RepositoryPath::new(&format!("{}/{}", mount.prefix.as_str(), relative.as_str()))
        .map_err(|_| LogicalMountError::Projection)
}

/// Removes one exact mount prefix from a projected logical path.
#[must_use]
pub fn mount_relative_path(
    mount: &LogicalMount,
    logical: &RepositoryPath,
) -> Option<RepositoryPath> {
    let remainder = logical
        .as_str()
        .strip_prefix(mount.prefix.as_str())?
        .strip_prefix('/')?;
    RepositoryPath::new(remainder).ok()
}

fn prefixes_overlap(left: &RepositoryPath, right: &RepositoryPath) -> bool {
    let left = left.portable_collision_key();
    let right = right.portable_collision_key();
    path_prefix(&left, &right) || path_prefix(&right, &left)
}

fn path_prefix(prefix: &str, path: &str) -> bool {
    prefix == path
        || path
            .strip_prefix(prefix)
            .is_some_and(|remainder| remainder.starts_with('/'))
}

#[cfg(all(test, feature = "coverage-controls"))]
#[allow(clippy::expect_used)]
mod coverage_tests {
    use super::*;

    #[test]
    fn invalid_reviewed_relative_path_reaches_projection_failure() {
        let mount = LogicalMount::new(
            LogicalMountId::new("project").expect("mount ID"),
            RepositoryPath::new("project").expect("mount prefix"),
        );
        assert_eq!(
            project_logical_path(&mount, &RepositoryPath::reviewed_control("/invalid")),
            Err(LogicalMountError::Projection)
        );
        assert!(path_prefix("src", "src"));
        assert!(path_prefix("src", "src/control"));
        assert!(!path_prefix("src", "src-other"));
        assert!(!path_prefix("src", "other"));
        assert_eq!(mount_relative_path(&mount, mount.prefix()), None);
    }
}
