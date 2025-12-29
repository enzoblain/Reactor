//! Event-driven I/O reactor module.
//!
//! This module provides the core event-driven I/O handling using kqueue on macOS.
//! It includes:
//! - [`core`]: The main reactor implementation
//! - [`event`]: kqueue event wrappers
//! - [`io`]: Connection state management
//! - [`future`]: Generic read/write futures for file descriptors
//! - [`socket`]: Socket acceptance utilities

pub mod core;
pub mod event;
pub mod future;
pub mod io;
pub mod socket;
