//! Bounded effectful oracle supervision with normalized receipts and noncanonical logs.

use crate::repository::OracleCommand;
use buoy_core::Verdict;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::thread;
use std::time::{Duration, Instant};

/// Stable normalized process-supervision outcome, orthogonal to verdict.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OracleTermination {
    /// Built-in deterministic sentinel; no process was started.
    Builtin,
    /// Process completed and emitted one valid semantic result.
    Completed,
    /// Program could not be started.
    SpawnError,
    /// Process exited abnormally or with a nonzero status.
    AbnormalExit,
    /// Zero-exit process omitted or malformed the semantic result protocol.
    MalformedResult,
    /// Configured stdout/stderr/combined output budget was exceeded.
    OutputOverflow,
    /// Configured timeout elapsed and the process tree was terminated/reaped.
    TimedOut,
    /// Caller cancellation terminated and reaped the process tree.
    Cancelled,
    /// Pipe reader, waiter, or process-tree cleanup failed.
    SupervisorError,
}

/// Cooperative cancellation handle for one or more oracle invocations.
#[derive(Clone, Debug, Default)]
pub struct CancellationToken(Arc<AtomicBool>);

impl CancellationToken {
    /// Creates a noncancelled token.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Requests deterministic cancellation.
    pub fn cancel(&self) {
        self.0.store(true, Ordering::Release);
    }

    /// Returns whether cancellation was requested.
    #[must_use]
    pub fn is_cancelled(&self) -> bool {
        self.0.load(Ordering::Acquire)
    }
}

/// Normalized semantic receipt separated from raw process output.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OracleReceipt {
    /// Stable oracle ID.
    pub id: String,
    /// Closed semantic verdict.
    pub verdict: Verdict,
    /// Stable process supervision outcome.
    pub termination: OracleTermination,
    /// Whether the exact semantic result protocol was observed.
    pub protocol_valid: bool,
    /// Whether all descendants and pipe readers were terminated and joined.
    pub cleanup_complete: bool,
    /// Noncanonical bounded stdout bytes.
    pub raw_stdout: Vec<u8>,
    /// Noncanonical bounded stderr bytes.
    pub raw_stderr: Vec<u8>,
    /// Optional derivation relation checked by this oracle.
    pub derivation: Option<String>,
}

/// Executes one explicit oracle with an empty environment and normalized verdict.
#[must_use]
pub fn run_oracle(command: &OracleCommand, root: &Path) -> OracleReceipt {
    run_oracle_controlled(command, root, &CancellationToken::new())
}

/// Deterministic bounded oracle-set supervision failure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OracleSetError {
    /// Concurrency is zero or above the frozen upper bound of 64.
    Concurrency,
    /// A worker could not return its receipt through the scoped channel.
    Worker,
}

/// Executes an oracle set with bounded concurrency and canonical result ordering.
///
/// # Errors
///
/// Rejects invalid concurrency and incomplete worker result collection.
#[allow(
    clippy::cast_possible_truncation,
    clippy::expect_used,
    clippy::missing_panics_doc
)]
pub fn run_oracles_controlled(
    commands: &[OracleCommand],
    root: &Path,
    concurrency: u64,
    cancellation: &CancellationToken,
) -> Result<Vec<OracleReceipt>, OracleSetError> {
    if !(1..=64).contains(&concurrency) {
        return Err(OracleSetError::Concurrency);
    }
    if commands.is_empty() {
        return Ok(Vec::new());
    }
    let worker_count = (concurrency as usize).min(commands.len());
    let next = AtomicUsize::new(0);
    let (sender, receiver) = std::sync::mpsc::channel();
    thread::scope(|scope| {
        for _ in 0..worker_count {
            let sender = sender.clone();
            let next = &next;
            scope.spawn(move || {
                loop {
                    let index = next.fetch_add(1, Ordering::AcqRel);
                    let Some(command) = commands.get(index) else {
                        break;
                    };
                    let receipt = run_oracle_controlled(command, root, cancellation);
                    sender
                        .send((index, receipt))
                        .expect("scoped receiver outlives every oracle worker");
                }
            });
        }
        drop(sender);
    });
    let mut receipts = receiver.into_iter().collect::<Vec<_>>();
    receipts.sort_unstable_by(|left, right| (&left.1.id, left.0).cmp(&(&right.1.id, right.0)));
    Ok(receipts.into_iter().map(|(_, receipt)| receipt).collect())
}

