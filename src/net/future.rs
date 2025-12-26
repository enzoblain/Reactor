//! Async TCP futures for accepting connections.
//!
//! This module provides low-level futures for TCP operations:
//! - [`AcceptFuture`]: Accepts a new client connection
//! - [`ReadFuture`] and [`WriteFuture`]: Re-exported generic fd futures
//!
//! These futures integrate with the reactor to provide non-blocking I/O.

use crate::net::utils::sockaddr_to_socketaddr;
use crate::reactor::core::ReactorHandle;
use crate::reactor::event::Event;

use libc::{EAGAIN, EWOULDBLOCK, accept, sockaddr, sockaddr_in, socklen_t};
use std::future::Future;
use std::io;
use std::mem;
use std::net::SocketAddr;
use std::pin::Pin;
use std::task::{Context, Poll};

/// Re-export generic read future for convenience.
pub use crate::reactor::future::ReadFuture;

/// Re-export generic write future for convenience.
pub use crate::reactor::future::WriteFuture;

/// A future that accepts a new client connection.
///
/// On `EAGAIN`/`EWOULDBLOCK`, it registers the listening socket for read
/// readiness and yields until the reactor wakes it.
pub struct AcceptFuture {
    listen_file_descriptor: i32,
    reactor: ReactorHandle,
    registered: bool,
}

impl AcceptFuture {
    pub(crate) fn new(listen_file_descriptor: i32, reactor: ReactorHandle) -> Self {
        Self {
            listen_file_descriptor,
            reactor,
            registered: false,
        }
    }
}

impl Future for AcceptFuture {
    type Output = io::Result<(i32, SocketAddr)>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut addr: sockaddr_in = unsafe { mem::zeroed() };
        let mut addr_len: socklen_t = mem::size_of::<sockaddr_in>() as socklen_t;

        let client_fd = unsafe {
            accept(
                self.listen_file_descriptor,
                &mut addr as *mut _ as *mut sockaddr,
                &mut addr_len,
            )
        };

        if client_fd >= 0 {
            Event::set_nonblocking(client_fd);
            let socket_addr = sockaddr_to_socketaddr(&addr);
            return Poll::Ready(Ok((client_fd, socket_addr)));
        }

        let error = unsafe { *libc::__error() };

        if error == EAGAIN || error == EWOULDBLOCK {
            if !self.registered {
                self.reactor
                    .lock()
                    .unwrap()
                    .register_read(self.listen_file_descriptor, cx.waker().clone());
                self.registered = true;
            }

            return Poll::Pending;
        }

        Poll::Ready(Err(io::Error::last_os_error()))
    }
}
