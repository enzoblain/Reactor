use crate::reactor::core::with_current_reactor;
use std::future::Future;
use std::io;
use std::pin::Pin;
use std::task::{Context, Poll};

pub struct AsyncRead {
    fd: i32,
    registered: bool,
}

impl AsyncRead {
    pub fn new(fd: i32) -> Self {
        Self {
            fd,
            registered: false,
        }
    }
}

impl Future for AsyncRead {
    type Output = io::Result<()>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if !self.registered {
            let _ = with_current_reactor(|r| {
                r.register_read(self.fd, cx.waker().clone());
            });
            self.registered = true;
            return Poll::Pending;
        }

        // On subsequent polls (after wake), consider ready
        Poll::Ready(Ok(()))
    }
}

pub struct AsyncWrite {
    fd: i32,
    registered: bool,
}

impl AsyncWrite {
    pub fn new(fd: i32) -> Self {
        Self {
            fd,
            registered: false,
        }
    }
}

impl Future for AsyncWrite {
    type Output = io::Result<()>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if !self.registered {
            let _ = with_current_reactor(|r| {
                r.register_write(self.fd, cx.waker().clone());
            });
            self.registered = true;
            return Poll::Pending;
        }

        // On subsequent polls (after wake), consider ready
        Poll::Ready(Ok(()))
    }
}