/// Executes an oracle set with one explicit physical working root per command.
///
/// Physical roots select process working directories only. They do not enter receipts.
///
/// # Errors
///
/// Rejects invalid concurrency or a root-count mismatch.
#[allow(
    clippy::cast_possible_truncation,
    clippy::expect_used,
    clippy::missing_panics_doc
)]
pub fn run_oracles_at_roots_controlled(
    commands: &[OracleCommand],
    roots: &[PathBuf],
    concurrency: u64,
    cancellation: &CancellationToken,
) -> Result<Vec<OracleReceipt>, OracleSetError> {
    if commands.len() != roots.len() {
        return Err(OracleSetError::Worker);
    }
    if !(1..=64).contains(&concurrency) {
        return Err(OracleSetError::Concurrency);
    }
    if commands.is_empty() {
        return Ok(Vec::new());
    }
    let worker_count = (concurrency as usize).min(commands.len());
    let next = AtomicUsize::new(0);
    let (sender, receiver) = std::sync::mpsc::channel();
    thread::scope(|scope| {
        for _ in 0..worker_count {
            let sender = sender.clone();
            let next = &next;
            scope.spawn(move || {
                loop {
                    let index = next.fetch_add(1, Ordering::AcqRel);
                    let (Some(command), Some(root)) = (commands.get(index), roots.get(index))
                    else {
                        break;
                    };
                    let receipt = run_oracle_controlled(command, root, cancellation);
                    sender
                        .send((index, receipt))
                        .expect("scoped receiver outlives every mounted oracle worker");
                }
            });
        }
        drop(sender);
    });
    let mut receipts = receiver.into_iter().collect::<Vec<_>>();
    receipts.sort_unstable_by(|left, right| (&left.1.id, left.0).cmp(&(&right.1.id, right.0)));
    Ok(receipts.into_iter().map(|(_, receipt)| receipt).collect())
}

/// Executes one explicit oracle with bounded output and cooperative cancellation.
#[must_use]
#[allow(clippy::too_many_lines)]
pub fn run_oracle_controlled(
    command: &OracleCommand,
    root: &Path,
    cancellation: &CancellationToken,
) -> OracleReceipt {
    run_oracle_internal(command, root, cancellation, false, false)
}

/// Injects a deterministic waiter failure for supervisor assurance coverage.
#[cfg(feature = "coverage-controls")]
#[must_use]
pub fn run_oracle_wait_fault_controlled(
    command: &OracleCommand,
    root: &Path,
    cancellation: &CancellationToken,
) -> OracleReceipt {
    run_oracle_internal(command, root, cancellation, true, false)
}

/// Injects a deterministic stdin-writer failure for cleanup assurance coverage.
#[cfg(feature = "coverage-controls")]
#[must_use]
pub fn run_oracle_input_fault_controlled(
    command: &OracleCommand,
    root: &Path,
    cancellation: &CancellationToken,
) -> OracleReceipt {
    run_oracle_internal(command, root, cancellation, false, true)
}

