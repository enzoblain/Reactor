//! Async file and directory primitives.
//!
//! Non-blocking file operations backed by the reactor and simple async directory
//! creation helpers. Files are opened in non-blocking mode and use the runtime's
//! reactor to coordinate readiness.
//!
//! Public API:
//! - [`File`]: Main handle for async file I/O
//! - [`Folder`]: OOP-style directory API with helpers like [`Folder::create`]
//! - \[`create_dir`], \[`create_dir_all`]: Function-style helpers for convenience

pub mod file;
pub mod folder;

pub use file::File;
pub use folder::Folder;
