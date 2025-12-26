use crate::net::{future::AcceptFuture, tcp_stream::TcpStream};

use super::utils::{parse_sockaddr, sockaddr_to_socketaddr};

use crate::reactor::event::Event;
use libc::{AF_INET, SOCK_STREAM, bind, close, getsockname, listen, sockaddr, sockaddr_in, socket};
use std::{io, mem, net::SocketAddr};

pub struct TcpListener {
    file_descriptor: i32,
}

impl TcpListener {
    pub async fn bind(addr: &str) -> io::Result<Self> {
        let addr = parse_sockaddr(addr)?;
        let file_descriptor = unsafe { socket(AF_INET, SOCK_STREAM, 0) };

        Event::set_nonblocking(file_descriptor);

        let ret = unsafe {
            bind(
                file_descriptor,
                &addr as *const _ as *const sockaddr,
                mem::size_of::<sockaddr_in>() as u32,
            )
        };

        if ret < 0 {
            return Err(io::Error::last_os_error());
        }

        let ret = unsafe { listen(file_descriptor, 128) };
        if ret < 0 {
            return Err(io::Error::last_os_error());
        }

        Ok(Self { file_descriptor })
    }

    pub async fn accept(&self) -> io::Result<(TcpStream, SocketAddr)> {
        let (file_descriptor, addr) = AcceptFuture::new(self.file_descriptor).await?;
        Ok((TcpStream::new(file_descriptor), addr))
    }

    pub fn local_addr(&self) -> io::Result<SocketAddr> {
        let mut addr: sockaddr_in = unsafe { mem::zeroed() };
        let mut len = mem::size_of::<sockaddr_in>() as u32;
        let res = unsafe {
            getsockname(
                self.file_descriptor,
                &mut addr as *mut _ as *mut sockaddr,
                &mut len,
            )
        };
        if res < 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(sockaddr_to_socketaddr(&addr))
    }
}

impl Drop for TcpListener {
    fn drop(&mut self) {
        unsafe {
            close(self.file_descriptor);
        }
    }
}