#[allow(
    clippy::expect_used,
    clippy::needless_bitwise_bool,
    clippy::too_many_lines
)]
fn run_oracle_internal(
    command: &OracleCommand,
    root: &Path,
    cancellation: &CancellationToken,
    inject_wait_failure: bool,
    inject_input_failure: bool,
) -> OracleReceipt {
    if let Some(verdict) = builtin_verdict(&command.program) {
        return receipt(
            command,
            verdict,
            OracleTermination::Builtin,
            true,
            true,
            Vec::new(),
            Vec::new(),
        );
    }
    if cancellation.is_cancelled() {
        return receipt(
            command,
            Verdict::Error,
            OracleTermination::Cancelled,
            false,
            true,
            Vec::new(),
            Vec::new(),
        );
    }

    let mut process = Command::new(&command.program);
    process
        .args(&command.args)
        .current_dir(root)
        .env_clear()
        .envs(&command.environment)
        .stdin(if command.input.is_some() {
            Stdio::piped()
        } else {
            Stdio::null()
        })
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    configure_process_tree(&mut process);
    let spawned = process.spawn();
    let Ok(mut child) = spawned else {
        return receipt(
            command,
            Verdict::Error,
            OracleTermination::SpawnError,
            false,
            true,
            Vec::new(),
            Vec::new(),
        );
    };
    let process_id = child.id();
    let input_writer = if let Some(input) = &command.input {
        let mut stdin = child
            .stdin
            .take()
            .expect("stdin is configured as piped when command input is present");
        let bytes = input.as_bytes().to_vec();
        Some(thread::spawn(move || {
            if inject_input_failure {
                false
            } else {
                stdin.write_all(&bytes).is_ok()
            }
        }))
    } else {
        None
    };
    let stdout = child
        .stdout
        .take()
        .expect("oracle stdout is always configured as piped");
    let stderr = child
        .stderr
        .take()
        .expect("oracle stderr is always configured as piped");

    let combined = Arc::new(AtomicU64::new(0));
    let overflow = Arc::new(AtomicBool::new(false));
    let stdout_reader = spawn_reader(
        Box::new(stdout),
        command.stdout_limit,
        command.combined_output_limit,
        command.raw_log_retention,
        true,
        Arc::clone(&combined),
        Arc::clone(&overflow),
    );
    let stderr_reader = spawn_reader(
        Box::new(stderr),
        command.stderr_limit,
        command.combined_output_limit,
        command.raw_log_retention,
        false,
        combined,
        Arc::clone(&overflow),
    );

    let started = Instant::now();
    let timeout = Duration::from_millis(command.timeout_ms);
    let (status, termination, cleanup) = loop {
        if cancellation.is_cancelled() {
            let cleanup = terminate_and_reap(&mut child, process_id);
            break (None, OracleTermination::Cancelled, cleanup);
        }
        if overflow.load(Ordering::Acquire) {
            let cleanup = terminate_and_reap(&mut child, process_id);
            break (None, OracleTermination::OutputOverflow, cleanup);
        }
        if started.elapsed() >= timeout {
            let cleanup = terminate_and_reap(&mut child, process_id);
            break (None, OracleTermination::TimedOut, cleanup);
        }
        let waited = if inject_wait_failure {
            Err(std::io::Error::other("injected oracle wait failure"))
        } else {
            child.try_wait()
        };
        match waited {
            Ok(Some(status)) => {
                let _ = terminate_process_tree(process_id);
                break (Some(status), OracleTermination::Completed, true);
            }
            Ok(None) => thread::sleep(Duration::from_millis(2)),
            Err(_) => {
                let cleanup = terminate_and_reap(&mut child, process_id);
                break (None, OracleTermination::SupervisorError, cleanup);
            }
        }
    };

    let stdout = stdout_reader
        .join()
        .expect("bounded stdout reader contains no panic path");
    let stderr = stderr_reader
        .join()
        .expect("bounded stderr reader contains no panic path");
    let input_complete = input_writer.is_none_or(|writer| {
        writer
            .join()
            .expect("bounded stdin writer contains no panic path")
    });
    let cleanup_complete = cleanup & stdout.complete & stderr.complete & input_complete;
    if (termination == OracleTermination::OutputOverflow) | stdout.overflow | stderr.overflow {
        return receipt(
            command,
            Verdict::Error,
            OracleTermination::OutputOverflow,
            false,
            cleanup_complete,
            stdout.retained,
            stderr.retained,
        );
    }
    if termination == OracleTermination::TimedOut {
        return receipt(
            command,
            Verdict::Timeout,
            termination,
            false,
            cleanup_complete,
            stdout.retained,
            stderr.retained,
        );
    }
    if termination == OracleTermination::Cancelled {
        return receipt(
            command,
            Verdict::Error,
            termination,
            false,
            cleanup_complete,
            stdout.retained,
            stderr.retained,
        );
    }
    let Some(status) = status else {
        return receipt(
            command,
            Verdict::Error,
            termination,
            false,
            cleanup_complete,
            stdout.retained,
            stderr.retained,
        );
    };
    if !status.success() {
        return receipt(
            command,
            Verdict::Error,
            OracleTermination::AbnormalExit,
            false,
            cleanup_complete,
            stdout.retained,
            stderr.retained,
        );
    }
    let semantic = parse_semantic_result(&stdout.protocol, &command.result_protocol);
    match semantic {
        Some(verdict) => receipt(
            command,
            verdict,
            OracleTermination::Completed,
            true,
            cleanup_complete,
            stdout.retained,
            stderr.retained,
        ),
        None => receipt(
            command,
            Verdict::Error,
            OracleTermination::MalformedResult,
            false,
            cleanup_complete,
            stdout.retained,
            stderr.retained,
        ),
    }
}

