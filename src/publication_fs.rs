//! Race-resistant, no-follow, atomic publication writes.
//!
//! Every descent stays relative to one retained root descriptor. The writer
//! rejects symlinked ancestors, component swaps, and non-regular existing
//! paths, and it never escapes the declared publication root.

#![allow(missing_docs)]

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WriteError {
    RootEscape,
    LinkAncestor,
    NotRegular,
    Busy,
    Io,
    /// A read-only descent (`create = false`) found a missing parent
    /// component. The declared path is therefore absent; this is not itself
    /// an operation failure.
    Absent,
}

impl WriteError {
    /// The stable, versioned diagnostic code this writer fault reports at
    /// the CLI boundary.
    ///
    /// Every writer failure is a real, distinguishable safety outcome that a
    /// caller must be able to act on differently: a symlinked ancestor
    /// (`TPT-OUTPUT-LINK`) means an attacker or a stale worktree tried to
    /// redirect the write outside the declared root, while a busy output
    /// (`TPT-OUTPUT-BUSY`) simply means another operation holds the lock and
    /// the caller should retry. Collapsing them all into one bare exit code
    /// discards exactly the distinction that makes the outcome actionable.
    #[must_use]
    pub const fn stable_code(self) -> &'static str {
        match self {
            Self::RootEscape => "TPT-OUTPUT-ESCAPE",
            Self::LinkAncestor => "TPT-OUTPUT-LINK",
            Self::NotRegular => "TPT-OUTPUT-NOT-REGULAR",
            Self::Busy => "TPT-OUTPUT-BUSY",
            Self::Io => "TPT-OUTPUT-IO",
            Self::Absent => "TPT-OUTPUT-ABSENT",
        }
    }

    /// The fixed, root- and clock-independent human message this writer
    /// fault reports alongside its stable code.
    #[must_use]
    pub const fn message(self) -> &'static str {
        match self {
            Self::RootEscape => "the declared output path escapes its publication root",
            Self::LinkAncestor => {
                "an output path ancestor is link-like and could redirect the write outside the root"
            }
            Self::NotRegular => "the existing output path is not a regular file",
            Self::Busy => "another operation holds the output lock for this normalized path",
            Self::Io => "the output could not be written or synchronized",
            Self::Absent => "the declared output path is absent",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WriteOutcome {
    Created,
    NoOp,
    Conflict,
    Replaced,
}

/// Splits one relative output path into its parent components and file name.
///
/// Rejects an empty path, a root-escaping `..` component, and an empty
/// component produced by a doubled separator.
///
/// # Errors
///
/// Returns [`WriteError::RootEscape`] for any path that cannot be resolved
/// safely under a retained root descriptor.
pub fn split_relative_path(path: &str) -> Result<(Vec<&str>, &str), WriteError> {
    let mut components: Vec<&str> = path.split('/').collect();
    if components
        .iter()
        .any(|component| component.is_empty() || *component == "..")
    {
        return Err(WriteError::RootEscape);
    }
    // `str::split` always returns at least one item. The validation above
    // rejects an empty final item before this index is computed.
    let filename_index = components.len() - 1;
    let filename = components[filename_index];
    components.truncate(filename_index);
    Ok((components, filename))
}

#[cfg(unix)]
pub mod unix {
    use super::{WriteError, WriteOutcome};
    use crate::publication::Lifecycle;
    use rustix::fd::OwnedFd;
    use rustix::fs::{FileType, Mode, OFlags, fstat, linkat, mkdirat, openat, renameat, unlinkat};
    use std::io::{Read as _, Write as _};
    use std::os::fd::AsFd;

    const DIRECTORY_FLAGS: OFlags = OFlags::RDONLY
        .union(OFlags::CLOEXEC)
        .union(OFlags::NOFOLLOW)
        .union(OFlags::DIRECTORY);

    /// Opens one root directory with no-follow semantics.
    ///
    /// # Errors
    ///
    /// Returns [`WriteError::Io`] when the root cannot be opened, and
    /// [`WriteError::LinkAncestor`] when the root itself is link-like.
    pub fn open_root(path: &std::path::Path) -> Result<OwnedFd, WriteError> {
        rustix::fs::open(path, DIRECTORY_FLAGS, Mode::empty()).map_err(|error| {
            if matches!(error, rustix::io::Errno::LOOP | rustix::io::Errno::NOTDIR) {
                WriteError::LinkAncestor
            } else {
                WriteError::Io
            }
        })
    }

    pub(super) fn descent_error(error: rustix::io::Errno, create: bool) -> WriteError {
        match error {
            rustix::io::Errno::LOOP | rustix::io::Errno::NOTDIR => WriteError::LinkAncestor,
            rustix::io::Errno::NOENT if !create => WriteError::Absent,
            _ => WriteError::Io,
        }
    }

    /// Descends each parent component with no-follow semantics.
    ///
    /// When `create` is `true`, a missing directory is created only when
    /// nothing occupies that name. When `create` is `false` (`check` and
    /// `evidence` never write), a missing component is never created: this
    /// keeps a read-only operation from leaving any incidental directory
    /// behind, and the repository byte-identical (including its directory
    /// set) before and after the operation runs.
    ///
    /// # Errors
    ///
    /// Returns [`WriteError::LinkAncestor`] for a symlink or non-directory
    /// component, [`WriteError::Absent`] for a missing component when
    /// `create` is `false`, and [`WriteError::Io`] for any other descent
    /// failure.
    pub fn descend_parents(
        root: &OwnedFd,
        components: &[&str],
        create: bool,
    ) -> Result<OwnedFd, WriteError> {
        let mut current =
            openat(root, ".", DIRECTORY_FLAGS, Mode::empty()).map_err(|_| WriteError::Io)?;
        for component in components {
            if create {
                // Best-effort creation; the following no-follow open is the
                // actual safety gate regardless of whether this call raced.
                let _ = mkdirat(&current, *component, Mode::RUSR | Mode::WUSR | Mode::XUSR);
            }
            let opened = openat(&current, *component, DIRECTORY_FLAGS, Mode::empty());
            current = match opened {
                Ok(descriptor) => descriptor,
                Err(error) => return Err(descent_error(error, create)),
            };
        }
        Ok(current)
    }

    /// Reads one existing regular sibling's complete bytes without writing.
    ///
    /// Returns `Ok(None)` for an absent path. A parity check (`check` and
    /// `evidence`) calls this directly; it never creates or replaces a file.
    ///
    /// # Errors
    ///
    /// Returns [`WriteError::NotRegular`] for a non-regular existing path,
    /// and [`WriteError::LinkAncestor`] for a symlinked final component.
    pub fn read_regular(parent: &OwnedFd, name: &str) -> Result<Option<Vec<u8>>, WriteError> {
        read_regular_with_fault(parent, name, None)
    }

    fn read_regular_with_fault(
        parent: &OwnedFd,
        name: &str,
        fault: Option<&str>,
    ) -> Result<Option<Vec<u8>>, WriteError> {
        let flags = OFlags::RDONLY | OFlags::CLOEXEC | OFlags::NOFOLLOW | OFlags::NONBLOCK;
        match openat(parent, name, flags, Mode::empty()) {
            Ok(descriptor) => {
                let stat = if fault == Some("stat") {
                    Err(rustix::io::Errno::IO)
                } else {
                    fstat(&descriptor)
                }
                .map_err(|_| WriteError::Io)?;
                if FileType::from_raw_mode(stat.st_mode) != FileType::RegularFile {
                    return Err(WriteError::NotRegular);
                }
                let mut file = std::fs::File::from(descriptor);
                let mut bytes = Vec::new();
                let read = if fault == Some("read") {
                    Err(std::io::Error::other("injected publication read fault"))
                } else {
                    file.read_to_end(&mut bytes)
                };
                read.map_err(|_| WriteError::Io)?;
                Ok(Some(bytes))
            }
            Err(rustix::io::Errno::NOENT) => Ok(None),
            Err(rustix::io::Errno::LOOP) => Err(WriteError::LinkAncestor),
            Err(_) => Err(WriteError::Io),
        }
    }

    #[cfg(feature = "coverage-controls")]
    #[doc(hidden)]
    pub fn read_regular_fault_control(
        parent: &OwnedFd,
        name: &str,
        fault: &str,
    ) -> Result<Option<Vec<u8>>, WriteError> {
        read_regular_with_fault(parent, name, Some(fault))
    }

    fn write_unique_sibling(
        parent: &OwnedFd,
        seed: &str,
        bytes: &[u8],
    ) -> Result<String, WriteError> {
        write_unique_sibling_with_fault(parent, seed, bytes, None)
    }

    /// The real `write_unique_sibling` implementation, with one optional
    /// deterministic fault injection point for otherwise
    /// environment-dependent `write(2)`/`fsync(2)` failures. `fault` is
    /// always `None` in production; only the `coverage-controls`-gated
    /// entry point below ever passes a real fault name, exactly like this
    /// module's sibling snapshot fault-injection controls.
    fn write_unique_sibling_with_fault(
        parent: &OwnedFd,
        seed: &str,
        bytes: &[u8],
        fault: Option<&str>,
    ) -> Result<String, WriteError> {
        for attempt in 0_u32..64 {
            let name = format!(".{seed}.tmp.{attempt}");
            let flags =
                OFlags::WRONLY | OFlags::CREATE | OFlags::EXCL | OFlags::CLOEXEC | OFlags::NOFOLLOW;
            match openat(parent, name.as_str(), flags, Mode::RUSR | Mode::WUSR) {
                Ok(descriptor) => {
                    let mut file = std::fs::File::from(descriptor);
                    let write_result = if fault == Some("write") {
                        Err(std::io::Error::other("injected publication write fault"))
                    } else {
                        file.write_all(bytes)
                    };
                    if let Err(error) = write_result {
                        drop(file);
                        cleanup_temporary(parent, &name);
                        let _ = error;
                        return Err(WriteError::Io);
                    }
                    let sync_result = if fault == Some("sync") {
                        Err(std::io::Error::other("injected publication sync fault"))
                    } else {
                        file.sync_all()
                    };
                    if let Err(error) = sync_result {
                        drop(file);
                        cleanup_temporary(parent, &name);
                        let _ = error;
                        return Err(WriteError::Io);
                    }
                    return Ok(name);
                }
                Err(rustix::io::Errno::EXIST) => {}
                Err(_) => return Err(WriteError::Io),
            }
        }
        Err(WriteError::Io)
    }

    /// Creates, no-ops, or rejects one publication write under an already
    /// no-follow-descended parent directory.
    ///
    /// `immutable` targets use create-if-absent semantics: an absent target
    /// is created, an equal target is a no-op, and an unequal target is
    /// rejected without replacement. `replaceable` targets always use one
    /// same-directory atomic replacement.
    ///
    /// # Errors
    ///
    /// Returns a deterministic [`WriteError`] and never leaves a partially
    /// written target.
    pub fn write_output(
        parent: &OwnedFd,
        filename: &str,
        bytes: &[u8],
        lifecycle: &Lifecycle,
    ) -> Result<WriteOutcome, WriteError> {
        write_output_with_fault(parent, filename, bytes, *lifecycle, None)
    }

    fn write_output_with_fault(
        parent: &OwnedFd,
        filename: &str,
        bytes: &[u8],
        lifecycle: Lifecycle,
        fault: Option<&str>,
    ) -> Result<WriteOutcome, WriteError> {
        match lifecycle {
            Lifecycle::Immutable => {
                if let Some(existing) = read_regular_with_fault(parent, filename, fault)? {
                    return Ok(if existing == bytes {
                        WriteOutcome::NoOp
                    } else {
                        WriteOutcome::Conflict
                    });
                }
                let temp = if fault == Some("temporary-write") {
                    Err(WriteError::Io)
                } else {
                    write_unique_sibling(parent, filename, bytes)
                }?;
                let linked = match fault {
                    Some(
                        "link-exists-equal"
                        | "link-exists-conflict"
                        | "link-exists-read"
                        | "race-write"
                        | "race-rename",
                    ) => {
                        let race_bytes = if fault == Some("link-exists-equal") {
                            bytes
                        } else {
                            b"injected conflicting publication bytes"
                        };
                        let race = if fault == Some("race-write") {
                            Err(WriteError::Io)
                        } else {
                            write_unique_sibling(parent, "race", race_bytes)
                        }?;
                        let renamed = if fault == Some("race-rename") {
                            Err(rustix::io::Errno::IO)
                        } else {
                            renameat(parent, race.as_str(), parent, filename)
                        };
                        renamed.map_err(|_| WriteError::Io)?;
                        Err(rustix::io::Errno::EXIST)
                    }
                    Some("link-exists-absent") => Err(rustix::io::Errno::EXIST),
                    Some("link-io") => Err(rustix::io::Errno::IO),
                    _ => linkat(
                        parent,
                        temp.as_str(),
                        parent,
                        filename,
                        rustix::fs::AtFlags::empty(),
                    ),
                };
                let _ = unlinkat(parent, temp.as_str(), rustix::fs::AtFlags::empty());
                match linked {
                    Ok(()) => Ok(WriteOutcome::Created),
                    Err(rustix::io::Errno::EXIST) => {
                        // A concurrent writer won the create race; report
                        // whatever state it produced rather than fail.
                        let existing = if fault == Some("link-exists-read") {
                            Err(WriteError::Io)
                        } else {
                            read_regular(parent, filename)
                        }?;
                        match existing {
                            Some(existing) if existing == bytes => Ok(WriteOutcome::NoOp),
                            Some(_) => Ok(WriteOutcome::Conflict),
                            None => Err(WriteError::Io),
                        }
                    }
                    Err(_) => Err(WriteError::Io),
                }
            }
            Lifecycle::Replaceable => {
                // A non-regular existing target (symlink, directory, device)
                // must never be silently replaced by an atomic rename: only
                // an absent or regular target is a legitimate replacement.
                // The real prior state (absent, equal, or a genuine
                // replacement) is read here too, so the reported outcome
                // reflects what actually happened rather than always
                // claiming a replacement occurred.
                let existing = match read_regular_with_fault(parent, filename, fault) {
                    Ok(existing) => existing,
                    Err(WriteError::NotRegular) => return Err(WriteError::NotRegular),
                    Err(WriteError::LinkAncestor) => return Err(WriteError::LinkAncestor),
                    Err(_) => return Err(WriteError::Io),
                };
                if existing.as_deref() == Some(bytes) {
                    // The saved target already carries the exact candidate
                    // bytes: no replacement is necessary or performed.
                    return Ok(WriteOutcome::NoOp);
                }
                let was_absent = existing.is_none();
                let temp = if fault == Some("replaceable-temporary-write") {
                    Err(WriteError::Io)
                } else {
                    write_unique_sibling(parent, filename, bytes)
                }?;
                let renamed = if fault == Some("rename") {
                    Err(rustix::io::Errno::IO)
                } else {
                    renameat(parent, temp.as_str(), parent, filename)
                };
                if renamed.is_err() {
                    // The rename failed after the temporary sibling was
                    // created: remove it so a failed replacement never
                    // leaves an operation-owned artifact behind.
                    cleanup_temporary(parent, &temp);
                    return Err(WriteError::Io);
                }
                Ok(if was_absent {
                    WriteOutcome::Created
                } else {
                    WriteOutcome::Replaced
                })
            }
        }
    }

    #[cfg(feature = "coverage-controls")]
    #[doc(hidden)]
    pub fn write_output_fault_control(
        parent: &OwnedFd,
        filename: &str,
        bytes: &[u8],
        lifecycle: &Lifecycle,
        fault: &str,
    ) -> Result<WriteOutcome, WriteError> {
        write_output_with_fault(parent, filename, bytes, *lifecycle, Some(fault))
    }

    /// Removes only a temporary sibling this operation owns.
    pub fn cleanup_temporary(parent: &OwnedFd, name: &str) {
        let _ = unlinkat(parent, name, rustix::fs::AtFlags::empty());
    }

    /// Exercises the REAL `write_unique_sibling` write/sync failure cleanup
    /// path with a deterministic injected fault, rather than depending on
    /// an environment-specific way to force a real `write(2)`/`fsync(2)`
    /// failure (a full disk, a killed process). `fault` must be `"write"`
    /// or `"sync"`.
    ///
    /// # Errors
    ///
    /// Returns [`WriteError::Io`], the same error the real failure path
    /// returns; callers assert on this and on the temporary's absence
    /// afterward.
    #[cfg(feature = "coverage-controls")]
    #[doc(hidden)]
    pub fn write_unique_sibling_fault_control(
        parent: &OwnedFd,
        seed: &str,
        bytes: &[u8],
        fault: &str,
    ) -> Result<String, WriteError> {
        write_unique_sibling_with_fault(parent, seed, bytes, Some(fault))
    }

    /// A bounded interprocess exclusion guard for one normalized output name.
    ///
    /// The guard records the exact inode identity it locked. Ownership means
    /// two facts together: this descriptor holds the exclusive `flock`, AND
    /// the directory entry `lock_name` still resolves to that same inode.
    /// Checking only the first is not enough -- see
    /// [`acquire_output_lock`] -- because an unlinked lock file keeps its
    /// `flock` working perfectly while a newcomer creates a fresh inode
    /// under the same name and locks that one instead.
    pub struct OutputLock {
        file: std::fs::File,
        inode: u64,
    }

    impl OutputLock {
        /// Releases this lock and removes its directory entry, in the one
        /// order that preserves mutual exclusion.
        ///
        /// The entry is unlinked **while the `flock` is still held**, and
        /// only when it still resolves to this guard's own inode. That
        /// ordering is what makes the removal safe:
        ///
        /// * Any waiter blocked on this inode will, after acquiring, observe
        ///   that the name no longer resolves to it and retry against the
        ///   current entry rather than proceed.
        /// * A newcomer that creates a fresh inode under the same name
        ///   cannot have its entry removed by this guard, because the
        ///   identity check fails.
        ///
        /// Releasing first and unlinking afterward -- the obvious order --
        /// is exactly the defect this avoids: between the release and the
        /// unlink another operation can legitimately acquire the lock, and
        /// this guard would then delete the entry that operation owns,
        /// letting a third operation acquire a second concurrent lock.
        pub fn release_and_unlink(self, parent: &OwnedFd, lock_name: &str) {
            if entry_inode(parent, lock_name) == Some(self.inode) {
                let _ = unlinkat(parent, lock_name, rustix::fs::AtFlags::empty());
            }
            drop(self.file);
        }
    }

    /// The inode number the directory entry `name` currently resolves to,
    /// without following a final symlink.
    fn entry_inode(parent: &OwnedFd, name: &str) -> Option<u64> {
        rustix::fs::statat(parent, name, rustix::fs::AtFlags::SYMLINK_NOFOLLOW)
            .ok()
            .map(|stat| stat.st_ino)
    }

    /// Acquires an exclusive advisory lock on one normalized output path.
    ///
    /// A second concurrent caller for the same path either waits within a
    /// bounded number of attempts or fails with [`WriteError::Busy`].
    ///
    /// Acquisition is **verify-after-lock**: taking the `flock` alone is not
    /// sufficient, because a lock file that another operation has already
    /// unlinked still grants its `flock` normally even though the name now
    /// resolves to a different inode (or to nothing). A caller that trusted
    /// the bare `flock` would run concurrently with whichever operation owns
    /// the current entry. After every successful lock this therefore
    /// re-resolves the name and confirms it still points at the very inode
    /// that was locked; a mismatch means the entry was replaced underneath
    /// this caller, so it drops the stale descriptor and retries against the
    /// current one, still inside the same declared bound.
    ///
    /// A process interrupted while holding the lock releases it
    /// automatically: `flock` ownership is tied to the open file
    /// description, which the kernel closes on process exit.
    ///
    /// # Errors
    ///
    /// Returns [`WriteError::Busy`] after the bounded wait elapses, and
    /// [`WriteError::Io`] for any other lock-acquisition failure.
    pub fn acquire_output_lock(
        parent: &OwnedFd,
        lock_name: &str,
        max_attempts: u32,
    ) -> Result<OutputLock, WriteError> {
        acquire_output_lock_with_fault(parent, lock_name, max_attempts, None)
    }

    fn acquire_output_lock_with_fault(
        parent: &OwnedFd,
        lock_name: &str,
        max_attempts: u32,
        fault: Option<&str>,
    ) -> Result<OutputLock, WriteError> {
        let flags = OFlags::WRONLY | OFlags::CREATE | OFlags::CLOEXEC | OFlags::NOFOLLOW;
        for attempt in 0..max_attempts {
            let opened = match fault {
                Some("open-noent") => Err(rustix::io::Errno::NOENT),
                Some("open-loop") => Err(rustix::io::Errno::LOOP),
                Some("open-io") => Err(rustix::io::Errno::IO),
                _ => openat(parent, lock_name, flags, Mode::RUSR | Mode::WUSR),
            };
            let descriptor = match opened {
                Ok(descriptor) => descriptor,
                // A create race is expected under exactly the contention this
                // lock exists to serialize, and is NOT a hard failure.
                //
                // `ENOENT` here is counter-intuitive but real: on some
                // platforms two operations issuing `openat(O_CREAT)` for the
                // same absent name at the same instant can transiently
                // observe `ENOENT` rather than one of them creating the file.
                // `ENOENT` can equally mean the previous holder unlinked the
                // entry between this caller's own attempts, which is the
                // normal release path. Returning immediately in either case
                // would fail an operation that should simply have waited,
                // violating REH-013's "wait within a declared bound OR fail
                // with a deterministic busy-output diagnostic": a hard I/O
                // error is neither. Retry inside the same declared bound and
                // report `Busy` if the bound elapses.
                Err(rustix::io::Errno::NOENT | rustix::io::Errno::EXIST) => {
                    if attempt + 1 == max_attempts {
                        return Err(WriteError::Busy);
                    }
                    std::thread::sleep(std::time::Duration::from_millis(5));
                    continue;
                }
                Err(rustix::io::Errno::LOOP) => return Err(WriteError::LinkAncestor),
                Err(_) => return Err(WriteError::Io),
            };
            let file = std::fs::File::from(descriptor);
            let locked = match fault {
                Some("flock-would-block") => Err(rustix::io::Errno::WOULDBLOCK),
                Some("flock-io") => Err(rustix::io::Errno::IO),
                _ => rustix::fs::flock(
                    file.as_fd(),
                    rustix::fs::FlockOperation::NonBlockingLockExclusive,
                ),
            };
            match locked {
                Ok(()) => {
                    let held = if fault == Some("fstat") {
                        Err(rustix::io::Errno::IO)
                    } else {
                        fstat(file.as_fd())
                    }
                    .map_err(|_| WriteError::Io)?
                    .st_ino;
                    if matches!(fault, Some("replace-entry" | "replace-entry-open")) && attempt == 0
                    {
                        let _ = unlinkat(parent, lock_name, rustix::fs::AtFlags::empty());
                        let replacement = if fault == Some("replace-entry-open") {
                            Err(rustix::io::Errno::IO)
                        } else {
                            openat(
                                parent,
                                lock_name,
                                OFlags::WRONLY | OFlags::CREATE | OFlags::EXCL | OFlags::CLOEXEC,
                                Mode::RUSR | Mode::WUSR,
                            )
                        }
                        .map_err(|_| WriteError::Io)?;
                        drop(replacement);
                    }
                    if entry_inode(parent, lock_name) == Some(held) {
                        return Ok(OutputLock { file, inode: held });
                    }
                    // The entry was replaced or removed after this
                    // descriptor was opened: this lock protects an inode
                    // nobody reaches by name any more. Drop it and retry
                    // against the entry that is current now.
                    drop(file);
                }
                Err(rustix::io::Errno::WOULDBLOCK) => {
                    drop(file);
                    if attempt + 1 == max_attempts {
                        return Err(WriteError::Busy);
                    }
                }
                Err(_) => return Err(WriteError::Io),
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        Err(WriteError::Busy)
    }

    #[cfg(feature = "coverage-controls")]
    #[doc(hidden)]
    pub fn acquire_output_lock_fault_control(
        parent: &OwnedFd,
        lock_name: &str,
        max_attempts: u32,
        fault: &str,
    ) -> Result<OutputLock, WriteError> {
        acquire_output_lock_with_fault(parent, lock_name, max_attempts, Some(fault))
    }
}

#[cfg(all(test, unix))]
#[allow(clippy::expect_used, clippy::too_many_lines)]
mod tests {
    use super::*;
    use std::fs;
    use std::os::unix::fs::symlink;

    #[test]
    fn writer_error_metadata_and_relative_paths_are_closed() {
        for (error, code, message) in [
            (
                WriteError::RootEscape,
                "TPT-OUTPUT-ESCAPE",
                "the declared output path escapes its publication root",
            ),
            (
                WriteError::LinkAncestor,
                "TPT-OUTPUT-LINK",
                "an output path ancestor is link-like and could redirect the write outside the root",
            ),
            (
                WriteError::NotRegular,
                "TPT-OUTPUT-NOT-REGULAR",
                "the existing output path is not a regular file",
            ),
            (
                WriteError::Busy,
                "TPT-OUTPUT-BUSY",
                "another operation holds the output lock for this normalized path",
            ),
            (
                WriteError::Io,
                "TPT-OUTPUT-IO",
                "the output could not be written or synchronized",
            ),
            (
                WriteError::Absent,
                "TPT-OUTPUT-ABSENT",
                "the declared output path is absent",
            ),
        ] {
            assert_eq!(error.stable_code(), code);
            assert_eq!(error.message(), message);
        }
        assert_eq!(
            split_relative_path("parent/output.md"),
            Ok((vec!["parent"], "output.md"))
        );
        for invalid in ["", "/output.md", "parent//output.md", "../output.md"] {
            assert_eq!(split_relative_path(invalid), Err(WriteError::RootEscape));
        }
        assert_eq!(
            unix::descent_error(rustix::io::Errno::LOOP, true),
            WriteError::LinkAncestor
        );
        assert_eq!(
            unix::descent_error(rustix::io::Errno::NOTDIR, false),
            WriteError::LinkAncestor
        );
        assert_eq!(
            unix::descent_error(rustix::io::Errno::NOENT, false),
            WriteError::Absent
        );
        assert_eq!(
            unix::descent_error(rustix::io::Errno::NOENT, true),
            WriteError::Io
        );
        assert_eq!(
            unix::descent_error(rustix::io::Errno::INVAL, false),
            WriteError::Io
        );
    }

    #[cfg(feature = "coverage-controls")]
    #[test]
    fn unix_writer_controls_cover_real_descriptor_failure_classes() {
        use crate::publication::Lifecycle;

        let root = tempfile::TempDir::new().expect("publication root");
        let parent = unix::open_root(root.path()).expect("root descriptor");
        assert_eq!(
            unix::open_root(&root.path().join("missing")).err(),
            Some(WriteError::Io)
        );
        let linked_root = root.path().join("linked-root");
        symlink(root.path(), &linked_root).expect("linked root");
        assert_eq!(
            unix::open_root(&linked_root).err(),
            Some(WriteError::LinkAncestor)
        );

        assert_eq!(
            unix::descend_parents(&parent, &["missing"], false).err(),
            Some(WriteError::Absent)
        );
        let nested = unix::descend_parents(&parent, &["created", "nested"], true)
            .expect("created descendants");
        fs::write(root.path().join("regular-parent"), b"regular").expect("regular ancestor");
        let invalid_parent: rustix::fd::OwnedFd =
            fs::File::open(root.path().join("regular-parent"))
                .expect("invalid parent descriptor")
                .into();
        assert_eq!(
            unix::descend_parents(&invalid_parent, &[], false).err(),
            Some(WriteError::Io)
        );
        assert_eq!(
            unix::descend_parents(&parent, &["regular-parent", "child"], false).err(),
            Some(WriteError::LinkAncestor)
        );

        assert_eq!(unix::read_regular(&nested, "absent"), Ok(None));
        fs::write(root.path().join("created/nested/regular"), b"bytes").expect("regular output");
        assert_eq!(
            unix::read_regular(&nested, "regular"),
            Ok(Some(b"bytes".to_vec()))
        );
        fs::create_dir(root.path().join("created/nested/directory")).expect("directory output");
        assert_eq!(
            unix::read_regular(&nested, "directory"),
            Err(WriteError::NotRegular)
        );
        symlink("regular", root.path().join("created/nested/link")).expect("linked output");
        assert_eq!(
            unix::read_regular(&nested, "link"),
            Err(WriteError::LinkAncestor)
        );
        assert_eq!(
            unix::read_regular(&nested, &"x".repeat(5000)),
            Err(WriteError::Io)
        );
        for fault in ["stat", "read"] {
            assert_eq!(
                unix::read_regular_fault_control(&nested, "regular", fault),
                Err(WriteError::Io)
            );
        }

        for fault in ["write", "sync"] {
            assert_eq!(
                unix::write_unique_sibling_fault_control(&nested, fault, b"candidate", fault),
                Err(WriteError::Io)
            );
            assert!(
                !root
                    .path()
                    .join(format!("created/nested/.{fault}.tmp.0"))
                    .exists()
            );
        }
        for attempt in 0..64 {
            fs::write(
                root.path()
                    .join(format!("created/nested/.exhausted.tmp.{attempt}")),
                b"occupied",
            )
            .expect("occupied temporary");
        }
        assert_eq!(
            unix::write_unique_sibling_fault_control(&nested, "exhausted", b"candidate", "none"),
            Err(WriteError::Io)
        );
        assert_eq!(
            unix::write_unique_sibling_fault_control(
                &nested,
                &"x".repeat(5000),
                b"candidate",
                "none",
            ),
            Err(WriteError::Io)
        );

        assert_eq!(
            unix::write_output(&nested, "immutable", b"first", &Lifecycle::Immutable),
            Ok(WriteOutcome::Created)
        );
        assert_eq!(
            unix::write_output(&nested, "immutable", b"first", &Lifecycle::Immutable),
            Ok(WriteOutcome::NoOp)
        );
        assert_eq!(
            unix::write_output(&nested, "immutable", b"second", &Lifecycle::Immutable),
            Ok(WriteOutcome::Conflict)
        );
        assert_eq!(
            unix::write_output(&nested, "replaceable", b"first", &Lifecycle::Replaceable),
            Ok(WriteOutcome::Created)
        );
        assert_eq!(
            unix::write_output(&nested, "replaceable", b"first", &Lifecycle::Replaceable),
            Ok(WriteOutcome::NoOp)
        );
        assert_eq!(
            unix::write_output(&nested, "replaceable", b"second", &Lifecycle::Replaceable),
            Ok(WriteOutcome::Replaced)
        );
        assert_eq!(
            unix::write_output(&nested, "directory", b"candidate", &Lifecycle::Replaceable),
            Err(WriteError::NotRegular)
        );
        assert_eq!(
            unix::write_output(&nested, "link", b"candidate", &Lifecycle::Replaceable),
            Err(WriteError::LinkAncestor)
        );
        for (fault, expected) in [
            ("link-exists-equal", Ok(WriteOutcome::NoOp)),
            ("link-exists-conflict", Ok(WriteOutcome::Conflict)),
            ("link-exists-absent", Err(WriteError::Io)),
            ("link-exists-read", Err(WriteError::Io)),
            ("link-io", Err(WriteError::Io)),
            ("temporary-write", Err(WriteError::Io)),
            ("race-write", Err(WriteError::Io)),
            ("race-rename", Err(WriteError::Io)),
        ] {
            assert_eq!(
                unix::write_output_fault_control(
                    &nested,
                    &format!("immutable-{fault}"),
                    b"candidate",
                    &Lifecycle::Immutable,
                    fault,
                ),
                expected
            );
        }
        fs::write(
            root.path().join("created/nested/immutable-read-fault"),
            b"prior",
        )
        .expect("immutable read-fault prior output");
        assert_eq!(
            unix::write_output_fault_control(
                &nested,
                "immutable-read-fault",
                b"candidate",
                &Lifecycle::Immutable,
                "read",
            ),
            Err(WriteError::Io)
        );
        fs::write(
            root.path().join("created/nested/replaceable-read-fault"),
            b"prior",
        )
        .expect("read-fault prior output");
        assert_eq!(
            unix::write_output_fault_control(
                &nested,
                "replaceable-read-fault",
                b"candidate",
                &Lifecycle::Replaceable,
                "read",
            ),
            Err(WriteError::Io)
        );
        assert_eq!(
            unix::write_output_fault_control(
                &nested,
                "replaceable-rename-fault",
                b"candidate",
                &Lifecycle::Replaceable,
                "rename",
            ),
            Err(WriteError::Io)
        );
        assert_eq!(
            unix::write_output_fault_control(
                &nested,
                "replaceable-temporary-fault",
                b"candidate",
                &Lifecycle::Replaceable,
                "replaceable-temporary-write",
            ),
            Err(WriteError::Io)
        );

        assert_eq!(
            unix::acquire_output_lock(&nested, ".zero-lock", 0).err(),
            Some(WriteError::Busy)
        );
        assert_eq!(
            unix::acquire_output_lock(&nested, &"x".repeat(5000), 1).err(),
            Some(WriteError::Io)
        );
        symlink("regular", root.path().join("created/nested/.linked-lock")).expect("linked lock");
        assert_eq!(
            unix::acquire_output_lock(&nested, ".linked-lock", 1).err(),
            Some(WriteError::LinkAncestor)
        );
        for (fault, attempts, expected) in [
            ("open-noent", 2, WriteError::Busy),
            ("open-loop", 1, WriteError::LinkAncestor),
            ("open-io", 1, WriteError::Io),
            ("flock-would-block", 1, WriteError::Busy),
            ("flock-io", 1, WriteError::Io),
            ("fstat", 1, WriteError::Io),
            ("replace-entry-open", 1, WriteError::Io),
        ] {
            assert_eq!(
                unix::acquire_output_lock_fault_control(
                    &nested,
                    &format!(".{fault}-lock"),
                    attempts,
                    fault,
                )
                .err(),
                Some(expected)
            );
        }
        let replaced_entry = unix::acquire_output_lock_fault_control(
            &nested,
            ".replace-entry-lock",
            2,
            "replace-entry",
        )
        .expect("replaced lock entry retry");
        replaced_entry.release_and_unlink(&nested, ".replace-entry-lock");

        let lock = unix::acquire_output_lock(&nested, ".output-lock", 1).expect("output lock");
        assert_eq!(
            unix::acquire_output_lock(&nested, ".output-lock", 1).err(),
            Some(WriteError::Busy)
        );
        lock.release_and_unlink(&nested, ".output-lock");
        let replaced = unix::acquire_output_lock(&nested, ".replaced-lock", 1)
            .expect("replaceable lock entry");
        fs::remove_file(root.path().join("created/nested/.replaced-lock"))
            .expect("remove held lock entry");
        fs::write(
            root.path().join("created/nested/.replaced-lock"),
            b"replacement",
        )
        .expect("replacement lock entry");
        replaced.release_and_unlink(&nested, ".replaced-lock");
        assert!(root.path().join("created/nested/.replaced-lock").exists());
    }
}
