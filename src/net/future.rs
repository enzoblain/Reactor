use crate::net::utils::sockaddr_to_socketaddr;
use crate::reactor::core::with_current_reactor;
use crate::reactor::event::Event;

use libc::{EAGAIN, EWOULDBLOCK, accept, read, sockaddr, sockaddr_in, socklen_t, write};
use std::future::Future;
use std::io;
use std::mem;
use std::net::SocketAddr;
use std::pin::Pin;
use std::task::{Context, Poll};

pub struct AcceptFuture {
    listen_fd: i32,
    registered: bool,
}

impl AcceptFuture {
    pub fn new(listen_fd: i32) -> Self {
        Self {
            listen_fd,
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
                self.listen_fd,
                &mut addr as *mut _ as *mut sockaddr,
                &mut addr_len,
            )
        };

        if client_fd >= 0 {
            Event::set_nonblocking(client_fd);
            let socket_addr = sockaddr_to_socketaddr(&addr);
            return Poll::Ready(Ok((client_fd, socket_addr)));
        }

        let err = unsafe { *libc::__error() };

        if err == EAGAIN || err == EWOULDBLOCK {
            if !self.registered {
                let _ = with_current_reactor(|r| {
                    r.register_read(self.listen_fd, cx.waker().clone());
                });
                self.registered = true;
            }
            return Poll::Pending;
        }

        Poll::Ready(Err(io::Error::last_os_error()))
    }
}

pub struct ReadFuture<'a> {
    fd: i32,
    buf: &'a mut [u8],
    registered: bool,
}

impl<'a> ReadFuture<'a> {
    pub fn new(fd: i32, buf: &'a mut [u8]) -> Self {
        Self {
            fd,
            buf,
            registered: false,
        }
    }
}

impl<'a> Future for ReadFuture<'a> {
    type Output = io::Result<usize>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = unsafe { self.as_mut().get_unchecked_mut() };

        let res = unsafe { read(this.fd, this.buf.as_mut_ptr() as *mut _, this.buf.len()) };

        if res > 0 {
            return Poll::Ready(Ok(res as usize));
        }

        if res == 0 {
            return Poll::Ready(Ok(0));
        }

        let err = unsafe { *libc::__error() };

        if err == EAGAIN || err == EWOULDBLOCK {
            if !this.registered {
                let _ = with_current_reactor(|r| {
                    r.register_read(this.fd, cx.waker().clone());
                });
                this.registered = true;
            }
            return Poll::Pending;
        }

        Poll::Ready(Err(io::Error::last_os_error()))
    }
}

pub struct WriteFuture<'a> {
    fd: i32,
    buf: &'a [u8],
    registered: bool,
}

impl<'a> WriteFuture<'a> {
    pub fn new(fd: i32, buf: &'a [u8]) -> Self {
        Self {
            fd,
            buf,
            registered: false,
        }
    }
}

impl<'a> Future for WriteFuture<'a> {
    type Output = io::Result<usize>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = unsafe { self.as_mut().get_unchecked_mut() };

        let res = unsafe { write(this.fd, this.buf.as_ptr() as *const _, this.buf.len()) };

        if res >= 0 {
            return Poll::Ready(Ok(res as usize));
        }

        let err = unsafe { *libc::__error() };

        if err == EAGAIN || err == EWOULDBLOCK {
            if !this.registered {
                let _ = with_current_reactor(|r| {
                    r.register_write(this.fd, cx.waker().clone());
                });
                this.registered = true;
            }
            return Poll::Pending;
        }

        Poll::Ready(Err(io::Error::last_os_error()))
    }
}
