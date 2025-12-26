//! TCP networking primitives.
//!
//! This module provides non-blocking TCP networking primitives for async I/O:
//! - [`tcp_listener`]: [`TcpListener`] for accepting connections
//! - [`tcp_stream`]: [`TcpStream`] for reading/writing data
//! - [`future`]: Accept future for listeners plus read/write futures re-exported from the reactor
//! - [`utils`]: Address parsing and conversion utilities
//!
//! # Example
//!
//! ```ignore
//! use reactor::net::tcp_listener::TcpListener;
//!
//! async fn server() {
//!     let listener = TcpListener::bind("127.0.0.1:8080").await.unwrap();
//!     loop {
//!         let (stream, addr) = listener.accept().await.unwrap();
//!         println!("New connection from {}", addr);
//!     }
//! }
//! ```
//!
//! [`TcpListener`]: tcp_listener::TcpListener
//! [`TcpStream`]: tcp_stream::TcpStream

pub mod future;
pub mod tcp_listener;
pub mod tcp_stream;
pub mod utils;
