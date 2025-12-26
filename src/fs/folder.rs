//! Object-oriented directory API.
//!
//! Provides the `Folder` type with ergonomic methods like [`Folder::create`]
//! and [`Folder::create_all`] for working with directories.
//!
//! # Examples
//!
//! Create a single directory and reuse its path:
//!
//! ```no_run
//! use reactor::fs::Folder;
//!
//! # async fn example() -> std::io::Result<()> {
//! let folder = Folder::create("/tmp/reactor-demo").await?;
//! println!("Created: {}", folder.path());
//!
//! Ok(())
//! # }
//! ```
//!
//! Recursively create nested directories (mkdir -p style):
//!
//! ```no_run
//! use reactor::fs::Folder;
//!
//! # async fn example() -> std::io::Result<()> {
//! let folder = Folder::create_all("/tmp/reactor-demo/a/b/c").await?;
//! assert!(folder.path().ends_with("a/b/c"));
//!
//! Ok(())
//! # }
//! ```

use std::io;

use std::ffi::CString;
use std::path::{Component, Path, PathBuf};

use libc::{EEXIST, mkdir};

/// A directory handle with helper methods.
///
/// `Folder` encapsulates a directory path and provides constructor-style
/// helpers to create the directory on the filesystem.
pub struct Folder {
    path: String,
}

impl Folder {
    /// Creates a single directory and returns a `Folder` for it.
    ///
    /// Fails if the directory already exists or if any parent component
    /// is missing. For recursive creation, use [`Folder::create_all`].
    pub async fn create(path: &str) -> io::Result<Self> {
        let c_path = CString::new(path)
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "path contains null byte"))?;

        let result = unsafe { mkdir(c_path.as_ptr(), 0o755) };

        if result < 0 {
            return Err(io::Error::last_os_error());
        }

        Ok(Self {
            path: path.to_string(),
        })
    }

    /// Recursively creates directories along `path` and returns a `Folder` for it.
    ///
    /// Similar to `mkdir -p`: creates intermediate directories as needed and
    /// does not error if a directory already exists.
    pub async fn create_all(path: &str) -> io::Result<Self> {
        let target = Path::new(path);

        if target.as_os_str().is_empty() {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, "empty path"));
        }

        let mut acc = PathBuf::new();

        if target.is_absolute() {
            acc.push(Path::new("/"));
        }

        for component in target.components() {
            match component {
                Component::RootDir => {}
                Component::CurDir => {}
                Component::ParentDir => {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "parent directory (..) not supported",
                    ));
                }
                Component::Normal(seg) => {
                    acc.push(seg);

                    let c_path = CString::new(acc.as_os_str().to_string_lossy().into_owned())
                        .map_err(|_| {
                            io::Error::new(io::ErrorKind::InvalidInput, "path contains null byte")
                        })?;

                    let result = unsafe { mkdir(c_path.as_ptr(), 0o755) };

                    if result < 0 {
                        let err = io::Error::last_os_error();
                        match err.raw_os_error() {
                            Some(code) if code == EEXIST => {}
                            _ => {
                                return Err(err);
                            }
                        }
                    }
                }
                _ => {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "unsupported path component",
                    ));
                }
            }
        }

        Ok(Self {
            path: path.to_string(),
        })
    }

    /// Returns the folder's path as a string slice.
    pub fn path(&self) -> &str {
        &self.path
    }

    /// Returns whether this folder currently exists on disk and is a directory.
    ///
    /// This checks the underlying filesystem for the presence of a directory at
    /// this `Folder`'s path. If the directory has been deleted after creation,
    /// this will return `false`.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use reactor::fs::Folder;
    ///
    /// # async fn example() -> std::io::Result<()> {
    /// let f = Folder::create("/tmp/reactor-exists").await?;
    /// assert!(f.exists());
    ///
    /// std::fs::remove_dir(f.path()).unwrap();
    /// assert!(!f.exists());
    ///
    /// Ok(())
    /// # }
    /// ```
    pub fn exists(&self) -> bool {
        let p = std::path::Path::new(&self.path);
        p.is_dir()
    }
}
