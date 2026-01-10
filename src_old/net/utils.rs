use libc::{AF_INET, in_addr, sockaddr_in};

use std::io;
use std::mem;
use std::net::SocketAddr;

pub(crate) fn parse_sockaddr(address: &str) -> io::Result<sockaddr_in> {
    let (ip_string, port_string) = address
        .rsplit_once(':')
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "invalid address"))?;

    let port: u16 = port_string
        .parse()
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "invalid port"))?;

    let mut octets = [0u8; 4];
    let parts: Vec<&str> = ip_string.split('.').collect();

    if parts.len() != 4 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "invalid IPv4 address",
        ));
    }

    for (index, part) in parts.iter().enumerate() {
        octets[index] = part
            .parse::<u8>()
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "invalid IPv4 octet"))?;
    }

    let ip_u32 = (octets[0] as u32) << 24
        | (octets[1] as u32) << 16
        | (octets[2] as u32) << 8
        | (octets[3] as u32);

    Ok(sockaddr_in {
        sin_len: mem::size_of::<sockaddr_in>() as u8,
        sin_family: AF_INET as u8,
        sin_port: port.to_be(),
        sin_addr: in_addr {
            s_addr: ip_u32.to_be(),
        },
        sin_zero: [0; 8],
    })
}

pub(crate) fn sockaddr_to_socketaddr(address: &sockaddr_in) -> SocketAddr {
    let ip_u32 = u32::from_be(address.sin_addr.s_addr);
    let octets = [
        (ip_u32 >> 24) as u8,
        (ip_u32 >> 16) as u8,
        (ip_u32 >> 8) as u8,
        ip_u32 as u8,
    ];
    let port = u16::from_be(address.sin_port);

    SocketAddr::from((octets, port))
}