struct StreamResult {
    retained: Vec<u8>,
    protocol: Vec<u8>,
    overflow: bool,
    complete: bool,
}

#[allow(clippy::too_many_arguments)]
fn spawn_reader(
    mut reader: Box<dyn Read + Send>,
    stream_limit: u64,
    combined_limit: u64,
    retention_limit: u64,
    capture_protocol: bool,
    combined: Arc<AtomicU64>,
    overflow: Arc<AtomicBool>,
) -> thread::JoinHandle<StreamResult> {
    thread::spawn(move || {
        let retention_limit = usize::try_from(retention_limit).unwrap_or(usize::MAX);
        let mut retained = Vec::new();
        let mut protocol = Vec::new();
        let mut seen = 0_u64;
        let mut local_overflow = false;
        let mut buffer = [0_u8; 8_192];
        loop {
            let read = reader.read(&mut buffer);
            let Ok(read) = read else {
                return StreamResult {
                    retained,
                    protocol,
                    overflow: local_overflow,
                    complete: false,
                };
            };
            if read == 0 {
                break;
            }
            let read_u64 = u64::try_from(read).unwrap_or(u64::MAX);
            seen = seen.saturating_add(read_u64);
            let previous = combined.fetch_add(read_u64, Ordering::AcqRel);
            if seen > stream_limit || previous.saturating_add(read_u64) > combined_limit {
                local_overflow = true;
                overflow.store(true, Ordering::Release);
            }
            if retained.len() < retention_limit {
                let available = retention_limit - retained.len();
                retained.extend_from_slice(&buffer[..read.min(available)]);
            }
            if capture_protocol && protocol.len() < 4_096 {
                let available = 4_096 - protocol.len();
                protocol.extend_from_slice(&buffer[..read.min(available)]);
            }
        }
        StreamResult {
            retained,
            protocol,
            overflow: local_overflow,
            complete: true,
        }
    })
}

/// Injects one deterministic pipe-read failure through the production reader.
#[cfg(feature = "coverage-controls")]
#[must_use]
#[allow(clippy::expect_used, clippy::missing_panics_doc)]
pub fn oracle_reader_fault_control() -> bool {
    struct FailingReader;
    impl Read for FailingReader {
        fn read(&mut self, _buffer: &mut [u8]) -> std::io::Result<usize> {
            Err(std::io::Error::other("injected oracle read failure"))
        }
    }
    let result = spawn_reader(
        Box::new(FailingReader),
        1,
        1,
        1,
        true,
        Arc::new(AtomicU64::new(0)),
        Arc::new(AtomicBool::new(false)),
    )
    .join()
    .expect("injected reader contains no panic path");
    let capped = spawn_reader(
        Box::new(std::io::Cursor::new(vec![0_u8; 8_193])),
        u64::MAX,
        u64::MAX,
        0,
        true,
        Arc::new(AtomicU64::new(0)),
        Arc::new(AtomicBool::new(false)),
    )
    .join()
    .expect("bounded reader contains no panic path");
    !result.complete & capped.complete & (capped.protocol.len() == 4_096)
}

