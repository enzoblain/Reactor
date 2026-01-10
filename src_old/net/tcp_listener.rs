use crate::net::future::AcceptFuture;
use crate::net::tcp_stream::TcpStream;
use crate::net::utils::sockaddr_to_socketaddr;
use crate::reactor::core::ReactorHandle;
use crate::reactor::event::Event;
use crate::runtime::context::current_reactor_io;

use libc::{AF_INET, SOCK_STREAM, bind, close, getsockname, listen, sockaddr, sockaddr_in, socket};
use std::io;
use std::mem;
use std::net::SocketAddr;

pub struct TcpListener {
    file_descriptor: i32,
    reactor: ReactorHandle,
}

impl TcpListener {
    pub async fn bind(address: &str) -> io::Result<Self> {
        Self::bind_with_reactor(address, current_reactor_io()).await
    }

    pub async fn bind_with_reactor(address: &str, reactor: ReactorHandle) -> io::Result<Self> {
        let addr = crate::net::utils::parse_sockaddr(address)?;
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

        Ok(Self {
            file_descriptor,
            reactor,
        })
    }

    pub async fn accept(&self) -> io::Result<(TcpStream, SocketAddr)> {
        let (file_descriptor, address) =
            AcceptFuture::new(self.file_descriptor, self.reactor.clone()).await?;

        Ok((
            TcpStream::new(file_descriptor, self.reactor.clone()),
            address,
        ))
    }

    pub fn local_addr(&self) -> io::Result<SocketAddr> {
        let mut addr: sockaddr_in = unsafe { mem::zeroed() };
        let mut length = mem::size_of::<sockaddr_in>() as u32;
        let result = unsafe {
            getsockname(
                self.file_descriptor,
                &mut addr as *mut _ as *mut sockaddr,
                &mut length,
            )
        };

        if result < 0 {
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
