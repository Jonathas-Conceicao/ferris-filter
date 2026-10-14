//! Guarded, backed-up, atomic filter write (EDIT-09, EDIT-10; D-07..D-10).
//!
//! `write_guarded` refuses to write when the file changed on disk since it was loaded, keeps a
//! one-time `<name>.ffbak` copy of the original, and replaces the target through a same-directory
//! temp file, retrying briefly on Windows sharing violations (the game folder is under OneDrive).
//!
//! The library never prints and every path is a parameter. The window between the re-read guard
//! and the rename is a known, accepted TOCTOU: the guard catches edits made minutes apart by other
//! tools, not millisecond races.

use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::time::Duration;

/// Maximum persist attempts on a sharing violation.
pub const MAX_ATTEMPTS: u32 = 5;
/// First backoff delay; doubles each retry (50, 100, 200, 400 ms).
const BASE_DELAY_MS: u64 = 50;

/// What a successful call did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WriteOutcome {
    /// New bytes equal the loaded bytes: nothing was touched.
    Unchanged,
    /// The target was replaced. `backup` is `Some` only when this call created the `.ffbak`.
    Written { backup: Option<PathBuf> },
}

/// Everything that can go wrong writing a filter.
#[derive(Debug, thiserror::Error, miette::Diagnostic)]
pub enum WriteError {
    /// The destination folder does not exist; never created.
    #[error(
        "Destination folder not found: `{path}` — make sure Path of Exile 2 has been run at least once, or pass an explicit destination."
    )]
    DestinationMissing { path: String },

    /// The file differs from what was loaded.
    #[error("`{path}` changed on disk since it was loaded; nothing was written.")]
    #[diagnostic(help(
        "Reload it to pick up the new version (this discards your unsaved changes), then apply again."
    ))]
    ChangedOnDisk { path: String },

    /// The re-read failed for a reason other than "not found".
    #[error("Cannot read `{path}`: {source}")]
    Unreadable {
        path: String,
        #[source]
        source: io::Error,
    },

    /// The backup could not be made; the target was not touched.
    #[error("Cannot create the backup `{path}`: {source}; the filter was not written")]
    Backup {
        path: String,
        #[source]
        source: io::Error,
    },

    /// A non-retriable write failure.
    #[error("Cannot write `{path}`: {source}")]
    Unwritable {
        path: String,
        #[source]
        source: io::Error,
    },

    /// The target is read-only, so it cannot be replaced; not retried.
    #[error(
        "Cannot write `{path}`: the file is read-only; clear its read-only attribute and apply again"
    )]
    ReadOnly {
        path: String,
        #[source]
        source: io::Error,
    },

    /// The target stayed locked through every retry.
    #[error(
        "`{path}` is in use (tried {attempts} times); close anything holding it, such as the game or OneDrive, and apply again"
    )]
    Busy {
        path: String,
        attempts: u32,
        #[source]
        source: io::Error,
    },
}

/// `<dir>/<name>.ffbak` for `<dir>/<name>`.
pub fn backup_path(path: &Path) -> PathBuf {
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    path.with_file_name(format!("{name}.ffbak"))
}

/// Create the one-time `<name>.ffbak` for `path` holding `original`, if it does not exist yet.
/// `Some(path)` when this call created it, `None` when a backup was already there (never replaced).
pub fn ensure_backup(path: &Path, original: &[u8]) -> Result<Option<PathBuf>, WriteError> {
    let bp = backup_path(path);
    let dir = match path.parent() {
        Some(p) if !p.as_os_str().is_empty() => p,
        _ => Path::new("."),
    };
    if !dir.is_dir() {
        return Err(WriteError::DestinationMissing {
            path: dir.display().to_string(),
        });
    }
    Ok(create_backup_with(&bp, dir, original, |f, bytes| f.write_all(bytes))?.then_some(bp))
}

/// Write `new_bytes` to `path`, guarded by the exact bytes read at load (`None` = did not exist).
pub fn write_guarded(
    path: &Path,
    loaded: Option<&[u8]>,
    new_bytes: &[u8],
) -> Result<WriteOutcome, WriteError> {
    write_guarded_with(
        path,
        loaded,
        new_bytes,
        |temp, target| temp.persist(target),
        std::thread::sleep,
    )
}