#[allow(clippy::expect_used)]
fn parse_semantic_result(bytes: &[u8], protocol: &str) -> Option<Verdict> {
    let first_line = bytes
        .split(|byte| *byte == b'\n')
        .next()
        .expect("splitting a byte slice always yields one segment");
    let line = core::str::from_utf8(first_line).ok()?;
    let (schema, verdict) = line.split_once('\t')?;
    if schema != protocol {
        return None;
    }
    match verdict {
        "pass" => Some(Verdict::Pass),
        "fail" => Some(Verdict::Fail),
        "error" => Some(Verdict::Error),
        "not-run" => Some(Verdict::NotRun),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::{CancellationToken, OracleSetError, run_oracles_at_roots_controlled};
    use std::path::PathBuf;

    #[test]
    fn controlled_set_input_boundaries_are_total() {
        let cancellation = CancellationToken::new();
        assert_eq!(
            run_oracles_at_roots_controlled(&[], &[PathBuf::from("extra")], 1, &cancellation),
            Err(OracleSetError::Worker)
        );
        assert_eq!(
            run_oracles_at_roots_controlled(&[], &[], 0, &cancellation),
            Err(OracleSetError::Concurrency)
        );
        assert_eq!(
            run_oracles_at_roots_controlled(&[], &[], 65, &cancellation),
            Err(OracleSetError::Concurrency)
        );
        assert_eq!(
            run_oracles_at_roots_controlled(&[], &[], 1, &cancellation),
            Ok(Vec::new())
        );
    }
}

#[cfg(unix)]
fn configure_process_tree(command: &mut Command) {
    use std::os::unix::process::CommandExt as _;
    command.process_group(0);
}

#[cfg(not(unix))]
fn configure_process_tree(_command: &mut Command) {}

#[cfg(unix)]
#[allow(clippy::expect_used)]
fn terminate_process_tree(process_id: u32) -> bool {
    let raw = i32::try_from(process_id).expect("operating-system process IDs fit in i32");
    let pid = rustix::process::Pid::from_raw(raw).expect("child process IDs are nonzero");
    rustix::process::kill_process_group(pid, rustix::process::Signal::KILL).is_ok()
}

#[cfg(not(unix))]
fn terminate_process_tree(_process_id: u32) -> bool {
    false
}

fn terminate_and_reap(child: &mut Child, process_id: u32) -> bool {
    let tree = terminate_process_tree(process_id);
    let direct = child.kill().is_ok();
    let waited = child.wait().is_ok();
    (tree | direct) & waited
}

fn builtin_verdict(program: &str) -> Option<Verdict> {
    match program {
        "builtin:pass" => Some(Verdict::Pass),
        "builtin:fail" => Some(Verdict::Fail),
        "builtin:error" => Some(Verdict::Error),
        "builtin:timeout" => Some(Verdict::Timeout),
        "builtin:not-run" => Some(Verdict::NotRun),
        _ => None,
    }
}

#[allow(clippy::too_many_arguments)]
fn receipt(
    command: &OracleCommand,
    verdict: Verdict,
    termination: OracleTermination,
    protocol_valid: bool,
    cleanup_complete: bool,
    raw_stdout: Vec<u8>,
    raw_stderr: Vec<u8>,
) -> OracleReceipt {
    OracleReceipt {
        id: command.id.clone(),
        verdict,
        termination,
        protocol_valid,
        cleanup_complete,
        raw_stdout,
        raw_stderr,
        derivation: command.derivation.clone(),
    }
}

#[cfg(all(test, feature = "coverage-controls"))]
#[allow(clippy::expect_used)]
mod coverage_tests {
    use super::*;
    use crate::repository::RepositoryConfig;

    #[test]
    fn mounted_oracle_controls_cover_empty_invalid_and_incomplete_sets() {
        let cancellation = CancellationToken::new();
        assert_eq!(
            run_oracles_at_roots_controlled(&[], &[], 0, &cancellation),
            Err(OracleSetError::Concurrency)
        );
        assert_eq!(
            run_oracles_at_roots_controlled(&[], &[], 1, &cancellation),
            Ok(Vec::new())
        );
        let config = RepositoryConfig::parse(
            br#"schema = "spec-provenance-config/v2"
digest_algorithms = ["xxh3-128"]

[[selectors]]
root = "spec"
adapter = "markdown/v1"
include = ["**/*.md"]
exclude = []
authority_expectation = "optional"

[[oracles]]
id = "ORC-SECOND"
program = "builtin:pass"
timeout_ms = 1000

[[oracles]]
id = "ORC-FIRST"
program = "builtin:pass"
timeout_ms = 1000
"#,
        )
        .expect("oracle set configuration");
        let root = std::env::current_dir().expect("current directory");
        let receipts = run_oracles_at_roots_controlled(
            config.oracles(),
            &[root.clone(), root],
            2,
            &cancellation,
        )
        .expect("mounted oracle set");
        assert_eq!(
            receipts
                .iter()
                .map(|receipt| receipt.id.as_str())
                .collect::<Vec<_>>(),
            ["ORC-FIRST", "ORC-SECOND"]
        );
    }
}
