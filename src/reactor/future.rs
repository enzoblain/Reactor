//! Asynchronous read and write futures for non-blocking file descriptors.
//!
//! These futures register the caller's waker with the reactor when the
//! operating system reports `EAGAIN` or `EWOULDBLOCK`. Once the descriptor is
//! ready again, the reactor wakes the task so the I/O operation can resume.
//!
//! # Examples
//!
//! Reading from a TCP stream:
//!
//! ```no_run
//! use reactor::net::tcp_stream::TcpStream;
//!
//! # async fn read_from_stream(stream: TcpStream) -> std::io::Result<usize> {
//! let mut buffer = [0u8; 1024];
//! let bytes_read = stream.read(&mut buffer).await?;
//!
//! Ok(bytes_read)
//! # }
//! ```
//!
//! Writing to a file descriptor obtained through the runtime:
//!
//! ```no_run
//! use reactor::fs::file::File;
//!
//! # async fn write_message(file: File) -> std::io::Result<()> {
//! file.write(b"hello world").await?;
//!
//! Ok(())
//! # }
//! ```

use crate::reactor::core::ReactorHandle;

use std::future::Future;
use std::io;
use std::pin::Pin;
use std::task::{Context, Poll};

use libc::{EAGAIN, EWOULDBLOCK, read, write};

/// Future that performs an asynchronous read on a non-blocking file descriptor.
///
/// Instances of this type are created by helpers such as
/// [File::read](crate::fs::file::File::read) and
/// [TcpStream::read](crate::net::tcp_stream::TcpStream::read). The future
/// resolves to the number of bytes read, returning `Ok(0)` when the descriptor
/// reaches end of stream.
///
/// # Examples
///
/// ```no_run
/// use reactor::net::tcp_stream::TcpStream;
///
/// # async fn receive(stream: TcpStream) -> std::io::Result<usize> {
/// let mut buffer = [0u8; 1024];
/// let read = stream.read(&mut buffer).await?;
///
/// Ok(read)
/// # }
/// ```
pub struct ReadFuture<'a> {
    file_descriptor: i32,
    buffer: &'a mut [u8],
    reactor: ReactorHandle,
    registered: bool,
}

impl<'a> ReadFuture<'a> {
    pub(crate) fn new(file_descriptor: i32, buffer: &'a mut [u8], reactor: ReactorHandle) -> Self {
        Self {
            file_descriptor,
            buffer,
            reactor,
            registered: false,
        }
    }
}

impl<'a> Future for ReadFuture<'a> {
    type Output = io::Result<usize>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.as_mut().get_mut();

        let result = unsafe {
            read(
                this.file_descriptor,
                this.buffer.as_mut_ptr() as *mut _,
                this.buffer.len(),
            )
        };

        if result > 0 {
            return Poll::Ready(Ok(result as usize));
        }

        if result == 0 {
            return Poll::Ready(Ok(0));
        }

        let error = unsafe { *libc::__error() };

        if error == EAGAIN || error == EWOULDBLOCK {
            if !this.registered {
                this.reactor
                    .lock()
                    .unwrap()
                    .register_read(this.file_descriptor, cx.waker().clone());
                this.registered = true;
            }

            return Poll::Pending;
        }

        Poll::Ready(Err(io::Error::last_os_error()))
    }
}

/// Future that performs an asynchronous write on a non-blocking file descriptor.
///
/// Instances of this type are produced by helpers such as
/// [File::write](crate::fs::file::File::write) and
/// [TcpStream::write](crate::net::tcp_stream::TcpStream::write). The future
/// resolves to the number of bytes successfully written.
///
/// # Examples
///
/// ```no_run
/// use reactor::fs::file::File;
///
/// # async fn send(file: File) -> std::io::Result<usize> {
/// let bytes = file.write(b"data").await?;
///
/// Ok(bytes)
/// # }
/// ```
pub struct WriteFuture<'a> {
    file_descriptor: i32,
    buffer: &'a [u8],
    reactor: ReactorHandle,
    registered: bool,
}

impl<'a> WriteFuture<'a> {
    pub(crate) fn new(file_descriptor: i32, buffer: &'a [u8], reactor: ReactorHandle) -> Self {
        Self {
            file_descriptor,
            buffer,
            reactor,
            registered: false,
        }
    }
}

impl<'a> Future for WriteFuture<'a> {
    type Output = io::Result<usize>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.as_mut().get_mut();

        let result = unsafe {
            write(
                this.file_descriptor,
                this.buffer.as_ptr() as *const _,
                this.buffer.len(),
            )
        };

        if result >= 0 {
            return Poll::Ready(Ok(result as usize));
        }

        let error = unsafe { *libc::__error() };

        if error == EAGAIN || error == EWOULDBLOCK {
            if !this.registered {
                this.reactor
                    .lock()
                    .unwrap()
                    .register_write(this.file_descriptor, cx.waker().clone());
                this.registered = true;
            }

            return Poll::Pending;
        }

        Poll::Ready(Err(io::Error::last_os_error()))
    }
}