/// Windows sharing/lock violations: `ERROR_ACCESS_DENIED` (5), `ERROR_SHARING_VIOLATION` (32),
/// `ERROR_LOCK_VIOLATION` (33). These are Windows error codes; elsewhere the same numbers mean
/// unrelated things (`EIO`, `EPIPE`, `EDOM`), so nothing is retried there.
#[cfg(windows)]
fn is_retriable(e: &io::Error) -> bool {
    matches!(e.raw_os_error(), Some(5 | 32 | 33))
}

#[cfg(not(windows))]
fn is_retriable(_e: &io::Error) -> bool {
    false
}

/// True when `path` exists and carries the read-only attribute. A replace onto such a target fails
/// with access denied, which is not a lock and never clears by waiting.
fn is_read_only(path: &Path) -> bool {
    fs::metadata(path).is_ok_and(|m| m.permissions().readonly())
}

/// Create the one-time backup `bp` holding `original`. Returns `Ok(true)` when this call created
/// it and `Ok(false)` when a backup file already exists (it is never overwritten).
///
/// The bytes go to a same-directory temp file, are synced, and only then published under the
/// final name without clobbering. A write that fails partway therefore never leaves a truncated
/// `.ffbak` behind to block every later backup: the temp file is deleted on drop. `write` is the
/// byte writer, injectable so tests can fail it partway.
fn create_backup_with(
    bp: &Path,
    dest_dir: &Path,
    original: &[u8],
    write: impl FnOnce(&mut fs::File, &[u8]) -> io::Result<()>,
) -> Result<bool, WriteError> {
    let err = |source: io::Error| WriteError::Backup {
        path: bp.display().to_string(),
        source,
    };
    if bp.is_file() {
        return Ok(false);
    }
    if bp.exists() {
        return Err(err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            "the backup path exists and is not a file",
        )));
    }
    let mut t = tempfile::NamedTempFile::new_in(dest_dir).map_err(err)?;
    write(t.as_file_mut(), original)
        .and_then(|_| t.as_file().sync_all())
        .map_err(err)?;
    match t.persist_noclobber(bp) {
        Ok(_) => Ok(true),
        // Lost a race with another writer that published a backup first: keep theirs.
        Err(e) if e.error.kind() == io::ErrorKind::AlreadyExists && bp.is_file() => Ok(false),
        Err(e) => Err(err(e.error)),
    }
}

fn write_guarded_with(
    path: &Path,
    loaded: Option<&[u8]>,
    new_bytes: &[u8],
    mut persist: impl FnMut(tempfile::NamedTempFile, &Path) -> Result<fs::File, tempfile::PersistError>,
    mut sleep: impl FnMut(Duration),
) -> Result<WriteOutcome, WriteError> {
    let shown = || path.display().to_string();

    // 1. Destination directory must exist; never create it.
    let dest_dir = match path.parent() {
        Some(p) if !p.as_os_str().is_empty() => p,
        _ => Path::new("."),
    };
    if !dest_dir.is_dir() {
        return Err(WriteError::DestinationMissing {
            path: dest_dir.display().to_string(),
        });
    }

    // 2. Changed-on-disk guard: full-bytes compare.
    let current = match fs::read(path) {
        Ok(b) => Some(b),
        Err(e) if e.kind() == io::ErrorKind::NotFound => None,
        Err(e) => {
            return Err(WriteError::Unreadable {
                path: shown(),
                source: e,
            });
        }
    };
    if current.as_deref() != loaded {
        return Err(WriteError::ChangedOnDisk { path: shown() });
    }

    // 3. No-op write.
    if loaded == Some(new_bytes) {
        return Ok(WriteOutcome::Unchanged);
    }

    // 4. One-time backup of the exact loaded bytes.
    let mut backup = None;
    if let Some(original) = loaded {
        let bp = backup_path(path);
        if create_backup_with(&bp, dest_dir, original, |f, bytes| f.write_all(bytes))? {
            backup = Some(bp);
        }
    }

    // 5. Same-directory temp file, persisted with bounded retry.
    let mut temp =
        tempfile::NamedTempFile::new_in(dest_dir).map_err(|e| WriteError::Unwritable {
            path: shown(),
            source: e,
        })?;
    // sync_all, not flush: the data must be durable before the rename publishes it, or a crash
    // in between can leave an empty or partial filter under the final name.
    temp.write_all(new_bytes)
        .and_then(|_| temp.as_file().sync_all())
        .map_err(|e| WriteError::Unwritable {
            path: shown(),
            source: e,
        })?;

    persist_with_retry(temp, path, &mut persist, &mut sleep)?;
    Ok(WriteOutcome::Written { backup })
}

