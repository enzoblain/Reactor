//! Non-blocking file handle backed by the reactor.
//!
//! `File` mirrors the networking primitives by exposing async read/write helpers
//! that integrate with the runtime's reactor. File descriptors are opened in
//! non-blocking mode so operations can yield when the OS reports `EAGAIN` or
//! `EWOULDBLOCK`.

use crate::reactor::core::ReactorHandle;
use crate::reactor::event::Event;
use crate::reactor::future::{ReadFuture, WriteFuture};
use crate::runtime::context::current_reactor_fs;

use libc::{O_CREAT, O_RDONLY, O_TRUNC, O_WRONLY, close, open};
use std::ffi::CString;
use std::io;

/// A non-blocking file backed by the reactor.
///
/// Files are opened in non-blocking mode and can be read or written using
/// futures that register with the reactor when the OS reports `EAGAIN` or
/// `EWOULDBLOCK`.
pub struct File {
    file_descriptor: i32,
    reactor: ReactorHandle,
}

impl File {
    /// Opens a file for reading.
    ///
    /// Equivalent to `open(path, O_RDONLY)`.
    pub async fn open(path: &str) -> io::Result<Self> {
        Self::open_with_flags(path, O_RDONLY).await
    }

    /// Creates or truncates a file for writing.
    ///
    /// Equivalent to `open(path, O_CREAT | O_WRONLY | O_TRUNC)` with mode 0o644.
    pub async fn create(path: &str) -> io::Result<Self> {
        Self::open_with_flags(path, O_CREAT | O_WRONLY | O_TRUNC).await
    }

    /// Opens a file with custom flags.
    ///
    /// This helper uses the current runtime's reactor. See
    /// [`open_with_reactor`](Self::open_with_reactor) when a specific reactor
    /// handle is needed.
    pub async fn open_with_flags(path: &str, flags: i32) -> io::Result<Self> {
        Self::open_with_reactor(path, flags, current_reactor_fs()).await
    }

    /// Opens a file with custom flags and an explicit reactor handle.
    pub async fn open_with_reactor(
        path: &str,
        flags: i32,
        reactor: ReactorHandle,
    ) -> io::Result<Self> {
        let file_descriptor = open_fd(path, flags)?;
        Event::set_nonblocking(file_descriptor);

        Ok(Self {
            file_descriptor,
            reactor,
        })
    }

    /// Reads data into the provided buffer.
    pub fn read<'a>(&'a self, buffer: &'a mut [u8]) -> ReadFuture<'a> {
        ReadFuture::new(self.file_descriptor, buffer, self.reactor.clone())
    }

    /// Writes data from the provided buffer.
    pub fn write<'a>(&'a self, buffer: &'a [u8]) -> WriteFuture<'a> {
        WriteFuture::new(self.file_descriptor, buffer, self.reactor.clone())
    }

    /// Writes the entire buffer to the file, retrying until complete.
    pub async fn write_all(&self, mut buffer: &[u8]) -> io::Result<()> {
        while !buffer.is_empty() {
            let written = self.write(buffer).await?;

            if written == 0 {
                return Err(io::Error::new(
                    io::ErrorKind::WriteZero,
                    "write returned zero bytes",
                ));
            }

            buffer = &buffer[written..];
        }

        Ok(())
    }
}

impl Drop for File {
    fn drop(&mut self) {
        unsafe {
            close(self.file_descriptor);
        }
    }
}

fn open_fd(path: &str, flags: i32) -> io::Result<i32> {
    let c_path = CString::new(path)
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "path contains null byte"))?;

    let file_descriptor = unsafe {
        if flags & O_CREAT != 0 {
            open(c_path.as_ptr(), flags, 0o644)
        } else {
            open(c_path.as_ptr(), flags)
        }
    };

    if file_descriptor < 0 {
        return Err(io::Error::last_os_error());
    }

    Ok(file_descriptor)
}
