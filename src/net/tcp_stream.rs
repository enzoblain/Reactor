use std::{io, task::Waker};

use crate::net::future::{ReadFuture, WriteFuture};
use libc::close;

pub struct TcpStream {
    file_descriptor: i32,

    read_waker: Option<Waker>,
    write_waker: Option<Waker>,
}

impl TcpStream {
    pub fn new(file_descriptor: i32) -> Self {
        Self {
            file_descriptor,
            read_waker: None,
            write_waker: None,
        }
    }
    pub fn read<'a>(&'a self, buf: &'a mut [u8]) -> ReadFuture<'a> {
        ReadFuture::new(self.file_descriptor, buf)
    }

    pub fn write<'a>(&'a self, buf: &'a [u8]) -> WriteFuture<'a> {
        WriteFuture::new(self.file_descriptor, buf)
    }

    pub async fn write_all(&self, mut buf: &[u8]) -> io::Result<()> {
        while !buf.is_empty() {
            let n = self.write(buf).await?;
            if n == 0 {
                return Err(io::Error::new(
                    io::ErrorKind::WriteZero,
                    "write returned zero bytes",
                ));
            }
            buf = &buf[n..];
        }
        Ok(())
    }
}

impl Drop for TcpStream {
    fn drop(&mut self) {
        unsafe {
            close(self.file_descriptor);
        }
    }
}