/// Persist `temp` onto `path` with the bounded retry policy shared by every writer here: a
/// read-only target is reported as such and never retried, a non-retriable error stops at once,
/// and a sharing violation backs off 50, 100, 200, 400 ms before giving up as `Busy`.
fn persist_with_retry(
    mut temp: tempfile::NamedTempFile,
    path: &Path,
    persist: &mut impl FnMut(tempfile::NamedTempFile, &Path) -> Result<fs::File, tempfile::PersistError>,
    sleep: &mut impl FnMut(Duration),
) -> Result<(), WriteError> {
    let shown = || path.display().to_string();
    let mut delay = Duration::from_millis(BASE_DELAY_MS);
    let mut attempt = 1;
    loop {
        match persist(temp, path) {
            Ok(_) => return Ok(()),
            Err(e) => {
                if is_read_only(path) {
                    return Err(WriteError::ReadOnly {
                        path: shown(),
                        source: e.error,
                    });
                }
                if !is_retriable(&e.error) {
                    return Err(WriteError::Unwritable {
                        path: shown(),
                        source: e.error,
                    });
                }
                if attempt >= MAX_ATTEMPTS {
                    return Err(WriteError::Busy {
                        path: shown(),
                        attempts: attempt,
                        source: e.error,
                    });
                }
                sleep(delay);
                delay *= 2;
                attempt += 1;
                temp = e.file;
            }
        }
    }
}

/// One file of a multi-file write.
#[derive(Debug, Clone, Copy)]
pub struct Target<'a> {
    pub path: &'a Path,
    /// The exact bytes read at load (`None` = the file did not exist).
    pub loaded: Option<&'a [u8]>,
    pub new: &'a [u8],
}

/// Why a multi-file write did not complete.
#[derive(Debug, thiserror::Error, miette::Diagnostic)]
pub enum AllOrNoneError {
    /// Refused before any file was replaced; every file is untouched.
    #[error("{0}")]
    NothingWritten(WriteError),

    /// A later file failed; the earlier ones were restored from the loaded bytes.
    #[error("{failed} Every file already replaced was restored, so nothing was changed.")]
    RolledBack { failed: WriteError },

    /// A later file failed and at least one earlier file could not be restored.
    #[error("{failed} {}", unrestored_text(.unrestored))]
    RollbackFailed {
        failed: WriteError,
        unrestored: Vec<PathBuf>,
    },
}

fn unrestored_text(paths: &[PathBuf]) -> String {
    paths
        .iter()
        .map(|p| {
            format!(
                "{} could not be restored to its previous contents.",
                p.display()
            )
        })
        .collect::<Vec<_>>()
        .join(" ")
}

impl AllOrNoneError {
    /// The path of a changed-on-disk refusal, so the caller can raise its outside-edit prompt.
    pub fn changed_on_disk_path(&self) -> Option<&str> {
        match self {
            AllOrNoneError::NothingWritten(WriteError::ChangedOnDisk { path }) => Some(path),
            _ => None,
        }
    }
}

/// Replace every target or none of them. Windows has no cross-file atomic rename, so: guard all,
/// stage all, preflight all, persist in order, and on a failure restore the already replaced files
/// from the in-memory loaded bytes. No `.ffbak` is made; the in-memory bytes are the safety net.
pub fn write_all_or_none(targets: &[Target]) -> Result<Vec<WriteOutcome>, AllOrNoneError> {
    write_all_or_none_with(
        targets,
        |temp, target| temp.persist(target),
        std::thread::sleep,
    )
}

fn parent_of(path: &Path) -> &Path {
    match path.parent() {
        Some(p) if !p.as_os_str().is_empty() => p,
        _ => Path::new("."),
    }
}

fn stage(path: &Path, bytes: &[u8]) -> Result<tempfile::NamedTempFile, WriteError> {
    let err = |source: io::Error| WriteError::Unwritable {
        path: path.display().to_string(),
        source,
    };
    let mut t = tempfile::NamedTempFile::new_in(parent_of(path)).map_err(err)?;
    t.write_all(bytes)
        .and_then(|_| t.as_file().sync_all())
        .map_err(err)?;
    Ok(t)
}

