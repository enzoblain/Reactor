use crate::reactor::core::ReactorHandle;

use std::future::Future;
use std::io;
use std::pin::Pin;
use std::task::{Context, Poll};

use libc::{EAGAIN, EWOULDBLOCK, read, write};

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
