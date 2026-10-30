//! Saved-snapshot lint and LSP adapters.
//!
//! Both adapters consume the same semantic library as the authoritative
//! static command and accept only explicit saved snapshots. No constructor
//! accepts daemon state, editor buffers, or unsaved text, so editor surfaces
//! cannot satisfy CI: only the static command report is the CI authority.

use crate::command::{CommandError, ReportDiagnostic, StaticOutcome, static_check};
use crate::snapshot::{ChelisConfiguration, ChelisSnapshot};

/// The single CI satisfaction authority for Chelis provenance checks.
pub const CI_AUTHORITY: &str = "chelis static command report over saved snapshots";

/// Returns lint findings for one saved snapshot.
///
/// # Errors
///
/// Returns an adapter or encoding failure outside the report protocol.
pub fn lint_saved_snapshot(
    snapshot: &ChelisSnapshot,
    configuration: &ChelisConfiguration,
) -> Result<Vec<ReportDiagnostic>, CommandError> {
    static_check(snapshot, configuration).map(|outcome| outcome.diagnostics)
}

/// Returns LSP-shaped findings for one saved snapshot.
///
/// The result is derived from the identical semantic library and cannot
/// diverge from the static command over the same saved input.
///
/// # Errors
///
/// Returns an adapter or encoding failure outside the report protocol.
pub fn lsp_saved_snapshot(
    snapshot: &ChelisSnapshot,
    configuration: &ChelisConfiguration,
) -> Result<Vec<ReportDiagnostic>, CommandError> {
    lint_saved_snapshot(snapshot, configuration)
}

/// Returns the full static outcome for editor consumers without granting CI
/// authority to the editor surface.
///
/// # Errors
///
/// Returns an adapter or encoding failure outside the report protocol.
pub fn editor_static_view(
    snapshot: &ChelisSnapshot,
    configuration: &ChelisConfiguration,
) -> Result<StaticOutcome, CommandError> {
    static_check(snapshot, configuration)
}