/// Test hook: `write_all_or_none` with an injectable persist (for forced rename failures).
pub(crate) fn write_all_or_none_with(
    targets: &[Target],
    mut persist: impl FnMut(tempfile::NamedTempFile, &Path) -> Result<fs::File, tempfile::PersistError>,
    mut sleep: impl FnMut(Duration),
) -> Result<Vec<WriteOutcome>, AllOrNoneError> {
    let nothing = AllOrNoneError::NothingWritten;

    // a. Guard every target before touching anything.
    for t in targets {
        let dir = parent_of(t.path);
        if !dir.is_dir() {
            return Err(nothing(WriteError::DestinationMissing {
                path: dir.display().to_string(),
            }));
        }
        let current = match fs::read(t.path) {
            Ok(b) => Some(b),
            Err(e) if e.kind() == io::ErrorKind::NotFound => None,
            Err(e) => {
                return Err(nothing(WriteError::Unreadable {
                    path: t.path.display().to_string(),
                    source: e,
                }));
            }
        };
        if current.as_deref() != t.loaded {
            return Err(nothing(WriteError::ChangedOnDisk {
                path: t.path.display().to_string(),
            }));
        }
    }

    // b. Unchanged targets drop out.
    let pending: Vec<usize> = (0..targets.len())
        .filter(|&i| targets[i].loaded != Some(targets[i].new))
        .collect();

    // c. Stage every remaining target.
    let mut staged = Vec::with_capacity(pending.len());
    for &i in &pending {
        staged.push(stage(targets[i].path, targets[i].new).map_err(nothing)?);
    }

    // d. Preflight: a read-only or locked target is found before any rename.
    for &i in &pending {
        let path = targets[i].path;
        if !path.exists() {
            continue;
        }
        let shown = path.display().to_string();
        if is_read_only(path) {
            return Err(nothing(WriteError::ReadOnly {
                path: shown,
                source: io::Error::from(io::ErrorKind::PermissionDenied),
            }));
        }
        if let Err(e) = fs::OpenOptions::new().write(true).open(path) {
            return Err(nothing(if is_retriable(&e) {
                WriteError::Busy {
                    path: shown,
                    attempts: 1,
                    source: e,
                }
            } else {
                WriteError::Unwritable {
                    path: shown,
                    source: e,
                }
            }));
        }
    }

    // e. Persist in input order; restore on failure.
    let mut replaced: Vec<usize> = Vec::new();
    for (&i, temp) in pending.iter().zip(staged) {
        match persist_with_retry(temp, targets[i].path, &mut persist, &mut sleep) {
            Ok(()) => replaced.push(i),
            Err(failed) => {
                let mut unrestored = Vec::new();
                for &j in replaced.iter().rev() {
                    let t = &targets[j];
                    let ok = match t.loaded {
                        Some(orig) => stage(t.path, orig)
                            .and_then(|tmp| {
                                persist_with_retry(tmp, t.path, &mut persist, &mut sleep)
                            })
                            .is_ok(),
                        None => fs::remove_file(t.path).is_ok(),
                    };
                    if !ok {
                        unrestored.push(t.path.to_path_buf());
                    }
                }
                return Err(if unrestored.is_empty() {
                    AllOrNoneError::RolledBack { failed }
                } else {
                    AllOrNoneError::RollbackFailed { failed, unrestored }
                });
            }
        }
    }

    // f. Report per input target.
    Ok((0..targets.len())
        .map(|i| {
            if pending.contains(&i) {
                WriteOutcome::Written { backup: None }
            } else {
                WriteOutcome::Unchanged
            }
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(dir: &Path) -> Vec<String> {
        let mut v: Vec<String> = fs::read_dir(dir)
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        v.sort();
        v
    }

    #[test]
    fn changed_on_disk_refuses_and_leaves_nothing() {
        let t = tempfile::tempdir().unwrap();
        let p = t.path().join("x.filter");
        fs::write(&p, "B").unwrap();
        let r = write_guarded(&p, Some(b"A"), b"C");
        assert!(matches!(r, Err(WriteError::ChangedOnDisk { .. })));
        assert_eq!(fs::read(&p).unwrap(), b"B");
        assert_eq!(names(t.path()), vec!["x.filter"]);
    }

    #[test]
    fn vanished_and_appeared_are_changed() {
        let t = tempfile::tempdir().unwrap();
        let p = t.path().join("x.filter");
        let r = write_guarded(&p, Some(b"A"), b"B");
        assert!(matches!(r, Err(WriteError::ChangedOnDisk { .. })));
        fs::write(&p, "Z").unwrap();
        let r = write_guarded(&p, None, b"B");
        assert!(matches!(r, Err(WriteError::ChangedOnDisk { .. })));
        assert_eq!(names(t.path()), vec!["x.filter"]);
    }

    #[test]
    fn identical_bytes_are_unchanged() {
        let t = tempfile::tempdir().unwrap();
        let p = t.path().join("x.filter");
        fs::write(&p, "A").unwrap();
        let before = fs::metadata(&p).unwrap().modified().unwrap();
        let r = write_guarded(&p, Some(b"A"), b"A").unwrap();
        assert_eq!(r, WriteOutcome::Unchanged);
        assert_eq!(fs::metadata(&p).unwrap().modified().unwrap(), before);
        assert_eq!(names(t.path()), vec!["x.filter"]);
    }

    #[test]
    fn first_write_makes_backup_second_does_not_overwrite_it() {
        let t = tempfile::tempdir().unwrap();
        let p = t.path().join("x.filter");
        fs::write(&p, "A").unwrap();
        let r = write_guarded(&p, Some(b"A"), b"B").unwrap();
        let bp = t.path().join("x.filter.ffbak");
        assert_eq!(
            r,
            WriteOutcome::Written {
                backup: Some(bp.clone())
            }
        );
        assert_eq!(backup_path(&p), bp);
        assert_eq!(fs::read(&bp).unwrap(), b"A");
        assert_eq!(fs::read(&p).unwrap(), b"B");

        let r = write_guarded(&p, Some(b"B"), b"C").unwrap();
        assert_eq!(r, WriteOutcome::Written { backup: None });
        assert_eq!(fs::read(&bp).unwrap(), b"A");
        assert_eq!(fs::read(&p).unwrap(), b"C");
        assert_eq!(names(t.path()), vec!["x.filter", "x.filter.ffbak"]);
    }

    #[test]
    fn new_file_gets_no_backup() {
        let t = tempfile::tempdir().unwrap();
        let p = t.path().join("x.filter");
        let r = write_guarded(&p, None, b"B").unwrap();
        assert_eq!(r, WriteOutcome::Written { backup: None });
        assert_eq!(names(t.path()), vec!["x.filter"]);
    }

    #[test]
    fn backup_failure_aborts_before_touching_target() {
        let t = tempfile::tempdir().unwrap();
        let p = t.path().join("x.filter");
        fs::write(&p, "A").unwrap();
        fs::create_dir(t.path().join("x.filter.ffbak")).unwrap();
        let r = write_guarded(&p, Some(b"A"), b"B");
        assert!(matches!(r, Err(WriteError::Backup { .. })), "{r:?}");
        assert_eq!(fs::read(&p).unwrap(), b"A");
        assert_eq!(names(t.path()), vec!["x.filter", "x.filter.ffbak"]);
    }

    #[test]
    fn failed_backup_write_leaves_no_partial_ffbak() {
        let t = tempfile::tempdir().unwrap();
        let p = t.path().join("x.filter");
        fs::write(&p, "ABCDEF").unwrap();
        let bp = backup_path(&p);

        // Fail partway: half the bytes land, then the write errors.
        let r = create_backup_with(&bp, t.path(), b"ABCDEF", |f, bytes| {
            f.write_all(&bytes[..3])?;
            Err(io::Error::other("disk full"))
        });
        assert!(matches!(r, Err(WriteError::Backup { .. })), "{r:?}");
        assert_eq!(
            names(t.path()),
            vec!["x.filter"],
            "no .ffbak and no temp file left"
        );

        // The next write still gets a complete backup.
        let r = write_guarded(&p, Some(b"ABCDEF"), b"B").unwrap();
        assert_eq!(
            r,
            WriteOutcome::Written {
                backup: Some(bp.clone())
            }
        );
        assert_eq!(fs::read(&bp).unwrap(), b"ABCDEF");
    }

    #[test]
    fn missing_parent_is_error_and_not_created() {
        let t = tempfile::tempdir().unwrap();
        let p = t.path().join("sub").join("x.filter");
        let r = write_guarded(&p, None, b"B");
        assert!(matches!(r, Err(WriteError::DestinationMissing { .. })));
        assert!(!t.path().join("sub").exists());
    }

    fn failing_persist(
        os_error: i32,
        fail_times: u32,
    ) -> impl FnMut(tempfile::NamedTempFile, &Path) -> Result<fs::File, tempfile::PersistError>
    {
        let mut calls = 0;
        move |file, target| {
            calls += 1;
            if calls <= fail_times {
                Err(tempfile::PersistError {
                    error: io::Error::from_raw_os_error(os_error),
                    file,
                })
            } else {
                file.persist(target)
            }
        }
    }

    #[cfg(windows)]
    #[test]
    fn retries_sharing_violation_then_succeeds() {
        let t = tempfile::tempdir().unwrap();
        let p = t.path().join("x.filter");
        let mut sleeps = Vec::new();
        let r =
            write_guarded_with(&p, None, b"B", failing_persist(32, 2), |d| sleeps.push(d)).unwrap();
        assert_eq!(r, WriteOutcome::Written { backup: None });
        assert_eq!(fs::read(&p).unwrap(), b"B");
        assert_eq!(sleeps.len(), 2);
        assert!(sleeps[1] > sleeps[0]);
        assert_eq!(names(t.path()), vec!["x.filter"]);
    }

    #[cfg(windows)]
    #[test]
    fn persistent_lock_is_busy_and_cleans_up() {
        let t = tempfile::tempdir().unwrap();
        let p = t.path().join("x.filter");
        let mut sleeps = 0;
        let r = write_guarded_with(&p, None, b"B", failing_persist(5, u32::MAX), |_| {
            sleeps += 1
        });
        match r {
            Err(e @ WriteError::Busy { .. }) => {
                assert!(matches!(
                    e,
                    WriteError::Busy {
                        attempts: MAX_ATTEMPTS,
                        ..
                    }
                ));
                assert!(e.to_string().contains("x.filter"));
            }
            other => panic!("{other:?}"),
        }
        assert_eq!(sleeps, MAX_ATTEMPTS - 1);
        assert!(names(t.path()).is_empty());
    }

    #[test]
    fn read_only_target_is_reported_as_read_only_not_busy() {
        let t = tempfile::tempdir().unwrap();
        let p = t.path().join("x.filter");
        fs::write(&p, "A").unwrap();
        let mut perms = fs::metadata(&p).unwrap().permissions();
        perms.set_readonly(true);
        fs::set_permissions(&p, perms.clone()).unwrap();

        let mut sleeps = 0;
        let r = write_guarded_with(&p, Some(b"A"), b"B", failing_persist(5, u32::MAX), |_| {
            sleeps += 1
        });

        // Clear the attribute so the tempdir can be removed.
        #[allow(clippy::permissions_set_readonly_false)]
        perms.set_readonly(false);
        fs::set_permissions(&p, perms).unwrap();

        match r {
            Err(e @ WriteError::ReadOnly { .. }) => assert!(e.to_string().contains("read-only")),
            other => panic!("{other:?}"),
        }
        assert_eq!(sleeps, 0, "a read-only target is not retried");
        assert_eq!(fs::read(&p).unwrap(), b"A");
    }

    // ---- write_all_or_none ----

    fn two(t: &Path) -> (PathBuf, PathBuf) {
        let a = t.join("a.json");
        let b = t.join("b.json");
        fs::write(&a, "A0").unwrap();
        fs::write(&b, "B0").unwrap();
        (a, b)
    }

    /// Persist that fails (non-retriably) on every call whose 1-based number is in `fail_on`.
    fn failing_on(
        fail_on: Vec<u32>,
    ) -> impl FnMut(tempfile::NamedTempFile, &Path) -> Result<fs::File, tempfile::PersistError>
    {
        let mut calls = 0;
        move |file, target| {
            calls += 1;
            if fail_on.contains(&calls) {
                Err(tempfile::PersistError {
                    error: io::Error::from_raw_os_error(2),
                    file,
                })
            } else {
                file.persist(target)
            }
        }
    }

    #[test]
    fn all_or_none_writes_both() {
        let t = tempfile::tempdir().unwrap();
        let (a, b) = two(t.path());
        let r = write_all_or_none(&[
            Target {
                path: &a,
                loaded: Some(b"A0"),
                new: b"A1",
            },
            Target {
                path: &b,
                loaded: Some(b"B0"),
                new: b"B1",
            },
        ])
        .unwrap();
        assert_eq!(r, vec![WriteOutcome::Written { backup: None }; 2]);
        assert_eq!(fs::read(&a).unwrap(), b"A1");
        assert_eq!(fs::read(&b).unwrap(), b"B1");
        assert_eq!(names(t.path()), vec!["a.json", "b.json"]);
    }

    #[test]
    fn guard_mismatch_writes_nothing() {
        for bad_second in [true, false] {
            let t = tempfile::tempdir().unwrap();
            let (a, b) = two(t.path());
            let (la, lb): (&[u8], &[u8]) = if bad_second {
                (b"A0", b"stale")
            } else {
                (b"stale", b"B0")
            };
            let r = write_all_or_none(&[
                Target {
                    path: &a,
                    loaded: Some(la),
                    new: b"A1",
                },
                Target {
                    path: &b,
                    loaded: Some(lb),
                    new: b"B1",
                },
            ]);
            let e = r.unwrap_err();
            assert!(matches!(
                e,
                AllOrNoneError::NothingWritten(WriteError::ChangedOnDisk { .. })
            ));
            let bad = if bad_second { &b } else { &a };
            assert_eq!(
                e.changed_on_disk_path(),
                Some(bad.display().to_string().as_str())
            );
            assert_eq!(fs::read(&a).unwrap(), b"A0");
            assert_eq!(fs::read(&b).unwrap(), b"B0");
            assert_eq!(names(t.path()), vec!["a.json", "b.json"]);
        }
    }

    #[test]
    fn unchanged_target_is_skipped() {
        let t = tempfile::tempdir().unwrap();
        let (a, b) = two(t.path());
        let before = fs::metadata(&a).unwrap().modified().unwrap();
        let r = write_all_or_none(&[
            Target {
                path: &a,
                loaded: Some(b"A0"),
                new: b"A0",
            },
            Target {
                path: &b,
                loaded: Some(b"B0"),
                new: b"B1",
            },
        ])
        .unwrap();
        assert_eq!(
            r,
            vec![
                WriteOutcome::Unchanged,
                WriteOutcome::Written { backup: None }
            ]
        );
        assert_eq!(fs::metadata(&a).unwrap().modified().unwrap(), before);
        assert_eq!(fs::read(&b).unwrap(), b"B1");

        let r = write_all_or_none(&[
            Target {
                path: &a,
                loaded: Some(b"A0"),
                new: b"A0",
            },
            Target {
                path: &b,
                loaded: Some(b"B1"),
                new: b"B1",
            },
        ])
        .unwrap();
        assert_eq!(r, vec![WriteOutcome::Unchanged; 2]);
    }

    #[test]
    fn missing_directory_writes_nothing() {
        let t = tempfile::tempdir().unwrap();
        let (a, _) = two(t.path());
        let b = t.path().join("sub").join("b.json");
        let r = write_all_or_none(&[
            Target {
                path: &a,
                loaded: Some(b"A0"),
                new: b"A1",
            },
            Target {
                path: &b,
                loaded: None,
                new: b"B1",
            },
        ]);
        assert!(matches!(
            r,
            Err(AllOrNoneError::NothingWritten(
                WriteError::DestinationMissing { .. }
            ))
        ));
        assert_eq!(fs::read(&a).unwrap(), b"A0");
    }

    #[test]
    fn read_only_second_target_writes_nothing() {
        let t = tempfile::tempdir().unwrap();
        let (a, b) = two(t.path());
        let mut perms = fs::metadata(&b).unwrap().permissions();
        perms.set_readonly(true);
        fs::set_permissions(&b, perms.clone()).unwrap();
        let r = write_all_or_none(&[
            Target {
                path: &a,
                loaded: Some(b"A0"),
                new: b"A1",
            },
            Target {
                path: &b,
                loaded: Some(b"B0"),
                new: b"B1",
            },
        ]);
        #[allow(clippy::permissions_set_readonly_false)]
        perms.set_readonly(false);
        fs::set_permissions(&b, perms).unwrap();
        assert!(
            matches!(
                r,
                Err(AllOrNoneError::NothingWritten(WriteError::ReadOnly { .. }))
            ),
            "{r:?}"
        );
        assert_eq!(fs::read(&a).unwrap(), b"A0");
        assert_eq!(names(t.path()), vec!["a.json", "b.json"]);
    }

    #[test]
    fn second_persist_fails_restores_first() {
        let t = tempfile::tempdir().unwrap();
        let (a, b) = two(t.path());
        let r = write_all_or_none_with(
            &[
                Target {
                    path: &a,
                    loaded: Some(b"A0"),
                    new: b"A1",
                },
                Target {
                    path: &b,
                    loaded: Some(b"B0"),
                    new: b"B1",
                },
            ],
            failing_on(vec![2]),
            |_| {},
        );
        let e = r.unwrap_err();
        assert!(matches!(e, AllOrNoneError::RolledBack { .. }));
        assert!(e.to_string().contains("restored"));
        assert_eq!(fs::read(&a).unwrap(), b"A0");
        assert_eq!(fs::read(&b).unwrap(), b"B0");
        assert_eq!(names(t.path()), vec!["a.json", "b.json"]);
    }

    #[test]
    fn three_targets_third_persist_fails_restores_first_two() {
        let t = tempfile::tempdir().unwrap();
        let (a, b) = two(t.path());
        let c = t.path().join("c.json");
        fs::write(&c, "C0").unwrap();
        let r = write_all_or_none_with(
            &[
                Target {
                    path: &a,
                    loaded: Some(b"A0"),
                    new: b"A1",
                },
                Target {
                    path: &b,
                    loaded: Some(b"B0"),
                    new: b"B1",
                },
                Target {
                    path: &c,
                    loaded: Some(b"C0"),
                    new: b"C1",
                },
            ],
            failing_on(vec![3]),
            |_| {},
        );
        assert!(matches!(r, Err(AllOrNoneError::RolledBack { .. })), "{r:?}");
        assert_eq!(fs::read(&a).unwrap(), b"A0");
        assert_eq!(fs::read(&b).unwrap(), b"B0");
        assert_eq!(fs::read(&c).unwrap(), b"C0");
        assert_eq!(names(t.path()), vec!["a.json", "b.json", "c.json"]);
    }

    #[test]
    fn ensure_backup_once() {
        let t = tempfile::tempdir().unwrap();
        let p = t.path().join("x.filter");
        fs::write(&p, "orig").unwrap();
        let bp = ensure_backup(&p, b"orig").unwrap();
        assert_eq!(bp, Some(backup_path(&p)));
        assert_eq!(fs::read(backup_path(&p)).unwrap(), b"orig");
        assert_eq!(ensure_backup(&p, b"later").unwrap(), None);
        assert_eq!(fs::read(backup_path(&p)).unwrap(), b"orig");
    }

    #[test]
    fn rollback_failure_names_the_unrestored_file() {
        let t = tempfile::tempdir().unwrap();
        let (a, b) = two(t.path());
        let r = write_all_or_none_with(
            &[
                Target {
                    path: &a,
                    loaded: Some(b"A0"),
                    new: b"A1",
                },
                Target {
                    path: &b,
                    loaded: Some(b"B0"),
                    new: b"B1",
                },
            ],
            failing_on(vec![2, 3]),
            |_| {},
        );
        match r {
            Err(e @ AllOrNoneError::RollbackFailed { .. }) => {
                let text = e.to_string();
                assert!(text.contains("a.json"), "{text}");
                assert!(text.contains("could not be restored"), "{text}");
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn rollback_deletes_a_file_that_did_not_exist() {
        let t = tempfile::tempdir().unwrap();
        let a = t.path().join("a.json");
        let b = t.path().join("b.json");
        fs::write(&b, "B0").unwrap();
        let r = write_all_or_none_with(
            &[
                Target {
                    path: &a,
                    loaded: None,
                    new: b"A1",
                },
                Target {
                    path: &b,
                    loaded: Some(b"B0"),
                    new: b"B1",
                },
            ],
            failing_on(vec![2]),
            |_| {},
        );
        assert!(matches!(r, Err(AllOrNoneError::RolledBack { .. })));
        assert!(!a.exists());
        assert_eq!(fs::read(&b).unwrap(), b"B0");
    }

    #[test]
    fn non_retriable_error_stops_after_one_attempt() {
        let t = tempfile::tempdir().unwrap();
        let p = t.path().join("x.filter");
        let mut sleeps = 0;
        let r = write_guarded_with(&p, None, b"B", failing_persist(2, u32::MAX), |_| {
            sleeps += 1
        });
        assert!(matches!(r, Err(WriteError::Unwritable { .. })), "{r:?}");
        assert_eq!(sleeps, 0);
        assert!(names(t.path()).is_empty());
    }
}
