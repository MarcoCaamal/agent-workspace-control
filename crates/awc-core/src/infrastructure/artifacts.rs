//! Injectable filesystem operations for artifact lifecycle commands.
//!
//! Every mutation follows the compensate-on-failure pattern: write temp,
//! validate, rename/move atomically, and undo on error. The trait allows
//! tests to inject failures at any step without touching real paths.

use std::path::{Path, PathBuf};

use crate::error::AwcError;

/// Filesystem primitives for artifact lifecycle. Implementations must be
/// safe against partial failure: `create_temp` + `rename` or `move_to_trash`
/// + `move_back` form a compensated pair.
pub trait ArtifactFs: Send + Sync {
    /// Creates an empty temporary file in the same directory as `target`.
    /// Returns the temp path; the caller must clean it up on failure.
    fn create_temp(&self, target: &Path) -> Result<PathBuf, AwcError>;

    /// Atomically renames `from` to `to`. Fails if `to` already exists.
    fn rename(&self, from: &Path, to: &Path) -> Result<(), AwcError>;

    /// Moves `from` to `to` (cross-directory). Fails if `to` exists.
    fn move_file(&self, from: &Path, to: &Path) -> Result<(), AwcError>;

    /// Removes a file. Used for temp cleanup after successful rename.
    fn remove_file(&self, path: &Path) -> Result<(), AwcError>;

    /// Checks whether a path exists on disk.
    fn exists(&self, path: &Path) -> bool;
}

/// Production filesystem implementation using `std::fs`.
pub struct OsFs;

impl ArtifactFs for OsFs {
    fn create_temp(&self, target: &Path) -> Result<PathBuf, AwcError> {
        let parent = target.parent().unwrap_or(target);
        let name = format!(
            ".awc-tmp-{}-{}",
            std::process::id(),
            target.file_name().unwrap_or_default().to_string_lossy()
        );
        let temp = parent.join(name);
        std::fs::File::create(&temp)?;
        Ok(temp)
    }

    fn rename(&self, from: &Path, to: &Path) -> Result<(), AwcError> {
        if self.exists(to) {
            return Err(AwcError::PathOwned(to.display().to_string()));
        }
        std::fs::rename(from, to)?;
        Ok(())
    }

    fn move_file(&self, from: &Path, to: &Path) -> Result<(), AwcError> {
        if self.exists(to) {
            return Err(AwcError::PathOwned(to.display().to_string()));
        }
        std::fs::rename(from, to)?;
        Ok(())
    }

    fn remove_file(&self, path: &Path) -> Result<(), AwcError> {
        std::fs::remove_file(path)?;
        Ok(())
    }

    fn exists(&self, path: &Path) -> bool {
        path.exists()
    }
}

#[cfg(test)]
pub mod test_support {
    use super::*;
    use std::collections::HashSet;
    use std::sync::Mutex;

    /// Fails at the named step. Used to verify compensation behavior.
    pub struct FailingFs {
        pub fail_at: Mutex<HashSet<String>>,
    }

    impl Default for FailingFs {
        fn default() -> Self {
            Self::new()
        }
    }

    impl FailingFs {
        pub fn new() -> Self {
            FailingFs {
                fail_at: Mutex::new(HashSet::new()),
            }
        }

        pub fn fail_on(&self, step: &str) {
            self.fail_at.lock().unwrap().insert(step.to_string());
        }

        fn check(&self, step: &str) -> Result<(), AwcError> {
            if self.fail_at.lock().unwrap().contains(step) {
                return Err(AwcError::Io(std::io::Error::other(format!(
                    "injected failure at {step}"
                ))));
            }
            Ok(())
        }
    }

    impl ArtifactFs for FailingFs {
        fn create_temp(&self, target: &Path) -> Result<PathBuf, AwcError> {
            self.check("create_temp")?;
            let temp = target.with_extension("tmp");
            std::fs::File::create(&temp)?;
            Ok(temp)
        }

        fn rename(&self, from: &Path, to: &Path) -> Result<(), AwcError> {
            self.check("rename")?;
            std::fs::rename(from, to)?;
            Ok(())
        }

        fn move_file(&self, from: &Path, to: &Path) -> Result<(), AwcError> {
            self.check("move_file")?;
            std::fs::rename(from, to)?;
            Ok(())
        }

        fn remove_file(&self, path: &Path) -> Result<(), AwcError> {
            self.check("remove_file")?;
            std::fs::remove_file(path)?;
            Ok(())
        }

        fn exists(&self, path: &Path) -> bool {
            path.exists()
        }
    }
}
