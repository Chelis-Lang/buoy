//! Effectful Chelis oracle and derivation-consistency orchestration.
//!
//! Execution consumes one immutable static identity and returns it unchanged.
//! Receipts retain the closed five-state verdict; raw logs stay noncanonical
//! and never enter canonical report bytes.

use crate::command::{CommandError, EXIT_EXECUTION, EXIT_OK};
use buoy::core::{SemanticDigest, Verdict};
use buoy::oracle::{OracleReceipt, run_oracle};
use buoy::repository::OracleCommand;
use serde::Serialize;
use std::path::Path;

/// Complete effectful execution outcome.
#[derive(Clone, Debug)]
pub struct ExecutionOutcome {
    /// Complete raw receipts including noncanonical logs.
    pub receipts: Vec<OracleReceipt>,
    /// Canonical machine-report bytes without logs, timing, or paths.
    pub report: String,
    /// Core exit class.
    pub exit: u8,
    /// The unchanged static graph identity.
    pub static_identity: SemanticDigest,
}

#[derive(Serialize)]
struct ExecutionReport<'a> {
    schema: &'static str,
    static_identity: String,
    verdicts: &'a [(String, &'static str)],
    verdict: &'static str,
}

/// Closed verdict name for canonical rendering.
#[must_use]
pub const fn verdict_name(verdict: Verdict) -> &'static str {
    match verdict {
        Verdict::NotRun => "not-run",
        Verdict::Pass => "pass",
        Verdict::Fail => "fail",
        Verdict::Error => "error",
        Verdict::Timeout => "timeout",
    }
}

/// Runs every selected oracle and renders one canonical execution report.
///
/// The static identity is an input and is returned byte-identical; a live run
/// cannot mutate static graph identity.
///
/// # Errors
///
/// Returns an encoding failure outside the report protocol.
pub fn execute(
    static_identity: SemanticDigest,
    commands: &[OracleCommand],
    root: &Path,
) -> Result<ExecutionOutcome, CommandError> {
    let receipts = commands
        .iter()
        .map(|command| run_oracle(command, root))
        .collect::<Vec<_>>();
    let mut verdicts = receipts
        .iter()
        .map(|receipt| (receipt.id.clone(), verdict_name(receipt.verdict)))
        .collect::<Vec<_>>();
    verdicts.sort();
    let all_pass = receipts
        .iter()
        .all(|receipt| receipt.verdict == Verdict::Pass);
    let report = serde_json::to_string(&ExecutionReport {
        schema: "chelis-execution-report/v1",
        static_identity: static_identity.canonical_text(),
        verdicts: &verdicts,
        verdict: if all_pass { "accept" } else { "reject" },
    })
    .map_err(|_| CommandError::Encoding)?;
    Ok(ExecutionOutcome {
        receipts,
        report,
        exit: if all_pass { EXIT_OK } else { EXIT_EXECUTION },
        static_identity,
    })
}
