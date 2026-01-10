use std::io;

use std::ffi::CString;
use std::path::{Component, Path, PathBuf};

use libc::{EEXIST, mkdir};

pub struct Folder {
    path: String,
}

impl Folder {
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

    pub fn path(&self) -> &str {
        &self.path
    }

    pub fn exists(&self) -> bool {
        let p = std::path::Path::new(&self.path);
        p.is_dir()
    }
}
