use crate::reactor::core::ReactorHandle;
use crate::reactor::future::{ReadFuture, WriteFuture};

use libc::close;
use std::io;

pub struct TcpStream {
    file_descriptor: i32,
    reactor: ReactorHandle,
}

impl TcpStream {
    pub fn new(file_descriptor: i32, reactor: ReactorHandle) -> Self {
        Self {
            file_descriptor,
            reactor,
        }
    }

    pub fn read<'a>(&'a self, buffer: &'a mut [u8]) -> ReadFuture<'a> {
        ReadFuture::new(self.file_descriptor, buffer, self.reactor.clone())
    }

    pub fn write<'a>(&'a self, buffer: &'a [u8]) -> WriteFuture<'a> {
        WriteFuture::new(self.file_descriptor, buffer, self.reactor.clone())
    }

    pub async fn write_all(&self, mut buffer: &[u8]) -> io::Result<()> {
        while !buffer.is_empty() {
            let n = self.write(buffer).await?;
            if n == 0 {
                return Err(io::Error::new(
                    io::ErrorKind::WriteZero,
                    "write returned zero bytes",
                ));
            }
            buffer = &buffer[n..];
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
