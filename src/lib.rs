//! Simple PID lock file management.
//!
//! This is a very basic library for creating and using PID files
//! to coordinate among processes.
//!
//! A PID file is a file that contains the PID of a process. It can be used
//! as a crude form of locking to prevent multiple instances of a process
//! from running at the same time, or to provide a lock for a resource which
//! should only be accessed by one process at a time.
//!
//! This library provides a simple API for creating and using PID files. PID
//! files are created at a given path, and are automatically removed when the
//! PID file object is dropped.
//!
//! # Example
//!
//! ```rust
//! use pidfile::PidFile;
//!
//! fn main() -> Result<(), Box<dyn std::error::Error>> {
//!    let pidfile = PidFile::new("/tmp/myapp.pid")?;
//!   // Do stuff
//!
//!   Ok(())
//! }
//! ```

use std::io;
use std::path::{Path, PathBuf};

/// A PID file is a file that contains the PID of a process. It is used to
/// prevent multiple instances of a process from running at the same time,
/// or to provide a lock for a resource which should only be accessed by one
/// process at a time.
#[derive(Debug)]
pub struct PidFile {
    path: PathBuf,
}

/// Check if a PID file is in use.
///
/// If the PID file corresponds to a currently unused PID, the file
/// will be removed by this function.
fn pid_file_in_use(path: &Path) -> Result<bool, io::Error> {
    match std::fs::read_to_string(path) {
        Ok(info) => {
            let pid: libc::pid_t = info.trim().parse().map_err(|error| {
                tracing::debug!(path=%path.display(), "Unable to parse PID file {path}: {error}", path = path.display());
                io::Error::new(io::ErrorKind::InvalidData, "expected a PID")
            })?;

            // SAFETY: I dunno? Libc is probably fine.
            #[allow(unsafe_code)]
            let errno = unsafe { libc::kill(pid, 0) };

            if errno == 0 {
                tracing::debug!(%pid, "PID {pid} is still running", pid = pid);
                // This PID still exists, so the pid file is valid.
                return Ok(true);
            }

            if errno == -1 {
                tracing::debug!(%pid, "Unkonwn error checking PID file: {errno}");
                return Ok(false);
            };

            let error = io::Error::from_raw_os_error(errno);
            match error.kind() {
                io::ErrorKind::NotFound => Ok(false),
                _ => Err(error),
            }
        }
        Err(error) => match error.kind() {
            io::ErrorKind::NotFound => Ok(false),
            _ => Err(error),
        },
    }
}

impl PidFile {
    /// Create a new PID file at the given path for this process.
    ///
    /// If the PID file already exists, this function will check if the
    /// PID file is still in use. If the PID file is in use, this function
    /// will return Err(io::ErrorKind::AddrInUse). If the PID file is not
    /// in use, it will be removed and a new PID file will be created.
    pub fn new(path: impl Into<PathBuf>) -> Result<Self, io::Error> {
        let path = path.into();
        if path.exists() {
            match pid_file_in_use(&path) {
                Ok(true) => {
                    tracing::error!(path=%path.display(), "PID File {path} is already in use", path = path.display());
                    return Err(io::Error::new(
                        io::ErrorKind::AddrInUse,
                        format!("PID File {path} is already in use", path = path.display()),
                    ));
                }
                Ok(false) => {
                    tracing::debug!(path=%path.display(), "Removing stale PID file at {path}", path = path.display());
                    let _ = std::fs::remove_file(&path);
                }
                Err(error) if error.kind() == io::ErrorKind::InvalidData => {
                    tracing::warn!(path=%path.display(), "Removing invalid PID file at {path}", path = path.display());
                    let _ = std::fs::remove_file(&path);
                }
                Err(error) => {
                    tracing::error!(path=%path.display(), "Unable to check PID file {path}: {error}", path = path.display());
                    return Err(error);
                }
            }
        }

        // SAFETY: What could go wrong?
        #[allow(unsafe_code)]
        let pid = unsafe { libc::getpid() };

        if pid <= 0 {
            tracing::error!("libc::getpid() returned a negative PID: {pid}");
            return Err(io::Error::new(io::ErrorKind::Other, "negative PID"));
        }

        std::fs::write(&path, format!("{}", pid))?;
        tracing::trace!(%pid, path=%path.display(), "Locked PID file at {path}", path = path.display());

        Ok(Self { path })
    }

    /// Check if a PID file is in use at this path.
    ///
    /// If this function returns an error, it indicates that either the PID file
    /// could not be accessed, or when accessed, it contained data which did not look like a PID.
    pub fn is_locked(path: &Path) -> Result<bool, io::Error> {
        match pid_file_in_use(path) {
            Ok(true) => Ok(true),
            Ok(false) => Ok(false),
            Err(error) if error.kind() == io::ErrorKind::InvalidData => {
                tracing::warn!(path=%path.display(), "Invalid PID file at {path}", path = path.display());
                Ok(false)
            }
            Err(error) => {
                tracing::error!(path=%path.display(), "Unable to check PID file {path}: {error}", path=path.display());
                Err(error)
            }
        }
    }
}

impl Drop for PidFile {
    fn drop(&mut self) {
        match std::fs::remove_file(&self.path) {
            Ok(_) => {}
            Err(error) => eprintln!(
                "Encountered an error removing the PID file at {}: {}",
                self.path.display(),
                error
            ),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_pid_file() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("pidfile-test.pid");
        let pid_file = PidFile::new(path.clone()).unwrap();
        assert!(PidFile::is_locked(&path).unwrap());
        drop(pid_file);
        assert!(!PidFile::is_locked(&path).unwrap());
    }

    #[test]
    fn test_invalid_file() {
        let path = Path::new("/tmp/pidfile-test.pid");
        std::fs::write(path, "not a pid").unwrap();
        tracing::subscriber::with_default(tracing::subscriber::NoSubscriber::new(), || {
            assert!(
                !PidFile::is_locked(path).unwrap(),
                "Invalid file should not be locked."
            )
        });
        assert!(
            path.exists(),
            "Invalid file should exist after checking for locks."
        );

        let pid_file =
            tracing::subscriber::with_default(tracing::subscriber::NoSubscriber::new(), || {
                PidFile::new(path).unwrap()
            });
        assert!(
            PidFile::is_locked(path).unwrap(),
            "PID file should be locked after creation."
        );
        drop(pid_file);
        assert!(
            !PidFile::is_locked(path).unwrap(),
            "PID file should not be locked after drop."
        );
    }
}
